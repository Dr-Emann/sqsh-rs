use sqsh_sys as ffi;

use bstr::BStr;
use std::error::Error as StdError;
use std::ffi::{c_int, CStr};
use std::fmt::{Debug, Display, Formatter};
use std::io;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Error(pub(crate) ffi::SqshError);

pub type Result<T, E = Error> = std::result::Result<T, E>;

pub(crate) const fn new(err: c_int) -> Error {
    let err = err.unsigned_abs();
    Error(ffi::SqshError(err))
}

impl Error {
    // Calls `f` with a string describing the error.
    // Safety: `f` must not call `with_str` on any Errors, or call sqsh_error_str
    // calls to `sqsh_error_str` will invalidate any previously returned pointer on that thread,
    // so we cannot allow it to be called again while we're looking at the result.
    unsafe fn with_str<F, O>(&self, f: F) -> O
    where
        // Rust lifetime rules ensure O cannot reference the CStr, which is good
        F: FnOnce(&CStr) -> O,
    {
        let s = ffi::sqsh_error_str(self.0 .0 as c_int);

        f(CStr::from_ptr(s))
    }

    #[must_use]
    pub fn io_error_kind(&self) -> io::ErrorKind {
        let Self(err) = *self;
        if err.0 < ffi::SqshError::SQSH_ERROR_SECTION_START.0 {
            let io_err = io::Error::from_raw_os_error(err.0 as _);
            return io_err.kind();
        }
        match err {
            ffi::SqshError::SQSH_ERROR_UNKNOWN_FILE_TYPE
            | ffi::SqshError::SQSH_ERROR_COMPRESSION_UNSUPPORTED
            | ffi::SqshError::SQSH_ERROR_CORRUPTED_DIRECTORY_ENTRY
            | ffi::SqshError::SQSH_ERROR_CORRUPTED_DIRECTORY_HEADER
            | ffi::SqshError::SQSH_ERROR_CORRUPTED_INODE
            | ffi::SqshError::SQSH_ERROR_BLOCKSIZE_MISMATCH
            | ffi::SqshError::SQSH_ERROR_SIZE_MISMATCH
            | ffi::SqshError::SQSH_ERROR_XATTR_SIZE_MISMATCH
            | ffi::SqshError::SQSH_ERROR_INODE_MAP_IS_INCONSISTENT
            | ffi::SqshError::SQSH_ERROR_INODE_PARENT_MISMATCH
            | ffi::SqshError::SQSH_ERROR_INODE_PARENT_UNSET
            | ffi::SqshError::SQSH_ERROR_NOT_A_SYMLINK => io::ErrorKind::InvalidData,
            ffi::SqshError::SQSH_ERROR_NO_SUCH_FILE | ffi::SqshError::SQSH_ERROR_NO_SUCH_XATTR => {
                io::ErrorKind::NotFound
            }
            ffi::SqshError::SQSH_ERROR_INVALID_ARGUMENT => io::ErrorKind::InvalidInput,
            _ => io::ErrorKind::Other,
        }
    }

    #[must_use]
    pub fn as_io_error(&self) -> Option<io::Error> {
        let Self(err) = *self;
        if err.0 < ffi::SqshError::SQSH_ERROR_SECTION_START.0 {
            Some(io::Error::from_raw_os_error(err.0 as _))
        } else {
            None
        }
    }

    #[must_use]
    pub fn into_io_error(self) -> io::Error {
        match self.as_io_error() {
            Some(err) => err,
            None => io::Error::new(self.io_error_kind(), self),
        }
    }
}

impl From<std::ffi::NulError> for Error {
    fn from(_: std::ffi::NulError) -> Self {
        Self(ffi::SqshError::SQSH_ERROR_INVALID_ARGUMENT)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe { self.with_str(|s| Debug::fmt(s, f)) }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe { self.with_str(|s| Display::fmt(BStr::new(s.to_bytes()), f)) }
    }
}

impl StdError for Error {}
