use crate::source::Source;
use crate::{error, File};
use sqsh_sys as ffi;
use std::marker::PhantomData;
use std::mem;
use std::path::Path;
use std::ptr::NonNull;

#[derive(Debug)]
pub struct Archive<'a> {
    pub(crate) inner: NonNull<ffi::SqshArchive>,
    _marker: PhantomData<&'a ()>,
}

// Safety: SqshArchive is uses a mutex internally for thread safety
unsafe impl<'a> Send for Archive<'a> {}
unsafe impl<'a> Sync for Archive<'a> {}

impl Archive<'static> {
    pub fn new<P>(path: P) -> error::Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::_new(path.as_ref())
    }

    fn _new(path: &Path) -> error::Result<Self> {
        Self::with_source(path)
    }
}

impl<'a> Archive<'a> {
    pub fn from_slice(data: &'a [u8]) -> error::Result<Self> {
        Self::with_source(data)
    }
}

impl<'a> Archive<'a> {
    pub fn with_source<S>(source: S) -> error::Result<Self>
    where
        S: Source<'a>,
    {
        let mut err = 0;
        let config = ffi::SqshConfig {
            archive_offset: 0,
            source_size: source.size(),
            source_mapper: source.source_mapper(),
            mapper_block_size: 0,
            mapper_lru_size: 0,
            compression_lru_size: 0,
            max_symlink_depth: 0,
            _reserved: unsafe { mem::zeroed() },
        };
        let archive = source.with_source_pointer(|source_ptr| unsafe {
            ffi::sqsh_archive_open(source_ptr, &config, &mut err)
        })?;
        match NonNull::new(archive) {
            Some(archive) => Ok(Self {
                inner: archive,
                _marker: PhantomData,
            }),
            None => Err(error::new(err)),
        }
    }

    pub fn root(&self) -> error::Result<File<'_>> {
        let superblock = self.superblock();
        let inode_ref = superblock.root_inode_ref();
        self.open_ref(inode_ref)
    }
}

impl Drop for Archive<'_> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqsh_archive_close(self.inner.as_ptr());
        }
    }
}
