use crate::{error, Archive};
use sqsh_sys as ffi;
use std::ptr;

impl Archive<'_> {
    pub fn id_table(&self) -> error::Result<IdTable<'_>> {
        let mut dst = ptr::null_mut();
        let err = unsafe { ffi::sqsh_archive_id_table(self.inner.as_ptr(), &mut dst) };
        let inner = unsafe {
            match dst.as_ref() {
                Some(inner) => inner,
                None => return Err(error::new(err)),
            }
        };
        Ok(IdTable { inner })
    }
}

pub struct IdTable<'archive> {
    inner: &'archive ffi::SqshIdTable,
}

impl<'archive> IdTable<'archive> {
    #[must_use]
    pub fn get(&self, index: usize) -> Option<u32> {
        let mut id = 0;
        let err = unsafe { ffi::sqsh_id_table_get(self.inner, index, &mut id) };
        if err == 0 {
            Some(id)
        } else {
            None
        }
    }
}
