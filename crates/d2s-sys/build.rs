use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct FastDds {
    include_dir: PathBuf,
    lib_dir: PathBuf,
    generator: OsString,
}

fn require_headers(dir: &Path) {
    assert!(
        dir.join("fastdds/dds/domain/DomainParticipantFactory.hpp")
            .is_file(),
        "Fast DDS 3.x headers not found under {}",
        dir.display()
    );
}

fn select_fastdds(workspace: &Path) -> FastDds {
    let include_override = env::var_os("FASTDDS_INCLUDE_DIR");
    let lib_override = env::var_os("FASTDDS_LIB_DIR");

    if let (Some(include_dir), Some(lib_dir)) = (&include_override, &lib_override) {
        let include_dir = PathBuf::from(include_dir);
        let lib_dir = PathBuf::from(lib_dir);
        require_headers(&include_dir);
        assert!(lib_dir.exists(), "Fast DDS lib directory does not exist");
        return FastDds {
            include_dir,
            lib_dir,
            generator: env::var_os("FASTDDSGEN")
                .unwrap_or_else(|| OsString::from("fastddsgen")),
        };
    }

    assert!(
        include_override.is_none() && lib_override.is_none(),
        "Set BOTH FASTDDS_INCLUDE_DIR and FASTDDS_LIB_DIR, or neither"
    );

    assert!(
        env::var("D2S_AUTO_INSTALL").as_deref() != Ok("0"),
        "D2S_AUTO_INSTALL=0: automatic installation disabled. Supply FASTDDS_INCLUDE_DIR \
         and FASTDDS_LIB_DIR, or enable automatic installation."
    );

    assert_eq!(
        env::var("HOST").expect("HOST missing"),
        env::var("TARGET").expect("TARGET missing"),
        "Automatic Fast DDS source builds currently require a native Rust target"
    );

    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS missing");
    assert_eq!(
        target_os, "linux",
        "Automatic Fast DDS source builds currently support Linux. \
         On other operating systems supply an existing Fast DDS installation."
    );

    let target = env::var("TARGET").expect("TARGET missing");
    let path_override = env::var_os("D2S_DEPS_DIR");
    let cache_path = if let Some(path) = path_override {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            path
        } else {
            workspace.join(path)
        }
    } else {
        let root = env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| workspace.join("target"));
        let root = if root.is_absolute() {
            root
        } else {
            workspace.join(root)
        };
        root.join("d2s-deps").join(target)
    };

    fs::create_dir_all(&cache_path).expect("cannot create Fast DDS dependency cache");
    let cache = cache_path.canonicalize().expect("cannot resolve dependency cache");
    let bootstrap = workspace.join("scripts/bootstrap-fastdds.sh");
    println!(
        "cargo:warning=Fast DDS: checking local pinned dependency cache at {}",
        cache.display()
    );

    let output = Command::new("bash")
        .arg(bootstrap)
        .arg(cache)
        .output()
        .expect("cannot invoke bash to bootstrap Fast DDS");

    if !output.status.success() {
        panic!(
            "Fast DDS auto-install failed ({}):\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let stdout = String::from_utf8(output.stdout).expect("bootstrap output was not UTF-8");
    let read_key = |key: &str| {
        stdout
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| panic!("bootstrap did not return {key}"))
            .to_owned()
    };

    let prefix = PathBuf::from(read_key("D2S_PREFIX="));
    let include_dir = prefix.join("include");
    let lib_dir = if prefix.join("lib/libfastdds.so").exists() {
        prefix.join("lib")
    } else {
        prefix.join("lib64")
    };
    require_headers(&include_dir);

    FastDds {
        include_dir,
        lib_dir,
        generator: OsString::from(read_key("D2S_FASTDDSGEN=")),
    }
}

fn generate_idl(idl: &Path, out_dir: &Path, generator: &OsString) -> Vec<PathBuf> {
    fs::create_dir_all(out_dir).expect("cannot create Fast DDS-Gen output directory");
    let result = Command::new(generator)
        .arg("-replace")
        .arg("-flat-output-dir")
        .arg("-d")
        .arg(out_dir)
        .arg(idl)
        .output()
        .unwrap_or_else(|error| {
            panic!("Cannot run Fast DDS-Gen ({generator:?}): {error}")
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
    assert!(!sources.is_empty(), "Fast DDS-Gen produced no C++ sources");
    sources
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest.join("../..").canonicalize().unwrap();
    let idl = workspace.join("schemas/D2sMessage.idl");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("generated");
    let selected = select_fastdds(&workspace);
    let sources = generate_idl(&idl, &out_dir, &selected.generator);

    let mut build = cxx_build::bridge("src/lib.rs");
    build
        .file("native/src/native.cpp")
        .include(manifest.parent().unwrap())
        .include(&out_dir)
        .include(&selected.include_dir)
        .flag_if_supported("-std=c++20")
        .flag_if_supported("/std:c++20");

    for source in sources {
        build.file(source);
    }
    build.compile("d2s_fastdds_native");

    println!("cargo:rustc-link-search=native={}", selected.lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=fastdds");
    println!("cargo:rustc-link-lib=dylib=fastcdr");
    // Cargo exposes this to a direct dependent crate via DEP_D2S_FASTDDS_LIBDIR.
    println!("cargo:metadata=libdir={}", selected.lib_dir.display());

    for path in [
        idl,
        manifest.join("src/lib.rs"),
        manifest.join("native/include/native.hpp"),
        manifest.join("native/src/native.cpp"),
        workspace.join("scripts/bootstrap-fastdds.sh"),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    for name in [
        "FASTDDSGEN",
        "FASTDDS_INCLUDE_DIR",
        "FASTDDS_LIB_DIR",
        "D2S_DEPS_DIR",
        "D2S_AUTO_INSTALL",
        "CARGO_TARGET_DIR",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
}
