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

## Prerequisites (Linux)

- Stable Rust toolchain **1.88 or newer**, Cargo and rustfmt.
- C++20 compiler (GCC or Clang), CMake (for building upstream packages).
- eProsima **Fast DDS 3.x** (headers + dynamically linked runtime).
- Compatible **Fast CDR 2.x** and upstream dependencies installed alongside Fast DDS.
- **Fast DDS-Gen** available as `fastddsgen` or indicated with `FASTDDSGEN`. Use a generator release compatible with Fast DDS 3.x (for example Fast DDS-Gen v4.3.0).
- Working local network or loopback discovery configuration that supports normal DDS participant matching.

Upstream source install instructions: [Fast DDS Linux guide](https://fast-dds.docs.eprosima.com/en/3.x/installation/sources/sources_linux.html). The CI workflow uses pinned Fast DDS-family versions as a reproducible reference.

For a non-system install, configure:

```sh
export FASTDDS_INCLUDE_DIR="$HOME/fastdds-install/include"
export FASTDDS_LIB_DIR="$HOME/fastdds-install/lib"
export LD_LIBRARY_PATH="$FASTDDS_LIB_DIR:${LD_LIBRARY_PATH:-}"
export FASTDDSGEN="$HOME/Fast-DDS-Gen/scripts/fastddsgen"
```

`FASTDDS_INCLUDE_DIR` must contain the `fastdds/` subdirectory, `FASTDDS_LIB_DIR` must contain `libfastdds.so` and `libfastcdr.so` (or equivalent versioned shared libraries). Their own shared dependencies must be resolvable by the dynamic loader. The build script searches `/usr/local/include` and `/usr/include` when no include override is given.

## Build

```sh
cargo build --workspace --examples
cargo test --workspace
```

At build time, `d2s-sys/build.rs` calls Fast DDS-Gen for `schemas/D2sMessage.idl` and compiles the generated C++ source with the CXX bridge and our adapter. Generated sources live only in Cargo's `OUT_DIR`, not in git.

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

- **Fast DDS headers missing:** install Fast DDS development files or set `FASTDDS_INCLUDE_DIR`.
- **fastddsgen not found:** install generator or set `FASTDDSGEN` to the executable path.
- **Linker cannot find -lfastdds:** set `FASTDDS_LIB_DIR`.
- **Loader cannot find .so:** add that library directory to `LD_LIBRARY_PATH` (Linux).
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
