use bstr::BString;
use sqsh_rs::{Archive, DirectoryIterator, FileType, Permissions};
use std::io::{BufRead, Read};

const ARCHIVE_PATH: &str = "tests/data/test.sqsh";

fn archive() -> Archive {
    Archive::new(ARCHIVE_PATH).unwrap()
}

#[test]
fn open_archive() {
    let _archive = archive();
}

#[test]
fn mem_open_archive() {
    let data = std::fs::read(ARCHIVE_PATH).unwrap();
    let _archive = Archive::mem_new(&data).unwrap();
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
    insta::assert_display_snapshot!(err, @"No such file or directory");
}

#[test]
fn easy_contents_empty() {
    let archive = archive();
    let data = archive.read("empty.file").unwrap();
    assert!(data.is_empty());
    assert!(!data.as_ptr().is_null());
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
    insta::assert_display_snapshot!(err, @"Not a file");
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
fn walker() {
    let archive = archive();
    let mut walker = archive.walker().unwrap();

    assert_eq!(walker.current_name(), None);
    assert_eq!(walker.current_file_type(), Some(FileType::Directory));

    let current = walker.open().unwrap();
    let root = archive.open("").unwrap();
    assert_eq!(format!("{current:?}"), format!("{root:?}"));

    walker.advance().unwrap();

    assert_eq!(walker.current_name(), Some(bstr::BStr::new(b"1MiB.file")));
    assert_eq!(walker.current_file_type(), Some(FileType::File));

    let current = walker.open().unwrap();
    assert_eq!(current.size(), 1024 * 1024);
}

#[test]
fn walk_whole_dir() {
    let archive = archive();
    let mut walker = archive.walker().unwrap();
    let expected_entries = [
        "1MiB.file",
        "broken.link",
        "dev",
        "empty.file",
        "empty_dir",
        "fifo",
        "one.file",
        "short.file",
        "short.link",
        "socket",
        "socket2",
        "subdir",
    ];

    let mut names = Vec::new();
    while walker.advance().unwrap() {
        names.push(
            std::str::from_utf8(walker.current_name().unwrap())
                .unwrap()
                .to_owned(),
        );
    }

    assert_eq!(names, expected_entries);
}

#[test]
fn recursive_directory() {
    #[derive(Debug)]
    enum Tree {
        File(BString),
        Directory(BString, Vec<Tree>),
    }
    fn visit_directory(mut iter: DirectoryIterator<'_>) -> Vec<Tree> {
        let mut trees = Vec::new();
        while let Some(entry) = iter.advance() {
            let entry = entry.unwrap();
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
