//! Cooperative single-asset download task. No background worker or sleep.
//! The caller polls step, observes retry deadlines and pauses between requests.
use crate::{
    catalog::Entry,
    download::{Downloader, Error},
    resume::{PartialDownload, Progress},
};
use std::time::{Duration, Instant};

pub const MAX_RETRIES: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Ready,
    Paused,
    RetryWait,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Snapshot {
    pub state: State,
    pub progress: Progress,
    pub bytes_per_second: Option<u64>,
    pub eta_seconds: Option<u64>,
    pub retries: u8,
    pub retry_after: Option<Duration>,
}

#[derive(Debug)]
pub struct Task {
    entry: Entry,
    name: String,
    partial: PartialDownload,
    chunk_bytes: u64,
    retry_limit: u8,
    state: State,
    retries: u8,
    retry_at: Option<Instant>,
    rate: Option<u64>,
    last_error: Option<Error>,
}

fn rate(bytes: u64, elapsed: Duration) -> Option<u64> {
    let value = (u128::from(bytes) * 1_000_000_000).checked_div(elapsed.as_nanos())?;
    let value = u64::try_from(value).unwrap_or(u64::MAX);
    (value > 0).then_some(value)
}

impl Task {
    /// The partial file and its ancestors remain caller-controlled.
    /// An existing complete-length file must still pass final verification.
    pub fn new(
        entry: Entry,
        name: String,
        partial: PartialDownload,
        chunk_bytes: u64,
        retry_limit: u8,
    ) -> Result<Self, Error> {
        entry.validate().map_err(Error::Catalog)?;
        if entry.files.get(&name) != Some(partial.asset()) || retry_limit > MAX_RETRIES {
            return Err(Error::Configuration);
        }
        partial.next_range(chunk_bytes).map_err(Error::Resume)?;
        Ok(Self {
            entry,
            name,
            partial,
            chunk_bytes,
            retry_limit,
            state: State::Ready,
            retries: 0,
            retry_at: None,
            rate: None,
            last_error: None,
        })
    }

    pub fn snapshot(&self, now: Instant) -> Snapshot {
        let progress = self.partial.progress();
        let remaining = progress.total - progress.received;
        let eta_seconds = if self.state == State::Complete {
            Some(0)
        } else if self.state == State::Ready && remaining > 0 {
            self.rate.map(|r| remaining.div_ceil(r))
        } else {
            None
        };
        Snapshot {
            state: self.state,
            progress,
            bytes_per_second: self.rate,
            eta_seconds,
            retries: self.retries,
            retry_after: self
                .retry_at
                .map(|deadline| deadline.saturating_duration_since(now)),
        }
    }

    pub fn last_error(&self) -> Option<&Error> {
        self.last_error.as_ref()
    }

    pub fn pause(&mut self) {
        if matches!(self.state, State::Ready | State::RetryWait) {
            self.state = State::Paused;
            self.rate = None;
        }
    }

    pub fn resume(&mut self) {
        if self.state == State::Paused {
            self.state = if self.retry_at.is_some() {
                State::RetryWait
            } else {
                State::Ready
            };
        }
    }

    /// Retain the partial on failure; an explicit restart can be performed
    /// by the owner after examining the error. No automatic truncation.
    pub fn into_partial(self) -> PartialDownload {
        self.partial
    }

    /// Perform at most one blocking bounded HTTPS request or verification.
    /// Pause and retry are cooperative at chunk boundaries, not mid-request.
    pub fn step(&mut self, downloader: &Downloader) -> Snapshot {
        self.step_with(Instant::now(), |entry, name, partial, chunk| {
            let start = Instant::now();
            let result = downloader.fetch_next(entry, name, partial, chunk);
            (result, start.elapsed())
        })
    }

    fn step_with(
        &mut self,
        now: Instant,
        fetch: impl FnOnce(
            &Entry,
            &str,
            &mut PartialDownload,
            u64,
        ) -> (Result<Progress, Error>, Duration),
    ) -> Snapshot {
        if !matches!(self.state, State::Ready | State::RetryWait)
            || self.retry_at.is_some_and(|deadline| deadline > now)
        {
            return self.snapshot(now);
        }
        self.retry_at = None;
        let before = self.partial.progress();
        let (result, elapsed) = fetch(&self.entry, &self.name, &mut self.partial, self.chunk_bytes);
        let after = self.partial.progress();
        let finished = now.checked_add(elapsed).unwrap_or(now);
        match result {
            Ok(progress)
                if progress == after
                    && after.received >= before.received
                    && (after.received > before.received || after.received == after.total) =>
            {
                self.retries = 0;
                self.last_error = None;
                self.rate = rate(after.received - before.received, elapsed);
                self.state = if after.received == after.total {
                    State::Complete
                } else {
                    State::Ready
                };
            }
            Ok(_) => {
                self.state = State::Failed;
                self.rate = None;
                self.last_error = Some(Error::Framing);
            }
            Err(error) => {
                self.rate = None;
                let retryable = matches!(error, Error::TransientTransport(_))
                    && before == after
                    && self.retries < self.retry_limit;
                self.last_error = Some(error);
                if retryable {
                    self.retries += 1;
                    let delay = Duration::from_secs(1_u64 << (self.retries - 1));
                    self.retry_at = finished.checked_add(delay);
                    self.state = if self.retry_at.is_some() {
                        State::RetryWait
                    } else {
                        State::Failed
                    };
                } else {
                    self.state = State::Failed;
                }
            }
        }
        self.snapshot(finished)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        catalog::{Asset, Precision, Version},
        resume,
    };
    use std::{collections::BTreeMap, fs, path::PathBuf};

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "rpmodel-task-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }

        fn task(&self, retry_limit: u8) -> Task {
            let asset = Asset {
                bytes: 3,
                sha256: "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                    .into(),
            };
            let entry = Entry {
                id: "fixture".into(),
                version: Version(1, 0, 0),
                precision: Precision::Int8,
                min_engine: Version(1, 0, 0),
                files: BTreeMap::from([("data.bin".into(), asset.clone())]),
            };
            let partial = PartialDownload::create(&self.0.join("data.part"), asset).unwrap();
            Task::new(entry, "data.bin".into(), partial, 2, retry_limit).unwrap()
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn chunk(task: &mut Task, now: Instant, data: &[u8]) -> Snapshot {
        task.step_with(now, |_, _, partial, budget| {
            let range = partial.next_range(budget).unwrap().unwrap();
            let header = format!("bytes {}-{}/{}", range.start, range.end, range.total);
            let result = partial
                .append(range, 206, &header, data)
                .and_then(|progress| {
                    if progress.received == progress.total {
                        partial.verify_complete()?;
                    }
                    Ok(progress)
                })
                .map_err(Error::Resume);
            (result, Duration::from_secs(1))
        })
    }

    fn transient(task: &mut Task, now: Instant) -> Snapshot {
        task.step_with(now, |_, _, _, _| {
            (Err(Error::TransientTransport(28)), Duration::ZERO)
        })
    }

    fn idle(task: &mut Task, now: Instant) -> Snapshot {
        task.step_with(now, |_, _, _, _| panic!("must not fetch"))
    }

    #[test]
    fn reports_rate_eta_and_verified_completion() {
        let root = Scratch::new();
        let mut task = root.task(2);
        let now = Instant::now();
        let first = chunk(&mut task, now, b"ab");
        assert_eq!(first.progress.received, 2);
        assert_eq!(first.bytes_per_second, Some(2));
        assert_eq!(first.eta_seconds, Some(1));
        let last = chunk(&mut task, now + Duration::from_secs(1), b"c");
        assert_eq!(last.state, State::Complete);
        assert_eq!(last.eta_seconds, Some(0));
        assert_eq!(idle(&mut task, now).state, State::Complete);
    }

    #[test]
    fn pause_does_not_fetch_or_clear_retry_budget() {
        let root = Scratch::new();
        let mut task = root.task(2);
        let now = Instant::now();
        assert_eq!(transient(&mut task, now).retries, 1);
        task.pause();
        assert_eq!(idle(&mut task, now).state, State::Paused);
        task.resume();
        let waiting = idle(&mut task, now);
        assert_eq!(waiting.state, State::RetryWait);
        assert_eq!(waiting.retries, 1);
        assert_eq!(waiting.retry_after, Some(Duration::from_secs(1)));
        assert_eq!(
            chunk(&mut task, now + Duration::from_secs(1), b"ab").state,
            State::Ready
        );
    }

    #[test]
    fn retry_backoff_and_exhaustion_preserve_partial() {
        let root = Scratch::new();
        let mut task = root.task(2);
        let now = Instant::now();
        chunk(&mut task, now, b"ab");
        let first = transient(&mut task, now);
        assert_eq!(first.retry_after, Some(Duration::from_secs(1)));
        assert_eq!(idle(&mut task, now).state, State::RetryWait);
        let second = transient(&mut task, now + Duration::from_secs(1));
        assert_eq!(second.retry_after, Some(Duration::from_secs(2)));
        let failed = transient(&mut task, now + Duration::from_secs(3));
        assert_eq!(failed.state, State::Failed);
        assert_eq!(failed.retries, 2);
        assert_eq!(failed.progress.received, 2);
        assert_eq!(task.last_error(), Some(&Error::TransientTransport(28)));
        assert_eq!(idle(&mut task, now).state, State::Failed);
    }

    #[test]
    fn successful_chunk_resets_retry_budget() {
        let root = Scratch::new();
        let mut task = root.task(1);
        let now = Instant::now();
        transient(&mut task, now);
        let good = chunk(&mut task, now + Duration::from_secs(1), b"ab");
        assert_eq!(good.retries, 0);
        assert_eq!(task.last_error(), None);
        let next = transient(&mut task, now + Duration::from_secs(2));
        assert_eq!(next.state, State::RetryWait);
        assert_eq!(next.retries, 1);
    }

    #[test]
    fn integrity_failure_never_retries_or_reports_complete() {
        let root = Scratch::new();
        let mut task = root.task(5);
        let now = Instant::now();
        chunk(&mut task, now, b"ab");
        let corrupt = chunk(&mut task, now, b"d");
        assert_eq!(corrupt.state, State::Failed);
        assert_eq!(corrupt.progress.received, 3);
        assert_eq!(corrupt.eta_seconds, None);
        assert_eq!(
            task.last_error(),
            Some(&Error::Resume(resume::Error::Catalog(
                crate::catalog::Error::Integrity
            )))
        );
        assert_eq!(idle(&mut task, now).state, State::Failed);
    }

    #[test]
    fn fatal_http_error_and_zero_retry_budget_stop() {
        let root = Scratch::new();
        let mut task = root.task(5);
        let now = Instant::now();
        let failed = task.step_with(now, |_, _, _, _| {
            (Err(Error::HttpStatus), Duration::ZERO)
        });
        assert_eq!(failed.state, State::Failed);
        assert_eq!(failed.retries, 0);
        let root = Scratch::new();
        let mut task = root.task(0);
        assert_eq!(transient(&mut task, now).state, State::Failed);
    }

    #[test]
    fn rate_handles_zero_duration_and_extreme_values() {
        assert_eq!(rate(1, Duration::ZERO), None);
        assert_eq!(rate(0, Duration::from_secs(1)), None);
        assert_eq!(rate(1, Duration::from_secs(2)), None);
        assert_eq!(rate(u64::MAX, Duration::from_nanos(1)), Some(u64::MAX));
    }
}
