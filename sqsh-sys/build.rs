use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    let submodule = Path::new("src/libsqsh");
    let mut roots = vec![submodule.join("common"), submodule.join("libsqsh")];
    for subproject in fs::read_dir(submodule.join("subprojects")).unwrap() {
        let subproject = subproject.unwrap();
        if subproject.file_type().map_or(true, |t| !t.is_dir()) {
            continue;
        }
        roots.push(subproject.path());
    }
    let c_files: Vec<PathBuf> = roots
        .iter()
        .flat_map(|p| [p.join("src"), p.join("lib")])
        .filter(|p| p.exists())
        .flat_map(WalkDir::new)
        .map(Result::unwrap)
        .filter(|e| !e.file_type().is_dir() && e.path().extension().unwrap_or_default() == "c")
        .map(|e| e.into_path())
        .collect();

    let extra_includes = roots.iter().map(|p| p.join("include")).collect::<Vec<_>>();
    for file in &c_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    cc::Build::new()
        .include(submodule.join("include"))
        .includes(extra_includes)
        .files(c_files)
        .flag("-pthread")
        .std("c11")
        .warnings(true)
        .define("CONFIG_ZLIB", None)
        .compile("sqsh");
}
