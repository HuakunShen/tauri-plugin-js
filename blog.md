# Tauri Without Electron Bloat: A Type-Safe JS Runtime Bridge with `tauri-plugin-js`

If you love Tauri's small footprint but still need a real JavaScript runtime process, you usually hit a wall: webview code is not enough for some workloads (native modules, long-running compute, local AI tooling, heavyweight filesystem orchestration). This repo shows a clean way around that wall.

`tauri-plugin-js` gives a Tauri v2 app a managed process layer for Bun, Node.js, and Deno, while keeping RPC strongly typed through `kkrpc`. The key design choice is simple and powerful: Rust handles lifecycle and I/O transport, but the RPC protocol remains end-to-end JavaScript.

---

## The architecture in one sentence

**Rust is a process/event relay, JavaScript is the RPC protocol owner.**

You can see this split directly in the code:

- Process management and stdio relay live in `src/desktop.rs`.
- Tauri command surface is in `src/commands.rs`, registered in `src/lib.rs`.
- Frontend transport adapter + API wrappers are in `guest-js/index.ts`.

That split avoids protocol duplication in Rust and keeps your app-level API types in TypeScript where your frontend and worker code already live.

---

## Why this model works better than ad-hoc sidecar wiring

Most sidecar implementations eventually re-invent three things:

1. Process lifecycle management
2. Message framing and stream handling
3. Typed request/response contracts

This plugin hardens all three.

### 1) Lifecycle is first-class, not an afterthought

The plugin exposes commands such as `spawn`, `kill`, `kill_all`, `restart`, `list_processes`, `get_status`, `write_stdin`, `detect_runtimes`, `set_runtime_path`, and `get_runtime_paths` (`src/lib.rs`, `src/commands.rs`).

On app exit, it force-cleans all child processes in the plugin event hook (`src/lib.rs`), preventing orphaned workers.

### 2) Stream transport has production-grade edge handling

Two subtle but critical details are implemented in `guest-js/index.ts`:

- **Newline framing restoration:** Rust uses `BufReader::lines()` (`src/desktop.rs`), which strips trailing `\n`. The JS adapter re-appends `\n` before handing messages to kkrpc.
- **No spin-loop on teardown:** `JsRuntimeIo.read()` returns a never-resolving promise when destroyed, paired with `isDestroyed` semantics used by kkrpc's channel loop. This prevents CPU spin when a channel is torn down.

These are exactly the kind of tiny transport bugs that make IPC feel flaky in real apps.

### 3) Type-safe API remains shared across UI and workers

In the example app, one interface (`examples/tauri-app/backends/shared-api.ts`) drives calls from Svelte UI to Bun/Node/Deno workers. The same typed calls (`createChannel<..., BackendAPI>`) are used regardless of runtime (`examples/tauri-app/src/App.svelte`).

---

## Runtime mode vs sidecar mode: practical deployment strategy

This repo demonstrates both paths clearly.

## Runtime mode (great for dev)

Use `spawn({ runtime, script, cwd })` to run source workers directly.

- In dev, scripts resolve from `backends/` (`resolve("..", "backends")`) in `examples/tauri-app/src/App.svelte`.
- Runtime availability is probed via `detectRuntimes()` and rendered in UI.

## Sidecar mode (great for distribution)

Compile workers into standalone binaries and spawn with `spawn({ sidecar: "name" })`.

The sidecar resolution logic in `src/desktop.rs` is especially solid:

- Tries plain binary name next to app executable (production bundle behavior).
- Falls back to target-triple suffix (`{name}-{triple}`) for development.
- Includes Windows `.exe` variants.

This allows one frontend API while supporting both dev and packaged execution models.

---

## The Deno compile trap (and the clean workaround)

One of the most valuable implementation lessons in this repo:

- `deno compile` is intentionally run from a separate package (`examples/deno-compile/`), not from `examples/tauri-app/backends/`.
- The separate package has its own `deno.json` import map (`examples/deno-compile/deno.json`).

The build script (`examples/tauri-app/scripts/build-sidecars.sh`) compiles:

- Bun sidecar from `backends/bun-worker.ts`
- Deno sidecar from `../../deno-compile/main.ts`

This separation avoids compilation behavior that can go wrong in directories containing `node_modules`.

---

## A small but smart DX trick: placeholder resources

`examples/tauri-app/scripts/ensure-resources.sh` creates placeholder files for:

- `externalBin` targets
- bundled `workers/*` resources

Why this matters: `tauri dev` can fail if configured assets do not exist yet. Creating placeholders keeps local dev bootable before running sidecar/resource build scripts.

The corresponding config is visible in `examples/tauri-app/src-tauri/tauri.conf.json` (`externalBin` + `resources`).

---

## Multi-window behavior is naturally supported

Capability config includes `"main"` and `"window-*"` (`examples/tauri-app/src-tauri/capabilities/default.json`), and the example UI can spawn extra windows via `WebviewWindow` (`examples/tauri-app/src/App.svelte`).

Because process events are emitted through Tauri's event system and process names are explicit, windows can share backend workers without inventing custom routing layers in Rust.

---

## Code-level flow from click to typed result

1. Frontend calls `spawn(name, config)` via wrapper in `guest-js/index.ts`.
2. Rust plugin launches child process and stores process entry in map (`src/desktop.rs`).
3. Rust emits stdout/stderr/exit events (`js-process-stdout`, `js-process-stderr`, `js-process-exit`).
4. Frontend `JsRuntimeIo` listens to process-specific events and feeds kkrpc transport.
5. `createChannel` returns typed remote API proxy.
6. UI calls `api.add`, `api.echo`, `api.getSystemInfo`, `api.fibonacci` (`examples/tauri-app/src/App.svelte`).

No protocol parsing in Rust, no duplicated schemas, no ad-hoc JSON glue per method.

---

## What to copy into your own Tauri app

If you are adopting this pattern, keep these decisions:

- Keep Rust transport-thin; do not parse RPC payloads in Rust.
- Preserve newline message framing across stdio boundaries.
- Enforce teardown semantics (`isDestroyed`) in your IO adapter.
- Support both runtime and sidecar execution paths behind one frontend API.
- Keep Deno compile inputs in a dedicated Deno package.
- Add dev-time placeholder generation for configured bundle assets.

If you copy only one thing: copy the **transport discipline** in `guest-js/index.ts` and the **sidecar resolution strategy** in `src/desktop.rs`.

---

## References

### Repo sources

- `README.md`
- `SKILL.md`
- `src/lib.rs`
- `src/commands.rs`
- `src/desktop.rs`
- `guest-js/index.ts`
- `examples/tauri-app/README.md`
- `examples/tauri-app/src/App.svelte`
- `examples/tauri-app/scripts/build-sidecars.sh`
- `examples/tauri-app/scripts/build-workers.sh`
- `examples/tauri-app/scripts/ensure-resources.sh`
- `examples/tauri-app/src-tauri/tauri.conf.json`
- `examples/tauri-app/src-tauri/capabilities/default.json`
- `examples/deno-compile/deno.json`
- `examples/deno-compile/main.ts`

### External docs (for production hardening)

- Tauri v2 permissions: https://v2.tauri.app/security/permissions/
- Tauri capabilities: https://v2.tauri.app/reference/acl/capability/
- Tauri sidecars: https://v2.tauri.app/develop/sidecar/
- kkrpc docs: https://docs.kkrpc.kunkun.sh/
