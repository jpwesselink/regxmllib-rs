# npm packaging for regxmllib-rs

This folder contains npm packages for distributing prebuilt `regxmllib-rs` CLI binaries.

## Package layout

- `regxmllib-rs`: main launcher package (`npx regxmllib-rs ...`)
- `regxmllib-rs-<os>-<arch>`: platform binary packages (optionalDependencies)

## Supported targets

- `darwin-x64`
- `darwin-arm64`
- `linux-x64`
- `linux-arm64`
- `win32-x64`
- `win32-arm64`

## Stage binaries into platform packages

From the `npm/` directory:

```bash
node ./scripts/stage-binary.ts --target=darwin-arm64 --source=../target/aarch64-apple-darwin/release/regxmllib-rs
node ./scripts/stage-binary.ts --target=darwin-x64   --source=../target/x86_64-apple-darwin/release/regxmllib-rs
node ./scripts/stage-binary.ts --target=linux-arm64  --source=../target/aarch64-unknown-linux-gnu/release/regxmllib-rs
node ./scripts/stage-binary.ts --target=linux-x64    --source=../target/x86_64-unknown-linux-gnu/release/regxmllib-rs
node ./scripts/stage-binary.ts --target=win32-arm64  --source=../target/aarch64-pc-windows-msvc/release/regxmllib-rs.exe
node ./scripts/stage-binary.ts --target=win32-x64    --source=../target/x86_64-pc-windows-msvc/release/regxmllib-rs.exe
```

Scripts are `.ts` — Node ≥ 22.12 runs them directly without a transpilation step.

## Release (current platform only)

```bash
# dry run — build + pack, nothing published
node ./scripts/local-publish.ts

# publish current platform package + launcher
node ./scripts/local-publish.ts --publish
```

## Release (all platforms via GitHub Actions)

1. Bump versions: `pnpm -r version <semver>`
2. Commit and push.
3. Trigger `.github/workflows/npm-publish.yml`:
   - `dry_run=true` — build, stage, and `npm pack` all packages without publishing
   - `dry_run=false` — publish platform packages, then `regxmllib-rs`

Required secret: `NPM_TOKEN` (npm automation token with publish rights)

## Local verification

```bash
node ./scripts/check-packages.ts
npm pack ./regxmllib-rs-darwin-arm64
npm pack ./regxmllib-rs
```
