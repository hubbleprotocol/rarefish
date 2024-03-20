#!/usr/bin/env bash

set -e
cd "$(dirname "$0")/.."

source ./ci/rust-version.sh stable
source ./ci/solana-version.sh

export RUSTFLAGS="-D warnings"
export RUSTBACKTRACE=1

set -x

# Build/test all host crates
RUST_LOG=fatal cargo +"$rust_stable" build --workspace
RUST_LOG=fatal cargo +"$rust_stable" test --workspace

exit 0
