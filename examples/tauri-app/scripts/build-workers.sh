#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKENDS_DIR="$SCRIPT_DIR/../backends"
OUT_DIR="$SCRIPT_DIR/../src-tauri/workers"

mkdir -p "$OUT_DIR"

echo "Output: $OUT_DIR"
echo ""

# Bun worker → bundle for bun target (inlines kkrpc)
echo "Bundling bun-worker.ts..."
bun build "$BACKENDS_DIR/bun-worker.ts" \
  --target bun \
  --outfile "$OUT_DIR/bun-worker.js"

# Node worker → bundle for node target (inlines kkrpc)
echo "Bundling node-worker.mjs..."
bun build "$BACKENDS_DIR/node-worker.mjs" \
  --target node \
  --outfile "$OUT_DIR/node-worker.mjs"

# Deno worker uses npm: specifier — Deno resolves it natively, just copy
echo "Copying deno-worker.ts..."
cp "$BACKENDS_DIR/deno-worker.ts" "$OUT_DIR/deno-worker.ts"

echo ""
echo "Done:"
ls -lh "$OUT_DIR"
