mod error;

pub use crate::error::Error;

use sqsh_sys as ffi;
use std::ffi::{c_void, CString};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::{io, mem, ptr};

#[derive(Debug)]
#[repr(transparent)]
pub struct Archive {
    inner: *mut ffi::SqshArchive,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct MemArchive<'a> {
    inner: *mut ffi::SqshArchive,
    _marker: std::marker::PhantomData<&'a [u8]>,
}

impl std::ops::Deref for MemArchive<'_> {
    type Target = Archive;

    fn deref(&self) -> &Self::Target {
        unsafe { mem::transmute(self) }
    }
}

impl Archive {
    pub fn open<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::_open(path.as_ref())
    }

    pub fn _open(path: &Path) -> io::Result<Self> {
        let path_str = CString::new(path.as_os_str().as_bytes())?;
        let mut err = 0;
        let archive = unsafe {
            ffi::sqsh_archive_open(path_str.as_ptr().cast::<c_void>(), ptr::null(), &mut err)
        };
        if archive.is_null() {
            Err(Error::new(err))
        } else {
            Ok(Self { inner: archive })
        }
    }

    pub fn mem_open(data: &[u8]) -> io::Result<MemArchive<'_>> {
        let mut err = 0;
        let config = ffi::SqshConfig {
            archive_offset: 0,
            source_size: data.len() as u64,
            source_mapper: unsafe { ffi::sqsh_mapper_impl_static.cast_mut() },
            mapper_block_size: 0,
            mapper_lru_size: 0,
            compression_lru_size: 0,
            max_symlink_depth: 0,
            _reserved: unsafe { mem::zeroed() },
        };
        let archive =
            unsafe { ffi::sqsh_archive_open(data.as_ptr().cast::<c_void>(), &config, &mut err) };
        if archive.is_null() {
            Err(Error::new(err))
        } else {
            Ok(MemArchive {
                inner: archive,
                _marker: std::marker::PhantomData,
            })
        }
    }
}

impl Drop for MemArchive<'_> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqsh_archive_close(self.inner);
        }
    }
}

impl Drop for Archive {
    fn drop(&mut self) {
        unsafe {
            ffi::sqsh_archive_close(self.inner);
        }
    }
}
