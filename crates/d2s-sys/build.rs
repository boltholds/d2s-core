use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct FastDds {
    include_dir: PathBuf,
    lib_dir: PathBuf,
    generator: OsString,
    bin_dir: Option<PathBuf>,
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
            bin_dir: if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
                let override_dir = env::var_os("FASTDDS_BIN_DIR").map(PathBuf::from);
                override_dir.or_else(|| lib_dir.parent().map(|root| root.join("bin")))
            } else {
                None
            },
            lib_dir,
            generator: env::var_os("FASTDDSGEN").unwrap_or_else(|| OsString::from("fastddsgen")),
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
    assert!(
        matches!(target_os.as_str(), "linux" | "windows"),
        "Automatic Fast DDS installation supports native Linux and Windows. \
         On other operating systems supply an existing Fast DDS installation."
    );
    if target_os == "windows" {
        assert_eq!(
            env::var("CARGO_CFG_TARGET_ENV").as_deref(),
            Ok("msvc"),
            "Automatic Fast DDS installation on Windows requires Rust MSVC toolchain"
        );
        assert_eq!(
            env::var("CARGO_CFG_TARGET_ARCH").as_deref(),
            Ok("x86_64"),
            "Automatic Fast DDS installation currently supports Windows x64 MSVC"
        );
    }

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
        if target_os == "windows" {
            let local = env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|| workspace.join("target"));
            local.join("d2s").join("deps").join(target)
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
        }
    };

    fs::create_dir_all(&cache_path).expect("cannot create Fast DDS dependency cache");
    // Windows PowerShell 5.1 cannot use Rust's verbatim \\?\ paths in every
    // filesystem API. All cache paths chosen above are already absolute.
    let cache = if target_os == "windows" {
        cache_path
    } else {
        cache_path
            .canonicalize()
            .expect("cannot resolve dependency cache")
    };
    let bootstrap = if target_os == "windows" {
        workspace.join("scripts/bootstrap-fastdds.ps1")
    } else {
        workspace.join("scripts/bootstrap-fastdds.sh")
    };
    println!(
        "cargo:warning=Fast DDS: checking local pinned dependency cache at {}",
        cache.display()
    );

    let output = if target_os == "windows" {
        Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(bootstrap)
            .arg("-CachePath")
            .arg(cache)
            .output()
            .expect("cannot invoke PowerShell to bootstrap Fast DDS")
    } else {
        Command::new("bash")
            .arg(bootstrap)
            .arg(cache)
            .output()
            .expect("cannot invoke bash to bootstrap Fast DDS")
    };

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
    let lib_dir = if target_os == "windows" || prefix.join("lib/libfastdds.so").exists() {
        prefix.join("lib")
    } else {
        prefix.join("lib64")
    };
    require_headers(&include_dir);

    FastDds {
        include_dir,
        lib_dir,
        generator: OsString::from(read_key("D2S_FASTDDSGEN=")),
        bin_dir: if target_os == "windows" {
            Some(PathBuf::from(read_key("D2S_BINDIR=")))
        } else {
            None
        },
    }
}

fn generate_idl(idl: &Path, out_dir: &Path, generator: &OsString) -> Vec<PathBuf> {
    fs::create_dir_all(out_dir).expect("cannot create Fast DDS-Gen output directory");
    let mut command = if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut command = Command::new("cmd.exe");
        command.args(["/D", "/C"]).arg(generator);
        command
    } else {
        Command::new(generator)
    };
    let result = command
        .arg("-replace")
        .arg("-flat-output-dir")
        .arg("-d")
        .arg(out_dir)
        .arg(idl)
        .output()
        .unwrap_or_else(|error| panic!("Cannot run Fast DDS-Gen ({generator:?}): {error}"));

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

fn stage_windows_dlls(bin_dir: &Path) {
    assert!(
        bin_dir.is_dir(),
        "Fast DDS runtime DLL directory is missing: {}",
        bin_dir.display()
    );
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("Unexpected Cargo OUT_DIR layout");
    let dlls: Vec<PathBuf> = fs::read_dir(bin_dir)
        .expect("Cannot list Fast DDS DLL directory")
        .map(|entry| entry.expect("Cannot read DLL directory entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"))
        })
        .collect();

    assert!(!dlls.is_empty(), "Fast DDS runtime DLL directory is empty");
    for destination_dir in [
        profile_dir.to_path_buf(),
        profile_dir.join("deps"),
        profile_dir.join("examples"),
    ] {
        fs::create_dir_all(&destination_dir).expect("Cannot create Rust binary directory");
        for dll in &dlls {
            let dest = destination_dir.join(dll.file_name().unwrap());
            let source_size = fs::metadata(dll).expect("Cannot stat DLL").len();
            let same_size = fs::metadata(&dest)
                .map(|metadata| metadata.len() == source_size)
                .unwrap_or(false);
            if !same_size {
                fs::copy(dll, &dest)
                    .unwrap_or_else(|error| panic!("Cannot stage DLL {}: {error}", dest.display()));
            }
        }
    }
}

fn main() {
    let manifest = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace = manifest.join("../..");
    let idl = workspace.join("schemas/D2sMessage.idl");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("generated");
    let selected = select_fastdds(&workspace);
    if let Some(bin_dir) = &selected.bin_dir {
        stage_windows_dlls(bin_dir);
    }
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

    println!(
        "cargo:rustc-link-search=native={}",
        selected.lib_dir.display()
    );
    println!("cargo:rustc-link-lib=dylib=fastdds");
    println!("cargo:rustc-link-lib=dylib=fastcdr");
    // Cargo exposes this to a direct dependent crate via DEP_D2S_FASTDDS_LIBDIR.
    println!("cargo:metadata=libdir={}", selected.lib_dir.display());
    if let Some(bin_dir) = &selected.bin_dir {
        println!("cargo:metadata=bindir={}", bin_dir.display());
    }

    for path in [
        idl,
        manifest.join("src/lib.rs"),
        manifest.join("native/include/native.hpp"),
        manifest.join("native/src/native.cpp"),
        workspace.join("scripts/bootstrap-fastdds.sh"),
        workspace.join("scripts/bootstrap-fastdds.ps1"),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    for name in [
        "FASTDDSGEN",
        "FASTDDS_INCLUDE_DIR",
        "FASTDDS_LIB_DIR",
        "FASTDDS_BIN_DIR",
        "D2S_DEPS_DIR",
        "D2S_AUTO_INSTALL",
        "CARGO_TARGET_DIR",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
}
