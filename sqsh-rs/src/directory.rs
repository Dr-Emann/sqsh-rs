use crate::{error, File, FileType, InodeRef};
use bstr::BStr;
use sqsh_sys as ffi;
use std::ffi::c_char;
use std::ptr::NonNull;

pub struct DirectoryIterator<'a> {
    inner: NonNull<ffi::SqshDirectoryIterator>,
    _marker: std::marker::PhantomData<&'a File<'a>>,
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
    pub fn advance(&mut self) -> error::Result<bool> {
        let mut err = 0;
        let did_advance =
            unsafe { ffi::sqsh_directory_iterator_next(self.inner.as_ptr(), &mut err) };
        if err == 0 {
            Ok(did_advance)
        } else {
            Err(error::new(err))
        }
    }

    /// Looks up the given name in the current directory.
    pub fn advance_lookup(&mut self, name: &[u8]) -> error::Result<()> {
        let err = unsafe {
            ffi::sqsh_directory_iterator_lookup(
                self.inner.as_ptr(),
                name.as_ptr().cast::<c_char>(),
                name.len(),
            )
        };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Retrieves the file type of the current entry.
    pub fn current_file_type(&self) -> Option<FileType> {
        let file_type = unsafe { ffi::sqsh_directory_iterator_file_type(self.inner.as_ptr()) };
        FileType::try_from(file_type).ok()
    }

    /// Retrieves the name of the current entry.
    pub fn current_name(&self) -> Option<&BStr> {
        let size = unsafe { ffi::sqsh_directory_iterator_name_size(self.inner.as_ptr()) };
        let data = unsafe { ffi::sqsh_directory_iterator_name(self.inner.as_ptr()) };
        if data.is_null() {
            None
        } else {
            let bytes = unsafe { std::slice::from_raw_parts(data.cast::<u8>(), usize::from(size)) };
            Some(BStr::new(bytes))
        }
    }

    /// Retrieves the inode number of the current entry.
    pub fn current_inode_number(&self) -> u32 {
        unsafe { ffi::sqsh_directory_iterator_inode(self.inner.as_ptr()) }
    }

    /// Retrieves the inode ref of the current entry.
    pub fn current_inode_ref(&self) -> InodeRef {
        InodeRef(unsafe { ffi::sqsh_directory_iterator_inode_ref(self.inner.as_ptr()) })
    }

    pub fn open(&self) -> error::Result<File<'a>> {
        let mut err = 0;
        let file = unsafe { ffi::sqsh_directory_iterator_open_file(self.inner.as_ptr(), &mut err) };
        let file = match NonNull::new(file) {
            Some(file) => file,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { File::new(file) })
    }
}

impl<'a> Drop for DirectoryIterator<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqsh_directory_iterator_free(self.inner.as_ptr());
        }
    }
}
