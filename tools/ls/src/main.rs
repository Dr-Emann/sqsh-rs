use bstr::BStr;
use clap::Parser;
use color_eyre::eyre::{OptionExt, WrapErr};
use sqsh_rs::traverse::Entry;
use sqsh_rs::{Archive, FileType};
use std::fmt::Write as _;
use std::io::{self, stdout, IsTerminal, Write as _};
use std::path::{Path, PathBuf};

type PrintSegment = fn(&BStr) -> io::Result<()>;

type PrintItem = fn(&BStr, Entry, bool, PrintSegment) -> color_eyre::Result<()>;

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// The byte offset in the file where the archive begins.
    #[arg(short, long)]
    offset: Option<u64>,
    /// List files recursively under the specified directory
    #[arg(short, long)]
    recursive: bool,
    /// Output in long listing format
    #[arg(short, long)]
    long: bool,
    /// Output modification times in UTC
    #[arg(short, long, requires = "long")]
    utc: bool,
    /// Escape special characters in paths
    ///
    /// By default, special characters are escaped when
    /// output to stdout, this allows forcing escaping
    /// either on or off
    #[arg(
        short,
        long,
        value_enum,
        require_equals = true,
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    escape: Option<bool>,

    /// File containing the squashfs archive
    file: PathBuf,

    /// Directories to list
    paths: Vec<PathBuf>,
}

#[derive(Debug, Copy, Clone)]
struct Config {
    print_segment: PrintSegment,
    print_item: PrintItem,
    utc: bool,
}

impl Config {
    fn print_item(&self, prefix: &BStr, entry: Entry) -> color_eyre::Result<()> {
        (self.print_item)(prefix, entry, self.utc, self.print_segment)
    }
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install().unwrap();
    let cli = Cli::parse();

    let archive = Archive::new(&cli.file)
        .wrap_err_with(|| format!("Failed to open archive: {}", cli.file.display()))?;

    let should_escape = cli.escape.unwrap_or_else(|| stdout().is_terminal());
    let print_segment: PrintSegment = if should_escape {
        print_segment_escaped
    } else {
        print_segment_raw
    };
    let print_item: PrintItem = if cli.long {
        print_item_detail
    } else {
        print_item_simple
    };
    let config = Config {
        utc: cli.utc,
        print_segment,
        print_item,
    };

    if cli.paths.is_empty() {
        ls_path(&archive, Path::new(""), config, cli.recursive)?;
    } else {
        for path in &cli.paths {
            ls_path(&archive, path, config, cli.recursive)?;
        }
    }

    Ok(())
}

fn ls_path(
    archive: &Archive,
    root: &Path,
    config: Config,
    recursive: bool,
) -> color_eyre::Result<()> {
    let root = root.to_string_lossy();
    let root = root.trim_matches('/');
    let file = archive
        .open(root)
        .wrap_err_with(|| format!("unable to open directory {}", root))?;
    let mut traversal = file.traversal()?;
    if !recursive {
        traversal.set_max_depth(1);
    }

    while let Some(entry) = traversal.advance()? {
        let state = entry.state();
        if entry.depth() == 0 || state.is_second_visit() {
            continue;
        }
        config.print_item(BStr::new(root), entry)?;
    }
    Ok(())
}

fn print_segment_raw(segment: &BStr) -> io::Result<()> {
    write!(stdout(), "{}", segment)
}

fn print_segment_escaped(segment: &BStr) -> io::Result<()> {
    write!(stdout(), "{}", segment.escape_ascii())
}

fn print_path(prefix: &BStr, entry: Entry, print_segment: PrintSegment) -> io::Result<()> {
    let mut stdout = stdout();
    print_segment(prefix)?;
    for segment in entry.path().segments() {
        stdout.write_all(b"/")?;
        print_segment(segment)?;
    }
    Ok(())
}

fn print_item_simple(
    prefix: &BStr,
    entry: Entry,
    _utc: bool,
    print_segment: PrintSegment,
) -> color_eyre::Result<()> {
    print_path(prefix, entry, print_segment)?;
    stdout().write_all(b"\n")?;
    Ok(())
}

fn print_item_detail(
    prefix: &BStr,
    entry: Entry,
    utc: bool,
    print_segment: PrintSegment,
) -> color_eyre::Result<()> {
    let file = entry.open()?;

    let mut buffer = String::with_capacity(128);
    let file_type = file.file_type().ok_or_eyre("unknown file type")?;
    let type_ch = match file_type {
        FileType::Directory => 'd',
        FileType::File => '-',
        FileType::Symlink => 'l',
        FileType::BlockDevice => 'b',
        FileType::CharacterDevice => 'c',
        FileType::Fifo => 'p',
        FileType::Socket => 's',
    };
    buffer.push(type_ch);
    buffer.push_str(&file.permissions().to_str());
    write!(
        buffer,
        " {:>6} {:>6} {:>10}",
        file.uid(),
        file.gid(),
        file.size()
    )?;

    let mtime = file.modified_time();
    let ts = jiff::Timestamp::from_second(mtime.into())?;
    let zone = if utc {
        jiff::tz::TimeZone::UTC
    } else {
        jiff::tz::TimeZone::system()
    };
    let ts = ts.to_zoned(zone);
    write!(buffer, " {} ", ts.strftime("%a, %d %b %Y %T %z"))?;
    stdout().write_all(buffer.as_bytes())?;

    print_path(prefix, entry, print_segment)?;

    if let Some(target) = file.symlink_path() {
        write!(stdout(), " -> ")?;
        print_segment(target)?;
    }

    stdout().write_all(b"\n")?;
    Ok(())
}
