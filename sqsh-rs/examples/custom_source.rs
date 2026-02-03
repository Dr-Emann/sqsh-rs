use sqsh_rs::Source;
use std::fs::File;
use std::io::{Read, Seek};
use std::ptr;

struct FileReadSource {
    file: File,
}

unsafe impl Source for FileReadSource {
    const BLOCK_SIZE_HINT: usize = 1024 * 1024;

    fn size(&mut self) -> sqsh_rs::Result<u64> {
        let size = self
            .file
            .seek(std::io::SeekFrom::End(0))
            .map_err(|_| sqsh_rs::ffi::SqshError::SQSH_ERROR_MAPPER_INIT)?;
        Ok(size)
    }

    unsafe fn map(&mut self, offset: u64, size: usize) -> sqsh_rs::Result<*mut u8> {
        let mut buf = vec![0; size].into_boxed_slice();
        self.file
            .seek(std::io::SeekFrom::Start(offset))
            .map_err(|_| sqsh_rs::ffi::SqshError::SQSH_ERROR_MAPPER_MAP)?;
        self.file.read_exact(&mut buf).unwrap();
        Ok(Box::into_raw(buf).cast())
    }

    unsafe fn unmap(&mut self, ptr: *mut u8, size: usize) -> sqsh_rs::Result<()> {
        let ptr: *mut [u8] = ptr::slice_from_raw_parts_mut(ptr, size);
        drop(Box::from_raw(ptr));
        Ok(())
    }
}

fn main() {
    let Some(path) = std::env::args_os().nth(1) else {
        eprintln!("Usage: {} <sqsh-file>", std::env::args().next().unwrap());
        std::process::exit(1);
    };
    let file = File::open(path).unwrap();
    let source = FileReadSource { file };
    let archive = sqsh_rs::Archive::with_source(source).unwrap();
    let root = archive.root().unwrap();
    let mut traversal = root.traversal().unwrap();
    while let Some(entry) = traversal.advance().unwrap() {
        println!("{:?}", entry);
    }
    drop(traversal);
    drop(root);
    drop(archive);
}
