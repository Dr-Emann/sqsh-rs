use sqsh_sys as ffi;
use sqsh_sys::SqshMapper;
use std::ffi::{c_int, c_void};
use std::ptr::NonNull;

pub(crate) struct SourceVtable<S> {
    inner: ffi::SqshMemoryMapperImpl,
    _phantom: std::marker::PhantomData<fn() -> S>,
}

impl<S: Source> SourceVtable<S> {
    pub(crate) fn mapper_impl(&self) -> &ffi::SqshMemoryMapperImpl {
        &self.inner
    }

    pub const fn new() -> Self {
        let block_size_hint = S::BLOCK_SIZE_HINT;
        assert!(
            block_size_hint > 0,
            "BLOCK_SIZE_HINT must be greater than 0"
        );
        let imp = ffi::SqshMemoryMapperImpl {
            block_size_hint,
            init: Some(init::<S>),
            map: Some(map::<S>),
            unmap: Some(unmap::<S>),
            cleanup: Some(cleanup::<S>),
        };
        Self {
            inner: imp,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<S: Source> Default for SourceVtable<S> {
    fn default() -> Self {
        Self::new()
    }
}

/// A trait for mapping sections of an archive into memory.
///
/// # Safety
///
/// Implementors must ensure that the `map` function returns a valid pointer to a buffer of `size`
/// bytes, and that the buffer remains valid until the `unmap` function is called.
pub unsafe trait Source {
    const BLOCK_SIZE_HINT: usize;

    /// Retrieve the size of the archive.
    fn size(&mut self) -> crate::error::Result<usize>;

    /// Map a section of a source into memory.
    ///
    /// The `offset` parameter is the offset in the archive starting from the
    /// beginning of the archive. That means that the mapper does not need to know about the
    /// configured offset for the archive. The `size` parameter is the size of the chunk
    /// that should be read. This function will return either a pointer to a buffer of `size` bytes
    /// or an error. The buffer must be allocated by the implementation and the pointer must be
    /// valid until the `unmap` function is called.
    ///
    /// The size requested may _not_ be the same as the BLOCK_SIZE_HINT, and the implementation
    /// must be able to handle any size requested.
    ///
    /// # Safety
    ///
    /// `map` must never be called for the same offset and size while the buffer is still mapped.
    /// The function must return a pointer to a buffer of `size` bytes.
    /// The buffer must remain valid until the `unmap` function is called.
    unsafe fn map(&mut self, offset: usize, size: usize) -> crate::error::Result<*mut u8>;

    /// Unmap a section of a source from memory.
    ///
    /// The `ptr` will be a pointer which was returned by the `map` function, and `size` will be the
    /// size passed to that map call.
    ///
    /// # Safety
    ///
    /// `ptr` must be a pointer returned by the `map` function, and `size` must be the size passed
    /// to that call.
    unsafe fn unmap(&mut self, ptr: *mut u8, size: usize) -> crate::error::Result<()>;
}

pub(crate) fn to_ptr<S: Source>(source: S) -> *mut c_void {
    let s_ptr = if size_of::<S>() == 0 {
        NonNull::dangling().as_ptr()
    } else {
        Box::into_raw(Box::new(source))
    };
    s_ptr.cast()
}

extern "C" fn init<S: Source>(
    mapper: *mut SqshMapper,
    input: *const c_void,
    size: *mut usize,
) -> c_int {
    let input = input.cast_mut();
    unsafe {
        let source: &mut S = &mut *input.cast::<S>();

        *size = match source.size() {
            Ok(size) => size,
            Err(e) => return e.to_ffi_result(),
        };

        ffi::sqsh_mapper_set_user_data(mapper, input);
    }
    0
}

extern "C" fn cleanup<S: Source>(mapper: *mut SqshMapper) -> c_int {
    if size_of::<S>() != 0 {
        drop(unsafe { Box::from_raw(ffi::sqsh_mapper_user_data(mapper).cast::<S>()) });
    }
    0
}

extern "C" fn map<S: Source>(
    mapper: *const SqshMapper,
    offset: usize,
    size: usize,
    ptr: *mut *mut u8,
) -> c_int {
    let source: &mut S = unsafe { &mut *ffi::sqsh_mapper_user_data(mapper).cast::<S>() };
    let res = unsafe { source.map(offset, size) };
    match res {
        Ok(p) => {
            unsafe { *ptr = p };
            0
        }
        Err(e) => e.to_ffi_result(),
    }
}

extern "C" fn unmap<S: Source>(mapper: *const SqshMapper, ptr: *mut u8, size: usize) -> c_int {
    let source: &mut S = unsafe { &mut *ffi::sqsh_mapper_user_data(mapper).cast::<S>() };
    let res = unsafe { source.unmap(ptr, size) };
    match res {
        Ok(()) => 0,
        Err(e) => e.to_ffi_result(),
    }
}
