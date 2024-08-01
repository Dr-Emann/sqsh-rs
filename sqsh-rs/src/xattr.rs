use crate::{error, File};
use bstr::BStr;
use sqsh_sys as ffi;
use std::fmt;
use std::ptr::NonNull;

pub struct XattrIterator<'file> {
    inner: NonNull<ffi::SqshXattrIterator>,
    _marker: std::marker::PhantomData<&'file File<'file>>,
}

pub struct XattrEntry<'it> {
    inner: &'it ffi::SqshXattrIterator,
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
                inner: unsafe { self.inner.as_ref() },
            }))
        } else {
            Err(error::new(err))
        }
    }
}

impl<'file> XattrEntry<'file> {
    /// Retrieves the prefix of the current entry.
    pub fn prefix(&self) -> &'static BStr {
        let size = unsafe { ffi::sqsh_xattr_iterator_prefix_size(self.inner) };
        let data = unsafe { ffi::sqsh_xattr_iterator_prefix(self.inner) };
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), usize::from(size)) };
        BStr::new(bytes)
    }

    /// Retrieves the name of the current entry.
    pub fn name(&self) -> &BStr {
        let size = unsafe { ffi::sqsh_xattr_iterator_name_size(self.inner) };
        let data = unsafe { ffi::sqsh_xattr_iterator_name(self.inner) };
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), usize::from(size)) };
        BStr::new(bytes)
    }

    /// Retrieves the value of the current entry.
    pub fn value(&self) -> &BStr {
        let size = unsafe { ffi::sqsh_xattr_iterator_value_size2(self.inner) };
        let data = unsafe { ffi::sqsh_xattr_iterator_value(self.inner) };
        let bytes = unsafe {
            std::slice::from_raw_parts(data.cast::<u8>(), usize::try_from(size).unwrap())
        };
        BStr::new(bytes)
    }

    /// Retrieves whether the current entry is indirect.
    pub fn is_indirect(&self) -> bool {
        unsafe { ffi::sqsh_xattr_iterator_is_indirect(self.inner) }
    }

    /// Retrieves the type of the current entry.
    pub fn kind(&self) -> Option<XattrType> {
        let file_type = unsafe { ffi::sqsh_xattr_iterator_type(self.inner) };
        let file_type = ffi::SqshXattrType(u32::from(file_type));
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UnknownXattrType(ffi::SqshXattrType);

impl fmt::Display for UnknownXattrType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("unknown xattr type")
    }
}
impl std::error::Error for UnknownXattrType {}

pub enum XattrType {
    User,
    Trusted,
    Security,
}

impl TryFrom<ffi::SqshXattrType> for XattrType {
    type Error = UnknownXattrType;

    fn try_from(value: ffi::SqshXattrType) -> Result<Self, Self::Error> {
        match value {
            ffi::SqshXattrType::SQSH_XATTR_USER => Ok(Self::User),
            ffi::SqshXattrType::SQSH_XATTR_TRUSTED => Ok(Self::Trusted),
            ffi::SqshXattrType::SQSH_XATTR_SECURITY => Ok(Self::Security),
            other => Err(UnknownXattrType(other)),
        }
    }
}
