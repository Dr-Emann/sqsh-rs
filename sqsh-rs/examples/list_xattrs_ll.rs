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
    while xattrs.advance().unwrap() {
        let prefix = xattrs.current_prefix();
        let name = BStr::new(xattrs.current_name());
        let value = BStr::new(xattrs.current_value());
        println!("{prefix}{name}={value}");
    }
}
