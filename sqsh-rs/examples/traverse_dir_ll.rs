use sqsh_rs::Archive;
use std::env;

fn main() {
    let path = env::args_os().nth(1).expect("missing path argument");
    let archive = Archive::new(path).unwrap();
    let root = archive.root().unwrap();

    let mut traversal = root.traversal().unwrap();
    while let Some(entry) = traversal.advance().unwrap() {
        if entry.state().is_second_visit() {
            continue;
        }

        println!("/{}", entry.path());
    }
}
