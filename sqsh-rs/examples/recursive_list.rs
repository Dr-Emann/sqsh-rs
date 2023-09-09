use bstr::{BStr, ByteVec};
use sqsh_rs::{Archive, DirectoryIterator};
use std::env;

fn visit_directory(path_so_far: &BStr, mut iter: DirectoryIterator<'_, '_>) -> sqsh_rs::Result<()> {
    while let Some(entry) = iter.advance() {
        let entry = entry.unwrap();
        let name = entry.name();
        let mut path = path_so_far.to_owned();
        if !path.is_empty() {
            path.push_char('/');
        }
        path.extend_from_slice(name);
        let file = entry.open().unwrap();
        println!(
            "{path:<20} {:7} {:7} {:7}",
            file.uid(),
            file.gid(),
            file.size()
        );
        if entry.file_type() == Some(sqsh_rs::FileType::Directory) {
            let iter = file.as_dir()?;
            visit_directory(BStr::new(&path), iter)?;
        }
    }
    Ok(())
}

fn main() {
    let path = env::args_os().nth(1).expect("missing path argument");
    let archive = Archive::new(path).unwrap();

    let root = archive.root().unwrap();
    let path = BStr::new("");
    visit_directory(path, root.as_dir().unwrap()).unwrap();
}
