use crate::{error, File, FileType, Inode, InodeRef};
use bstr::BStr;
use sqsh_sys as ffi;
use std::ffi::c_char;
use std::fmt;
use std::ptr::NonNull;

pub struct DirectoryIterator<'file, 'archive> {
    inner: NonNull<ffi::SqshDirectoryIterator>,
    _marker: std::marker::PhantomData<&'file File<'archive>>,
}

#[derive(Clone, Copy)]
pub struct DirectoryEntry<'dir, 'archive> {
    inner: &'dir ffi::SqshDirectoryIterator,
    // Because 'dir is shorter (a subtype) of 'file, and we don't need 'file, we use
    // 'dir as the first parameter to DirectoryIterator
    _marker: std::marker::PhantomData<&'dir DirectoryIterator<'dir, 'archive>>,
}

impl<'file, 'archive> DirectoryIterator<'file, 'archive> {
    pub(crate) unsafe fn new(inner: NonNull<ffi::SqshDirectoryIterator>) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Advances the iterator to the next entry.
    ///
    /// Returns `true` if the iterator was advanced, or `false` if the end of the directory was reached.
    pub fn advance(&mut self) -> error::Result<Option<DirectoryEntry<'_, 'archive>>> {
        let mut err = 0;
        let did_advance =
            unsafe { ffi::sqsh_directory_iterator_next(self.inner.as_ptr(), &mut err) };
        if err == 0 {
            Ok(did_advance.then(|| unsafe { DirectoryEntry::new(self.inner.as_ref()) }))
        } else {
            Err(error::new(err))
        }
    }

    /// Looks up the given name in the current directory.
    pub fn advance_lookup(
        &mut self,
        name: &[u8],
    ) -> error::Result<Option<DirectoryEntry<'_, 'archive>>> {
        let err = unsafe {
            ffi::sqsh_directory_iterator_lookup(
                self.inner.as_ptr(),
                name.as_ptr().cast::<c_char>(),
                name.len(),
            )
        };
        if err == 0 {
            Ok(Some(unsafe { DirectoryEntry::new(self.inner.as_ref()) }))
        } else {
            let err = error::new(err);
            if err == error::Error(ffi::SqshError::SQSH_ERROR_NO_SUCH_FILE) {
                Ok(None)
            } else {
                Err(err)
            }
        }
    }
}

impl Drop for DirectoryIterator<'_, '_> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqsh_directory_iterator_free(self.inner.as_ptr());
        }
    }
}

impl<'dir, 'archive> DirectoryEntry<'dir, 'archive> {
    pub(crate) unsafe fn new(inner: &'dir ffi::SqshDirectoryIterator) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Retrieves the file type of the current entry.
    pub fn file_type(&self) -> Option<FileType> {
        let file_type = unsafe { ffi::sqsh_directory_iterator_file_type(self.inner) };
        FileType::try_from(file_type).ok()
    }

    /// Retrieves the name of the current entry.
    pub fn name(&self) -> &BStr {
        let mut size = 0;
        let data = unsafe { ffi::sqsh_directory_iterator_name2(self.inner, &mut size) };
        assert!(!data.is_null());
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size) };
        BStr::new(bytes)
    }

    /// Retrieves the inode number of the current entry.
    pub fn inode(&self) -> Inode {
        let inode_num = unsafe { ffi::sqsh_directory_iterator_inode(self.inner) };
        inode_num.try_into().unwrap()
    }

    /// Retrieves the inode ref of the current entry.
    pub fn inode_ref(&self) -> InodeRef {
        InodeRef(unsafe { ffi::sqsh_directory_iterator_inode_ref(self.inner) })
    }

    pub fn open(&self) -> error::Result<File<'archive>> {
        let mut err = 0;
        let file = unsafe { ffi::sqsh_directory_iterator_open_file(self.inner, &mut err) };
        let file = match NonNull::new(file) {
            Some(file) => file,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { File::new(file) })
    }
}

impl fmt::Debug for DirectoryEntry<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DirectoryEntry")
            .field("file_type", &self.file_type())
            .field("name", &self.name())
            .field("inode", &self.inode())
            .field("inode_ref", &self.inode_ref())
            .finish_non_exhaustive()
    }
}
