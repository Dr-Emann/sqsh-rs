use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() {
    let mut c_files: Vec<PathBuf> = WalkDir::new("src/libsqsh/lib")
        .into_iter()
        .map(Result::unwrap)
        .filter(|e| !e.file_type().is_dir() && e.path().extension().unwrap_or_default() == "c")
        .map(|e| e.into_path())
        .collect();

    let mut extra_includes = Vec::new();
    for subproject in fs::read_dir("src/libsqsh/subprojects").unwrap() {
        let subproject = subproject.unwrap();
        if subproject.file_type().map_or(true, |t| !t.is_dir()) {
            continue;
        }
        c_files.extend(
            WalkDir::new(subproject.path().join("lib"))
                .into_iter()
                .map(Result::unwrap)
                .filter(|e| {
                    !e.file_type().is_dir() && e.path().extension().unwrap_or_default() == "c"
                })
                .map(|e| e.into_path()),
        );
        let include_dir = subproject.path().join("include");
        if include_dir.is_dir() {
            extra_includes.push(include_dir);
        }
    }
    for file in &c_files {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    cc::Build::new()
        .include("src/libsqsh/include")
        .includes(extra_includes)
        .files(c_files)
        .flag("-pthread")
        .std("c11")
        .warnings(true)
        .define("CONFIG_ZLIB", None)
        .compile("sqsh");
}
