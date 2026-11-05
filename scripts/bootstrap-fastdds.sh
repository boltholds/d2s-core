#!/usr/bin/env bash
# D2S's pinned, user-local Fast DDS bootstrap. Invoked by d2s-sys/build.rs.
set -Eeuo pipefail

if [[ "$(uname -s)" != "Linux" ]]; then
    echo "D2S automatic Fast DDS bootstrap currently supports Linux only." >&2
    echo "Supply FASTDDS_INCLUDE_DIR, FASTDDS_LIB_DIR and FASTDDSGEN for another OS." >&2
    exit 1
fi

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <absolute-cache-directory>" >&2
    exit 2
fi

CACHE="$1"
mkdir -p "$CACHE"
CACHE="$(cd "$CACHE" && pwd -P)"
PREFIX="$CACHE/install"
SOURCES="$CACHE/sources"
BUILDS="$CACHE/build"
GENERATOR="$SOURCES/fastddsgen/scripts/fastddsgen"
LOG="$CACHE/bootstrap.log"
MARKER="$CACHE/.ready-v1"

FASTDDS_VERSION="v3.6.2"
FASTCDR_VERSION="v2.3.0"
FOONATHAN_VERSION="v1.3.1"
GENERATOR_VERSION="v4.3.0"

# A process-level lock also protects against independent Cargo invocations
# using the same cache.
if ! command -v flock >/dev/null 2>&1; then
    echo "Missing 'flock' (util-linux). It is required for concurrent-safe bootstrap." >&2
    exit 1
fi
exec 9>"$CACHE/.bootstrap.lock"
flock 9

lib_dir() {
    if [[ -f "$PREFIX/lib/libfastdds.so" ]]; then
        echo "$PREFIX/lib"
    elif [[ -f "$PREFIX/lib64/libfastdds.so" ]]; then
        echo "$PREFIX/lib64"
    else
        return 1
    fi
}

is_ready() {
    [[ -f "$MARKER" ]] &&
        grep -Fxq "fastdds=$FASTDDS_VERSION fastcdr=$FASTCDR_VERSION foonathan=$FOONATHAN_VERSION generator=$GENERATOR_VERSION" "$MARKER" &&
        [[ -f "$PREFIX/include/fastdds/dds/domain/DomainParticipantFactory.hpp" ]] &&
        [[ -x "$GENERATOR" ]] &&
        lib_dir >/dev/null
}

if is_ready; then
    printf 'D2S_PREFIX=%s\nD2S_FASTDDSGEN=%s\n' "$PREFIX" "$GENERATOR"
    exit 0
fi

for tool in git cmake ninja c++ java; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Required build tool missing: $tool" >&2
        echo "Install a C++20 toolchain, CMake, Ninja, Git and a JDK (17+)." >&2
        exit 1
    fi
done

mkdir -p "$PREFIX" "$SOURCES" "$BUILDS"
: >"$LOG"

run() {
    if ! "$@" >>"$LOG" 2>&1; then
        echo "D2S Fast DDS bootstrap failed: $*" >&2
        echo "Log: $LOG" >&2
        tail -n 90 "$LOG" >&2 || true
        exit 1
    fi
}

clone_pin() {
    local name="$1" repository="$2" tag="$3"
    local destination="$SOURCES/$name"

    if [[ -e "$destination" && ! -d "$destination/.git" ]]; then
        echo "Incomplete or unexpected source directory: $destination" >&2
        echo "Remove that directory before retrying." >&2
        exit 1
    fi

    if [[ ! -d "$destination/.git" ]]; then
        run git clone --recursive --depth 1 --branch "$tag" "$repository" "$destination"
    else
        local current
        current="$(git -C "$destination" describe --tags --exact-match 2>/dev/null || true)"
        if [[ "$current" != "$tag" ]]; then
            echo "$destination is checked out at '$current', expected '$tag'." >&2
            echo "Remove that source directory to reinstall the pinned version." >&2
            exit 1
        fi
    fi
    # Fast DDS-Gen contains an IDL parser submodule; always ensure all
    # submodules are present, including when recovering an interrupted build.
    run git -C "$destination" submodule update --init --recursive
}

cmake_install() {
    local name="$1"
    shift
    run cmake -S "$SOURCES/$name" -B "$BUILDS/$name" -G Ninja \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX="$PREFIX" \
        -DCMAKE_PREFIX_PATH="$PREFIX" \
        "-DCMAKE_INSTALL_RPATH=$PREFIX/lib;$PREFIX/lib64" \
        -DBUILD_SHARED_LIBS=ON \
        "$@"
    run cmake --build "$BUILDS/$name" --target install --parallel 2
}

echo "Bootstrapping pinned Fast DDS sources into $CACHE" >&2

clone_pin foonathan https://github.com/eProsima/foonathan_memory_vendor.git "$FOONATHAN_VERSION"
cmake_install foonathan

clone_pin fastcdr https://github.com/eProsima/Fast-CDR.git "$FASTCDR_VERSION"
cmake_install fastcdr

clone_pin fastdds https://github.com/eProsima/Fast-DDS.git "$FASTDDS_VERSION"
cmake_install fastdds

clone_pin fastddsgen https://github.com/eProsima/Fast-DDS-Gen.git "$GENERATOR_VERSION"
# The Gradle wrapper downloads its own Gradle distribution/dependencies.
run bash -c 'cd "$1" && ./gradlew --no-daemon assemble' _ "$SOURCES/fastddsgen"

if [[ ! -x "$GENERATOR" ]] || ! lib_dir >/dev/null; then
    echo "Fast DDS installation is incomplete. Inspect $LOG" >&2
    exit 1
fi

printf '%s\n' "fastdds=$FASTDDS_VERSION fastcdr=$FASTCDR_VERSION foonathan=$FOONATHAN_VERSION generator=$GENERATOR_VERSION" >"$MARKER"
printf 'D2S_PREFIX=%s\nD2S_FASTDDSGEN=%s\n' "$PREFIX" "$GENERATOR"
