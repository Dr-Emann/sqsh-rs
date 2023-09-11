use crate::{error, Inode, InodeRef};
use sqsh_sys as ffi;

pub struct InodeMap<'a> {
    inner: &'a ffi::SqshInodeMap,
}

impl<'a> InodeMap<'a> {
    pub(crate) unsafe fn new(inner: *const ffi::SqshInodeMap) -> Self {
        let inner = unsafe { inner.as_ref().expect("null inode map pointer") };
        Self { inner }
    }

    /// Gets the inode reference for a given inode number.
    pub fn get(&self, inode_number: Inode) -> error::Result<InodeRef> {
        let mut err = 0;
        let inode_ref =
            unsafe { ffi::sqsh_inode_map_get2(self.inner, inode_number.0.get(), &mut err) };
        if err != 0 {
            return Err(error::new(err));
        }
        Ok(InodeRef(inode_ref))
    }

    // TODO: sqsh_inode_map_set
}
