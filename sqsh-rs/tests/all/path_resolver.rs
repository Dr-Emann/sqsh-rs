use bstr::{BStr, ByteSlice};
use sqsh_rs::FileType;

#[test]
fn navigation() {
    let archive = crate::archive();
    let root = archive.root().unwrap();
    let mut resolver = archive.path_resolver().unwrap();

    assert_eq!(resolver.current_name(), None);
    assert_eq!(resolver.current_dir_inode_ref(), root.inode_ref());
    assert_eq!(resolver.current_dir_inode(), root.parent_inode());
    insta::assert_debug_snapshot!("root", resolver);

    assert!(resolver.up().is_err());

    resolver.advance().unwrap();
    assert_eq!(resolver.current_dir_inode(), root.inode());
    assert_ne!(resolver.current_dir_inode_ref(), root.inode_ref());
    insta::assert_debug_snapshot!("first_file", resolver);

    resolver.advance_lookup(b"subdir").unwrap();
    assert_eq!(
        resolver.open().unwrap().inode_ref(),
        archive.open("subdir").unwrap().inode_ref()
    );
    assert_eq!(resolver.current_dir_inode(), root.inode());
    assert_ne!(resolver.current_dir_inode_ref(), root.inode_ref());
    assert_eq!(resolver.current_name(), Some(BStr::new("subdir")));
    assert_eq!(resolver.current_file_type(), Some(FileType::Directory));
    insta::assert_debug_snapshot!("at_subdir", resolver);

    resolver.down().unwrap();
    insta::assert_debug_snapshot!("in_subdir", resolver);

    assert!(resolver.advance().unwrap());
    insta::assert_debug_snapshot!("first_subdir_file", resolver);
    let first_file = archive
        .open(&format!(
            "subdir/{}",
            resolver.current_name().unwrap().to_str().unwrap()
        ))
        .unwrap();
    assert_eq!(first_file.inode_ref(), resolver.open().unwrap().inode_ref());

    // subdir only contains files
    assert!(resolver.down().is_err());

    assert!(resolver.advance().unwrap());
    insta::assert_debug_snapshot!("second_subdir_file", resolver);
    // Only two items
    assert!(!resolver.advance().unwrap());
}
