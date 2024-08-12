use crate::{error, Archive, Inode, InodeRef};
use sqsh_sys as ffi;
use std::ptr;

impl Archive<'_> {
    pub fn export_table(&self) -> error::Result<ExportTable<'_>> {
        let mut dst = ptr::null_mut();
        let err = unsafe { ffi::sqsh_archive_export_table(self.inner.as_ptr(), &mut dst) };
        let inner = unsafe {
            match dst.as_ref() {
                Some(inner) => inner,
                None => return Err(error::new(err)),
            }
        };
        Ok(ExportTable { inner })
    }
}

/// A mapping from inode numbers to inode references.
///
/// A squashfs archive can optionally contain an export table, which maps inode numbers to inode
/// references.
pub struct ExportTable<'archive> {
    inner: &'archive ffi::SqshExportTable,
}

impl<'archive> ExportTable<'archive> {
    /// Retrieves an element from the export table.
    pub fn resolve_inode(&self, inode: Inode) -> error::Result<InodeRef> {
        let mut inode_ref = 0;
        let res = unsafe {
            ffi::sqsh_export_table_resolve_inode(self.inner, inode.index().into(), &mut inode_ref)
        };
        if res != 0 {
            return Err(error::new(res));
        }
        Ok(InodeRef(inode_ref))
    }
}
