#!/usr/bin/env python3
"""Validate the actual completed multi-seed fuzz campaigns, not just exit code."""
import argparse
import hashlib
import json
import re
from pathlib import Path

SEEDS = (12345, 67890, 314159)
RUNS = 50000
MAX_LOG = 16 * 1024 * 1024
DONE = re.compile(r"^#(\d+)\s+DONE\s+cov:\s*(\d+)\s+ft:\s*(\d+)\s+corp:\s*(\d+)/\S+\s+lim:\s*(\d+)\s+exec/s:\s*(\d+)\s+rss:\s*(\d+)Mb\s*$", re.M)
FINISH = re.compile(r"^Done (\d+) runs in (\d+) second\(s\)\s*$", re.M)

def parse_log(log):
    done, finish = DONE.findall(log), FINISH.findall(log)
    if len(done) != 1 or len(finish) != 1:
        raise ValueError("missing or duplicate completion record")
    runs, cov, ft, corpus, length, rate, rss = map(int, done[0])
    confirmed, seconds = map(int, finish[0])
    if runs != RUNS or confirmed != RUNS or seconds > 120:
        raise ValueError("incomplete run budget")
    if not (cov > 0 and ft > 0 and corpus >= 3 and 0 < length <= 4096 and 0 < rss <= 1024):
        raise ValueError("invalid coverage/resource record")
    if not re.search(r"^INFO: seed corpus: files: 3\b", log, re.M):
        raise ValueError("initial corpus evidence missing")
    if re.search(r"ERROR: (?:libFuzzer|AddressSanitizer)|panicked at|runtime error:|SUMMARY: AddressSanitizer", log):
        raise ValueError("reported failure")
    return dict(runs=runs, cov=cov, features=ft, evolved_corpus=corpus,
                length_limit=length, exec_per_second=rate, ending_rss_mb=rss, seconds=seconds)

def self_test():
    good = ("INFO: seed corpus: files: 3 min: 180b max: 389b total: 888b rss: 44Mb\n"
            "#50000\tDONE   cov: 3004 ft: 5783 corp: 443/129Kb lim: 405 exec/s: 5555 rss: 447Mb\n"
            "Done 50000 runs in 9 second(s)\n")
    assert parse_log(good)["cov"] == 3004
    for bad in (good.replace("#50000","#49999"), good.replace("files: 3","files: 2"),
                good.replace("rss: 447Mb","rss: 1025Mb"),
                good.replace("9 second(s)","121 second(s)"), good+good,
                good+"ERROR: AddressSanitizer: test\n", ""):
        try:
            parse_log(bad)
        except ValueError:
            pass
        else:
            raise AssertionError("invalid log accepted")

def main():
    p = argparse.ArgumentParser()
    p.add_argument("--report", type=Path, required=True)
    p.add_argument("--commit", required=True)
    a = p.parse_args()
    self_test()
    rows = []
    for seed in SEEDS:
        path = a.report / f"seed-{seed}.log"
        with path.open("rb") as f:
            raw = f.read(MAX_LOG+1)
        if len(raw) > MAX_LOG:
            raise ValueError("fuzz log exceeds size cap")
        row = parse_log(raw.decode("utf-8", errors="replace"))
        rows.append(dict(seed=seed, sha256=hashlib.sha256(raw).hexdigest(), **row))
    result = dict(schema=1, commit=a.commit, status="pass", campaigns=rows,
                  total_runs=sum(r["runs"] for r in rows),
                  note="Coverage counters overlap; RSS is instrumented ending RSS, not decoder peak.")
    (a.report/"summary.json").write_text(json.dumps(result, indent=2)+"\n")
    print(f"Verified fuzz completion: {result['total_runs']} runs across {len(rows)} independent campaigns")

if __name__ == "__main__":
    main()
