use std::env;
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
        .map(walkdir::DirEntry::into_path)
        .collect();

    let extra_includes = roots.iter().map(|p| p.join("include")).collect::<Vec<_>>();

    let mut build = cc::Build::new();
    build
        .include(sqsh_tools.join("include"))
        .includes(extra_includes)
        .files(c_files)
        .flag("-pthread")
        .std("c11")
        .warnings(true);
    if cfg!(feature = "zlib") {
        if let Some(include) = env::var_os("DEP_Z_INCLUDE") {
            build.include(include);
        }
        build.define("CONFIG_ZLIB", None);
    }
    if cfg!(feature = "lz4") {
        if let Some(include) = env::var_os("DEP_LZ4_INCLUDE") {
            build.include(include);
        }
        build.define("CONFIG_LZ4", None);
    }
    if cfg!(feature = "lzma") {
        if let Some(include) = env::var_os("DEP_LZMA_INCLUDE") {
            build.include(include);
        } else if let Ok(library) = pkg_config::probe_library("liblzma") {
            // lzma-sys defaults to using pkg-config, but it doesn't expose the DEP_LZMA_INCLUDE
            // var unless it builds it
            build.includes(library.include_paths);
        }
        build.define("CONFIG_LZMA", None);
    }
    if cfg!(feature = "zstd") {
        if let Some(include) = env::var_os("DEP_ZSTD_INCLUDE") {
            build.include(include);
        }
        build.define("CONFIG_ZSTD", None);
    }
    build.compile("sqsh");
}
