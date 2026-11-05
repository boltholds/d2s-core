use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn require_fastdds_headers() -> PathBuf {
    let provided = env::var_os("FASTDDS_INCLUDE_DIR");
    let candidates: Vec<PathBuf> = match provided {
        Some(path) => vec![PathBuf::from(path)],
        None => vec![
            PathBuf::from("/usr/local/include"),
            PathBuf::from("/usr/include"),
        ],
    };

    candidates
        .into_iter()
        .find(|dir| {
            dir.join("fastdds/dds/domain/DomainParticipantFactory.hpp")
                .is_file()
        })
        .unwrap_or_else(|| {
            panic!(
                "Fast DDS 3.x headers not found. Install Fast DDS or set \
                 FASTDDS_INCLUDE_DIR to the directory containing fastdds/. \
                 See docs/phase-0.md."
            )
        })
}

fn generate_idl(idl: &Path, out_dir: &Path) -> Vec<PathBuf> {
    fs::create_dir_all(out_dir).expect("cannot create Fast DDS-Gen output directory");
    let generator = env::var_os("FASTDDSGEN")
        .unwrap_or_else(|| OsString::from("fastddsgen"));

    let result = Command::new(&generator)
        .arg("-replace")
        .arg("-flat-output-dir")
        .arg("-d")
        .arg(out_dir)
        .arg(idl)
        .output()
        .unwrap_or_else(|error| {
            panic!(
                "Cannot start Fast DDS-Gen ({:?}): {error}. \
                 Install the Fast DDS-Gen tool and set FASTDDSGEN if needed.",
                generator
            )
        });

    if !result.status.success() {
        panic!(
            "Fast DDS-Gen failed (status {}):\n{}\n{}",
            result.status,
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr),
        );
    }

    let mut sources: Vec<PathBuf> = fs::read_dir(out_dir)
        .expect("cannot list generated IDL directory")
        .map(|entry| entry.expect("cannot read generated entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "cxx"))
        .collect();

    sources.sort();
    assert!(
        !sources.is_empty(),
        "Fast DDS-Gen produced no C++ sources in {}",
        out_dir.display()
    );
    sources
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest.join("../..");
    let idl = workspace.join("schemas/D2sMessage.idl");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("generated");
    let generated_sources = generate_idl(&idl, &out_dir);
    let fastdds_include = require_fastdds_headers();

    // Include root /crates, so cxx-generated glue can resolve
    // "d2s-sys/native/include/native.hpp".
    let mut build = cxx_build::bridge("src/lib.rs");
    build
        .file("native/src/native.cpp")
        .include(manifest.parent().unwrap())
        .include(&out_dir)
        .include(fastdds_include)
        .flag_if_supported("-std=c++20")
        .flag_if_supported("/std:c++20");

    for source in &generated_sources {
        build.file(source);
    }
    build.compile("d2s_fastdds_native");

    if let Some(libdir) = env::var_os("FASTDDS_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", PathBuf::from(libdir).display());
    }
    println!("cargo:rustc-link-lib=dylib=fastdds");
    println!("cargo:rustc-link-lib=dylib=fastcdr");

    for path in [
        idl,
        manifest.join("src/lib.rs"),
        manifest.join("native/include/native.hpp"),
        manifest.join("native/src/native.cpp"),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    for name in [
        "FASTDDSGEN",
        "FASTDDS_INCLUDE_DIR",
        "FASTDDS_LIB_DIR",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
}
