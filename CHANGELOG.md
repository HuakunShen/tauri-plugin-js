# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-15

### Added
- Tauri v2 plugin (`tauri-plugin-js`) for spawning and managing JavaScript runtime processes.
- Frontend API package (`tauri-plugin-js-api`) with command wrappers and event helpers.
- `JsRuntimeIo` adapter that bridges Tauri events to kkrpc `IoInterface` over stdio.
- `createChannel<LocalAPI, RemoteAPI>()` helper for typed bidirectional RPC.
- Support for Bun, Node.js, and Deno runtimes, plus Tauri sidecar binaries.
- Runtime auto-detection, custom runtime paths, and clean shutdown on app exit.
- Example app under `examples/tauri-app/` demonstrating all features.

### Notes
- This is the first published release. Public API may evolve before `1.0`.
