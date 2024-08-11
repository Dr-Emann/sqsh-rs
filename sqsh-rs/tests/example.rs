use bstr::BString;
use sqsh_rs::traverse::Traversal;
use sqsh_rs::{Archive, DirectoryIterator, FileType, Permissions};
use std::fmt::Write;
use std::io::{BufRead, Read};

mod all;

const ARCHIVE_PATH: &str = "tests/data/test.sqsh";

fn archive() -> Archive<'static> {
    Archive::new(ARCHIVE_PATH).unwrap()
}

#[test]
fn open_archive() {
    let _archive = archive();
}

#[test]
fn mem_open_archive() {
    let data = std::fs::read(ARCHIVE_PATH).unwrap();
    let _archive = Archive::from_slice(&data).unwrap();
}

#[test]
fn superblock() {
    let archive = archive();
    let superblock = archive.superblock();
    insta::assert_debug_snapshot!(superblock);
}

#[test]
fn easy_contents_not_exists() {
    let archive = archive();
    let err = archive.read("not_exists").unwrap_err();
    assert_eq!(err.io_error_kind(), std::io::ErrorKind::NotFound);
    insta::assert_snapshot!(err, @"No such file or directory");
}

#[test]
fn easy_contents_empty() {
    let archive = archive();
    let data = archive.read("empty.file").unwrap();
    assert!(data.is_empty());
}

#[test]
fn easy_contents_one() {
    let archive = archive();
    let data = archive.read("one.file").unwrap();
    assert_eq!(data.len(), 1);
    assert_eq!(data, "a".as_bytes());
}

#[test]
fn easy_contents_of_directory() {
    let archive = archive();
    let err = archive.read("subdir").unwrap_err();
    assert_eq!(err.io_error_kind(), std::io::ErrorKind::Other);
    insta::assert_snapshot!(err, @"Not a file");
}

#[test]
fn easy_permissions() {
    let archive = archive();
    let permissions = archive.permissions("one.file").unwrap();
    assert_eq!(
        permissions,
        Permissions::UserRW | Permissions::GroupRead | Permissions::OtherRead
    );
}

#[test]
fn open_file() {
    let archive = archive();
    let file = archive.open("one.file").unwrap();
    assert!(!file.is_extended());
}

#[test]
fn open_dir() {
    let archive = archive();
    let dir = archive.open("subdir").unwrap();
    assert_eq!(dir.file_type(), Some(sqsh_rs::FileType::Directory));
    insta::assert_debug_snapshot!("subdir debug", dir);
}

#[test]
fn reopen_by_id() {
    let archive = archive();
    let file1 = archive.open("one.file").unwrap();
    let inode_ref = file1.inode_ref();
    let file2 = archive.open_ref(inode_ref).unwrap();
    assert_eq!(format!("{file1:?}"), format!("{file2:?}"));
}

#[test]
fn reader_read_by_byte() {
    let archive = archive();
    let file = archive.open("short.file").unwrap();
    let mut reader = file.reader().unwrap();
    let mut buf = [0u8; 1];
    for i in 0..4 {
        let n = reader.read(&mut buf).unwrap();
        assert_eq!(n, 1);
        let expected = match i {
            0 => b'a',
            1 => b'b',
            2 => b'c',
            3 => b'\n',
            _ => unreachable!(),
        };
        assert_eq!(buf[0], expected);
    }
    let n = reader.read(&mut buf).unwrap();
    assert_eq!(n, 0);
}

#[test]
fn reader_buf_read() {
    let archive = archive();
    let file = archive.open("1MiB.file").unwrap();
    let mut reader = file.reader().unwrap();
    let mut total_size = 0;
    loop {
        let buf = reader.fill_buf().unwrap();
        if buf.is_empty() {
            break;
        }
        let len = buf.len();
        assert!(buf.iter().all(|&b| b == b'A'));
        total_size += buf.len();

        // Only consume part of the buffer, ensure that the remaining part is returned by fill_buf
        reader.consume(2);
        let remaining_buf = reader.fill_buf().unwrap();
        assert_eq!(remaining_buf.len(), len - 2);
        reader.consume(len - 2);
    }
    assert_eq!(total_size, 1024 * 1024);
}

#[test]
fn resolver() {
    let archive = archive();
    let mut resolver = archive.path_resolver().unwrap();

    assert_eq!(resolver.current_name(), None);
    assert_eq!(resolver.current_file_type(), Some(FileType::Directory));

    let current = resolver.open().unwrap();
    let root = archive.open("").unwrap();
    assert_eq!(format!("{current:?}"), format!("{root:?}"));

    resolver.advance().unwrap();

    assert_eq!(resolver.current_name(), Some(bstr::BStr::new(b"1MiB.file")));
    assert_eq!(resolver.current_file_type(), Some(FileType::File));

    let current = resolver.open().unwrap();
    assert_eq!(current.size(), 1024 * 1024);
}

#[test]
fn walk_whole_dir() {
    let archive = archive();
    let mut path_resolver = archive.path_resolver().unwrap();

    let mut names = Vec::new();
    while path_resolver.advance().unwrap() {
        names.push(
            std::str::from_utf8(path_resolver.current_name().unwrap())
                .unwrap()
                .to_owned(),
        );
    }

    insta::assert_debug_snapshot!(names);
}

#[test]
fn recursive_directory() {
    #[derive(Debug)]
    #[allow(dead_code)]
    enum Tree {
        File(BString),
        Directory(BString, Vec<Tree>),
    }
    fn visit_directory(mut iter: DirectoryIterator) -> Vec<Tree> {
        let mut trees = Vec::new();
        while let Some(entry) = iter.advance().unwrap() {
            let name = entry.name();
            let tree = if entry.file_type() == Some(FileType::Directory) {
                let dir = entry.open().unwrap();
                let children = visit_directory(dir.as_dir().unwrap());
                Tree::Directory(name.into(), children)
            } else {
                Tree::File(name.into())
            };
            trees.push(tree);
        }
        trees
    }
    let archive = archive();
    let root = archive.root().unwrap();
    let trees = visit_directory(root.as_dir().unwrap());
    insta::assert_debug_snapshot!(trees);
}

#[test]
fn skip_to_last_byte() {
    let archive = archive();
    let file = archive.open("1MiB.file").unwrap();
    assert_eq!(file.size(), 1024 * 1024);

    let mut reader = file.reader().unwrap();
    assert!(reader.block_size() < 1024 * 1024);

    let mut buf = [0u8; 2];

    // Read a partial block in the front
    assert_eq!(reader.read(&mut buf).unwrap(), 2);
    assert_eq!(buf, [b'A', b'A']);

    // Then skip ahead to the last byte
    reader.skip(1024 * 1024 - buf.len() as u64 - 1).unwrap();
    let n = reader.read(&mut buf).unwrap();
    assert_eq!(n, 1);
    assert_eq!(buf[0], b'A');
}

#[test]
fn skip_past_end() {
    let archive = archive();
    let file = archive.open("1MiB.file").unwrap();
    assert_eq!(file.size(), 1024 * 1024);

    let mut reader = file.reader().unwrap();

    assert_eq!(
        reader.skip(1024 * 1024 + 1).unwrap_err().to_string(),
        "Out of bounds"
    );
}

#[test]
fn compression_options() {
    let archive = archive();
    insta::assert_debug_snapshot!(archive.compression_options());
}

fn traversal_str(traversal: &mut Traversal) -> String {
    let mut result = String::new();
    while let Some(entry) = traversal.advance().unwrap() {
        writeln!(
            result,
            "/{} {:?} {}\n{:?}",
            entry.path(),
            entry.state(),
            entry.depth(),
            entry.directory_entry(),
        )
        .unwrap();

        assert_eq!(
            entry.path().segments().next_back().unwrap_or_default(),
            entry.name(),
        );
    }
    result
}

#[test]
fn traverse() {
    let archive = archive();
    let root = archive.root().unwrap();
    let mut traversal = root.traversal().unwrap();
    insta::assert_snapshot!(traversal_str(&mut traversal));
}

#[test]
fn traverse_start_subdir() {
    let archive = archive();
    let subdir = archive.open("subdir").unwrap();

    let mut traversal = subdir.traversal().unwrap();
    insta::assert_snapshot!(traversal_str(&mut traversal));
}

#[test]
fn traverse_start_file() {
    let archive = archive();
    let file = archive.open("/one.file").unwrap();

    let mut traversal = file.traversal().unwrap();
    insta::assert_snapshot!(traversal_str(&mut traversal));
}

#[test]
fn traverse_max_depth() {
    let archive = archive();
    let root = archive.root().unwrap();
    let mut traversal = root.traversal().unwrap();
    traversal.set_max_depth(1);

    insta::assert_snapshot!(traversal_str(&mut traversal));
}
