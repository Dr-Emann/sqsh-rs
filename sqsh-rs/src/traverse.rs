use crate::{error, Archive, File, FileType};
use bstr::BStr;
use sqsh_sys as ffi;
use std::fmt;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::ptr::NonNull;

/// An efficient traversal of the archive.
///
/// This is an efficient but low-level interface to the archive.
///
/// This acts as a lending iterator (the entries borrow from the iterator), and so can't actually
/// implement the `Iterator` trait.
pub struct Traversal<'archive> {
    inner: NonNull<ffi::SqshTreeTraversal>,
    _marker: PhantomData<&'archive Archive<'archive>>,
}

#[derive(Copy, Clone)]
pub struct Entry<'traversal, 'archive> {
    inner: &'traversal ffi::SqshTreeTraversal,
    _marker: PhantomData<&'traversal Traversal<'archive>>,
}

#[derive(Copy, Clone)]
pub struct Path<'traversal> {
    entry: Entry<'traversal, 'traversal>,
}

/// An iterator over the segments of a path.
#[derive(Debug, Clone)]
pub struct PathSegments<'traversal> {
    inner: &'traversal ffi::SqshTreeTraversal,
    depth: usize,
    index: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The traversal iterator is currently pointing at an object it will visit only once.
    ///
    /// This includes regular files, special files like named pipes, symlinks, or devices, and
    /// directories that will not be descended into because of a configured max depth.
    Normal,
    /// The traversal is currently at a directory entry, the traversal will enter the directory
    DirectoryFirst,
    /// The traversal is currently at a directory entry, the traversal just exited the directory
    ///
    /// It may be desirable to skip this entry, since it was visited in [`Self::DirectoryFirst`]
    /// already.
    DirectorySecond,
}

impl State {
    pub fn is_second_visit(self) -> bool {
        self == Self::DirectorySecond
    }
}

impl<'archive> Traversal<'archive> {
    pub(crate) unsafe fn new(inner: NonNull<ffi::SqshTreeTraversal>) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }

    pub fn set_max_depth(&mut self, max_depth: usize) {
        unsafe { ffi::sqsh_tree_traversal_set_max_depth(self.inner.as_ptr(), max_depth) }
    }

    /// Attempt to advance the traversal to the next entry.
    pub fn advance(&mut self) -> error::Result<Option<Entry<'_, 'archive>>> {
        let mut err = 0;
        let has_next = unsafe { ffi::sqsh_tree_traversal_next(self.inner.as_ptr(), &mut err) };
        if err != 0 {
            return Err(error::new(err));
        }
        Ok(has_next.then_some(Entry {
            inner: unsafe { self.inner.as_ref() },
            _marker: PhantomData,
        }))
    }
}

impl Drop for Traversal<'_> {
    fn drop(&mut self) {
        unsafe {
            ffi::sqsh_tree_traversal_free(self.inner.as_ptr());
        }
    }
}

impl<'traversal, 'archive> Entry<'traversal, 'archive> {
    /// The depth of this entry.
    ///
    /// The root entry has a depth of 0.
    pub fn depth(self) -> usize {
        unsafe { ffi::sqsh_tree_traversal_depth(self.inner) }
    }

    /// The name of this entry.
    ///
    /// This is the same as the last [path segment][Self::path_segments], or an empty string if
    /// there are no path segments.
    ///
    /// The root entry has an empty name.
    pub fn name(self) -> &'traversal BStr {
        let mut len = 0;
        let ptr = unsafe { ffi::sqsh_tree_traversal_name(self.inner, &mut len) };
        let slice = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len) };
        BStr::new(slice)
    }

    /// The path of this entry.
    ///
    /// This will be the relative path from the root of the traversal to this entry.
    pub fn path(self) -> Path<'traversal> {
        Path::new(self)
    }

    /// Open the current entry.
    pub fn open(self) -> error::Result<File<'archive>> {
        let mut err = 0;
        let file = unsafe { ffi::sqsh_tree_traversal_open_file(self.inner, &mut err) };
        let file = match NonNull::new(file) {
            Some(file) => file,
            None => return Err(error::new(err)),
        };
        Ok(unsafe { File::new(file) })
    }

    pub fn file_type(self) -> FileType {
        let file_type = unsafe { ffi::sqsh_tree_traversal_type(self.inner) };
        FileType::try_from(file_type).unwrap()
    }

    /// The directory entry for this entry. This will be present for everything but the root entry.
    pub fn directory_entry(self) -> Option<crate::directory::DirectoryEntry<'traversal, 'archive>> {
        let iterator = unsafe { ffi::sqsh_tree_traversal_iterator(self.inner) };
        if iterator.is_null() {
            return None;
        }
        Some(unsafe { crate::directory::DirectoryEntry::new(&*iterator) })
    }

    pub fn state(self) -> State {
        let state = unsafe { ffi::sqsh_tree_traversal_state(self.inner) };
        match state {
            ffi::SqshTreeTraversalState::SQSH_TREE_TRAVERSAL_STATE_INIT
            | ffi::SqshTreeTraversalState::SQSH_TREE_TRAVERSAL_STATE_FILE => State::Normal,
            ffi::SqshTreeTraversalState::SQSH_TREE_TRAVERSAL_STATE_DIRECTORY_BEGIN => {
                State::DirectoryFirst
            }
            ffi::SqshTreeTraversalState::SQSH_TREE_TRAVERSAL_STATE_DIRECTORY_END => {
                State::DirectorySecond
            }
            _ => {
                debug_assert!(false, "unexpected state");
                State::Normal
            }
        }
    }
}

impl fmt::Debug for Entry<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Entry")
            .field("state", &self.state())
            .field("depth", &self.depth())
            .field("path", &self.path())
            .field("file_type", &self.file_type())
            .finish_non_exhaustive()
    }
}

impl<'traversal> Path<'traversal> {
    pub(crate) fn new(entry: Entry<'traversal, '_>) -> Self {
        Self { entry }
    }

    /// The path segments of the current entry.
    ///
    /// This is an iterator over the segments of the path (relative to the root of the traversal)
    ///
    /// For example, if the current entry is `foo/bar/baz`,
    /// the path segments are `["foo", "bar", "baz"]`.
    pub fn segments(self) -> PathSegments<'traversal> {
        PathSegments::new(self.entry)
    }
}

impl fmt::Debug for Path<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}

impl fmt::Display for Path<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut segments = self.segments();
        if let Some(segment) = segments.next() {
            write!(f, "{}", segment)?;
            for segment in segments {
                write!(f, "/{}", segment)?;
            }
        }
        Ok(())
    }
}

impl<'a> PathSegments<'a> {
    pub(crate) fn new(entry: Entry<'a, '_>) -> Self {
        Self {
            inner: entry.inner,
            depth: entry.depth(),
            index: 0,
        }
    }

    fn segment(&self, index: usize) -> &'a BStr {
        let mut len = 0;
        let segment = unsafe { ffi::sqsh_tree_traversal_path_segment(self.inner, &mut len, index) };

        debug_assert!(!segment.is_null());
        let slice = unsafe { std::slice::from_raw_parts(segment.cast::<u8>(), len) };
        BStr::new(slice)
    }
}

impl<'a> Iterator for PathSegments<'a> {
    type Item = &'a BStr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.depth {
            return None;
        }

        let result = self.segment(self.index);
        self.index += 1;
        Some(result)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.index = self.depth.min(self.index.saturating_add(n));
        self.next()
    }
}

impl ExactSizeIterator for PathSegments<'_> {
    fn len(&self) -> usize {
        self.depth - self.index
    }
}

impl FusedIterator for PathSegments<'_> {}

impl<'a> DoubleEndedIterator for PathSegments<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.index >= self.depth {
            return None;
        }
        self.depth -= 1;
        Some(self.segment(self.depth))
    }

    fn nth_back(&mut self, n: usize) -> Option<Self::Item> {
        self.depth = self.index.max(self.depth.saturating_sub(n));
        self.next_back()
    }
}
