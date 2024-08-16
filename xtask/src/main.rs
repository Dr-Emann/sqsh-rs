use bindgen::callbacks::ParseCallbacks;
use clap::{Parser, Subcommand};
use color_eyre::eyre::{eyre, OptionExt as _, WrapErr as _};
use regex_lite::Regex;
use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::LazyLock;

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: SubCmd,
}

#[derive(Debug, Subcommand)]
enum SubCmd {
    /// Regenerate the bindings
    Bindgen,
    /// Update the version of libsqsh
    Update(Update),
}

#[derive(Debug, Parser)]
struct Update {
    #[clap(long, short)]
    /// Version to update to, defaults to the latest released version tag
    upstream_version: Option<String>,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    match cli.command {
        SubCmd::Bindgen => {
            run_bindgen()?;
        }
        SubCmd::Update(Update { upstream_version }) => {
            let submodules = submodules_dir()?;
            let sqsh_tools = submodules.join("sqsh-tools");
            fetch_git_tags(&sqsh_tools)?;
            let version = match upstream_version {
                Some(version) => version,
                None => get_latest_version_by_tag(&sqsh_tools)?,
            };
            println!("Updating to version: {}", version);

            checkout_git(&sqsh_tools, &version)?;

            let cextras_rev = get_cextras_revision(&sqsh_tools)?;
            update_cextras(&submodules, &cextras_rev)?;
            run_bindgen()?;
        }
    }
    Ok(())
}

fn update_cextras(submodules_dir: &Path, rev: &str) -> color_eyre::Result<()> {
    let cextras = submodules_dir.join("cextras");
    fetch_git_tags(&cextras)?;
    checkout_git(&cextras, &rev)?;
    Ok(())
}

fn get_cextras_revision(sqsh_tools_dir: &Path) -> color_eyre::Result<String> {
    let wrap_path = sqsh_tools_dir.join("subprojects").join("cextras.wrap");
    let contents = fs::read_to_string(&wrap_path)
        .wrap_err_with(|| format!("unable to open {}", wrap_path.display()))?;
    for line in contents.lines() {
        if let Some(rev) = line.strip_prefix("revision = ") {
            return Ok(rev.trim().to_string());
        }
    }
    Err(eyre!(
        "unable to find revision line in {}",
        wrap_path.display()
    ))
}

fn checkout_git(dir: &Path, git_ref: &str) -> color_eyre::Result<()> {
    let status = Command::new("git")
        .args(["checkout", git_ref])
        .current_dir(dir)
        .spawn()
        .wrap_err("Failed to execute git command")?
        .wait()
        .wrap_err("Failed to wait for git command")?;
    if !status.success() {
        return Err(eyre!("Failed to checkout git ref"));
    }
    Ok(())
}

fn get_latest_version_by_tag(dir: &Path) -> color_eyre::Result<String> {
    let output = Command::new("git")
        .args(["tag", "--list", "v*", "--sort=-v:refname"])
        .current_dir(dir)
        .output()
        .wrap_err("Failed to execute git command")?;
    if !output.status.success() {
        return Err(eyre!("Failed to get latest version"));
    }
    let output = String::from_utf8_lossy(&output.stdout);
    let newest_version = output.lines().next().unwrap().trim();
    Ok(newest_version.to_string())
}

fn fetch_git_tags(dir: &Path) -> color_eyre::Result<()> {
    let status = Command::new("git")
        .args(["fetch", "--tags"])
        .current_dir(dir)
        .spawn()
        .wrap_err("Failed to execute git command")?
        .wait()
        .wrap_err("Failed to wait for git command")?;
    if !status.success() {
        return Err(eyre!("Failed to fetch git tags"));
    }
    Ok(())
}

fn submodules_dir() -> color_eyre::Result<PathBuf> {
    let mut submodules = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Leave xtask directory
    if !submodules.pop() {
        return Err(eyre!("unable to find parent of manifest dir"));
    }
    submodules.push("sqsh-sys");
    submodules.push("submodules");
    Ok(submodules)
}

fn run_bindgen() -> color_eyre::Result<()> {
    println!("Running bindgen");
    let sqsh_sys = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_eyre("unable to find parent of manifest dir")?
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
        .generate()?;

    let mut writer = Vec::new();

    bindings.write(Box::new(&mut writer))?;

    let mut data = String::from_utf8(writer)?;
    data = DEPRECATED_FINDER
        .replace_all(&data, "$1#[deprecated]\n$1$2")
        .to_string();

    fs::write(sqsh_sys.join("src/bindings.rs"), data)?;
    Ok(())
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
