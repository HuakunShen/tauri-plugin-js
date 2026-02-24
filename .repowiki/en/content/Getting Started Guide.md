# Getting Started Guide

<cite>
**Referenced Files in This Document**
- [Cargo.toml](file://Cargo.toml)
- [package.json](file://package.json)
- [README.md](file://README.md)
- [permissions/default.toml](file://permissions/default.toml)
- [examples/tauri-app/src-tauri/src/lib.rs](file://examples/tauri-app/src-tauri/src/lib.rs)
</cite>

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Quick Start](#quick-start)
4. [Basic Usage](#basic-usage)
5. [Next Steps](#next-steps)

## Prerequisites

Before using tauri-plugin-js, ensure you have:

- **Rust toolchain** — Install via [rustup](https://rustup.rs/)
- **Tauri v2** — Follow the [Tauri v2 guide](https://tauri.app/start/)
- **pnpm** — Package manager for JavaScript dependencies
- **At least one JS runtime**: [Bun](https://bun.sh/), [Node.js](https://nodejs.org/), or [Deno](https://deno.com/)

**Section sources**

- [README.md](file://README.md#L64-L90)

## Installation

### Rust Side

Add the plugin to your `src-tauri/Cargo.toml`:

```toml
[dependencies]
tauri-plugin-js = "0.1"
```

Register the plugin in `src-tauri/src/lib.rs`:

```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_js::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

**Section sources**

- [Cargo.toml](file://Cargo.toml)
- [examples/tauri-app/src-tauri/src/lib.rs](file://examples/tauri-app/src-tauri/src/lib.rs)

### Frontend Side

Install the frontend API package:

```bash
pnpm add tauri-plugin-js-api kkrpc
```

**Section sources**

- [package.json](file://package.json)
- [README.md](file://README.md#L86-L90)

### Permissions

Add the required permissions to `src-tauri/capabilities/default.json`:

```json
{
  "permissions": [
    "core:default",
    "js:default"
  ]
}
```

The `js:default` permission grants access to all 10 commands:
- `spawn`, `kill`, `kill-all`, `restart`
- `list-processes`, `get-status`, `write-stdin`
- `detect-runtimes`, `set-runtime-path`, `get-runtime-paths`

**Section sources**

- [permissions/default.toml](file://permissions/default.toml)
- [README.md](file://README.md#L92-L105)

## Quick Start

### 1. Define a Shared API Type

Create a TypeScript interface that describes your backend API:

```typescript
// backends/shared-api.ts
export interface BackendAPI {
  add(a: number, b: number): Promise<number>;
  echo(message: string): Promise<string>;
  getSystemInfo(): Promise<{ runtime: string; pid: number; platform: string; arch: string }>;
}
```

**Section sources**

- [examples/tauri-app/backends/shared-api.ts](file://examples/tauri-app/backends/shared-api.ts)

### 2. Write a Backend Worker

Create a worker script for your chosen runtime:

**Bun Worker** (`backends/bun-worker.ts`):
```typescript
import { RPCChannel, BunIo } from "kkrpc";
import type { BackendAPI } from "./shared-api";

const api: BackendAPI = {
  async add(a, b) { return a + b; },
  async echo(msg) { return `[bun] ${msg}`; },
  async getSystemInfo() {
    return { runtime: "bun", pid: process.pid, platform: process.platform, arch: process.arch };
  },
};

const io = new BunIo(Bun.stdin.stream());
const channel = new RPCChannel(io, { expose: api });
```

**Section sources**

- [README.md](file://README.md#L109-L137)

### 3. Spawn and Call from Frontend

```typescript
import { spawn, createChannel, onStdout, onStderr, onExit } from "tauri-plugin-js-api";
import type { BackendAPI } from "../backends/shared-api";

// Spawn a worker
await spawn("my-worker", { runtime: "bun", script: "bun-worker.ts", cwd: "/path/to/backends" });

// Listen to stdio events
onStdout("my-worker", (data) => console.log("[stdout]", data));
onStderr("my-worker", (data) => console.error("[stderr]", data));
onExit("my-worker", (code) => console.log("exited with", code));

// Create a typed RPC channel
const { api } = await createChannel<Record<string, never>, BackendAPI>("my-worker");

// Type-safe calls — checked at compile time
const sum = await api.add(5, 3);        // => 8
const info = await api.getSystemInfo(); // => { runtime: "bun", pid: 1234, ... }
```

**Section sources**

- [guest-js/index.ts](file://guest-js/index.ts)
- [README.md](file://README.md#L159-L178)

## Basic Usage

### Available Commands

| Function | Description |
|----------|-------------|
| `spawn(name, config)` | Start a named process |
| `kill(name)` | Kill a named process |
| `killAll()` | Kill all managed processes |
| `restart(name, config?)` | Restart a process (optionally with new config) |
| `listProcesses()` | List all running processes |
| `getStatus(name)` | Get status of a named process |
| `writeStdin(name, data)` | Write raw string to a process's stdin |
| `detectRuntimes()` | Detect installed runtimes (bun, node, deno) |
| `setRuntimePath(rt, path)` | Override executable path for a runtime |
| `getRuntimePaths()` | Get all custom path overrides |

**Section sources**

- [src/commands.rs](file://src/commands.rs)
- [README.md](file://README.md#L267-L283)

### Events

| Event | Payload | Description |
|-------|---------|-------------|
| `js-process-stdout` | `{ name, data }` | Line from process stdout |
| `js-process-stderr` | `{ name, data }` | Line from process stderr |
| `js-process-exit` | `{ name, code }` | Process exited |

**Section sources**

- [src/models.rs](file://src/models.rs#L31-L43)
- [README.md](file://README.md#L285-L291)

## Next Steps

- See [Architecture Overview](Architecture/Architecture%20Overview.md) for system design details
- See [Example Application](Development/Example%20Application.md) for a complete working demo
- See [Sidecar Support](Services/Sidecar%20Support.md) for production deployment without runtimes
