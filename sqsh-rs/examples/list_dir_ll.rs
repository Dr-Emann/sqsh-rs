use bstr::BStr;
use sqsh_rs::Archive;
use std::env;

fn main() {
    let path = env::args_os().nth(1).expect("missing path argument");
    let archive = Archive::new(path).unwrap();
    let superblock = archive.superblock();
    let file = archive.open_ref(superblock.root_inode_ref()).unwrap();

    let mut iterator = file.as_dir().unwrap();
    while iterator.advance().unwrap() {
        let name = iterator.current_name().unwrap();
        println!("{}", BStr::new(name));
    }
}
