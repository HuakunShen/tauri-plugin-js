# Sidecar Support

<cite>
**Referenced Files in This Document**
- [src/desktop.rs](file://src/desktop.rs)
- [build.rs](file://build.rs)
- [examples/tauri-app/src-tauri/tauri.conf.json](file://examples/tauri-app/src-tauri/tauri.conf.json)
- [README.md](file://README.md)
</cite>

## Table of Contents

1. [Overview](#overview)
2. [Compiling Workers](#compiling-workers)
3. [Sidecar Resolution](#sidecar-resolution)
4. [Configuration](#configuration)
5. [Usage](#usage)

## Overview

Sidecars are standalone executable binaries compiled from your worker scripts. They allow your app to run JS workers without requiring users to have Bun, Node, or Deno installed.

### Benefits

- **No runtime dependency** — Users don't need any JS runtime
- **Faster startup** — No runtime initialization overhead
- **Smaller distribution** — Only the worker code, not the entire runtime
- **Simpler deployment** — Single binary per platform

**Section sources**

- [README.md](file://README.md#L180-L216)

## Compiling Workers

### Bun

Bun can compile TypeScript directly:

```bash
TARGET=$(rustc -vV | grep host | cut -d' ' -f2)

bun build --compile --minify backends/bun-worker.ts \
  --outfile src-tauri/binaries/bun-worker-$TARGET
```

**Section sources**

- [README.md](file://README.md#L184-L189)

### Deno

> **Important**: `deno compile` crashes with a stack overflow if run from a directory containing `node_modules`. Deno worker source must live in a separate directory with its own `deno.json`.

```bash
TARGET=$(rustc -vV | grep host | cut -d' ' -f2)

deno compile --allow-all \
  --output src-tauri/binaries/deno-worker-$TARGET \
  path/to/deno-package/main.ts
```

**Section sources**

- [README.md](file://README.md#L191-L192)
- [examples/deno-compile/main.ts](file://examples/deno-compile/main.ts)

### Deno Package Structure

The example uses a separate Deno package:

```
examples/deno-compile/
├── deno.json        # Declares kkrpc dependency
├── deno.lock
├── main.ts          # Worker implementation
└── shared-api.ts    # API types
```

**Section sources**

- [examples/deno-compile/main.ts](file://examples/deno-compile/main.ts)

## Sidecar Resolution

The plugin resolves sidecar binaries by looking next to the app executable.

### Resolution Logic

```rust
fn resolve_sidecar(&self, name: &str) -> crate::Result<std::path::PathBuf> {
    let current_exe = std::env::current_exe().map_err(crate::Error::Io)?;
    let exe_dir = current_exe.parent().ok_or_else(|| {
        crate::Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine executable directory",
        ))
    })?;

    // Production: bundler strips the target triple
    let candidate = exe_dir.join(name);
    if candidate.exists() {
        return Ok(candidate);
    }

    #[cfg(windows)]
    {
        let candidate = exe_dir.join(format!("{name}.exe"));
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // Development: tauri dev preserves the target triple suffix
    let triple = env!("TARGET_TRIPLE");
    let candidate = exe_dir.join(format!("{name}-{triple}"));
    if candidate.exists() {
        return Ok(candidate);
    }

    #[cfg(windows)]
    {
        let candidate = exe_dir.join(format!("{name}-{triple}.exe"));
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(crate::Error::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("sidecar not found: {name} (looked in {})", exe_dir.display()),
    )))
}
```

**Section sources**

- [src/desktop.rs](file://src/desktop.rs#L220-L262)

### Target Triple

The target triple is embedded at compile time via `build.rs`:

```rust
fn main() {
    println!(
        "cargo:rustc-env=TARGET_TRIPLE={}",
        std::env::var("TARGET").unwrap()
    );
    // ...
}
```

**Section sources**

- [build.rs](file://build.rs#L15-L19)

### Resolution Table

| Environment | Binary Name | Looks For |
|-------------|-------------|-----------|
| Production (macOS) | `bun-worker` | `bun-worker` |
| Production (Windows) | `bun-worker` | `bun-worker.exe` |
| Development (macOS) | `bun-worker` | `bun-worker-aarch64-apple-darwin` |
| Development (Windows) | `bun-worker` | `bun-worker-x86_64-pc-windows-msvc.exe` |

## Configuration

### Tauri Config

Add sidecars to `externalBin` in `tauri.conf.json`:

```json
{
  "bundle": {
    "externalBin": [
      "binaries/bun-worker",
      "binaries/deno-worker"
    ]
  }
}
```

**Section sources**

- [examples/tauri-app/src-tauri/tauri.conf.json](file://examples/tauri-app/src-tauri/tauri.conf.json#L31-L34)

### Directory Structure

```
src-tauri/
├── binaries/
│   ├── bun-worker-aarch64-apple-darwin
│   ├── bun-worker-x86_64-pc-windows-msvc.exe
│   ├── deno-worker-aarch64-apple-darwin
│   └── deno-worker-x86_64-pc-windows-msvc.exe
└── tauri.conf.json
```

## Usage

### Spawning a Sidecar

```typescript
import { spawn, createChannel } from "tauri-plugin-js-api";
import type { BackendAPI } from "./shared-api";

// Spawn using sidecar config (not runtime)
await spawn("my-compiled-worker", {
  sidecar: "bun-worker"
});

// RPC works identically to runtime-based workers
const { api } = await createChannel<Record<string, never>, BackendAPI>(
  "my-compiled-worker"
);

const sum = await api.add(5, 3); // => 8
```

**Section sources**

- [README.md](file://README.md#L203-L212)

### Worker Code

The worker code is identical whether running via runtime or as a sidecar. The compiled binary preserves stdin/stdout behavior, so kkrpc works unchanged:

```typescript
// Works the same in both runtime and compiled binary
import { RPCChannel, BunIo } from "kkrpc";
import type { BackendAPI } from "./shared-api";

const api: BackendAPI = { /* ... */ };
const io = new BunIo(Bun.stdin.stream());
const channel = new RPCChannel(io, { expose: api });
```

**Section sources**

- [README.md](file://README.md#L213-L216)
