use bstr::{BString, ByteVec};
use sqsh_rs::Archive;
use std::env;

fn main() {
    let path = env::args_os().nth(1).expect("missing path argument");
    let archive = Archive::new(path).unwrap();

    let mut pending_inode_refs = vec![(BString::from("."), archive.superblock().root_inode_ref())];

    while let Some((path, inode_ref)) = pending_inode_refs.pop() {
        let file = archive.open_ref(inode_ref).unwrap();
        println!(
            "{path:<20} {:7} {:7} {:7}",
            file.uid(),
            file.gid(),
            file.size()
        );

        if file.file_type() == Some(sqsh_rs::FileType::Directory) {
            let mut iter = file.as_dir().unwrap();
            while let Some(entry) = iter.advance() {
                let entry = entry.unwrap();
                let name = entry.name();
                let mut path = path.clone();
                if !path.is_empty() {
                    path.push_char('/');
                }
                path.extend_from_slice(name);
                pending_inode_refs.push((path, entry.inode_ref()));
            }
        }
    }
}
