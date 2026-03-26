---
name: release
description: Use when the user wants to release a new version of bflow - supports natural language like "do a release", "major release", "minor release", "patch release", or explicit version numbers
user_invokable: true
---

# Release bflow

Release a new version of the bflow CLI tool.

## Parsing the user's intent

The user can invoke this in natural language. Parse their intent into one of these modes:

| User says | Mode |
|-----------|------|
| "do a release", "release", "release it" | **auto-detect** |
| "do a major release", "release major", "major bump" | **force major** |
| "do a minor release", "release minor", "minor bump" | **force minor** |
| "do a patch release", "release patch", "patch bump" | **force patch** |
| "release 1.2.3", any explicit semver | **explicit version** |

If the intent is ambiguous, ask the user to clarify.

## Version detection (auto-detect mode)

When no version or bump level is specified:

1. Read current version from `Cargo.toml`
2. Find the last release tag: `git tag --list 'v*' --sort=-v:refname | head -1`
3. Get commits since that tag: `git log <tag>..HEAD --format="%s%n%b"`
4. Determine bump level from conventional commits:
   - Any commit with `!` after type (e.g. `feat!:`, `fix!:`, `refactor!:`) OR `BREAKING CHANGE` in body → **major**
   - Any `feat:` commit → **minor**
   - Otherwise (`fix:`, `refactor:`, `test:`, `docs:`, `chore:`, etc.) → **patch**
5. Compute the new version by applying the bump to the current version
6. Present the proposed version with reasoning:
   - List the commits since last tag
   - Explain why this bump level was chosen
   - Ask the user to confirm before proceeding

## Version detection (force major/minor/patch)

1. Read current version from `Cargo.toml`
2. Apply the requested bump level
3. Present the proposed version and ask the user to confirm

## Execution steps (after confirmation)

1. **Update version in `Cargo.toml`** — bump the `version` field
2. **Update version in `packaging/chocolatey/bflow.nuspec`** — bump the `<version>` field to match
3. **Run tests** — `cargo test --all` must pass (use `~/.cargo/bin/cargo` if `cargo` is not in PATH)
4. **Commit** all version files: `git add Cargo.toml Cargo.lock packaging/chocolatey/bflow.nuspec && git commit -m "chore: bump version to X.Y.Z"`
5. **Tag** with `v` prefix: `git tag vX.Y.Z`
6. **Push** commit and tag: `git push && git push origin vX.Y.Z`

## What happens automatically

The CI pipeline (`.github/workflows/ci.yml`) triggers on `v*` tags and:

1. Runs tests on Linux, macOS, Windows
2. Builds release binaries: `bflow-macos-aarch64`, `bflow-macos-x86_64`, `bflow-windows-x86_64.exe`
3. Creates a GitHub Release at `Beans-BV/beans-gitflow` with auto-generated release notes and binaries attached
4. Updates the Homebrew formula at `Beans-BV/homebrew-tap` (requires `HOMEBREW_TAP_TOKEN` secret)
5. Publishes to Chocolatey (requires `CHOCOLATEY_API_KEY` secret)
