use sqsh_sys as ffi;
use std::borrow::Cow;

use std::error::Error as StdError;
use std::ffi::c_int;
use std::fmt::{Debug, Display, Formatter};
use std::io;

#[derive(Copy, Clone)]
pub struct Error(pub(crate) ffi::SqshError);

impl Error {
    pub(crate) fn new(err: c_int) -> io::Error {
        let err = err.saturating_abs();
        if err < ffi::SqshError::SQSH_ERROR_SECTION_START.0 as c_int {
            io::Error::from_raw_os_error(err)
        } else {
            io::Error::new(
                io::ErrorKind::Other,
                Self(ffi::SqshError(err.unsigned_abs())),
            )
        }
    }
}

impl Error {
    fn to_str(&self) -> Cow<'static, str> {
        // sqsh_error_str is _not_ thread safe or reentrant
        let s = match self.0 {
            ffi::SqshError::SQSH_SUCCESS => "Success",
            ffi::SqshError::SQSH_ERROR_NO_COMPRESSION_OPTIONS => "No compression options",
            ffi::SqshError::SQSH_ERROR_SUPERBLOCK_TOO_SMALL => "Superblock too small",
            ffi::SqshError::SQSH_ERROR_WRONG_MAGIC => "Wrong magic",
            ffi::SqshError::SQSH_ERROR_BLOCKSIZE_MISMATCH => "Blocksize mismatch",
            ffi::SqshError::SQSH_ERROR_SIZE_MISMATCH => "Size mismatch",
            ffi::SqshError::SQSH_ERROR_COMPRESSION_INIT => "Compression init",
            ffi::SqshError::SQSH_ERROR_COMPRESSION_DECOMPRESS => "Compression decompress",
            ffi::SqshError::SQSH_ERROR_UNKOWN_FILE_TYPE => "Unknown file type",
            ffi::SqshError::SQSH_ERROR_NOT_A_DIRECTORY => "Not a directory",
            ffi::SqshError::SQSH_ERROR_NOT_A_FILE => "Not a file",
            ffi::SqshError::SQSH_ERROR_MALLOC_FAILED => "Malloc Failed",
            ffi::SqshError::SQSH_ERROR_MUTEX_INIT_FAILED => "Mutex init failed",
            ffi::SqshError::SQSH_ERROR_MUTEX_LOCK_FAILED => "Mutex lock failed",
            ffi::SqshError::SQSH_ERROR_MUTEX_DESTROY_FAILED => "Mutex destroy failed",
            ffi::SqshError::SQSH_ERROR_OUT_OF_BOUNDS => "Out of bounds",
            ffi::SqshError::SQSH_ERROR_INTEGER_OVERFLOW => "Integer overflow",
            ffi::SqshError::SQSH_ERROR_NO_SUCH_FILE => "No such file or directory",
            ffi::SqshError::SQSH_ERROR_NO_SUCH_XATTR => "No such xattr",
            ffi::SqshError::SQSH_ERROR_NO_EXTENDED_DIRECTORY => "No extended directory",
            ffi::SqshError::SQSH_ERROR_NO_FRAGMENT_TABLE => "No fragment table",
            ffi::SqshError::SQSH_ERROR_NO_EXPORT_TABLE => "No export table",
            ffi::SqshError::SQSH_ERROR_NO_XATTR_TABLE => "No xattr table",
            ffi::SqshError::SQSH_ERROR_MAPPER_INIT => "Mapper init error",
            ffi::SqshError::SQSH_ERROR_MAPPER_MAP => "Mapper mapping error",
            ffi::SqshError::SQSH_ERROR_COMPRESSION_UNSUPPORTED => "Compression unknown",
            ffi::SqshError::SQSH_ERROR_CURL_INVALID_RANGE_HEADER => "Invalid range header",
            ffi::SqshError::SQSL_ERROR_ELEMENT_NOT_FOUND => "Element not found",
            ffi::SqshError::SQSH_ERROR_INVALID_ARGUMENT => "Invalid argument",
            ffi::SqshError::SQSH_ERROR_WALKER_CANNOT_GO_UP => "Walker cannot go up",
            ffi::SqshError::SQSH_ERROR_WALKER_CANNOT_GO_DOWN => "Walker cannot go down",
            ffi::SqshError::SQSH_ERROR_CORRUPTED_INODE => "Corrupted inode",
            ffi::SqshError::SQSH_ERROR_CORRUPTED_DIRECTORY_ENTRY => "Corrupted directory entry",
            ffi::SqshError::SQSH_ERROR_INTERNAL => "Internal error",
            ffi::SqshError::SQSH_ERROR_INODE_MAP_IS_INCONSISTENT => "Inode map is inconsistent",
            ffi::SqshError::SQSH_ERROR_XATTR_SIZE_MISMATCH => "Xattr size mismatch",
            ffi::SqshError::SQSH_ERROR_UNSUPPORTED_VERSION => "Unsupported version",
            ffi::SqshError::SQSH_ERROR_TOO_MANY_SYMLINKS_FOLLOWED => "Too many symlinks followed",
            ffi::SqshError(other) => {
                return Cow::Owned(io::Error::from_raw_os_error(other as i32).to_string());
            }
        };
        Cow::Borrowed(s)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_str())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_str())
    }
}

impl StdError for Error {}
