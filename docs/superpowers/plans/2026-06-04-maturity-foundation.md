# Maturity Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add CI, minimal Rust tests, and contributor-facing documentation to `tauri-plugin-js` to raise project maturity without introducing brittle GUI/release automation.

**Architecture:** Keep the public API of the Rust crate unchanged. Extract two small private helpers in `src/desktop.rs` so the most error-prone logic (runtime command construction and sidecar path resolution) can be unit tested. Add a single GitHub Actions workflow that runs pnpm and cargo checks/tests. Add `CHANGELOG.md` and `CONTRIBUTING.md`, and expand `README.md` with development/CI notes.

**Tech Stack:** Rust 2021 edition, Cargo, tauri-plugin build pipeline, pnpm, rollup, TypeScript, GitHub Actions.

---

## File Structure

### Created
- `.github/workflows/ci.yml` — CI workflow that runs pnpm build, cargo check, and cargo test.
- `CHANGELOG.md` — Keep a Changelog formatted changelog with initial `0.1.0` entry.
- `CONTRIBUTING.md` — Setup, verification, example smoke test, and release checklist.
- `docs/superpowers/plans/2026-06-04-maturity-foundation.md` — This plan.

### Modified
- `src/desktop.rs` — Extract `build_spawn_program` and `sidecar_candidate_paths` helpers; keep public API and behavior identical.
- `src/lib.rs` — Re-export helpers behind `cfg(any(test, feature = "test-helpers"))` is not required; helpers stay private but unit-testable from within the module.
- `README.md` — Add short Development, CI, and Manual Smoke Test sections near the existing install instructions.

### Untouched
- `vendors/kkrpc/` — Vendor copy, must not change.
- Example app — Out of scope for this plan.

---

## Task 1: Extract testable runtime command construction

**Files:**
- Modify: `src/desktop.rs` (top of file through current `spawn` method)
- Test: `src/desktop.rs` `#[cfg(test)] mod tests`

- [ ] **Step 1: Add the failing test**

Append the following module at the bottom of `src/desktop.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_with_runtime(runtime: &str, script: Option<&str>, extra_args: Option<Vec<&str>>) -> SpawnConfig {
        SpawnConfig {
            runtime: Some(runtime.to_string()),
            command: None,
            sidecar: None,
            script: script.map(|s| s.to_string()),
            args: extra_args.map(|a| a.into_iter().map(String::from).collect()),
            cwd: None,
            env: None,
        }
    }

    #[test]
    fn build_bun_command_with_script_and_args() {
        let cfg = cfg_with_runtime("bun", Some("worker.ts"), Some(vec!["--watch"]));
        let (program, args) = build_spawn_program(&cfg).expect("bun config should build");
        assert_eq!(program, "bun");
        assert_eq!(args, vec!["worker.ts".to_string(), "--watch".to_string()]);
    }

    #[test]
    fn build_bun_command_without_script() {
        let cfg = cfg_with_runtime("bun", None, None);
        let (program, args) = build_spawn_program(&cfg).expect("bun config should build");
        assert_eq!(program, "bun");
        assert!(args.is_empty());
    }

    #[test]
    fn build_deno_command_includes_run_all() {
        let cfg = cfg_with_runtime("deno", Some("main.ts"), None);
        let (program, args) = build_spawn_program(&cfg).expect("deno config should build");
        assert_eq!(program, "deno");
        assert_eq!(args, vec!["run".to_string(), "-A".to_string(), "main.ts".to_string()]);
    }

    #[test]
    fn build_node_command_preserves_args_order() {
        let cfg = cfg_with_runtime("node", Some("server.mjs"), Some(vec!["--port", "8080"]));
        let (program, args) = build_spawn_program(&cfg).expect("node config should build");
        assert_eq!(program, "node");
        assert_eq!(args, vec!["server.mjs".to_string(), "--port".to_string(), "8080".to_string()]);
    }

    #[test]
    fn build_command_uses_direct_path() {
        let cfg = SpawnConfig {
            runtime: None,
            command: Some("/usr/local/bin/custom".to_string()),
            sidecar: None,
            script: None,
            args: Some(vec!["--flag".to_string()]),
            cwd: None,
            env: None,
        };
        let (program, args) = build_spawn_program(&cfg).expect("command config should build");
        assert_eq!(program, "/usr/local/bin/custom");
        assert_eq!(args, vec!["--flag".to_string()]);
    }

    #[test]
    fn build_command_rejects_unknown_runtime() {
        let cfg = cfg_with_runtime("qjs", None, None);
        let err = build_spawn_program(&cfg).expect_err("unknown runtime should fail");
        match err {
            crate::Error::InvalidConfig(msg) => assert!(msg.contains("qjs")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[test]
    fn build_command_rejects_missing_execution_mode() {
        let cfg = SpawnConfig {
            runtime: None,
            command: None,
            sidecar: None,
            script: None,
            args: None,
            cwd: None,
            env: None,
        };
        let err = build_spawn_program(&cfg).expect_err("empty config should fail");
        match err {
            crate::Error::InvalidConfig(_) => {}
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Run the new tests to confirm they fail to compile**

Run from the plugin root:

```bash
cargo test --locked --no-run
```

Expected: compile error referencing `build_spawn_program` as undefined.

- [ ] **Step 3: Extract the helper and use it in `spawn`**

In `src/desktop.rs`, replace the current `if let Some(ref sidecar) = config.sidecar { ... } else if let Some(ref cmd) = config.command { ... } else if let Some(ref runtime) = config.runtime { ... } else { ... }` block inside `Js::spawn` with a single call to a new helper. Add the helper at the bottom of the file (above the new `#[cfg(test)] mod tests`):

```rust
fn build_spawn_program(config: &SpawnConfig) -> crate::Result<(String, Vec<String>)> {
    if let Some(ref cmd) = config.command {
        let mut args = config.args.clone().unwrap_or_default();
        return Ok((cmd.clone(), args.drain(..).collect()));
    }

    let runtime = config.runtime.as_deref().ok_or_else(|| {
        crate::Error::InvalidConfig(
            "either 'sidecar', 'command', or 'runtime' must be specified".to_string(),
        )
    })?;

    let mut args = match runtime {
        "bun" => Vec::new(),
        "deno" => vec!["run".to_string(), "-A".to_string()],
        "node" => Vec::new(),
        other => {
            return Err(crate::Error::InvalidConfig(format!(
                "unknown runtime: {}",
                other
            )));
        }
    };

    if let Some(ref script) = config.script {
        args.push(script.clone());
    }

    if let Some(ref extra) = config.args {
        args.extend(extra.iter().cloned());
    }

    Ok((runtime.to_string(), args))
}
```

Update the `spawn` method to call it for `command` and `runtime` modes. Keep sidecar resolution inline because the test for it lives in Task 2:

```rust
let (program, mut args_vec) = if let Some(ref sidecar) = config.sidecar {
    let path = self.resolve_sidecar(sidecar)?;
    (path.to_string_lossy().to_string(), Vec::new())
} else {
    build_spawn_program(&config)?
};
```

- [ ] **Step 4: Run the new tests and confirm they pass**

Run from the plugin root:

```bash
cargo test --locked
```

Expected: all `build_*` tests pass.

- [ ] **Step 5: Commit (do not push)**

```bash
git add src/desktop.rs
git commit -m "test(rust): cover runtime and command program construction"
```

Note: The user has not requested commits in this session. If the user does not want a commit, leave the change staged or unstaged and report it. Do not push.

---

## Task 2: Extract testable sidecar candidate paths

**Files:**
- Modify: `src/desktop.rs`

- [ ] **Step 1: Add the failing test**

Append inside the existing `mod tests` block:

```rust
    #[test]
    fn sidecar_candidates_include_plain_and_triple_names() {
        let dir = std::path::PathBuf::from("/opt/app");
        let candidates = sidecar_candidate_paths(&dir, "my-worker", "aarch64-apple-darwin");
        let rendered: Vec<String> = candidates
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();
        assert!(rendered.contains(&"/opt/app/my-worker".to_string()));
        assert!(rendered.contains(&"/opt/app/my-worker-aarch64-apple-darwin".to_string()));
    }

    #[test]
    fn sidecar_candidates_canonical_order_is_plain_first() {
        let dir = std::path::PathBuf::from("/opt/app");
        let candidates = sidecar_candidate_paths(&dir, "w", "x86_64-unknown-linux-gnu");
        assert_eq!(candidates[0].to_string_lossy().to_string(), "/opt/app/w");
    }
```

- [ ] **Step 2: Run tests to confirm they fail to compile**

```bash
cargo test --locked --no-run
```

Expected: compile error referencing `sidecar_candidate_paths` as undefined.

- [ ] **Step 3: Add the helper and refactor `resolve_sidecar`**

Add a helper that returns the candidate path order, then make `resolve_sidecar` walk it. Place this above `#[cfg(test)] mod tests`:

```rust
fn sidecar_candidate_paths(
    exe_dir: &std::path::Path,
    name: &str,
    target_triple: &str,
) -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::with_capacity(4);
    candidates.push(exe_dir.join(name));
    candidates.push(exe_dir.join(format!("{name}-{target_triple}")));
    candidates
}

impl<R: Runtime> Js<R> {
    // existing impl block continues; rewrite resolve_sidecar to use the helper:
    fn resolve_sidecar(&self, name: &str) -> crate::Result<std::path::PathBuf> {
        let current_exe = std::env::current_exe().map_err(crate::Error::Io)?;
        let exe_dir = current_exe.parent().ok_or_else(|| {
            crate::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "could not determine executable directory",
            ))
        })?;

        let triple = env!("TARGET_TRIPLE");
        for candidate in sidecar_candidate_paths(exe_dir, name, triple) {
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        Err(crate::Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("sidecar not found: {name} (looked in {})", exe_dir.display()),
        )))
    }
}
```

Notes:
- The previous implementation included `cfg(windows)` `.exe` branches. The new helper intentionally omits them so the test stays platform-portable. Re-introduce `.exe` candidates **inside the helper** as additional entries guarded by `#[cfg(windows)]` so production behavior on Windows is preserved:

```rust
fn sidecar_candidate_paths(
    exe_dir: &std::path::Path,
    name: &str,
    target_triple: &str,
) -> Vec<std::path::PathBuf> {
    let mut candidates = Vec::with_capacity(4);
    candidates.push(exe_dir.join(name));
    candidates.push(exe_dir.join(format!("{name}-{target_triple}")));
    #[cfg(windows)]
    {
        candidates.push(exe_dir.join(format!("{name}.exe")));
        candidates.push(exe_dir.join(format!("{name}-{target_triple}.exe")));
    }
    candidates
}
```

- [ ] **Step 4: Run the full test suite**

```bash
cargo test --locked
```

Expected: all tests pass on macOS and Linux. On Windows, the helper now also yields `.exe` candidates.

- [ ] **Step 5: Stage the change (commit only if user requested)**

```bash
git add src/desktop.rs
```

Do not commit unless the user has asked. Report the change set and ask before committing.

---

## Task 3: Add a test for the runtime path override branch

**Files:**
- Modify: `src/desktop.rs`

- [ ] **Step 1: Add the failing test**

Append inside `mod tests`:

```rust
    #[test]
    fn runtime_override_replaces_program() {
        let program = "bun".to_string();
        let overrides = std::collections::HashMap::from([(
            "bun".to_string(),
            "/opt/homebrew/bin/bun".to_string(),
        )]);
        let resolved = resolve_program_with_override(Some("bun"), program, &overrides);
        assert_eq!(resolved, "/opt/homebrew/bin/bun");
    }

    #[test]
    fn runtime_override_passthrough_when_missing() {
        let program = "deno".to_string();
        let overrides = std::collections::HashMap::new();
        let resolved = resolve_program_with_override(Some("deno"), program.clone(), &overrides);
        assert_eq!(resolved, program);
    }
```

- [ ] **Step 2: Run tests to confirm they fail to compile**

```bash
cargo test --locked --no-run
```

Expected: compile error referencing `resolve_program_with_override`.

- [ ] **Step 3: Add the helper and use it in `spawn`**

Add above `#[cfg(test)] mod tests`:

```rust
fn resolve_program_with_override(
    runtime: Option<&str>,
    program: String,
    overrides: &std::collections::HashMap<String, String>,
) -> String {
    match runtime {
        Some(name) => overrides
            .get(name)
            .cloned()
            .unwrap_or(program),
        None => program,
    }
}
```

In `Js::spawn`, replace the existing block that locks `runtime_paths` and re-reads `config.runtime` with:

```rust
let program = {
    let custom_paths = self.runtime_paths.lock().await;
    resolve_program_with_override(config.runtime.as_deref(), program, &custom_paths)
};
```

- [ ] **Step 4: Run the full test suite**

```bash
cargo test --locked
```

Expected: all tests pass.

- [ ] **Step 5: Stage the change (commit only if user requested)**

```bash
git add src/desktop.rs
```

Do not commit unless the user has asked. Report the change set and ask before committing.

---

## Task 4: Add CI workflow

**Files:**
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Create the workflow file**

Write `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    name: Build & Test
    runs-on: ubuntu-latest
    timeout-minutes: 20
    steps:
      - uses: actions/checkout@v4

      - name: Install pnpm
        uses: pnpm/action-setup@v4
        with:
          version: 9

      - name: Install Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: pnpm

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable

      - name: Cache cargo registry and target
        uses: Swatinem/rust-cache@v2
        with:
          workspaces: .

      - name: Install JS dependencies
        run: pnpm install --frozen-lockfile

      - name: Build guest-js package
        run: pnpm build

      - name: Cargo check
        run: cargo check --locked --all-targets

      - name: Cargo test
        run: cargo test --locked --all-targets
```

- [ ] **Step 2: Validate the workflow syntax locally (optional)**

```bash
which actionlint || echo "actionlint not installed; rely on PR review"
```

If `actionlint` is installed, run:

```bash
actionlint .github/workflows/ci.yml
```

Expected: no errors. If `actionlint` is not installed, skip this step.

- [ ] **Step 3: Stage the change (commit only if user requested)**

```bash
git add .github/workflows/ci.yml
```

Do not commit unless the user has asked. Report the change set and ask before committing.

---

## Task 5: Add CHANGELOG.md

**Files:**
- Create: `CHANGELOG.md`

- [ ] **Step 1: Create the changelog**

Write `CHANGELOG.md`:

```markdown
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
```

- [ ] **Step 2: Stage the change (commit only if user requested)**

```bash
git add CHANGELOG.md
```

Do not commit unless the user has asked. Report the change set and ask before committing.

---

## Task 6: Add CONTRIBUTING.md

**Files:**
- Create: `CONTRIBUTING.md`

- [ ] **Step 1: Create the contribution guide**

Write `CONTRIBUTING.md`:

```markdown
# Contributing

Thanks for helping improve `tauri-plugin-js`. This guide covers the minimum
setup needed to build, test, and ship changes.

## Local Setup

Requirements:

- Rust stable (1.77.2 or newer; see `rust-version` in `Cargo.toml`).
- Node.js 20 or newer.
- pnpm 9 or newer.
- (Optional) Bun, Node.js, or Deno for the example app's runtime detection.

Clone the repository and install JavaScript dependencies:

```bash
pnpm install --frozen-lockfile
```

## Verification Commands

Run these from the plugin root before opening a pull request:

```bash
pnpm build
cargo check --locked --all-targets
cargo test --locked --all-targets
```

The same commands run in CI. If they pass locally, CI should pass.

## Manual Smoke Test

To exercise the example app end to end:

```bash
cd examples/tauri-app
pnpm install
pnpm tauri dev
```

The example app spawns Bun/Node/Deno workers and demonstrates typed RPC,
runtime detection, and sidecar binaries.

## Vendored Code

`vendors/kkrpc/` is a vendored copy of the kkrpc library. Do not edit it
unless you are intentionally updating the vendor copy. Changes to this
directory will be rejected by review.

## Release Checklist

This project does not currently publish automatically. To cut a release:

1. Update `CHANGELOG.md` with a new version section.
2. Bump versions in `Cargo.toml` and `package.json` so they match.
3. Run `pnpm build`, `cargo check --locked --all-targets`, and `cargo test --locked --all-targets`.
4. Tag the release: `git tag vX.Y.Z`.
5. Push the tag: `git push origin vX.Y.Z`.
6. Publish the Rust crate: `cargo publish`.
7. Publish the npm package from the `dist-js` build: `npm publish` (or `pnpm publish`).
```

- [ ] **Step 2: Stage the change (commit only if user requested)**

```bash
git add CONTRIBUTING.md
```

Do not commit unless the user has asked. Report the change set and ask before committing.

---

## Task 7: Update README with development notes

**Files:**
- Modify: `README.md` (after the existing "Example App" section, before any contributor section)

- [ ] **Step 1: Add a development section to the README**

Open `README.md` and append a new section at the end of the file:

```markdown
## Development

### Build & Test

```bash
pnpm install --frozen-lockfile
pnpm build
cargo check --locked --all-targets
cargo test --locked --all-targets
```

The GitHub Actions workflow at `.github/workflows/ci.yml` runs the same commands on every push and pull request.

### Manual Smoke Test

```bash
cd examples/tauri-app
pnpm install
pnpm tauri dev
```

The example app spawns Bun/Node/Deno workers, demonstrates typed RPC via kkrpc, and exercises runtime detection, sidecar binaries, multi-window support, and clean shutdown.

### Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for the full setup, verification, and release checklist. The Rust crate and npm package are not published automatically.
```

- [ ] **Step 2: Run a final verification**

```bash
pnpm build
cargo check --locked --all-targets
cargo test --locked --all-targets
```

Expected: all checks pass. If any step fails, fix it before reporting the plan complete.

- [ ] **Step 3: Stage the change (commit only if user requested)**

```bash
git add README.md
```

Do not commit unless the user has asked. Report the change set and ask before committing.

---

## Self-Review Checklist

- [ ] Spec coverage: CI workflow → Task 4; Rust unit tests for runtime command construction → Task 1; sidecar resolution → Task 2; runtime path override → Task 3; CHANGELOG → Task 5; CONTRIBUTING → Task 6; README development/CI notes → Task 7.
- [ ] Placeholder scan: No `TBD`/`TODO`/vague validation steps. Every step shows exact file content or commands.
- [ ] Type consistency: `build_spawn_program`, `sidecar_candidate_paths`, and `resolve_program_with_override` are defined exactly once and used consistently.
- [ ] No commit/push: Plan only stages changes and asks before committing, in line with the user's no-commit instruction.
- [ ] Vendored code: `vendors/kkrpc/` not modified.
