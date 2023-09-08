use crate::{error, File, FileType, InodeRef};
use bstr::BStr;
use sqsh_sys as ffi;
use std::ffi::c_char;
use std::ptr::NonNull;

pub struct DirectoryIterator<'a> {
    inner: NonNull<ffi::SqshDirectoryIterator>,
    _marker: std::marker::PhantomData<&'a File<'a>>,
}

pub struct DirectoryEntry<'dir, 'archive> {
    inner: NonNull<ffi::SqshDirectoryIterator>,
    _marker: std::marker::PhantomData<&'dir mut DirectoryIterator<'archive>>,
}

impl<'a> DirectoryIterator<'a> {
    pub(crate) unsafe fn new(inner: NonNull<ffi::SqshDirectoryIterator>) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Advances the iterator to the next entry.
    ///
    /// Returns `true` if the iterator was advanced, or `false` if the end of the directory was reached.
    pub fn advance(&mut self) -> Option<error::Result<DirectoryEntry<'_, 'a>>> {
        let mut err = 0;
        let did_advance =
            unsafe { ffi::sqsh_directory_iterator_next(self.inner.as_ptr(), &mut err) };
        if err == 0 {
            if did_advance {
                Some(Ok(DirectoryEntry {
                    inner: self.inner,
                    _marker: std::marker::PhantomData,
                }))
            } else {
                None
            }
        } else {
            Some(Err(error::new(err)))
        }
    }

    /// Looks up the given name in the current directory.
    pub fn advance_lookup(&mut self, name: &[u8]) -> Option<error::Result<DirectoryEntry<'_, 'a>>> {
        let err = unsafe {
            ffi::sqsh_directory_iterator_lookup(
                self.inner.as_ptr(),
                name.as_ptr().cast::<c_char>(),
                name.len(),
            )
        };
        if err == 0 {
            Some(Ok(DirectoryEntry {
                inner: self.inner,
                _marker: std::marker::PhantomData,
            }))
        } else {
            let err = error::new(err);
            if err == error::Error(ffi::SqshError::SQSH_ERROR_NO_SUCH_FILE) {
                None
            } else {
                Some(Err(err))
            }
        }
    }
}

impl<'a> Drop for DirectoryIterator<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqsh_directory_iterator_free(self.inner.as_ptr());
        }
    }
}

impl<'dir, 'archive> DirectoryEntry<'dir, 'archive> {
    /// Retrieves the file type of the current entry.
    pub fn file_type(&self) -> Option<FileType> {
        let file_type = unsafe { ffi::sqsh_directory_iterator_file_type(self.inner.as_ptr()) };
        FileType::try_from(file_type).ok()
    }

    /// Retrieves the name of the current entry.
    pub fn name(&self) -> &BStr {
        let size = unsafe { ffi::sqsh_directory_iterator_name_size(self.inner.as_ptr()) };
        let data = unsafe { ffi::sqsh_directory_iterator_name(self.inner.as_ptr()) };
        assert!(!data.is_null());
        let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), usize::from(size)) };
        BStr::new(bytes)
    }

    /// Retrieves the inode number of the current entry.
    pub fn inode_number(&self) -> u32 {
        unsafe { ffi::sqsh_directory_iterator_inode(self.inner.as_ptr()) }
    }

    /// Retrieves the inode ref of the current entry.
    pub fn inode_ref(&self) -> InodeRef {
        InodeRef(unsafe { ffi::sqsh_directory_iterator_inode_ref(self.inner.as_ptr()) })
    }

    pub fn open(&self) -> error::Result<File<'archive>> {
        let mut err = 0;
        let file = unsafe { ffi::sqsh_directory_iterator_open_file(self.inner.as_ptr(), &mut err) };
        let file = match NonNull::new(file) {
            Some(file) => file,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { File::new(file) })
    }
}
