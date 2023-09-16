use sqsh_sys as ffi;
use std::ffi::CString;
use std::io::BufRead;
use std::ptr;

use crate::{error, Archive, Error, Permissions};

/// High level "easy" methods for interacting with the archive.
impl Archive<'_> {
    /// Read the file at the given path
    pub fn read(&self, path: &str) -> error::Result<Vec<u8>> {
        let file = self.open(path)?;
        let mut reader = file.reader()?;
        let size = match usize::try_from(file.size()) {
            Ok(size) => size,
            Err(_) => return Err(Error(ffi::SqshError::SQSH_ERROR_INTEGER_OVERFLOW)),
        };

        let mut dst = Vec::with_capacity(size);
        loop {
            let buf = reader.fill_buf_raw()?;
            if buf.is_empty() {
                break;
            }
            dst.extend_from_slice(buf);
            let len = buf.len();
            reader.consume(len);
        }
        Ok(dst)
    }

    /// Check if anything exists at the given path
    pub fn exists(&self, path: &str) -> bool {
        let path = match CString::new(path) {
            Ok(path) => path,
            Err(_) => return false,
        };
        unsafe { ffi::sqsh_easy_file_exists(self.inner.as_ptr(), path.as_ptr(), ptr::null_mut()) }
    }

    pub fn permissions(&self, path: &str) -> error::Result<Permissions> {
        let path = CString::new(path)?;
        let mut err = 0;
        let raw_permissions =
            unsafe { ffi::sqsh_easy_file_permission(self.inner.as_ptr(), path.as_ptr(), &mut err) };
        if err != 0 {
            return Err(error::new(err));
        }
        Ok(Permissions::from_bits_retain(raw_permissions))
    }
}
