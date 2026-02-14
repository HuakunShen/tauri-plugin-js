#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKENDS_DIR="$SCRIPT_DIR/../backends"
BIN_DIR="$BACKENDS_DIR/bin"
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
cp "$BIN_DIR/bun-worker-$TARGET_TRIPLE" "$BIN_DIR/bun-worker"

# Deno
echo "Compiling deno-worker..."
deno compile --allow-all \
  --output "$BIN_DIR/deno-worker-$TARGET_TRIPLE" \
  "$BACKENDS_DIR/deno-worker.ts"
cp "$BIN_DIR/deno-worker-$TARGET_TRIPLE" "$BIN_DIR/deno-worker"

echo ""
echo "Done:"
ls -lh "$BIN_DIR"
