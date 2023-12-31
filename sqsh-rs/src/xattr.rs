use crate::{error, File};
use bstr::BStr;
use sqsh_sys as ffi;
use std::ptr::NonNull;

pub struct XattrIterator<'file> {
    inner: NonNull<ffi::SqshXattrIterator>,
    _marker: std::marker::PhantomData<&'file File<'file>>,
}

pub struct XattrEntry<'file> {
    inner: NonNull<ffi::SqshXattrIterator>,
    _marker: std::marker::PhantomData<&'file XattrIterator<'file>>,
}

impl<'file> XattrIterator<'file> {
    pub(crate) unsafe fn new(inner: NonNull<ffi::SqshXattrIterator>) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Advances the iterator to the next entry.
    ///
    /// Returns `true` if the iterator was advanced, or `false` if the end of the directory was reached.
    pub fn advance(&mut self) -> error::Result<Option<XattrEntry<'_>>> {
        let mut err = 0;
        let did_advance = unsafe { ffi::sqsh_xattr_iterator_next(self.inner.as_ptr(), &mut err) };
        if err == 0 {
            Ok(did_advance.then_some(XattrEntry {
                inner: self.inner,
                _marker: std::marker::PhantomData,
            }))
        } else {
            Err(error::new(err))
        }
    }
}

impl<'file> XattrEntry<'file> {
    /// Retrieves the prefix of the current entry.
    pub fn prefix(&self) -> &'static BStr {
        let size = unsafe { ffi::sqsh_xattr_iterator_prefix_size(self.inner.as_ptr()) };
        let data = unsafe { ffi::sqsh_xattr_iterator_prefix(self.inner.as_ptr()) };
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), usize::from(size)) };
        BStr::new(bytes)
    }

    /// Retrieves the name of the current entry.
    pub fn name(&self) -> &BStr {
        let size = unsafe { ffi::sqsh_xattr_iterator_name_size(self.inner.as_ptr()) };
        let data = unsafe { ffi::sqsh_xattr_iterator_name(self.inner.as_ptr()) };
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), usize::from(size)) };
        BStr::new(bytes)
    }

    /// Retrieves the value of the current entry.
    pub fn value(&self) -> &BStr {
        let size = unsafe { ffi::sqsh_xattr_iterator_value_size(self.inner.as_ptr()) };
        let data = unsafe { ffi::sqsh_xattr_iterator_value(self.inner.as_ptr()) };
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), usize::from(size)) };
        BStr::new(bytes)
    }

    /// Retrieves whether the current entry is indirect.
    pub fn is_indirect(&self) -> bool {
        unsafe { ffi::sqsh_xattr_iterator_is_indirect(self.inner.as_ptr()) }
    }

    /// Retrieves the type of the current entry.
    pub fn kind(&self) -> Option<XattrType> {
        let file_type = unsafe { ffi::sqsh_xattr_iterator_type(self.inner.as_ptr()) };
        let file_type = ffi::SqshXattrType(file_type as _);
        XattrType::try_from(file_type).ok()
    }
}

impl<'file> Drop for XattrIterator<'file> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqsh_xattr_iterator_free(self.inner.as_ptr());
        }
    }
}

pub enum XattrType {
    User,
    Trusted,
    Security,
}

impl TryFrom<ffi::SqshXattrType> for XattrType {
    type Error = ();

    fn try_from(value: ffi::SqshXattrType) -> Result<Self, Self::Error> {
        match value {
            ffi::SqshXattrType::SQSH_XATTR_USER => Ok(Self::User),
            ffi::SqshXattrType::SQSH_XATTR_TRUSTED => Ok(Self::Trusted),
            ffi::SqshXattrType::SQSH_XATTR_SECURITY => Ok(Self::Security),
            _ => Err(()),
        }
    }
}
