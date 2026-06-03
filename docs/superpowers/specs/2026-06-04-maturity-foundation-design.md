# Maturity Foundation Design

## Goal

Improve `tauri-plugin-js` project maturity without turning the first pass into a full release automation project. The work should make the repository easier to trust, build, test, and contribute to while keeping the implementation small and maintainable.

## Scope

This design covers four areas:

- Continuous integration for core build checks.
- Minimal automated tests for pure Rust logic and TypeScript package output.
- Project documentation for changelog, contribution workflow, and verification commands.
- Small source refactors only where needed to make behavior testable.

This design does not include automated publishing to npm or crates.io, full GUI end-to-end tests, or cross-platform sidecar compilation in CI.

## Current State

The project already has a strong feature set: Bun/Node/Deno process management, typed RPC via kkrpc, sidecar support, runtime detection, a complete example app, and detailed documentation. The main maturity gaps are operational rather than architectural:

- No `.github/workflows` CI exists.
- No first-party automated test suite exists.
- `AGENTS.md` explicitly notes that testing currently happens through `examples/tauri-app`.
- There is no `CHANGELOG.md` or `CONTRIBUTING.md`.
- Some important behavior in `src/desktop.rs` is embedded directly in methods, making focused unit tests harder.

## Recommended Approach

Use a minimal maturity foundation:

- Add CI that proves the Rust crate and TypeScript guest package build on clean machines.
- Add focused unit tests for pure logic rather than trying to launch Tauri or GUI flows.
- Add docs that tell contributors and future maintainers exactly how to verify changes locally.
- Extract only the smallest functions needed for testing command construction and sidecar path candidates.

This approach raises confidence materially without creating brittle CI jobs that depend on full desktop GUI support or installed runtime binaries.

## Architecture Changes

### Rust Testability

`src/desktop.rs` currently builds runtime commands and resolves sidecar paths inline. The implementation should keep the public API the same but extract small private helpers:

- A helper that converts `SpawnConfig` into a program plus argument vector for runtime or command-based spawning.
- A helper that produces sidecar candidate paths for a given executable directory, sidecar name, and target triple.
- A helper or small branch that applies runtime path overrides without spawning a process.

These helpers should remain private unless Rust module visibility requires `pub(crate)` for tests. They should not introduce a new module unless the file becomes noticeably clearer by doing so.

### CI Workflow

Add `.github/workflows/ci.yml` with jobs that run on pull requests and pushes to the default branch. The workflow should prefer stable, predictable checks:

- Install Rust stable.
- Install pnpm.
- Run `pnpm install --frozen-lockfile`.
- Run `pnpm build` for `guest-js`/`dist-js` generation.
- Run `cargo check --locked` at the plugin root.
- Run `cargo test --locked` at the plugin root.

Example app GUI startup is intentionally excluded. The example can be documented as a manual smoke test.

### Tests

Add minimal Rust unit tests for logic that does not require a Tauri app instance:

- Bun command construction with script and extra args.
- Deno command construction, including `run -A` prefix.
- Node command construction.
- Custom command mode.
- Invalid runtime or missing execution mode errors.
- Sidecar candidate path ordering for production and development names.

TypeScript verification should rely on `pnpm build` initially. A dedicated TS test framework is not required for this pass.

### Documentation

Add `CHANGELOG.md` using the Keep a Changelog style and SemVer framing. Include an initial `0.1.0` entry matching the current package version.

Add `CONTRIBUTING.md` with:

- Local setup commands.
- Verification commands.
- Example app smoke-test instructions.
- Notes on not modifying `vendors/kkrpc/` unless intentionally updating the vendor copy.
- Release checklist without automated publishing.

Update `README.md` with concise sections for:

- Development checks.
- CI status expectations.
- Manual example smoke test.

## Error Handling

Test helper extraction must preserve existing error behavior. Invalid spawn configuration should still produce `Error::InvalidConfig`. I/O errors should still flow through `Error::Io`. CI should fail fast on build or test failures.

## Verification Plan

Local verification after implementation:

- `pnpm build`
- `cargo check --locked`
- `cargo test --locked`

Optional manual smoke test:

- `cd examples/tauri-app`
- `pnpm install`
- `pnpm tauri dev`

## Non-Goals

- Do not add automated npm or crates.io publishing.
- Do not add full Tauri GUI end-to-end tests in CI.
- Do not require Bun, Node, or Deno runtime binaries in CI beyond what the JavaScript package manager needs.
- Do not redesign the stdio/kkrpc transport.
- Do not modify `vendors/kkrpc/`.

## Success Criteria

The work is successful when:

- The repository has a CI workflow that runs core build and test checks.
- The Rust crate has meaningful tests for spawn configuration and sidecar resolution behavior.
- Documentation explains how to contribute, verify, and perform a release manually.
- Existing public API and example behavior remain unchanged.
