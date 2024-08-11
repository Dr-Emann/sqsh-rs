#![doc = include_str!("../README.md")]

mod archive;
mod directory;
mod easy;
mod error;
mod export_table;
mod file;
mod id_table;
mod inode;
mod inode_map;
mod path_resolver;
mod reader;
mod source;
pub mod superblock;
pub mod traverse;
mod utils;
mod xattr;

pub use crate::archive::Archive;
pub use crate::directory::{DirectoryEntry, DirectoryIterator};
pub use crate::error::{Error, Result};
pub use crate::export_table::ExportTable;
pub use crate::file::File;
pub use crate::id_table::IdTable;
pub use crate::inode::{Inode, InodeRef, ZeroInode};
pub use crate::inode_map::InodeMap;
pub use crate::path_resolver::PathResolver;
pub use crate::reader::Reader;
pub use crate::source::Source;
pub use crate::superblock::{Compression, Superblock};
pub use crate::xattr::{UnknownXattrType, XattrEntry, XattrIterator, XattrType};
use std::fmt;

use bitflags::bitflags;
pub use sqsh_sys as ffi;
use std::fmt::Debug;
use std::ops::Deref;

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

        const SetUID = 0o4000;
        const SetGID = 0o2000;
        const Sticky = 0o1000;

        const _ = !0;
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PermissionsStr([u8; 3 * 3]);

impl Deref for PermissionsStr {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl PermissionsStr {
    pub fn as_str(&self) -> &str {
        // SAFETY: The `PermissionsStr` will contain only ascii.
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

impl fmt::Display for PermissionsStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for PermissionsStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PermissionsStr")
            .field(&self.as_str())
            .finish()
    }
}

impl fmt::Display for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_str())
    }
}

impl Permissions {
    pub const fn to_str(self) -> PermissionsStr {
        let mut bytes = [0xFF; 3 * 3];

        bytes[0] = if self.contains(Self::UserRead) {
            b'r'
        } else {
            b'-'
        };
        bytes[1] = if self.contains(Self::UserWrite) {
            b'w'
        } else {
            b'-'
        };
        bytes[2] = match (self.contains(Self::SetUID), self.contains(Self::UserExec)) {
            (true, true) => b's',
            (true, false) => b'S',
            (false, true) => b'x',
            (false, false) => b'-',
        };

        bytes[3] = if self.contains(Self::GroupRead) {
            b'r'
        } else {
            b'-'
        };
        bytes[4] = if self.contains(Self::GroupWrite) {
            b'w'
        } else {
            b'-'
        };
        bytes[5] = match (self.contains(Self::SetGID), self.contains(Self::GroupExec)) {
            (true, true) => b's',
            (true, false) => b'S',
            (false, true) => b'x',
            (false, false) => b'-',
        };

        bytes[6] = if self.contains(Self::OtherRead) {
            b'r'
        } else {
            b'-'
        };
        bytes[7] = if self.contains(Self::OtherWrite) {
            b'w'
        } else {
            b'-'
        };
        bytes[8] = match (self.contains(Self::Sticky), self.contains(Self::OtherExec)) {
            (true, true) => b't',
            (true, false) => b'T',
            (false, true) => b'x',
            (false, false) => b'-',
        };

        PermissionsStr(bytes)
    }
}
