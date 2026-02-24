# tauri-plugin-js - PROJECT KNOWLEDGE BASE

**Generated:** 2026-02-25  
**Plugin:** Tauri v2 plugin for JS runtime process management  
**Stack:** Rust (tokio), TypeScript, kkrpc

---

## OVERVIEW

A Tauri v2 plugin that spawns and manages JavaScript runtime processes (Bun, Node.js, Deno) from desktop apps. Rust manages process lifecycles and relays stdio via Tauri events. Frontend communicates with backend JS processes through type-safe RPC powered by kkrpc.

---

## STRUCTURE

```
.
├── src/                  # Rust plugin source
│   ├── lib.rs            # Plugin init, command registration
│   ├── desktop.rs        # Desktop implementation (process mgmt)
│   ├── commands.rs       # Tauri command handlers (thin wrappers)
│   ├── models.rs         # Serde types for commands/events
│   ├── error.rs          # Error enum + thiserror
│   └── mobile.rs         # Mobile stub (not implemented)
├── guest-js/             # Frontend npm package
│   └── index.ts          # API + JsRuntimeIo (kkrpc adapter)
├── permissions/          # Tauri v2 permission system
├── examples/             # Demo tauri-app
└── vendors/kkrpc/        # RPC library (separate project)
```

---

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add command | `src/commands.rs` + `src/desktop.rs` | Register in `lib.rs` `generate_handler![]` |
| Event types | `src/models.rs` | Use `#[serde(rename_all = "camelCase")]` |
| Process spawn/kill | `src/desktop.rs` `Js` struct | Uses `tokio::process::Command` |
| Frontend API | `guest-js/index.ts` | Tauri invoke wrappers + event helpers |
| kkrpc IO adapter | `guest-js/index.ts` `JsRuntimeIo` | Bridges Tauri events ↔ kkrpc |
| Permissions | `permissions/default.toml` | Tauri v2 capability system |
| Build config | `build.rs` | Command list for tauri-plugin |

---

## CODE MAP

| Symbol | Type | Location | Role |
|--------|------|----------|------|
| `Js` | Struct | `desktop.rs:18` | Process manager (spawn, kill, stdio relay) |
| `ProcessEntry` | Struct | `desktop.rs:12` | Child + stdin handle + config |
| `spawn` | Command | `commands.rs:10` | Start named JS runtime process |
| `kill` / `kill_all` | Command | `commands.rs:19,24` | Terminate processes |
| `write_stdin` | Command | `commands.rs:48` | Send data to process stdin |
| `JsRuntimeIo` | Class | `guest-js:133` | kkrpc IoInterface implementation |
| `createChannel` | Function | `guest-js:228` | Helper to create typed RPC channel |
| `Error` | Enum | `error.rs:6` | thiserror-based errors, serializable |

---

## CONVENTIONS

### Rust

- **Error handling:** `thiserror::Error` enum in `error.rs`, exported as `crate::Result<T>`
- **Async runtime:** `tokio` (process, io-util, sync features) + `tauri::async_runtime`
- **State:** `Arc<Mutex<HashMap<...>>>` pattern in `Js` struct
- **Naming:** Snake_case commands (`spawn`, `kill_all`), camelCase events
- **Events:** Emit via `app.emit("js-process-stdout", payload)`

### TypeScript

- **Event filtering:** By process name: `if (event.payload.name !== this.processName) return`
- **Newline handling:** Re-append `\n` stripped by Rust `BufReader::lines()`
- **Destroy guard:** `isDestroyed` prevents spin loops; `read()` returns never-resolving promise when destroyed
- **Dynamic import:** `kkrpc/browser` loaded on-demand in `createChannel()`

### Tauri Patterns

- **Commands:** Thin async wrappers that delegate to `app.js().method().await`
- **Extension trait:** `JsExt<R>` provides `.js()` accessor on `AppHandle`
- **Cleanup:** `.on_event(|app, RunEvent::Exit| ... kill_all())` in plugin builder

---

## ANTI-PATTERNS (THIS PROJECT)

- ❌ **Do not parse RPC in Rust** — Rust is a thin relay; never parse JSON payloads
- ❌ **Do not forget `\n`** — `BufReader::lines()` strips newlines; frontend MUST re-append for kkrpc
- ❌ **Do not hold mutex across await** — Take data out of lock before `.await`
- ❌ **Do not drop stdin before taking** — `entry.stdin.take()` then `child.kill()`
- ❌ **Do not block kkrpc read() with null** — Return never-resolving promise when destroyed

---

## UNIQUE STYLES

### Stdio Relay Architecture

```
Frontend (kkrpc) ←→ Tauri Events ←→ Rust (BufReader lines) ←→ Child stdout/stderr
```

Rust spawns `tokio::spawn()` tasks per process to read stdout/stderr line-by-line and emit Tauri events.

### Sidecar Resolution

Production: `{name}` (bundler strips triple)  
Development: `{name}-{TARGET_TRIPLE}` (from `build.rs` env)  
Windows: Also checks `.exe` variants

### JsRuntimeIo Message Queue

```typescript
private queue: string[] = [];
private waitResolve: ((value: string | null) => void) | null = null;
```

Supports both push (listener) and pull (read promise) patterns for Tauri event → kkrpc adapter.

---

## COMMANDS

```bash
# Build plugin
pnpm build              # Rollup compiles guest-js

# Run example
cd examples/tauri-app
pnpm install
pnpm tauri dev          # Runs with dev mode runtime detection

# Test
# No test suite currently — test via examples/tauri-app
```

---

## NOTES

### kkrpc Integration

- **Browser entry:** Use `kkrpc/browser` (not `kkrpc`) for webview
- **IO adapter:** `JsRuntimeIo` implements `IoInterface` structurally (no explicit `implements`)
- **Newline protocol:** Critical for message framing — see ANTI-PATTERNS

### Process Lifecycle

1. `spawn(name, config)` → stores `ProcessEntry` in map
2. Stdio tasks emit `js-process-stdout/stderr` events
3. Exit watcher polls `child.try_wait()` every 100ms
4. On exit: remove from map, emit `js-process-exit`
5. `kill()` → drop stdin, then kill (avoids stdin write errors)

### Mobile

Mobile module is a stub (`mobile.rs`). Desktop implementation only.

### Vendors

`vendors/kkrpc/` is a complete separate project with its own AGENTS.md. Do not modify.
