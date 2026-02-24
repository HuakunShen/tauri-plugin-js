# Example Application

<cite>
**Referenced Files in This Document**
- [examples/tauri-app/README.md](file://examples/tauri-app/README.md)
- [examples/tauri-app/src-tauri/src/lib.rs](file://examples/tauri-app/src-tauri/src/lib.rs)
- [examples/tauri-app/src-tauri/tauri.conf.json](file://examples/tauri-app/src-tauri/tauri.conf.json)
- [examples/tauri-app/backends/shared-api.ts](file://examples/tauri-app/backends/shared-api.ts)
- [examples/deno-compile/main.ts](file://examples/deno-compile/main.ts)
</cite>

## Table of Contents

1. [Overview](#overview)
2. [Project Structure](#project-structure)
3. [Running the Example](#running-the-example)
4. [Features Demonstrated](#features-demonstrated)
5. [Deno Compile Package](#deno-compile-package)

## Overview

The example application in `examples/tauri-app/` demonstrates all plugin features:

- Process spawning with Bun, Node.js, and Deno
- Type-safe RPC communication
- Runtime detection and path overrides
- Compiled binary sidecars
- Multi-window support

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md)

## Project Structure

```
examples/
├── deno-compile/              # Separate Deno package for compilation
│   ├── deno.json              # kkrpc dependency declaration
│   ├── main.ts                # Deno worker source
│   └── shared-api.ts          # API types (copy for Deno)
│
└── tauri-app/                 # Main demo app
    ├── backends/
    │   ├── shared-api.ts      # Shared BackendAPI interface
    │   ├── bun-worker.ts      # Bun worker implementation
    │   ├── node-worker.mjs    # Node worker implementation
    │   └── deno-worker.ts     # Deno worker implementation (dev only)
    ├── scripts/
    │   ├── build-sidecars.sh  # Compile binaries
    │   └── build-workers.sh   # Bundle for production
    ├── src/
    │   ├── App.svelte         # Main UI component
    │   ├── main.js            # Entry point
    │   └── app.css            # Tailwind styles
    ├── src-tauri/
    │   ├── binaries/          # Compiled sidecars (gitignored)
    │   ├── workers/           # Bundled scripts (gitignored)
    │   ├── src/lib.rs         # Tauri app setup
    │   ├── capabilities/      # Permission config
    │   └── tauri.conf.json    # App configuration
    ├── package.json
    └── vite.config.js
```

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md#L73-L113)

## Running the Example

### Setup

From the plugin root:

```bash
# Build the plugin's guest-js
pnpm install
pnpm build

# Install example dependencies
cd examples/tauri-app
pnpm install
```

### Build Sidecars (Optional)

```bash
cd examples/tauri-app
pnpm build:sidecars
```

This compiles `bun-worker.ts` and `deno-worker.ts` into standalone binaries in `src-tauri/binaries/`.

### Build Workers for Production

```bash
cd examples/tauri-app
pnpm build:workers
```

This bundles worker scripts with kkrpc inlined into `src-tauri/workers/`.

### Run

```bash
cd examples/tauri-app
pnpm tauri dev
```

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md#L14-L39)

## Features Demonstrated

### Spawn Section

Three spawn buttons with live runtime detection:
- Green dot + version = runtime found
- Red dot + "not found" = runtime not installed (button disabled)

Clicking spawns the corresponding worker script.

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md#L50-L54)

### Sidecars Section

Two buttons for compiled binaries:
- Purple dot = sidecar (always enabled)
- `bun-worker (sidecar)` — compiled via `bun build --compile`
- `deno-worker (sidecar)` — compiled via `deno compile`

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md#L56-L66)

### Process List

Shows running processes with:
- PID display
- Restart button
- Kill button

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md#L68-L70)

### RPC Calls

Four type-safe call buttons:
- `add(5, 3)` — arithmetic
- `echo("hello")` — string echo with runtime prefix
- `getSystemInfo()` — returns runtime, PID, platform, arch
- `fibonacci(10)` — compute

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md#L72-L78)

### Output Log

Real-time log panel showing:
- stdout (blue)
- stderr (red)
- exit events
- RPC results
- System messages

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md#L80-L82)

### Settings Dialog

- Shows detected path and version for each runtime
- Allows overriding executable paths
- "Refresh detection" button

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md#L84-L88)

### Multi-Window

Click "+ window" to open another instance. All windows share backend processes.

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md#L90-L91)

## Deno Compile Package

The `examples/deno-compile/` directory is a separate Deno package used for compiling the Deno worker.

### Why Separate?

`deno compile` crashes with a stack overflow if run from a directory containing `node_modules`. The separate package avoids this issue.

### Structure

```
examples/deno-compile/
├── deno.json          # Declares kkrpc dependency
├── deno.lock
├── main.ts            # Worker implementation
└── shared-api.ts      # API types
```

### deno.json

```json
{
  "imports": {
    "kkrpc": "npm:kkrpc@^0.6.0",
    "kkrpc/deno": "npm:kkrpc@^0.6.0/deno"
  }
}
```

### main.ts

```typescript
import { DenoIo, RPCChannel } from "kkrpc/deno";
import type { BackendAPI } from "./shared-api.ts";

const api: BackendAPI = {
  async add(a, b) { return a + b; },
  async echo(msg) { return `[deno] ${msg}`; },
  async getSystemInfo() {
    return {
      runtime: "deno",
      pid: Deno.pid,
      platform: Deno.build.os,
      arch: Deno.build.arch,
    };
  },
  async fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
  },
};

const io = new DenoIo(Deno.stdin.readable);
const channel = new RPCChannel(io, { expose: api });
```

**Section sources**

- [examples/deno-compile/main.ts](file://examples/deno-compile/main.ts)

### Compiling

```bash
TARGET=$(rustc -vV | grep host | cut -d' ' -f2)
deno compile --allow-all \
  --output ../tauri-app/src-tauri/binaries/deno-worker-$TARGET \
  main.ts
```

**Section sources**

- [examples/tauri-app/README.md](file://examples/tauri-app/README.md#L29-L31)
