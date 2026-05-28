#!/bin/sh
set -e

cd "$(dirname "$0")"

WORKSPACE_ROOT="$(cd ../.. && pwd)"

cargo build -p aws-rs --target wasm32-wasip1 --release

mkdir -p "$WORKSPACE_ROOT/plugins/compiled"
cp "$WORKSPACE_ROOT/target/wasm32-wasip1/release/aws_rs.wasm" \
   "$WORKSPACE_ROOT/plugins/compiled/aws-rs.wasm"
