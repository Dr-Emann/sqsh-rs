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
pub use crate::superblock::Superblock;
pub use crate::walker::Walker;
pub use crate::xattr::XattrIterator;

use bitflags::bitflags;
use sqsh_sys as ffi;
use std::ffi::{c_void, CString};
use std::fmt::Debug;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::ptr::NonNull;
use std::{mem, ptr};

#[derive(Debug)]
pub struct Archive {
    inner: NonNull<ffi::SqshArchive>,
}

// Safety: SqshArchive is uses a mutex internally for thread safety
unsafe impl Send for Archive {}
unsafe impl Sync for Archive {}

#[derive(Debug)]
#[repr(transparent)]
pub struct MemArchive<'a> {
    inner: NonNull<ffi::SqshArchive>,
    _marker: std::marker::PhantomData<&'a [u8]>,
}

impl std::ops::Deref for MemArchive<'_> {
    type Target = Archive;

    fn deref(&self) -> &Self::Target {
        unsafe { mem::transmute(self) }
    }
}

impl Archive {
    pub fn new<P>(path: P) -> error::Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::_new(path.as_ref())
    }

    fn _new(path: &Path) -> error::Result<Self> {
        let path_str = CString::new(path.as_os_str().as_bytes())?;
        let mut err = 0;
        let archive = unsafe {
            ffi::sqsh_archive_open(path_str.as_ptr().cast::<c_void>(), ptr::null(), &mut err)
        };
        match NonNull::new(archive) {
            Some(archive) => Ok(Self { inner: archive }),
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

    pub fn mem_new(data: &[u8]) -> error::Result<MemArchive<'_>> {
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

        match NonNull::new(archive) {
            Some(archive) => Ok(MemArchive {
                inner: archive,
                _marker: std::marker::PhantomData,
            }),
            None => Err(error::new(err)),
        }
    }
}

impl Drop for MemArchive<'_> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqsh_archive_close(self.inner.as_ptr());
        }
    }
}

impl Drop for Archive {
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
