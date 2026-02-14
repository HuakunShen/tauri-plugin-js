#!/bin/bash
# Creates placeholder files for externalBin and resources entries in tauri.conf.json
# so that `tauri dev` doesn't fail when sidecars/workers haven't been built yet.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TAURI_DIR="$SCRIPT_DIR/../src-tauri"
TARGET_TRIPLE=$(rustc -vV | grep host | cut -d' ' -f2)

mkdir -p "$TAURI_DIR/binaries" "$TAURI_DIR/workers"

# externalBin placeholders (Tauri expects {name}-{triple})
for bin in bun-worker deno-worker; do
  f="$TAURI_DIR/binaries/${bin}-${TARGET_TRIPLE}"
  [ -f "$f" ] || touch "$f"
done

# resources placeholders
for res in workers/bun-worker.js workers/node-worker.mjs workers/deno-worker.ts; do
  f="$TAURI_DIR/$res"
  [ -f "$f" ] || touch "$f"
done
