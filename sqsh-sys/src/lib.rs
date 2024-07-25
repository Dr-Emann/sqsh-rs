#![no_std]

// Ensure zlib linked in
extern crate flate2;

#[allow(non_camel_case_types)]
mod bindings;

pub use bindings::*;
