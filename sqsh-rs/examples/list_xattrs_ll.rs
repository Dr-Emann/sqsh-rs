use bstr::BStr;
use sqsh_rs::Archive;

fn main() {
    let archive_path = std::env::args_os()
        .nth(1)
        .expect("missing archive path argument");
    let file_path = std::env::args().nth(2).expect("missing file path argument");
    let archive = Archive::new(archive_path).unwrap();
    let file = archive.open(&file_path).unwrap();
    let mut xattrs = file.xattrs().unwrap();
    while let Some(xattr) = xattrs.advance().unwrap() {
        let prefix = xattr.prefix();
        let name = BStr::new(xattr.name());
        let value = BStr::new(xattr.value());
        println!("{prefix}{name}={value}");
    }
}
