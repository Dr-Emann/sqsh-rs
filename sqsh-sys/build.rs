use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn is_source_entry(entry: &walkdir::DirEntry) -> bool {
    // Skip any codegen directories
    entry.file_name() != "codegen"
}

fn main() {
    let submodules_dir = Path::new("submodules");
    println!("cargo:rerun-if-changed={}", submodules_dir.display());
    let sqsh_tools = submodules_dir.join("sqsh-tools");
    let roots = [
        submodules_dir.join("cextras"),
        sqsh_tools.join("common"),
        sqsh_tools.join("libsqsh"),
    ];
    let c_files: Vec<PathBuf> = roots
        .iter()
        .flat_map(|p| [p.join("src"), p.join("lib")])
        .filter(|p| p.exists())
        .flat_map(|p| WalkDir::new(p).into_iter().filter_entry(is_source_entry))
        .map(Result::unwrap)
        .filter(|e| !e.file_type().is_dir() && e.path().extension().unwrap_or_default() == "c")
        .map(|e| e.into_path())
        .collect();

    let extra_includes = roots.iter().map(|p| p.join("include")).collect::<Vec<_>>();

    cc::Build::new()
        .include(sqsh_tools.join("include"))
        .includes(extra_includes)
        .files(c_files)
        .flag("-pthread")
        .std("c11")
        .warnings(true)
        .define("CONFIG_ZLIB", None)
        .compile("sqsh");
}
