use bindgen::callbacks::ParseCallbacks;
use clap::{Parser, Subcommand};
use regex_lite::Regex;
use std::cell::Cell;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Regenerate the bindings
    Bindgen,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Bindgen => {
            println!("Running bindgen");
            let sqsh_sys = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("sqsh-sys");

            let bindings = bindgen::builder()
                .layout_tests(false)
                .allowlist_item("(?i-u:sqsh).*")
                .generate_cstr(true)
                .generate_comments(true)
                .merge_extern_blocks(true)
                .default_enum_style(bindgen::EnumVariation::NewType {
                    is_bitfield: false,
                    is_global: false,
                })
                .default_alias_style(bindgen::AliasVariation::TypeAlias)
                .enable_function_attribute_detection()
                .blocklist_type("FILE|mode_t|fpos_t|time_t|__.*")
                .raw_line("use libc::{mode_t, time_t, FILE};")
                .use_core()
                .sort_semantically(true)
                .parse_callbacks(Box::new(FixupDoxyComments::default()))
                .clang_arg(format!(
                    "-I{}/submodules/sqsh-tools/include",
                    sqsh_sys.display(),
                ))
                .header(format!(
                    "{}/submodules/sqsh-tools/include/sqsh.h",
                    sqsh_sys.display()
                ))
                .generate()
                .unwrap();

            let mut writer = Vec::new();

            bindings.write(Box::new(&mut writer)).unwrap();

            let mut data = String::from_utf8(writer).unwrap();
            data = DEPRECATED_FINDER
                .replace_all(&data, "$1#[deprecated]\n$1$2")
                .to_string();

            fs::write(sqsh_sys.join("src/bindings.rs"), data).unwrap();
        }
    }
}

static REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?mix)
^[\x20\t]*@(?:
(brief|memberof.*)|
(param(?:\[[a-z,]*\])?)|
(return)|
(retval)|
(deprecated)|
(internal|privatesection)|
[a-z]+
)"#,
    )
    .unwrap()
});

static DEPRECATED_FINDER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        &(String::from(r"(?m)^([ \t]*)(#\[doc\s*=.*)") + &regex_lite::escape(DEPRECATED_MAGIC)),
    )
    .unwrap()
});

#[derive(Debug, Default)]
struct FixupDoxyComments {
    seen_duplicated_deprecation: Cell<bool>,
}

const DEPRECATED_MAGIC: &str = "!!!DEPRECATED!!!";

// This deprecation message is duplicated for some reason. We only want it to apply once.
const MANY_DUPLICATE_DEPRECATION: &str = "Use SQSH_ERROR_UNKNOWN_FILE_TYPE instead";

impl ParseCallbacks for FixupDoxyComments {
    fn process_comment(&self, comment: &str) -> Option<String> {
        if comment.contains(MANY_DUPLICATE_DEPRECATION)
            && self.seen_duplicated_deprecation.replace(true)
        {
            // We've already seen this deprecation message, on a previous item.
            return Some(String::new());
        }
        let mut seen_args = false;
        let mut seen_returns = false;

        let result = REGEX.replace_all(comment, |caps: &regex_lite::Captures| {
            let replacement: &str = if caps.get(1).is_some() {
                ""
            } else if caps.get(2).is_some() {
                if !seen_args {
                    seen_args = true;
                    "# Arguments\n-"
                } else {
                    "-"
                }
            } else if caps.get(3).is_some() {
                seen_returns = true;
                "# Returns\n"
            } else if caps.get(4).is_some() {
                if seen_returns {
                    "-"
                } else {
                    seen_returns = true;
                    "# Returns\n-"
                }
            } else if caps.get(5).is_some() {
                DEPRECATED_MAGIC
            } else if let Some(m) = caps.get(6) {
                return match m.as_str() {
                    "internal" => r#"<div class="warning">INTERNAL API</div>"#,
                    "privatesection" => r#"<div class="warning">PRIVATE</div>"#,
                    _ => unreachable!(),
                };
            } else {
                unreachable!("Unknown doxygen tag @{}", caps.get(0).unwrap().as_str());
            };
            replacement
        });
        Some(result.into_owned())
    }
}
