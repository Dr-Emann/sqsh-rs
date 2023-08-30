use sqsh_rs::Archive;
use std::io;
use std::io::{BufRead, Write};

fn main() {
    let archive_path = std::env::args_os()
        .nth(1)
        .expect("missing archive path argument");
    let file_path = std::env::args().nth(2).expect("missing file path argument");

    let archive = Archive::new(archive_path).unwrap();
    let file = archive.open(&file_path).unwrap();
    let mut file_reader = file.reader().unwrap();

    let mut stdout = io::stdout().lock();
    loop {
        let buf = file_reader.fill_buf().unwrap();
        if buf.is_empty() {
            break;
        }
        stdout.write_all(buf).unwrap();
        let len = buf.len();
        file_reader.consume(len);
    }
}
