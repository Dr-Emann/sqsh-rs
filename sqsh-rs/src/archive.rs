use crate::source::SourceVtable;
use crate::utils::small_c_string::run_with_cstr;
use crate::{error, File, Source};
use sqsh_sys as ffi;
use sqsh_sys::SqshMemoryMapperImpl;
use std::ffi::c_void;
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
        run_with_cstr(path.as_os_str().as_encoded_bytes(), |path| unsafe {
            Self::new_raw_simple(&*ffi::sqsh_mapper_impl_mmap, 0, path.as_ptr().cast())
        })
    }
}

impl<'a> Archive<'a> {
    pub fn from_slice(data: &'a [u8]) -> error::Result<Self> {
        unsafe {
            Self::new_raw_simple(
                &*ffi::sqsh_mapper_impl_static,
                data.len(),
                data.as_ptr().cast(),
            )
        }
    }
}

impl<'a> Archive<'a> {
    unsafe fn new_raw(config: &ffi::SqshConfig, source_ptr: *const c_void) -> error::Result<Self> {
        let mut err = 0;
        let archive = ffi::sqsh_archive_open(source_ptr, config, &mut err);

        match NonNull::new(archive) {
            Some(archive) => Ok(Self {
                inner: archive,
                _marker: PhantomData,
            }),
            None => Err(error::new(err)),
        }
    }

    unsafe fn new_raw_simple(
        source_mapper: &'a SqshMemoryMapperImpl,
        size: usize,
        source_ptr: *const c_void,
    ) -> error::Result<Self> {
        let config = ffi::SqshConfig {
            archive_offset: 0,
            source_size: size.try_into().unwrap(),
            source_mapper,
            mapper_block_size: 0,
            mapper_lru_size: 0,
            compression_lru_size: 0,
            max_symlink_depth: 0,
            _reserved: unsafe { mem::zeroed() },
        };
        Self::new_raw(&config, source_ptr)
    }

    pub fn with_source<S: Source + 'a>(source: S) -> error::Result<Self> {
        let vtable: &'a SourceVtable<S> = &const { SourceVtable::new() };
        let source_ptr = crate::source::to_ptr(source);
        unsafe { Self::new_raw_simple(vtable.mapper_impl(), 0, source_ptr) }
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
