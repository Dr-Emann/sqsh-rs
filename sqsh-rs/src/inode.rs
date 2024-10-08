use std::fmt;
use std::num::NonZeroU32;

/// A unique reference to an inode.
///
/// This identifies an inode by position in the archive.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InodeRef(pub u64);

/// An inode number.
///
/// All inodes are numbered sequentially starting from 1 (inode 0 is invalid).
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Inode(pub(crate) NonZeroU32);

impl Inode {
    /// Create an inode from an inode number.
    ///
    /// # Errors
    ///
    /// Returns an error if the inode number is zero.
    pub fn new(value: u32) -> Result<Self, ZeroInode> {
        NonZeroU32::new(value).map(Self).ok_or(ZeroInode)
    }

    /// The one-based index of the inode.
    #[must_use]
    pub fn index(self) -> u32 {
        self.0.get()
    }
}

/// An error indicating that an inode number was zero.
#[derive(Debug, Copy, Clone)]
pub struct ZeroInode;

impl fmt::Debug for InodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let upper = self.0 >> (32 + 16);
        let middle = (self.0 >> 16) & 0xFFFF_FFFF;
        let lower = self.0 & 0xFFFF;

        f.debug_tuple("InodeRef")
            .field(&format_args!("0x{upper:04X}_{middle:08X}_{lower:04X}"))
            .finish()
    }
}

impl fmt::Display for ZeroInode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid inode value: 0")
    }
}

impl std::error::Error for ZeroInode {}

impl TryFrom<u32> for Inode {
    type Error = ZeroInode;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
