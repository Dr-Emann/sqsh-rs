use std::fmt;
use std::num::NonZeroU32;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InodeRef(pub u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Inode(pub(crate) NonZeroU32);

impl Inode {
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ZeroInode;

impl fmt::Debug for InodeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let upper = self.0 >> (32 + 16);
        let middle = (self.0 >> 16) & 0xFFFF_FFFF;
        let lower = self.0 & 0xFFFF;

        f.debug_tuple("InodeRef")
            .field(&format_args!(
                "0x{:04X}_{:08X}_{:04X}",
                upper, middle, lower
            ))
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
        NonZeroU32::new(value).map(Self).ok_or(ZeroInode)
    }
}
