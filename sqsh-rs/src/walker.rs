use crate::{error, Archive, File, FileType};
use bstr::BStr;
use sqsh_sys as ffi;
use std::ffi::{c_char, CStr, CString};
use std::fmt;
use std::ptr::NonNull;

/// A walker over the tree of entries in an archive.
///
/// This is a low-level interface to the archive, and is not recommended for general use.
pub struct Walker<'archive> {
    inner: NonNull<ffi::SqshTreeWalker>,
    _marker: std::marker::PhantomData<&'archive Archive>,
}

impl Archive {
    /// Create a new walker for the archive.
    ///
    /// The walker starts at the root directory of the archive.
    pub fn walker(&self) -> error::Result<Walker<'_>> {
        let mut err = 0;
        let walker = unsafe { ffi::sqsh_tree_walker_new(self.inner.as_ptr(), &mut err) };
        let walker = match NonNull::new(walker) {
            Some(walker) => walker,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { Walker::new(walker) })
    }
}

impl<'archive> Walker<'archive> {
    pub(crate) unsafe fn new(inner: NonNull<ffi::SqshTreeWalker>) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Return a new File for the current entry.
    pub fn open(&self) -> error::Result<File<'archive>> {
        let mut err = 0;
        let file = unsafe { ffi::sqsh_tree_walker_open_file(self.inner.as_ptr(), &mut err) };
        let file = match NonNull::new(file) {
            Some(file) => file,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { File::new(file) })
    }

    /// Attempt to move the walker up to the parent directory.
    pub fn up(&mut self) -> error::Result<()> {
        let err = unsafe { ffi::sqsh_tree_walker_up(self.inner.as_ptr()) };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Attempt to move the walker down into the current entry.
    pub fn down(&mut self) -> error::Result<()> {
        let err = unsafe { ffi::sqsh_tree_walker_down(self.inner.as_ptr()) };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Moves the walker to the next entry in the current directory.
    ///
    /// Returns `true` if the walker was advanced, or `false` if the end of the directory was reached.
    pub fn advance(&mut self) -> error::Result<bool> {
        let mut err = 0;
        let did_advance = unsafe { ffi::sqsh_tree_walker_next2(self.inner.as_ptr(), &mut err) };
        if err == 0 {
            Ok(did_advance)
        } else {
            Err(error::new(err))
        }
    }

    /// Reverts the walker to the beginning of the current directory.
    pub fn revert(&mut self) -> error::Result<()> {
        let err = unsafe { ffi::sqsh_tree_walker_revert(self.inner.as_ptr()) };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Looks up an entry in the current directory.
    pub fn advance_lookup(&mut self, name: &[u8]) -> error::Result<()> {
        let err = unsafe {
            ffi::sqsh_tree_walker_lookup(
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

    /// Resets the walker to the root directory.
    pub fn reset_to_root(&mut self) -> error::Result<()> {
        let err = unsafe { ffi::sqsh_tree_walker_to_root(self.inner.as_ptr()) };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Resolve a path with the tree walker.
    ///
    /// The path is resolved relative to the current directory.
    pub fn resolve_path(&mut self, path: &str, follow_symlinks: bool) -> error::Result<()> {
        let path = CString::new(path)?;
        self.resolve_path_raw(&path, follow_symlinks)
    }

    /// Resolve a path with the tree walker.
    ///
    /// The path is resolved relative to the current directory.
    pub fn resolve_path_raw(&mut self, path: &CStr, follow_symlinks: bool) -> error::Result<()> {
        let err = unsafe {
            ffi::sqsh_tree_walker_resolve(self.inner.as_ptr(), path.as_ptr(), follow_symlinks)
        };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Returns the file type of the current entry.
    pub fn current_file_type(&self) -> Option<FileType> {
        let raw_file_type = unsafe { ffi::sqsh_tree_walker_type(self.inner.as_ptr()) };
        FileType::try_from(raw_file_type).ok()
    }

    /// Get the name of the current entry.
    ///
    /// This is borrows into the walker itself: the following will not compile:
    ///
    /// ```compile_fail
    /// let archive = sqsh_rs::Archive::new("tests/data/test.sqsh").unwrap();
    /// let mut walker = archive.walker().unwrap();
    /// walker.advance().unwrap();
    /// let name = walker.current_name().unwrap();
    /// walker.advance().unwrap();
    /// // Name is invalidated by moving the walker
    /// println!("{:?}", name);
    /// ```
    pub fn current_name(&self) -> Option<&BStr> {
        let size = unsafe { ffi::sqsh_tree_walker_name_size(self.inner.as_ptr()) };
        let name = unsafe { ffi::sqsh_tree_walker_name(self.inner.as_ptr()) };
        if name.is_null() {
            None
        } else {
            let bytes = unsafe { std::slice::from_raw_parts(name.cast::<u8>(), usize::from(size)) };
            Some(BStr::new(bytes))
        }
    }
}

impl fmt::Debug for Walker<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current = self.open();
        f.debug_struct("Walker")
            .field("current_name", &self.current_name())
            .field("current_file_type", &self.current_file_type())
            .field("current_file", &current)
            .finish_non_exhaustive()
    }
}

unsafe impl<'archive> Send for Walker<'archive> {}
unsafe impl<'archive> Sync for Walker<'archive> {}

impl Drop for Walker<'_> {
    fn drop(&mut self) {
        unsafe { ffi::sqsh_tree_walker_free(self.inner.as_ptr()) };
    }
}
