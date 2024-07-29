#![no_std]

// Ensure the compression libraries are linked in.
#[cfg(feature = "zlib")]
extern crate libz_sys;
#[cfg(feature = "lz4")]
extern crate lz4_sys;
#[cfg(feature = "lzma")]
extern crate lzma_sys;
#[cfg(feature = "zstd")]
extern crate zstd_sys;

#[allow(non_camel_case_types)]
mod bindings;

pub use bindings::*;
