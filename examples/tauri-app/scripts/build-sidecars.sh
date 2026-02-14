#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKENDS_DIR="$SCRIPT_DIR/../backends"
DENO_COMPILE_DIR="$SCRIPT_DIR/../../deno-compile"
BIN_DIR="$SCRIPT_DIR/../src-tauri/binaries"
TARGET_TRIPLE=$(rustc -vV | grep host | cut -d' ' -f2)

mkdir -p "$BIN_DIR"

echo "Target: $TARGET_TRIPLE"
echo "Output: $BIN_DIR"
echo ""

# Bun
echo "Compiling bun-worker..."
bun build --compile --minify \
  "$BACKENDS_DIR/bun-worker.ts" \
  --outfile "$BIN_DIR/bun-worker-$TARGET_TRIPLE"

# Deno — compile from separate deno-compile directory to avoid node_modules
echo "Compiling deno-worker..."
deno compile --allow-all \
  --output "$BIN_DIR/deno-worker-$TARGET_TRIPLE" \
  "$DENO_COMPILE_DIR/main.ts"

echo ""
echo "Done:"
ls -lh "$BIN_DIR"
