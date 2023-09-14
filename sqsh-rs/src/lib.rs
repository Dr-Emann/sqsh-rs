mod directory;
mod easy;
mod error;
mod export_table;
mod file;
mod id_table;
mod inode;
mod inode_map;
mod reader;
mod superblock;
mod walker;
mod xattr;

pub use crate::directory::DirectoryIterator;
pub use crate::error::{Error, Result};
pub use crate::export_table::ExportTable;
pub use crate::file::File;
pub use crate::id_table::IdTable;
pub use crate::inode::{Inode, InodeRef, ZeroInode};
pub use crate::inode_map::InodeMap;
pub use crate::reader::Reader;
pub use crate::superblock::{Compression, Superblock};
pub use crate::walker::Walker;
pub use crate::xattr::XattrIterator;

use bitflags::bitflags;
use sqsh_sys as ffi;
use std::ffi::{c_void, CString};
use std::fmt::Debug;
use std::marker::PhantomData;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr::NonNull;
use std::{mem, ptr};

pub unsafe trait Source {
    /// Represents the addressable size of the source in bytes
    ///
    /// Only really useful for slice-based sources, files know their own length
    fn source_mapper() -> *const ffi::SqshMemoryMapperImpl;

    fn size(&self) -> u64;

    fn with_source_pointer<O, F>(&self, f: F) -> O
    where
        F: FnOnce(*const c_void) -> O;
}

unsafe impl<'a> Source for &'a [u8] {
    fn source_mapper() -> *const ffi::SqshMemoryMapperImpl {
        unsafe { ffi::sqsh_mapper_impl_static }
    }

    fn size(&self) -> u64 {
        self.len().try_into().unwrap()
    }

    fn with_source_pointer<O, F>(&self, f: F) -> O
    where
        F: FnOnce(*const c_void) -> O,
    {
        f(self.as_ptr().cast())
    }
}

unsafe impl Source for Path {
    fn source_mapper() -> *const ffi::SqshMemoryMapperImpl {
        unsafe { ffi::sqsh_mapper_impl_mmap }
    }

    fn size(&self) -> u64 {
        0
    }

    fn with_source_pointer<O, F>(&self, f: F) -> O
    where
        F: FnOnce(*const c_void) -> O,
    {
        let path = CString::new(self.as_os_str().as_bytes()).unwrap();
        f(path.as_ptr().cast())
    }
}

#[derive(Debug)]
pub struct Archive<S: Source + ?Sized = Path> {
    inner: NonNull<ffi::SqshArchive>,
    _marker: PhantomData<S>,
}

// Safety: SqshArchive is uses a mutex internally for thread safety
unsafe impl<S: Source + ?Sized + Send> Send for Archive<S> {}
unsafe impl<S: Source + ?Sized + Sync> Sync for Archive<S> {}

impl Archive<Path> {
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

impl<'a> Archive<&'a [u8]> {
    pub fn from_slice(data: &'a [u8]) -> error::Result<Self> {
        Self::with_source(&data)
    }
}

impl<S: Source + ?Sized> Archive<S> {
    pub fn with_source(source: &S) -> error::Result<Self> {
        let mut err = 0;
        let config = ffi::SqshConfig {
            archive_offset: 0,
            source_size: source.size(),
            source_mapper: S::source_mapper().cast_mut(),
            mapper_block_size: 0,
            mapper_lru_size: 0,
            compression_lru_size: 0,
            max_symlink_depth: 0,
            _reserved: unsafe { mem::zeroed() },
        };
        let archive = source.with_source_pointer(|source_ptr| unsafe {
            ffi::sqsh_archive_open(source_ptr, &config, &mut err)
        });
        match NonNull::new(archive) {
            Some(archive) => Ok(Self {
                inner: archive,
                _marker: PhantomData,
            }),
            None => Err(error::new(err)),
        }
    }

    pub fn superblock(&self) -> Superblock<'_> {
        unsafe { Superblock::new(ffi::sqsh_archive_superblock(self.inner.as_ptr())) }
    }

    pub fn id_table(&self) -> error::Result<IdTable<'_>> {
        let mut dst = ptr::null_mut();
        let rc = unsafe { ffi::sqsh_archive_id_table(self.inner.as_ptr(), &mut dst) };
        if rc != 0 {
            return Err(error::new(rc));
        }
        Ok(unsafe { IdTable::new(dst) })
    }

    pub fn inode_map(&self) -> error::Result<InodeMap<'_>> {
        let mut dst = ptr::null_mut();
        let rc = unsafe { ffi::sqsh_archive_inode_map(self.inner.as_ptr(), &mut dst) };
        if rc != 0 {
            return Err(error::new(rc));
        }
        Ok(unsafe { InodeMap::new(dst) })
    }

    pub fn export_table(&self) -> error::Result<ExportTable<'_>> {
        let mut dst = ptr::null_mut();
        let rc = unsafe { ffi::sqsh_archive_export_table(self.inner.as_ptr(), &mut dst) };
        if rc != 0 {
            return Err(error::new(rc));
        }
        Ok(unsafe { ExportTable::new(dst) })
    }

    pub fn root(&self) -> error::Result<File<'_>> {
        let superblock = self.superblock();
        let inode_ref = superblock.root_inode_ref();
        self.open_ref(inode_ref)
    }
}

impl<S: Source + ?Sized> Drop for Archive<S> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqsh_archive_close(self.inner.as_ptr());
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FileType {
    Directory = ffi::SqshFileType::SQSH_FILE_TYPE_DIRECTORY.0 as _,
    File = ffi::SqshFileType::SQSH_FILE_TYPE_FILE.0 as _,
    Symlink = ffi::SqshFileType::SQSH_FILE_TYPE_SYMLINK.0 as _,
    BlockDevice = ffi::SqshFileType::SQSH_FILE_TYPE_BLOCK.0 as _,
    CharacterDevice = ffi::SqshFileType::SQSH_FILE_TYPE_CHAR.0 as _,
    Socket = ffi::SqshFileType::SQSH_FILE_TYPE_SOCKET.0 as _,
    Fifo = ffi::SqshFileType::SQSH_FILE_TYPE_FIFO.0 as _,
}

impl TryFrom<ffi::SqshFileType> for FileType {
    type Error = ();

    fn try_from(value: ffi::SqshFileType) -> Result<Self, Self::Error> {
        match value {
            ffi::SqshFileType::SQSH_FILE_TYPE_DIRECTORY => Ok(FileType::Directory),
            ffi::SqshFileType::SQSH_FILE_TYPE_FILE => Ok(FileType::File),
            ffi::SqshFileType::SQSH_FILE_TYPE_SYMLINK => Ok(FileType::Symlink),
            ffi::SqshFileType::SQSH_FILE_TYPE_BLOCK => Ok(FileType::BlockDevice),
            ffi::SqshFileType::SQSH_FILE_TYPE_CHAR => Ok(FileType::CharacterDevice),
            ffi::SqshFileType::SQSH_FILE_TYPE_SOCKET => Ok(FileType::Socket),
            ffi::SqshFileType::SQSH_FILE_TYPE_FIFO => Ok(FileType::Fifo),
            _ => Err(()),
        }
    }
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct Permissions: u16 {
        const UserRead = 0o0400;
        const UserWrite = 0o0200;
        const UserExec = 0o0100;
        const UserRW = Self::UserRead.bits() | Self::UserWrite.bits();
        const UserRWX = Self::UserRW.bits() | Self::UserExec.bits();

        const GroupRead = 0o0040;
        const GroupWrite = 0o0020;
        const GroupExec = 0o0010;
        const GroupRW = Self::GroupRead.bits() | Self::GroupWrite.bits();
        const GroupRWX = Self::GroupRW.bits() | Self::GroupExec.bits();

        const OtherRead = 0o0004;
        const OtherWrite = 0o0002;
        const OtherExec = 0o0001;
        const OtherRW = Self::OtherRead.bits() | Self::OtherWrite.bits();
        const OtherRWX = Self::OtherRW.bits() | Self::OtherExec.bits();

        const _ = !0;
    }
}
