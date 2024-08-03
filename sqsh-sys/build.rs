use meson_next::{build, Config};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

fn main() {
    let build_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("build");
    let build_path = build_path.to_str().unwrap();
    let options = HashMap::from([
        ("default_library", "static"),
        ("tools", "false"),
        (
            "lz4",
            cfg!(feature = "lz4")
                .then_some("enabled")
                .unwrap_or("disabled"),
        ),
        (
            "lzma",
            cfg!(feature = "lzma")
                .then_some("enabled")
                .unwrap_or("disabled"),
        ),
        (
            "zlib",
            cfg!(feature = "zlib")
                .then_some("enabled")
                .unwrap_or("disabled"),
        ),
        (
            "zstd",
            cfg!(feature = "zstd")
                .then_some("enabled")
                .unwrap_or("disabled"),
        ),
    ]);
    let config = Config::new().options(options);
    println!("cargo:rustc-link-lib=static=sqsh");
    println!("cargo:rustc-link-search=native={}/libsqsh", build_path);
    build("sqsh-tools", build_path, config);
}
