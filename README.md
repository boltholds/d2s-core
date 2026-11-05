# D2S Platform

Rust-first experimental DDS middleware and SDK. D2S uses [eProsima Fast DDS](https://github.com/eProsima/Fast-DDS) through a narrow C++20/[CXX](https://cxx.rs) bridge.

**Status: Phase 0 — experimental prototype.** There is no stable API, federation, or general-purpose schema system yet.

## What is in the repository

- `crates/d2s` — safe Rust API, Tokio-based convenience polling, and two examples.
- `crates/d2s-sys` — CXX bridge and a small C++20 Fast DDS adapter.
- `schemas/D2sMessage.idl` — the one message type used by the prototype.
- `scripts/smoke.sh` / `scripts/smoke.ps1` — Linux and Windows publisher/subscriber smoke tests.
- `docs/` — vision, RFCs, ADRs, and the [Phase 0 build guide](docs/phase-0.md).

## Try the prototype (Windows or Linux)

Prerequisites: Rust 1.88+ and build tools. On **Windows x64**, use the Rust **MSVC** toolchain, Visual Studio 2022 Build Tools with **Desktop development with C++**, Git, CMake and JDK 17+. On Linux, use a C++20 compiler, Git, CMake, Ninja, JDK and Asio/TinyXML2/OpenSSL headers. Fast DDS and the IDL generator are downloaded and built automatically by Cargo.

In terminal 1:

```sh
cargo run -p d2s --example subscriber -- 10 5
```

In terminal 2:

```sh
cargo run -p d2s --example publisher -- 10 5
```

Both processes use DDS DomainId 10 and Topic `d2s.phase0.hello`. The publisher waits until at least one matching reader has been discovered. The sample is intentionally constrained to the single generated `D2sMessage` type.

Pinned native dependencies are cached: **Windows:** `%LOCALAPPDATA%\d2s\deps\` (Windows vcpkg); **Linux:** `target/d2s-deps/`. You do not need to install Fast DDS system-wide; the initial build requires network access. Windows DLLs are staged beside Cargo-built examples.

Detailed setup, environment variables, troubleshooting and testing: [docs/phase-0.md](docs/phase-0.md).

## Documentation

Start from [docs/README.md](docs/README.md). The proposal for a limited-visibility federated graph is **not implemented** by Phase 0.

Licensed under [Apache License 2.0](LICENSE).
