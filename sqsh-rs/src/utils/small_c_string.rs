use crate::{error, Error};
use sqsh_sys::SqshError;
use std::ffi::{CStr, CString};
use std::mem::MaybeUninit;
use std::ptr;
use std::slice;

const MAX_STACK_ALLOCATION: usize = 384;

#[inline]
pub fn run_with_cstr<T>(
    bytes: impl AsRef<[u8]>,
    f: impl FnOnce(&CStr) -> error::Result<T>,
) -> error::Result<T> {
    let bytes = bytes.as_ref();
    if bytes.len() >= MAX_STACK_ALLOCATION {
        run_with_cstr_allocating(bytes, f)
    } else {
        unsafe { run_with_cstr_stack(bytes, f) }
    }
}

/// # Safety
///
/// `bytes` must have a length less than `MAX_STACK_ALLOCATION`.
unsafe fn run_with_cstr_stack<T>(
    bytes: &[u8],
    f: impl FnOnce(&CStr) -> error::Result<T>,
) -> error::Result<T> {
    let mut buf = MaybeUninit::<[u8; MAX_STACK_ALLOCATION]>::uninit();
    let buf_ptr = buf.as_mut_ptr() as *mut u8;

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), buf_ptr, bytes.len());
        buf_ptr.add(bytes.len()).write(0);
    }

    let c = CStr::from_bytes_with_nul(unsafe { slice::from_raw_parts(buf_ptr, bytes.len() + 1) })
        .map_err(|_| Error(SqshError::SQSH_ERROR_INVALID_ARGUMENT))?;
    f(c)
}

#[cold]
#[inline(never)]
fn run_with_cstr_allocating<T>(
    bytes: &[u8],
    f: impl FnOnce(&CStr) -> error::Result<T>,
) -> error::Result<T> {
    match CString::new(bytes) {
        Ok(s) => f(&s),
        Err(_) => Err(Error(SqshError::SQSH_ERROR_INVALID_ARGUMENT)),
    }
}
