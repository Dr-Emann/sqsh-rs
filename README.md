# Sqsh-rs [![Crates.io](https://img.shields.io/crates/v/sqsh-rs)](https://crates.io/crates/sqsh-rs) [![Docs.rs](https://docs.rs/sqsh-rs/badge.svg)](https://docs.rs/sqsh-rs) [![License](https://img.shields.io/crates/l/sqsh-rs)](LICENSE)

A Rust wrapper for the [libsqsh] library.

## Example

This is a simple example that a) prints the content of a file and b) lists the
content of a directory.

```rust
use std::io::Write;
use sqsh_rs::Archive;

fn example() -> std::io::Result<()> {
    let mut archive = Archive::new("tests/data/test.sqsh")?;
    let contents: Vec<u8> = archive.read("/subdir/short.file")?;
    std::io::stdout().write_all(&contents)?;

    let directory = archive.open("/subdir")?;
    let mut iter = directory.as_dir()?;
    while let Some(entry) = iter.advance()? {
        println!("{}", entry.name());
    }

    Ok(())
}

example().unwrap();
```

[libsqsh]: https://github.com/Gottox/sqsh-tools