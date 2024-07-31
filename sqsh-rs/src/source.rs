use crate::utils::small_c_string::run_with_cstr;
use sqsh_sys as ffi;
use std::ffi::c_void;
use std::path::Path;

mod sealed {
    use sqsh_sys as ffi;
    use std::ffi::c_void;

    pub unsafe trait SourceData<'a> {
        /// Represents the addressable size of the source in bytes
        ///
        /// Only really useful for slice-based sources, files know their own length
        fn source_mapper(&self) -> *const ffi::SqshMemoryMapperImpl;

        fn size(&self) -> u64;

        /// This is a callback because some sources (paths) may need to create a new object and pass
        /// a pointer to that (to add a null terminator)
        fn with_source_pointer<O, F>(&self, f: F) -> crate::Result<O>
        where
            F: FnOnce(*const c_void) -> O;
    }
}

pub trait Source<'a>: sealed::SourceData<'a> {}

impl<'a, S: sealed::SourceData<'a>> Source<'a> for S {}

unsafe impl<'a> sealed::SourceData<'a> for &'a [u8] {
    fn source_mapper(&self) -> *const ffi::SqshMemoryMapperImpl {
        unsafe { ffi::sqsh_mapper_impl_static }
    }

    fn size(&self) -> u64 {
        self.len().try_into().unwrap()
    }

    fn with_source_pointer<O, F>(&self, f: F) -> crate::Result<O>
    where
        F: FnOnce(*const c_void) -> O,
    {
        Ok(f(self.as_ptr().cast()))
    }
}

unsafe impl sealed::SourceData<'static> for &Path {
    fn source_mapper(&self) -> *const ffi::SqshMemoryMapperImpl {
        unsafe { ffi::sqsh_mapper_impl_mmap }
    }

    fn size(&self) -> u64 {
        0
    }

    fn with_source_pointer<O, F>(&self, f: F) -> crate::Result<O>
    where
        F: FnOnce(*const c_void) -> O,
    {
        run_with_cstr(self.as_os_str().as_encoded_bytes(), |cstr| {
            Ok(f(cstr.as_ptr().cast()))
        })
    }
}
