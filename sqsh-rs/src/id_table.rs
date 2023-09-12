use sqsh_sys as ffi;

pub struct IdTable<'archive> {
    inner: &'archive ffi::SqshIdTable,
}

impl<'archive> IdTable<'archive> {
    pub(crate) unsafe fn new(inner: *const ffi::SqshIdTable) -> Self {
        let inner = unsafe { inner.as_ref().expect("null id table pointer") };
        Self { inner }
    }

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
