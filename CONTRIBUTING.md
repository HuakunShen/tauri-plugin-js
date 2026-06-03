# Contributing

## Local Setup

Use Rust stable 1.77.2 or newer, Node.js 20 or newer, and pnpm 9 or newer. Bun and Deno are optional for broader example runtime detection coverage; Node runtime coverage uses the required Node.js install.

Install dependencies from the plugin root:

```sh
pnpm install --frozen-lockfile
```

## Verification Commands

Run these commands from the plugin root before sending changes for review:

```sh
pnpm build
cargo check --locked --all-targets
cargo test --locked --all-targets
```

## Manual Smoke Test

Use the Tauri example app for a manual smoke test:

```sh
pnpm build
cd examples/tauri-app
pnpm install --frozen-lockfile
pnpm tauri dev
```

Run the root build first so the example app can link package exports from `dist-js`. The example app covers typed RPC and runtime detection by default. Sidecar behavior can be tested after running `pnpm build:sidecars` from `examples/tauri-app` if Bun and Deno are installed.

## Vendored Code

Do not edit `vendors/kkrpc` unless you are intentionally updating the vendored copy. Keep project changes outside the vendor tree whenever possible.

## Release Checklist

1. Update `CHANGELOG.md`.
2. Bump versions in `Cargo.toml` and `package.json`.
3. Run `pnpm build`, `cargo check --locked --all-targets`, and `cargo test --locked --all-targets`.
4. Tag release with `git tag vX.Y.Z`.
5. Push tag with `git push origin vX.Y.Z`.
6. Maintainers only: validate the Rust package with `cargo publish --dry-run`, then run `cargo publish`.
7. Maintainers only: inspect the npm package with `pnpm pack --dry-run`, then run `npm publish` or `pnpm publish`.
