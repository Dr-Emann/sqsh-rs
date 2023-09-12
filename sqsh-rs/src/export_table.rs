use crate::{error, Inode, InodeRef};
use sqsh_sys as ffi;

pub struct ExportTable<'archive> {
    inner: &'archive ffi::SqshExportTable,
}

impl<'archive> ExportTable<'archive> {
    pub(crate) unsafe fn new(inner: *const ffi::SqshExportTable) -> Self {
        let inner = unsafe { inner.as_ref().expect("null export table pointer") };
        Self { inner }
    }

    /// Retrieves an element from the export table.
    pub fn resolve_inode(&self, inode: Inode) -> error::Result<InodeRef> {
        let mut inode_ref = 0;
        let res = unsafe {
            ffi::sqsh_export_table_resolve_inode(self.inner, inode.get().into(), &mut inode_ref)
        };
        if res != 0 {
            return Err(error::new(res));
        }
        Ok(InodeRef(inode_ref))
    }
}
