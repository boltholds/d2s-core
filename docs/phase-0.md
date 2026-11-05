# Phase 0 — Rust + CXX + Fast DDS

**Status:** Experimental implementation, API not stable.

## Goal

Two separate Rust processes exchange IDL-typed messages over real Fast DDS, without an application broker:

```text
Rust publisher  ─┐                     ┌─  Rust subscriber
                 CXX adapter → Fast DDS ← CXX adapter
                 └──── DomainId 10 ────┘
```

This establishes a credible base for future SDK and federation work. There is **no federation, global graph, NAT traversal, persistence or general schema generator** yet.

## Automatic Windows installation

On Windows **x64 MSVC**, `cargo build` uses a project-controlled Windows bootstrap script:

1. Checks out a pinned [microsoft/vcpkg](https://github.com/microsoft/vcpkg) revision containing Fast DDS 3.6.2.
2. Builds Fast DDS and its dependencies locally with `vcpkg install fastdds:x64-windows`.
3. Builds [Fast DDS-Gen v4.3.0](https://github.com/eProsima/Fast-DDS-Gen) using the Gradle wrapper.
4. Stages runtime DLLs next to the Rust example and test executables.

It does not need a system-wide Fast DDS installation or elevated permissions. It **does** need Rust (`x86_64-pc-windows-msvc`), Visual Studio 2022 C++ Build Tools and Windows SDK, Git, CMake and a Java JDK (17+). `vcpkg` can download additional build dependencies. All downloads run on the first Cargo build; subsequent builds reuse the cache.

To check the toolchain in `cmd.exe`:

```bat
rustup show active-toolchain
where cargo
where cmake
where git
java -version
```

To build and test:

```bat
cargo build --workspace --examples
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\smoke.ps1
```

**Cache and diagnostics:** the default Windows location is `%LOCALAPPDATA%\d2s\deps\x86_64-pc-windows-msvc`; `bootstrap-windows.log` contains native build output. For a shorter path or to reset a failed install, set `D2S_DEPS_DIR=C:\d2s-cache` or remove the cached directory. Custom installs remain available with `FASTDDS_INCLUDE_DIR`, `FASTDDS_LIB_DIR`, `FASTDDS_BIN_DIR` and `FASTDDSGEN`.

**Supported:** native `x86_64-pc-windows-msvc`. Windows GNU and ARM64 toolchains and cross-compilation need dedicated support. On Windows, `d2s-sys/build.rs` copies `*.dll` from the selected dependency bin directory next to Cargo test/examples executables. This is a Phase 0 convenience; future consumers of the library must package runtime dependencies deliberately.

### Limitations and supply-chain notes

- The Windows dependency path is built by `vcpkg` rather than the Linux CMake bootstrap, but both target Fast DDS v3.6.2.
- The pin uses a specific vcpkg Git commit; the Fast DDS port verifies upstream sources by archive checksum.
- The tool executes Git, CMake, vcpkg and Gradle code from external projects; review this dependency chain for production use.
- Building native Fast DDS from scratch may be resource-intensive. An already completed installation is reused.

## Automatic Fast DDS installation (native Linux)

On the first `cargo build`, D2S automatically downloads and builds:

- [eProsima Fast DDS](https://github.com/eProsima/Fast-DDS) **v3.6.2**.
- Fast CDR **v2.3.0**, foonathan_memory_vendor **v1.3.1**.
- Fast DDS-Gen **v4.3.0** (including its Git submodules).

Everything is installed in the project-local cache, by default `target/d2s-deps/<target-triple>/`. It does not use sudo, modify system libraries or reinstall on every build. An OS-level file lock prevents concurrent writes to the same cache; logs are saved as `bootstrap.log`. Initial setup needs Internet access.

### Required system build tools

Rust 1.88+, C++20 compiler, CMake 3.22+, Ninja, Git, `flock`, JDK 17+, and development headers for Asio, TinyXML2 and OpenSSL. These build tools are not installed by Cargo; they must already be available.

Ubuntu 24.04:

```sh
sudo apt-get update
sudo apt-get install -y build-essential git cmake ninja-build \
    libasio-dev libtinyxml2-dev libssl-dev openjdk-17-jdk
```

Then run:

```sh
cargo build --workspace --examples
cargo test --workspace
bash scripts/smoke.sh
```

Cargo executes `scripts/bootstrap-fastdds.sh` from `crates/d2s-sys/build.rs`, then runs the installed generator for `schemas/D2sMessage.idl` and compiles the generated C++ with the CXX bridge. No manual Fast DDS commands or environment variables are needed for the default Linux build.

### Configuration

| Variable | Effect |
|---|---|
| `D2S_DEPS_DIR` | Custom path for the local native dependency cache |
| `CARGO_TARGET_DIR` | Changes the default native dependency cache root |
| `D2S_AUTO_INSTALL=0` | Disable downloading; requires explicitly supplied Fast DDS paths |
| `FASTDDS_INCLUDE_DIR` | Fast DDS header directory containing `fastdds/` |
| `FASTDDS_LIB_DIR` | Fast DDS and Fast CDR shared library directory |
| `FASTDDS_BIN_DIR` | Windows runtime DLL location for a custom installation |
| `FASTDDSGEN` | Path to an existing Fast DDS-Gen when using a custom installation |

To use a preinstalled version instead of downloading:

```sh
export FASTDDS_INCLUDE_DIR=/opt/fastdds/include
export FASTDDS_LIB_DIR=/opt/fastdds/lib
export FASTDDSGEN=/opt/Fast-DDS-Gen/scripts/fastddsgen
cargo build --workspace --examples
```

Both include and library overrides must be set together. Native Linux builds automatically add the chosen library directory to example executable RUNPATH. Existing custom installations must still provide transitive shared libraries. Cross-compilation and automatic macOS/Windows installation are not supported yet.

**Supply-chain note:** the first build runs upstream CMake/Gradle build scripts downloaded from eProsima GitHub (or vcpkg on Windows). Versions are pinned by release tag; pinning commit SHA and verifying artifacts is a future hardening task.

## Run

In separate terminals, **start subscriber first**:

```sh
cargo run -p d2s --example subscriber -- 10 5
```

Then publisher:

```sh
cargo run -p d2s --example publisher -- 10 5
```

Arguments: `<domain_id> <number_of_messages>`. The default is `10 5`. Both processes must agree on DomainId and use the fixed Topic `d2s.phase0.hello`.

Example stdout (illustrative, not a record of execution):

```text
Publisher ready on DDS domain 10; waiting for reader...
SENT [1] Hello from D2S Rust publisher #1
...
Subscriber listening on DDS domain 10...
RECEIVED [1] Hello from D2S Rust publisher #1
...
```

## Two-process smoke test

```sh
bash scripts/smoke.sh
```

The script builds the examples, starts the subscriber, publishes three messages and checks that the subscriber receives all three. Requires the Unix `timeout` command.

## Implementation boundaries

| Component | Responsibility |
|---|---|
| `d2s` | Safe Rust-facing structs, async timeout/wait helpers |
| `d2s-sys` | CXX definitions and opaque native handles |
| `native.cpp` | Fast DDS participants, generated type registration, matching, read/write |
| `D2sMessage.idl` | Wire type: uint32 index + UTF-8 text |
| Fast DDS | Discovery, DDS/RTPS transport, reader/writer history and reliability |

Each Publisher/Subscriber owns its own DomainParticipant at this stage. They are deliberately not implemented as Send/Sync wrappers; the public API must not expose native pointers. Dropping the handle releases the underlying DDS entities.

## Semantic limitations

- `publish()` reports Fast DDS write success, **not** persistence, application processing, or guaranteed delivery.
- QoS is fixed to Reliable + KeepLast depth 16 for this example; configurable QoS is a future feature.
- Publisher waits for one DDS match before sending; this **reduces** demo startup races but is not a persistence guarantee.
- Subscriber performs bounded-interval polling every 20 ms via Tokio; no C++ callback is forwarded across the FFI yet.
- Only the `D2sMessage` IDL type is supported.
- No transparent reconnect beyond DDS's native discovery behavior is promised.
- Large or malformed inputs have not yet undergone adversarial fuzzing.

## Troubleshooting

- **Fast DDS headers missing:** check the local cache bootstrap log or supply matching custom Fast DDS include/lib paths.
- **fastddsgen not found:** inspect the local bootstrap log or set `FASTDDSGEN` for a custom installation.
- **Linker cannot find -lfastdds:** inspect the local cache or set `FASTDDS_LIB_DIR` for a custom installation.
- **Loader cannot find .so:** check executable RUNPATH, external OS libraries and custom install paths.
- **No matching reader:** confirm both DomainId values, allowed DDS transport/discovery, firewalls/multicast and compatible QoS. Default RTPS UDP ports make very large DomainId values impractical.
- **Generated API incompatibility:** use generator release compatible with your installed Fast DDS 3.x; do not mix Fast DDS 2.x and 3.x code-generation artifacts.

## Exit criteria for Phase 0

1. Both Rust examples build on a reproducible Linux environment.
2. Two distinct processes discover each other without a broker.
3. Three consecutive typed samples are received in order in the smoke test.
4. Resource cleanup is safe on normal shutdown and repeated runs.
5. Integration pipeline logs the actual library and generator versions.
6. Compare the wrapper overhead against equivalent upstream Fast DDS code before making performance claims.

The implementation is an exploratory prototype; completion of these criteria requires successful builds and tests, not just creation of source files.
