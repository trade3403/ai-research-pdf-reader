//! Explicit Linux 64-bit read-only mapping; not an immutable snapshot.
use std::{
    ffi::c_void,
    fs::File,
    io,
    os::fd::AsRawFd,
    ptr::{self, NonNull},
    slice,
};

unsafe extern "C" {
    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> i32;
}

pub struct MappedFile {
    ptr: NonNull<u8>,
    len: usize,
}

impl MappedFile {
    /// Map a nonempty regular file within the byte budget.
    ///
    /// # Safety
    /// The caller must ensure the file is not modified or truncated, including
    /// through other handles, hard links or processes, until the mapping drops.
    /// MAP_PRIVATE does not guarantee immutability.
    pub unsafe fn open(file: &File, limit: usize) -> io::Result<Self> {
        let metadata = file.metadata()?;
        let len = usize::try_from(metadata.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "mapping size"))?;
        if !metadata.is_file() || len == 0 || len > limit || len > isize::MAX as usize {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "mapping size"));
        }
        // Linux PROT_READ = 1, MAP_PRIVATE = 2.
        let addr = unsafe { mmap(ptr::null_mut(), len, 1, 2, file.as_raw_fd(), 0) };
        if addr as isize == -1 {
            return Err(io::Error::last_os_error());
        }
        let Some(ptr) = NonNull::new(addr.cast::<u8>()) else {
            unsafe { munmap(addr, len) };
            return Err(io::Error::other("null mapping"));
        };
        Ok(Self { ptr, len })
    }

    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: live bounded mapping, immutable by open's contract.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for MappedFile {
    fn drop(&mut self) {
        // SAFETY: owns the mapping; borrowed slices cannot outlive it.
        unsafe { munmap(self.ptr.as_ptr().cast(), self.len) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::DirBuilderExt};

    #[test]
    fn mapping_budget_and_bytes() {
        let dir = std::env::temp_dir().join(format!("rpmodel-map-{}", std::process::id()));
        fs::DirBuilder::new().mode(0o700).create(&dir).unwrap();
        let path = dir.join("weights");
        fs::write(&path, b"abc").unwrap();
        let file = File::open(&path).unwrap();
        // Private test fixture, no writes during mapping lifetime.
        assert!(unsafe { MappedFile::open(&file, 2) }.is_err());
        let mapped = unsafe { MappedFile::open(&file, 3) }.unwrap();
        assert_eq!(mapped.as_bytes(), b"abc");
        drop(mapped);
        drop(file);
        fs::remove_dir_all(dir).unwrap();
    }
}
