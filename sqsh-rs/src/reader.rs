use crate::{error, Archive, Error};
use sqsh_sys as ffi;
use std::io;
use std::io::BufRead;
use std::marker::PhantomData;
use std::ptr::NonNull;

pub struct Reader<'archive> {
    inner: NonNull<ffi::SqshFileIterator>,
    consumed: usize,
    _marker: PhantomData<&'archive Archive<'archive>>,
}

impl<'archive> Reader<'archive> {
    pub(crate) unsafe fn new(inner: NonNull<ffi::SqshFileIterator>) -> Self {
        Self {
            inner,
            consumed: 0,
            _marker: PhantomData,
        }
    }

    #[must_use]
    pub fn block_size(&self) -> usize {
        unsafe { ffi::sqsh_file_iterator_block_size(self.inner.as_ptr()) }
    }

    /// Skip `n` bytes in the file.
    pub fn skip(&mut self, mut n: u64) -> error::Result<()> {
        // Offset is measured from the _start_ of the current block
        n = n.saturating_add(self.consumed.try_into().unwrap());
        self.consumed = 0;

        // Loop because we want a 64 bit offset, but need to skip in usize chunks.
        while n > 0 {
            let n_usize: usize = n.try_into().unwrap_or(usize::MAX);
            n -= n_usize as u64;

            let mut offset_remaining = n_usize;
            let err = unsafe {
                ffi::sqsh_file_iterator_skip(self.inner.as_ptr(), &mut offset_remaining, usize::MAX)
            };

            if err != 0 {
                return Err(error::new(err));
            }
            debug_assert!(self.current_chunk_size() >= offset_remaining);
            self.consume(offset_remaining);
        }

        Ok(())
    }

    fn current_chunk_size(&self) -> usize {
        unsafe { ffi::sqsh_file_iterator_size(self.inner.as_ptr()) }
    }

    pub(crate) fn fill_buf_raw(&mut self) -> error::Result<&[u8]> {
        let mut size = self.current_chunk_size();
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
            size = self.current_chunk_size();
            debug_assert!(size > 0);
        }
        let data_ptr = unsafe { ffi::sqsh_file_iterator_data(self.inner.as_ptr()) };
        let data = unsafe { std::slice::from_raw_parts(data_ptr, size) };
        Ok(&data[self.consumed..])
    }
}

impl<'archive> io::Read for Reader<'archive> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let src = self.fill_buf()?;
        let len = src.len().min(buf.len());
        buf[..len].copy_from_slice(&src[..len]);
        self.consume(len);
        Ok(len)
    }
}

impl<'archive> BufRead for Reader<'archive> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.fill_buf_raw().map_err(Error::into_io_error)
    }

    fn consume(&mut self, amt: usize) {
        self.consumed += amt;
    }
}

unsafe impl Send for Reader<'_> {}

impl<'archive> Drop for Reader<'archive> {
    fn drop(&mut self) {
        unsafe { ffi::sqsh_file_iterator_free(self.inner.as_ptr()) };
    }
}
