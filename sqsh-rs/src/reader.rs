use crate::{error, Error, File};
use sqsh_sys as ffi;
use std::io;
use std::io::BufRead;
use std::marker::PhantomData;
use std::ptr::NonNull;

pub struct Reader<'file> {
    inner: NonNull<ffi::SqshFileIterator>,
    consumed: usize,
    _marker: PhantomData<&'file File<'file>>,
}

impl<'file> Reader<'file> {
    pub(crate) unsafe fn new(inner: NonNull<ffi::SqshFileIterator>) -> Self {
        Self {
            inner,
            consumed: 0,
            _marker: PhantomData,
        }
    }

    pub fn block_size(&self) -> usize {
        unsafe { ffi::sqsh_file_iterator_block_size(self.inner.as_ptr()) }
    }

    pub(crate) fn fill_buf_raw(&mut self) -> error::Result<&[u8]> {
        let mut size = unsafe { ffi::sqsh_file_iterator_size(self.inner.as_ptr()) };
        if self.consumed >= size {
            self.consumed = 0;
            let mut err = 0;
            let iter_advanced =
                unsafe { ffi::sqsh_file_iterator_next(self.inner.as_ptr(), usize::MAX, &mut err) };
            if !iter_advanced {
                return if err == 0 {
                    Ok(&[])
                } else {
                    Err(error::new(err))
                };
            }
            size = unsafe { ffi::sqsh_file_iterator_size(self.inner.as_ptr()) };
            debug_assert!(size > 0);
        }
        let data_ptr = unsafe { ffi::sqsh_file_iterator_data(self.inner.as_ptr()) };
        let data = unsafe { std::slice::from_raw_parts(data_ptr, size) };
        Ok(&data[self.consumed..])
    }
}

impl<'file> io::Read for Reader<'file> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let src = self.fill_buf()?;
        let len = src.len().min(buf.len());
        buf[..len].copy_from_slice(&src[..len]);
        self.consume(len);
        Ok(len)
    }
}

impl<'file> BufRead for Reader<'file> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.fill_buf_raw().map_err(Error::into_io_error)
    }

    fn consume(&mut self, amt: usize) {
        self.consumed += amt;
    }
}

unsafe impl<'file> Send for Reader<'file> {}
unsafe impl<'file> Sync for Reader<'file> {}

impl<'file> Drop for Reader<'file> {
    fn drop(&mut self) {
        unsafe { ffi::sqsh_file_iterator_free(self.inner.as_ptr()) };
    }
}
