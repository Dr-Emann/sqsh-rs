use crate::{error, Archive, Inode, InodeRef};
use sqsh_sys as ffi;
use std::ptr;

impl Archive<'_> {
    pub fn inode_map(&self) -> error::Result<InodeMap<'_>> {
        let mut dst = ptr::null_mut();
        let err = unsafe { ffi::sqsh_archive_inode_map(self.inner.as_ptr(), &mut dst) };
        let inner = unsafe {
            match dst.as_ref() {
                Some(inner) => inner,
                None => return Err(error::new(err)),
            }
        };
        Ok(InodeMap { inner })
    }
}

pub struct InodeMap<'archive> {
    inner: &'archive ffi::SqshInodeMap,
}

impl<'archive> InodeMap<'archive> {
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
