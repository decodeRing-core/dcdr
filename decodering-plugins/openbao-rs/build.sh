#!/bin/sh
set -e

cd "$(dirname "$0")"

WORKSPACE_ROOT="$(cd ../.. && pwd)"

cargo build -p openbao-rs --target wasm32-unknown-unknown --release

mkdir -p "$WORKSPACE_ROOT/plugins/compiled"
cp "$WORKSPACE_ROOT/target/wasm32-unknown-unknown/release/openbao_rs.wasm" \
   "$WORKSPACE_ROOT/plugins/compiled/openbao-rs.wasm"
