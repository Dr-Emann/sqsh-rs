use crate::archive;
use insta::{assert_debug_snapshot, assert_snapshot};
use sqsh_rs::Inode;

#[test]
fn inode_map_retrieves_valid_inode() {
    let archive = archive();
    let inode_map = archive.inode_map().unwrap();
    let inode_ref = inode_map.get(Inode::new(1).unwrap()).unwrap();
    assert_debug_snapshot!(inode_ref, @r###"
    InodeRef(
        0x0000_00000000_0000,
    )
    "###);

    let root = archive.root().unwrap();
    let mut traversal = root.traversal().unwrap();
    while let Some(entry) = traversal.advance().unwrap() {
        let entry_ref = entry.open().unwrap().inode_ref();
        if entry_ref == inode_ref {
            assert_snapshot!(entry.path(), @"1MiB.file");
            break;
        }
    }
}

#[test]
fn inode_map_fails_on_invalid_inode() {
    let archive = archive();
    let inode_map = archive.inode_map().unwrap();
    let result = inode_map.get(Inode::new(u32::MAX).unwrap());
    assert!(result.is_err());
}
