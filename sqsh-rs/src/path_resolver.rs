use crate::archive::Archive;
use crate::{error, File, FileType, Inode, InodeRef};
use bstr::BStr;
use sqsh_sys as ffi;
use std::ffi::{c_char, CStr, CString};
use std::fmt;
use std::ptr::NonNull;

/// A walker over the tree of entries in an archive.
///
/// This is a low-level interface to the archive, and is not recommended for general use.
pub struct PathResolver<'archive> {
    inner: NonNull<ffi::SqshPathResolver>,
    _marker: std::marker::PhantomData<&'archive Archive<'archive>>,
}

impl Archive<'_> {
    /// Create a new walker for the archive.
    ///
    /// The walker starts at the root directory of the archive.
    pub fn path_resolver(&self) -> error::Result<PathResolver<'_>> {
        let mut err = 0;
        let walker = unsafe { ffi::sqsh_path_resolver_new(self.inner.as_ptr(), &mut err) };
        let walker = match NonNull::new(walker) {
            Some(walker) => walker,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { PathResolver::new(walker) })
    }
}

impl<'archive> PathResolver<'archive> {
    pub(crate) unsafe fn new(inner: NonNull<ffi::SqshPathResolver>) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Return a new File for the current entry.
    pub fn open(&self) -> error::Result<File<'archive>> {
        let mut err = 0;
        let file = unsafe { ffi::sqsh_path_resolver_open_file(self.inner.as_ptr(), &mut err) };
        let file = match NonNull::new(file) {
            Some(file) => file,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { File::new(file) })
    }

    /// Attempt to move the resolver up to the parent directory.
    ///
    /// On success, the resolver will be positioned immediately before the first
    /// entry in the parent directory.
    pub fn up(&mut self) -> error::Result<()> {
        let err = unsafe { ffi::sqsh_path_resolver_up(self.inner.as_ptr()) };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Attempt to move the resolver down into the current entry.
    pub fn down(&mut self) -> error::Result<()> {
        let err = unsafe { ffi::sqsh_path_resolver_down(self.inner.as_ptr()) };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Moves the resolver to the next entry in the current directory.
    ///
    /// Returns `true` if the resolver was advanced, or `false` if the end of the directory was reached.
    pub fn advance(&mut self) -> error::Result<bool> {
        let mut err = 0;
        let did_advance = unsafe { ffi::sqsh_path_resolver_next(self.inner.as_ptr(), &mut err) };
        if err == 0 {
            Ok(did_advance)
        } else {
            Err(error::new(err))
        }
    }

    /// Reverts the resolver to the beginning of the current directory.
    pub fn revert(&mut self) -> error::Result<()> {
        let err = unsafe { ffi::sqsh_path_resolver_revert(self.inner.as_ptr()) };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Looks up an entry in the current directory.
    pub fn advance_lookup(&mut self, name: &[u8]) -> error::Result<()> {
        let err = unsafe {
            ffi::sqsh_path_resolver_lookup(
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

    /// Resets the resolver to the root directory.
    pub fn reset_to_root(&mut self) -> error::Result<()> {
        let err = unsafe { ffi::sqsh_path_resolver_to_root(self.inner.as_ptr()) };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Resolve a path with the resolver.
    ///
    /// The path is resolved relative to the current directory.
    pub fn resolve_path(&mut self, path: &str, follow_symlinks: bool) -> error::Result<()> {
        let path = CString::new(path)?;
        self.resolve_path_raw(&path, follow_symlinks)
    }

    /// Resolve a path with the resolver.
    ///
    /// The path is resolved relative to the current directory.
    pub fn resolve_path_raw(&mut self, path: &CStr, follow_symlinks: bool) -> error::Result<()> {
        let err = unsafe {
            ffi::sqsh_path_resolver_resolve(self.inner.as_ptr(), path.as_ptr(), follow_symlinks)
        };
        if err == 0 {
            Ok(())
        } else {
            Err(error::new(err))
        }
    }

    /// Returns the file type of the current entry.
    pub fn current_file_type(&self) -> Option<FileType> {
        let raw_file_type = unsafe { ffi::sqsh_path_resolver_type(self.inner.as_ptr()) };
        FileType::try_from(raw_file_type).ok()
    }

    /// Returns the inode number of the current directory
    pub fn current_dir_inode(&self) -> Inode {
        let inode_num = unsafe { ffi::sqsh_path_resolver_dir_inode(self.inner.as_ptr()) };
        inode_num.try_into().unwrap()
    }

    /// Returns an inode reference of the current directory
    pub fn current_dir_inode_ref(&self) -> InodeRef {
        InodeRef(unsafe { ffi::sqsh_path_resolver_inode_ref(self.inner.as_ptr()) })
    }

    /// Get the name of the current entry.
    ///
    /// This is borrows into the resolver itself: the following will not compile:
    ///
    /// ```compile_fail
    /// let archive = sqsh_rs::Archive::new("tests/data/test.sqsh").unwrap();
    /// let mut resolver = archive.path_resolver().unwrap();
    /// resolver.advance().unwrap();
    /// let name = resolver.current_name().unwrap();
    /// resolver.advance().unwrap();
    /// // Name is invalidated by moving the resolver
    /// println!("{:?}", name);
    /// ```
    pub fn current_name(&self) -> Option<&BStr> {
        let size = unsafe { ffi::sqsh_path_resolver_name_size(self.inner.as_ptr()) };
        let name = unsafe { ffi::sqsh_path_resolver_name(self.inner.as_ptr()) };
        if name.is_null() {
            None
        } else {
            let bytes = unsafe { std::slice::from_raw_parts(name.cast::<u8>(), usize::from(size)) };
            Some(BStr::new(bytes))
        }
    }
}

impl fmt::Debug for PathResolver<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current = self.open();
        f.debug_struct("PathResolver")
            .field("current_name", &self.current_name())
            .field("current_file_type", &self.current_file_type())
            .field("current_dir_inode", &self.current_dir_inode())
            .field("current_dir_inode_ref", &self.current_dir_inode_ref())
            .field("current_file", &current)
            .finish_non_exhaustive()
    }
}

unsafe impl<'archive> Send for PathResolver<'archive> {}
unsafe impl<'archive> Sync for PathResolver<'archive> {}

impl Drop for PathResolver<'_> {
    fn drop(&mut self) {
        unsafe { ffi::sqsh_path_resolver_free(self.inner.as_ptr()) };
    }
}
