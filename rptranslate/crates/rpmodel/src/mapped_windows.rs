//! Explicit Windows 64-bit read-only mapping; not an immutable snapshot.
use std::{ffi::c_void, fs::File, io, os::windows::io::AsRawHandle, ptr::NonNull, slice};

type Handle = *mut c_void;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateFileMappingW(
        file: Handle,
        attributes: *mut c_void,
        protect: u32,
        size_high: u32,
        size_low: u32,
        name: *const u16,
    ) -> Handle;
    fn MapViewOfFile(
        mapping: Handle,
        access: u32,
        offset_high: u32,
        offset_low: u32,
        bytes: usize,
    ) -> *mut c_void;
    fn UnmapViewOfFile(address: *const c_void) -> i32;
    fn CloseHandle(handle: Handle) -> i32;
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
    pub unsafe fn open(file: &File, limit: usize) -> io::Result<Self> {
        let metadata = file.metadata()?;
        let len = usize::try_from(metadata.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "mapping size"))?;
        if !metadata.is_file() || len == 0 || len > limit || len > isize::MAX as usize {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "mapping size"));
        }
        // PAGE_READONLY = 2; FILE_MAP_READ = 4.
        let handle = unsafe {
            CreateFileMappingW(
                file.as_raw_handle(),
                std::ptr::null_mut(),
                2,
                0,
                0,
                std::ptr::null(),
            )
        };
        if handle.is_null() {
            return Err(io::Error::last_os_error());
        }
        let addr = unsafe { MapViewOfFile(handle, 4, 0, 0, len) };
        let result = NonNull::new(addr.cast::<u8>())
            .map(|ptr| Self { ptr, len })
            .ok_or_else(io::Error::last_os_error);
        // A mapped view retains its own reference to the mapping object.
        unsafe { CloseHandle(handle) };
        result
    }

    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: live bounded mapping, immutable by open's contract.
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for MappedFile {
    fn drop(&mut self) {
        // SAFETY: owns the view; borrowed slices cannot outlive it.
        unsafe { UnmapViewOfFile(self.ptr.as_ptr().cast()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn mapping_budget_and_bytes() {
        let dir = std::env::temp_dir().join(format!("rpmodel-win-map-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let path = dir.join("weights");
        fs::write(&path, b"").unwrap();
        let empty = File::open(&path).unwrap();
        assert!(unsafe { MappedFile::open(&empty, 3) }.is_err());
        drop(empty);
        fs::write(&path, b"abc").unwrap();
        let file = File::open(&path).unwrap();
        assert!(unsafe { MappedFile::open(&file, 2) }.is_err());
        // Private fixture; immutable until the view drops.
        let mapped = unsafe { MappedFile::open(&file, 3) }.unwrap();
        drop(file);
        assert_eq!(mapped.as_bytes(), b"abc");
        drop(mapped);
        fs::remove_dir_all(dir).unwrap();
    }
}
