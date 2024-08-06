use crate::traverse::Traversal;
use crate::utils::small_c_string::run_with_cstr;
use crate::{
    error, Archive, DirectoryIterator, FileType, Inode, InodeRef, Permissions, Reader,
    XattrIterator,
};
use bstr::BStr;
use sqsh_sys as ffi;
use std::ffi::CStr;
use std::fmt;
use std::ptr::NonNull;

/// Methods for opening files on an archive.
impl Archive<'_> {
    pub fn open(&self, path: &str) -> error::Result<File<'_>> {
        run_with_cstr(path, |path| self.open_raw(path))
    }

    pub fn open_nofollow(&self, path: &str) -> error::Result<File<'_>> {
        run_with_cstr(path, |path| self.open_raw_nofollow(path))
    }

    pub fn open_raw(&self, path: &CStr) -> error::Result<File<'_>> {
        let mut err = 0;
        let file = unsafe { ffi::sqsh_open(self.inner.as_ptr(), path.as_ptr(), &mut err) };
        let file = match NonNull::new(file) {
            Some(file) => file,
            None => return Err(error::new(err)),
        };

        Ok(unsafe { File::new(file) })
    }

    pub fn open_raw_nofollow(&self, path: &CStr) -> error::Result<File<'_>> {
        let mut err = 0;
        let file = unsafe { ffi::sqsh_lopen(self.inner.as_ptr(), path.as_ptr(), &mut err) };
        let file = match NonNull::new(file) {
            Some(file) => file,
            None => return Err(error::new(err)),
        };

        Ok(unsafe { File::new(file) })
    }

    pub fn open_ref(&self, inode_ref: InodeRef) -> error::Result<File<'_>> {
        let mut err = 0;
        let file = unsafe { ffi::sqsh_open_by_ref(self.inner.as_ptr(), inode_ref.0, &mut err) };
        let file = match NonNull::new(file) {
            Some(file) => file,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { File::new(file) })
    }
}

pub struct File<'archive> {
    inner: NonNull<ffi::SqshFile>,
    _marker: std::marker::PhantomData<&'archive Archive<'archive>>,
}

impl<'archive> File<'archive> {
    pub(crate) unsafe fn new(inner: NonNull<ffi::SqshFile>) -> Self {
        Self {
            inner,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the type of the file.
    #[must_use]
    pub fn file_type(&self) -> Option<FileType> {
        let raw_file_type = unsafe { ffi::sqsh_file_type(self.inner.as_ptr()) };
        FileType::try_from(raw_file_type).ok()
    }

    /// Returns the permissions of the file.
    #[must_use]
    pub fn permissions(&self) -> Permissions {
        let raw_permissions = unsafe { ffi::sqsh_file_permission(self.inner.as_ptr()) };
        Permissions::from_bits_retain(raw_permissions)
    }

    /// Returns the inode reference of the file.
    #[must_use]
    pub fn inode_ref(&self) -> InodeRef {
        let inode_ref = unsafe { ffi::sqsh_file_inode_ref(self.inner.as_ptr()) };
        InodeRef(inode_ref)
    }

    /// Returns whether the file is an extended structure
    #[must_use]
    pub fn is_extended(&self) -> bool {
        unsafe { ffi::sqsh_file_is_extended(self.inner.as_ptr()) }
    }

    /// Getter for the inode hard link count
    #[must_use]
    pub fn hard_link_count(&self) -> u32 {
        unsafe { ffi::sqsh_file_hard_link_count(self.inner.as_ptr()) }
    }

    /// Getter for the file size. 0 if the file has no size.
    #[must_use]
    pub fn size(&self) -> u64 {
        unsafe { ffi::sqsh_file_size(self.inner.as_ptr()) }
    }

    /// Getter for the inode number.
    #[must_use]
    pub fn inode(&self) -> Inode {
        let inode_num = unsafe { ffi::sqsh_file_inode(self.inner.as_ptr()) };
        inode_num.try_into().unwrap()
    }

    /// Getter for the inode number of the parent directory.
    #[must_use]
    pub fn parent_inode(&self) -> Inode {
        let inode_num = unsafe { ffi::sqsh_file_directory_parent_inode(self.inner.as_ptr()) };
        inode_num.try_into().unwrap()
    }

    /// Follow a single symbolic link.
    ///
    /// After calling this function, the file is (in-place) changed to the target of the symlink.
    pub fn follow_symlink(&mut self) -> error::Result<()> {
        let err = unsafe { ffi::sqsh_file_symlink_resolve(self.inner.as_ptr()) };
        if err != 0 {
            return Err(error::new(err));
        }
        Ok(())
    }

    /// Follow symlink targets recursively until a non-symlink file is found.
    ///
    /// After calling this function, the file is (in-place) changed to the target of the symlink(s).
    ///
    /// This function performs recursion detection and will return an error if a
    /// symlink loop is detected.
    pub fn follow_all_symlinks(&mut self) -> error::Result<()> {
        let err = unsafe { ffi::sqsh_file_symlink_resolve_all(self.inner.as_ptr()) };
        if err != 0 {
            return Err(error::new(err));
        }
        Ok(())
    }

    /// Getter for the modification time.
    ///
    /// Returns the number of seconds since the Unix epoch.
    #[must_use]
    pub fn modified_time(&self) -> u32 {
        unsafe { ffi::sqsh_file_modified_time(self.inner.as_ptr()) }
    }

    /// Symbolic link target path.
    ///
    /// If this file is not a symbolic link, this will return `None`.
    #[must_use]
    pub fn symlink_path(&self) -> Option<&BStr> {
        let path_ptr = unsafe { ffi::sqsh_file_symlink(self.inner.as_ptr()) };
        if path_ptr.is_null() {
            return None;
        }
        let len = unsafe { ffi::sqsh_file_symlink_size(self.inner.as_ptr()) };
        let len = usize::try_from(len).unwrap();
        let bytes = unsafe { std::slice::from_raw_parts(path_ptr.cast::<u8>(), len) };
        Some(BStr::new(bytes))
    }

    /// Returns the device id of the device inode.
    #[must_use]
    pub fn device_id(&self) -> u32 {
        unsafe { ffi::sqsh_file_device_id(self.inner.as_ptr()) }
    }

    /// Returns the owner user id of the file.
    #[must_use]
    pub fn uid(&self) -> u32 {
        unsafe { ffi::sqsh_file_uid(self.inner.as_ptr()) }
    }

    /// Returns the owner group id of the file.
    #[must_use]
    pub fn gid(&self) -> u32 {
        unsafe { ffi::sqsh_file_gid(self.inner.as_ptr()) }
    }

    /// Returns index of the extended attribute inside of the xattr table.
    #[must_use]
    pub fn xattr_id(&self) -> u32 {
        unsafe { ffi::sqsh_file_xattr_index(self.inner.as_ptr()) }
    }

    /// Returns an iterator over the directory entries of the file.
    pub fn as_dir(&self) -> error::Result<DirectoryIterator<'archive>> {
        let mut err = 0;
        let dir_iter = unsafe { ffi::sqsh_directory_iterator_new(self.inner.as_ptr(), &mut err) };
        let dir_iter = match NonNull::new(dir_iter) {
            Some(dir_iter) => dir_iter,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { DirectoryIterator::new(dir_iter) })
    }

    pub fn xattrs(&self) -> error::Result<XattrIterator<'archive>> {
        let mut err = 0;
        let xattr_iter = unsafe { ffi::sqsh_xattr_iterator_new(self.inner.as_ptr(), &mut err) };
        let xattr_iter = match NonNull::new(xattr_iter) {
            Some(xattr_iter) => xattr_iter,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { XattrIterator::new(xattr_iter) })
    }

    /// Returns a new reader for the file.
    pub fn reader(&self) -> error::Result<Reader<'_>> {
        let mut err = 0;
        let iterator = unsafe { ffi::sqsh_file_iterator_new(self.inner.as_ptr(), &mut err) };
        let iterator = match NonNull::new(iterator) {
            Some(iterator) => iterator,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { Reader::new(iterator) })
    }

    pub fn traversal(&self) -> error::Result<Traversal> {
        let mut err = 0;
        let traversal = unsafe { ffi::sqsh_tree_traversal_new(self.inner.as_ptr(), &mut err) };
        let traversal = match NonNull::new(traversal) {
            Some(traversal) => traversal,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { Traversal::new(traversal) })
    }
}

impl<'archive> fmt::Debug for File<'archive> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File")
            .field("file_type", &self.file_type())
            .field("permissions", &self.permissions())
            .field("inode_ref", &self.inode_ref())
            .field("is_extended", &self.is_extended())
            .field("hard_link_count", &self.hard_link_count())
            .field("size", &self.size())
            .field("inode", &self.inode())
            .field("parent_inode", &self.parent_inode())
            .field("modified_time", &self.modified_time())
            .field("symlink_path", &self.symlink_path())
            .field("device_id", &self.device_id())
            .field("uid", &self.uid())
            .field("gid", &self.gid())
            .field("xattr_id", &self.xattr_id())
            .finish()
    }
}

impl<'archive> Drop for File<'archive> {
    fn drop(&mut self) {
        unsafe { ffi::sqsh_close(self.inner.as_ptr()) };
    }
}

unsafe impl<'archive> Send for File<'archive> {}
unsafe impl<'archive> Sync for File<'archive> {}
