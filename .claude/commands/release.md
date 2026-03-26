# Release bflow

Release a new version of the bflow CLI tool.

## Steps

1. **Update version in Cargo.toml** — bump the `version` field to the new version (e.g., `"0.2.0"`)
2. **Run tests** — `cargo test --all` must pass (use `~/.cargo/bin/cargo` if `cargo` is not in PATH)
3. **Commit** both `Cargo.toml` and `Cargo.lock`: `git add Cargo.toml Cargo.lock && git commit -m "chore: bump version to X.Y.Z"`
4. **Tag** with `v` prefix: `git tag vX.Y.Z`
5. **Push** commit and tag: `git push && git push origin vX.Y.Z`

## What happens automatically

The CI pipeline (`.github/workflows/ci.yml`) triggers on `v*` tags and:

1. Runs tests on Linux, macOS, Windows
2. Builds release binaries:
   - `bflow-macos-aarch64` (Apple Silicon)
   - `bflow-macos-x86_64` (Intel Mac)
   - `bflow-windows-x86_64.exe`
3. Creates a GitHub Release at `Beans-BV/beans-gitflow` with auto-generated release notes and binaries attached
4. Updates the Homebrew formula at `Beans-BV/homebrew-tap` (requires `HOMEBREW_TAP_TOKEN` secret)
5. Publishes to Chocolatey (requires `CHOCOLATEY_API_KEY` secret)

## Installation after release

- **macOS:** `brew install Beans-BV/tap/bflow` (or `brew upgrade bflow`)
- **Windows:** `choco install bflow` (or `choco upgrade bflow`)
- **Manual:** Download binary from the GitHub Release page

## Arguments

If the user provides a version number, use it. Otherwise, read the current version from Cargo.toml and suggest the next appropriate version (patch bump for fixes, minor bump for features).
