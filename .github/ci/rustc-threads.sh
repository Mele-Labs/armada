#!/bin/sh
# Cargo's RUSTC_WORKSPACE_WRAPPER in CI's test jobs: Armada's own crates are
# type-checked on four threads. `-Zthreads` is unstable, so RUSTC_BOOTSTRAP is
# set here, for these crates only, and never seen by a dependency's build
# script. docs/spikes/ci-test-compile.md, *Four threads and no debug info*.
rustc="$1"
shift
RUSTC_BOOTSTRAP=1 exec "$rustc" "$@" -Zthreads=4
