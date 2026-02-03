use sqsh_rs::{Archive, Source};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ptr;

struct ReadSource {
    file: File,
    // Mapping from the buffer to the size
    outstanding_maps: HashMap<*mut u8, usize>,
}

unsafe impl Source for ReadSource {
    // Be mean: only read one byte at a time
    const BLOCK_SIZE_HINT: usize = 1;

    fn size(&mut self) -> sqsh_rs::Result<u64> {
        Ok(self.file.seek(SeekFrom::End(0)).unwrap())
    }

    unsafe fn map(&mut self, offset: u64, size: usize) -> sqsh_rs::Result<*mut u8> {
        let mut buf = vec![0; size].into_boxed_slice();
        self.file
            .seek(SeekFrom::Start(offset))
            .map_err(|_| sqsh_rs::ffi::SqshError::SQSH_ERROR_MAPPER_MAP)?;
        self.file.read_exact(&mut buf).unwrap();
        let ptr = Box::into_raw(buf).cast::<u8>();

        assert!(self.outstanding_maps.insert(ptr, size).is_none());
        Ok(ptr)
    }

    unsafe fn unmap(&mut self, ptr: *mut u8, size: usize) -> sqsh_rs::Result<()> {
        assert_eq!(self.outstanding_maps.remove(&ptr), Some(size));
        let ptr: *mut [u8] = ptr::slice_from_raw_parts_mut(ptr, size);
        drop(Box::from_raw(ptr));
        Ok(())
    }
}

impl Drop for ReadSource {
    fn drop(&mut self) {
        assert!(self.outstanding_maps.is_empty());
    }
}

#[test]
fn custom_source() {
    let archive = Archive::with_source(ReadSource {
        file: File::open("tests/data/test.sqsh").unwrap(),
        outstanding_maps: HashMap::new(),
    })
    .unwrap();

    let root = archive.root().unwrap();
    let mut traversal = root.traversal().unwrap();
    while let Some(entry) = traversal.advance().unwrap() {
        println!("{:?}", entry);
    }
}
