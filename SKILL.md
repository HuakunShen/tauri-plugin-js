---
name: tauri-js-runtime
description: Add JavaScript runtime backend capabilities to Tauri v2 desktop apps. Covers both using the tauri-plugin-js plugin and building from scratch. Use when integrating Bun, Node.js, or Deno as backend processes in Tauri, setting up type-safe RPC between frontend and JS runtimes, creating Electron-like architectures in Tauri, or managing child processes with stdio communication.
version: 1.0.0
license: MIT
metadata:
  domain: desktop-apps
  tags:
    - tauri
    - bun
    - node
    - deno
    - rpc
    - kkrpc
    - electron-alternative
    - process-management
---

# Tauri + JS Runtime Integration

Give Tauri apps full JS runtime backends (Bun, Node.js, Deno) with type-safe bidirectional RPC. This covers two approaches: using the `tauri-plugin-js` plugin, and building the integration from scratch.

## When to Use

- User wants to run JS/TS backend code from a Tauri desktop app
- User asks about Electron alternatives or "Electron-like" features in Tauri
- User needs to spawn/manage child processes (Bun, Node, Deno) from Rust
- User wants type-safe RPC between a Tauri webview and a JS runtime
- User needs stdio-based IPC between Rust and a child process
- User asks about kkrpc integration with Tauri
- User wants multi-window apps where windows share backend processes
- User needs runtime detection (which runtimes are installed, paths, versions)

## Core Architecture

```
Frontend (Webview)  <-- Tauri Events -->  Rust Core  <-- stdio -->  JS Runtime
```

- **Rust** spawns child processes, pipes their stdin/stdout/stderr, and relays data via Tauri events
- **Rust never parses RPC payloads** — it forwards raw newline-delimited strings
- **kkrpc** handles the RPC protocol on both ends (frontend webview + backend runtime)
- **Frontend IO adapter** bridges Tauri events to kkrpc's IoInterface (read/write/on/off)
- **Multi-window** works because all windows receive the same Tauri events; kkrpc request IDs handle routing

## Approach A: Using tauri-plugin-js (Recommended)

The plugin handles process management, stdio relay, event emission, and provides a frontend npm package with typed wrappers and an IO adapter.

### Step 1: Install

**Rust** — add to `src-tauri/Cargo.toml`:
```toml
[dependencies]
tauri-plugin-js = { path = "../path/to/tauri-plugin-js" }
```

**Frontend** — install npm packages:
```bash
pnpm add tauri-plugin-js-api kkrpc
```

### Step 2: Register the plugin

In `src-tauri/src/lib.rs`:
```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_js::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Step 3: Add permissions

In `src-tauri/capabilities/default.json`:
```json
{
  "permissions": [
    "core:default",
    "js:default"
  ]
}
```

### Step 4: Define a shared API type

Create a type definition shared between frontend and backend workers:
```typescript
// backends/shared-api.ts
export interface BackendAPI {
  add(a: number, b: number): Promise<number>;
  echo(message: string): Promise<string>;
  getSystemInfo(): Promise<{
    runtime: string;
    pid: number;
    platform: string;
    arch: string;
  }>;
}
```

### Step 5: Write backend workers

Each runtime has its own IO adapter from kkrpc:

**Bun** (`backends/bun-worker.ts`):
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

**Node** (`backends/node-worker.mjs`):
```javascript
import { RPCChannel, NodeIo } from "kkrpc";

const api = {
  async add(a, b) { return a + b; },
  async echo(msg) { return `[node] ${msg}`; },
  async getSystemInfo() {
    return { runtime: "node", pid: process.pid, platform: process.platform, arch: process.arch };
  },
};

const io = new NodeIo(process.stdin, process.stdout);
const channel = new RPCChannel(io, { expose: api });
```

**Deno** (`backends/deno-worker.ts`):
```typescript
import { DenoIo, RPCChannel } from "npm:kkrpc/deno";
import type { BackendAPI } from "./shared-api.ts";  // .ts extension required by Deno

const api: BackendAPI = {
  async add(a, b) { return a + b; },
  async echo(msg) { return `[deno] ${msg}`; },
  async getSystemInfo() {
    return { runtime: "deno", pid: Deno.pid, platform: Deno.build.os, arch: Deno.build.arch };
  },
};

const io = new DenoIo(Deno.stdin.readable);
const channel = new RPCChannel(io, { expose: api });
```

### Step 6: Frontend — spawn and call

```typescript
import { spawn, createChannel, onStdout, onStderr, onExit } from "tauri-plugin-js-api";
import type { BackendAPI } from "../backends/shared-api";

// Spawn
const cwd = await resolve("..", "backends");  // from @tauri-apps/api/path
await spawn("my-worker", { runtime: "bun", script: "bun-worker.ts", cwd });

// Events
onStdout("my-worker", (data) => console.log(data));
onStderr("my-worker", (data) => console.error(data));
onExit("my-worker", (code) => console.log("exited", code));

// Type-safe RPC
const { api } = await createChannel<Record<string, never>, BackendAPI>("my-worker");
const result = await api.add(5, 3);  // compile-time checked
```

### Step 7: Runtime detection (optional)

```typescript
import { detectRuntimes, setRuntimePath } from "tauri-plugin-js-api";

const runtimes = await detectRuntimes();
// [{ name: "bun", available: true, version: "1.2.0", path: "/usr/local/bin/bun" }, ...]

// Override path for a specific runtime
await setRuntimePath("node", "/custom/path/to/node");
```

### Plugin API Summary

| Command | Description |
|---------|-------------|
| `spawn(name, config)` | Start a named process |
| `kill(name)` | Kill by name |
| `killAll()` | Kill all |
| `restart(name, config?)` | Restart with optional new config |
| `listProcesses()` | List running processes |
| `getStatus(name)` | Get process status |
| `writeStdin(name, data)` | Write raw string to stdin |
| `detectRuntimes()` | Detect bun/node/deno availability |
| `setRuntimePath(rt, path)` | Set custom executable path |
| `getRuntimePaths()` | Get custom path overrides |

| Event | Payload |
|-------|---------|
| `js-process-stdout` | `{ name: string, data: string }` |
| `js-process-stderr` | `{ name: string, data: string }` |
| `js-process-exit` | `{ name: string, code: number \| null }` |

---

## Approach B: Building from Scratch

When you need full control or a different architecture (e.g., single shared process instead of named processes, Tauri event relay instead of direct stdio, or SvelteKit/other frameworks).

### Step 1: Rust — spawn and relay

Add tokio to `src-tauri/Cargo.toml`:
```toml
[dependencies]
tokio = { version = "1", features = ["process", "io-util", "sync", "rt"] }
```

Core Rust pattern in `src-tauri/src/lib.rs`:
```rust
use std::sync::Arc;
use tauri::{async_runtime, AppHandle, Emitter, Listener, Manager};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;

struct ProcessState {
    child: Child,
    stdin: ChildStdin,
}

struct AppState {
    process: Arc<Mutex<Option<ProcessState>>>,
}

fn spawn_runtime(app: &AppHandle) -> Result<ProcessState, String> {
    let mut cmd = Command::new("bun");
    cmd.args(["src/backend/main.ts"]);
    cmd.stdin(std::process::Stdio::piped());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let stderr = child.stderr.take().ok_or("no stderr")?;

    // Relay stdout to all frontend windows via Tauri events
    let handle = app.clone();
    async_runtime::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let _ = handle.emit("runtime-stdout", &line);
        }
    });

    // Relay stderr
    let handle2 = app.clone();
    async_runtime::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("[runtime stderr] {}", line);
            let _ = handle2.emit("runtime-stderr", &line);
        }
    });

    Ok(ProcessState { child, stdin })
}
```

Listen for frontend-to-runtime messages:
```rust
// In .setup() closure:
app.listen("frontend-to-runtime", move |event| {
    let payload = event.payload().to_string();
    let state = state_clone.clone();
    async_runtime::spawn(async move {
        let mut guard = state.lock().await;
        if let Some(ref mut proc) = *guard {
            let msg: String = serde_json::from_str(&payload).unwrap_or(payload);
            let mut to_write = msg;
            if !to_write.ends_with('\n') {
                to_write.push('\n');
            }
            let _ = proc.stdin.write_all(to_write.as_bytes()).await;
            let _ = proc.stdin.flush().await;
        }
    });
});
```

### Step 2: Frontend IO adapter

Bridge Tauri events to kkrpc's IoInterface:
```typescript
import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";

export class TauriEventIo {
  name = "tauri-event-io";
  isDestroyed = false;
  private listeners: Set<(msg: string) => void> = new Set();
  private queue: string[] = [];
  private pendingReads: Array<(value: string | null) => void> = [];
  private unlisten: UnlistenFn | null = null;

  async initialize(): Promise<void> {
    this.unlisten = await listen<string>("runtime-stdout", (event) => {
      // CRITICAL: re-append \n that BufReader::lines() strips
      const message = event.payload + "\n";

      for (const listener of this.listeners) listener(message);

      if (this.pendingReads.length > 0) {
        this.pendingReads.shift()!(message);
      } else {
        this.queue.push(message);
      }
    });
  }

  async read(): Promise<string | null> {
    if (this.isDestroyed) return new Promise(() => {});  // hang, don't spin
    if (this.queue.length > 0) return this.queue.shift()!;
    return new Promise((resolve) => this.pendingReads.push(resolve));
  }

  async write(message: string): Promise<void> {
    await emit("frontend-to-runtime", message);
  }

  on(event: "message" | "error", listener: (msg: string) => void) {
    if (event === "message") this.listeners.add(listener);
  }

  off(event: "message" | "error", listener: Function) {
    if (event === "message") this.listeners.delete(listener as any);
  }

  destroy() {
    this.isDestroyed = true;
    this.unlisten?.();
    this.pendingReads.forEach((r) => r(null));
    this.pendingReads = [];
    this.queue = [];
    this.listeners.clear();
  }
}
```

### Step 3: Connect kkrpc

```typescript
import { RPCChannel } from "kkrpc/browser";
import type { BackendAPI } from "../backend/types";

const io = new TauriEventIo();
await io.initialize();

const channel = new RPCChannel<{}, BackendAPI>(io, { expose: {} });
const api = channel.getAPI() as BackendAPI;

// Type-safe calls
const result = await api.add(5, 3);
```

### Step 4: Clean shutdown

```rust
// In .build().run() callback:
.run(move |app_handle, event| {
    if let RunEvent::ExitRequested { .. } = &event {
        let state = app_handle.state::<AppState>();
        let proc = state.process.clone();
        async_runtime::block_on(async {
            let mut guard = proc.lock().await;
            if let Some(mut proc) = guard.take() {
                drop(proc.stdin);  // drop stdin first
                let _ = proc.child.kill().await;
                let _ = proc.child.wait().await;
            }
        });
    }
});
```

### Step 5: Capabilities for multi-window

```json
{
  "windows": ["main", "window-*"],
  "permissions": [
    "core:default",
    "core:event:default",
    "core:webview:allow-create-webview-window"
  ]
}
```

Note: `WebviewWindow` from `@tauri-apps/api/webviewWindow` requires `core:webview:allow-create-webview-window`, not `core:window:allow-create`.

---

## Critical Pitfalls

### 1. Newline framing
Rust's `BufReader::lines()` strips trailing `\n`. kkrpc (and all newline-delimited JSON protocols) need `\n` to delimit messages. **The frontend IO adapter MUST re-append `\n`** to every payload received from Tauri events.

### 2. kkrpc read loop spin
kkrpc's internal `listen()` loop continues on `null` reads — it only stops if the IO adapter has `isDestroyed === true`. If `read()` returns `null` without `isDestroyed` being set, the loop spins at 100% CPU. **Solution:** `read()` should return a never-resolving promise when destroyed, and expose `isDestroyed`.

### 3. Channel cleanup
Call `channel.destroy()` (not just `io.destroy()`) to properly reject pending RPC promises. The channel's destroy will call io.destroy internally.

### 4. Mutex contention in Rust
The Tauri event listener for stdin writes and the kill/restart commands both need the process mutex. **Take the process handle out of the lock scope before kill/wait.** Drop stdin first to unblock pending writes.

### 5. Tauri event serialization
Tauri events serialize payloads as JSON strings. When the Rust event listener receives a message to forward to stdin, it may need to deserialize the outer JSON string wrapper: `serde_json::from_str::<String>(&payload)`.

### 6. Vite pre-bundle cache
When using the plugin with `file:` dependency links, Vite caches the pre-bundled version. After rebuilding the plugin's guest-js, delete `node_modules/.vite` in the consuming app and run `pnpm install` to pick up new exports.

### 7. Deno imports
Deno workers must use `npm:kkrpc/deno` for the import specifier and `.ts` file extensions for local imports (e.g., `./shared-api.ts`).

### 8. Dev vs Prod mode
In dev mode, spawn runtimes directly (`bun script.ts`). In production, consider:
- **Compiled sidecar:** `bun build --compile` produces a standalone binary, use Tauri's `externalBin` config
- **Bundled JS:** `bun build --outfile main.js` produces a single file, bundle it as a Tauri resource and run with system-installed runtime
- The Rust code should check for sidecar first, then fall back to bundled JS with system runtime

## Production Deployment

### Option 1: Compiled sidecar (no runtime needed on user machine)
```bash
bun build --compile src/backend/main.ts --outfile src-tauri/binaries/backend-$(rustc -vV | grep host | cut -d' ' -f2)
```
Add to `tauri.conf.json`:
```json
{ "bundle": { "externalBin": ["binaries/backend"] } }
```

### Option 2: Bundled JS (requires runtime on user machine)
```bash
bun build src/backend/main.ts --outfile dist-backend/main.js --target=bun
```
Add to `tauri.conf.json`:
```json
{ "bundle": { "resources": ["../dist-backend/**"] } }
```

## References

- [kkrpc](https://github.com/nicepkg/kkrpc) — cross-runtime RPC library
- [Tauri v2 Plugin Guide](https://tauri.app/develop/plugins/)
- [Tauri v2 Capabilities](https://tauri.app/security/capabilities/)
