---
name: release
description: Use when the user wants to release a new version of bflow - supports natural language like "do a release", "major release", "minor release", "patch release", or explicit version numbers
user_invokable: true
---

# Release bflow

Release a new version of the bflow CLI tool using bflow's own gitflow process.

## Parsing the user's intent

| User says | Mode |
|-----------|------|
| "do a release", "release", "release it" | **auto-detect** |
| "do a major release", "release major", "major bump" | **force major** |
| "do a minor release", "release minor", "minor bump" | **force minor** |
| "do a patch release", "release patch", "patch bump" | **patch (hotfix flow)** |
| "release 1.2.3", any explicit semver | **explicit version** |

If the intent is ambiguous, ask the user to clarify.

**Important:** Patch releases use the **hotfix** flow (not the release flow). See the "Patch release (hotfix flow)" section below.

## Pre-flight checks

1. For major/minor: confirm you are on `develop`. If not, ask user if you should switch.
2. For patch: confirm you are on `main`. If not, ask user if you should switch.
3. Ensure working tree is clean (bflow auto-stashes for start, but version file updates need a clean state).

## Major/minor release flow

### Phase 1: Start the release branch

| Mode | Command |
|------|---------|
| auto-detect | `bflow start release` (bflow detects from commits, prompts major/minor) |
| force major | `bflow start release --major` |
| force minor | `bflow start release --minor` |
| explicit (e.g. 3.0.0) | Determine if major or minor bump, run matching flag. Verify branch version matches. |

After starting, read the version from the branch name:
```bash
git branch --show-current  # e.g. release/2.1.0 → version is 2.1.0
```

If a release branch already exists, bflow checks it out. Verify whether version files are already updated before proceeding.

### Phase 2: Update version files

1. **Update `Cargo.toml`** — set the `version` field to `X.Y.Z`
2. **Update `packaging/chocolatey/bflow.nuspec`** — set the `<version>` field to `X.Y.Z`
3. **Update `CHANGELOG.md`** — follows [Keep a Changelog](https://keepachangelog.com/) format:
   - Find the previous clean release tag: `git tag --list 'v*' --sort=-v:refname | grep -v '\-' | head -1`
   - Add a new `## [X.Y.Z] - YYYY-MM-DD` section at the top (below the header)
   - Categorize commits since last clean tag into: `### Added`, `### Changed`, `### Fixed`, `### Removed` (only include sections that have entries)
   - Write human-readable descriptions (not raw commit messages)
   - Add a comparison link at the bottom: `[X.Y.Z]: https://github.com/Beans-BV/beans-gitflow/compare/vPREVIOUS...vX.Y.Z`
4. **Run tests** — `cargo test --all` must pass (use `~/.cargo/bin/cargo` if `cargo` is not in PATH)
5. **Commit** all version files:
   ```bash
   git add Cargo.toml Cargo.lock packaging/chocolatey/bflow.nuspec CHANGELOG.md
   git commit -m "chore: bump version to X.Y.Z"
   ```

### Phase 3: RC tag and finish

6. **Bump RC** — run `bflow bump` to tag HEAD as the next RC (e.g., `vX.Y.0-rc.2`)
   - This satisfies the RC-head guard required by `bflow finish`
   - The RC tag triggers CI tests
7. **Confirm with user:**
   > RC tag `vX.Y.0-rc.N` has been pushed. CI will run tests.
   > Confirm when you are ready to finish the release (merge to main + develop and create the production tag).
8. **Finish release** — run `bflow finish`
   - Merges release branch into `main` and `develop`
   - Creates final clean tag `vX.Y.0`
   - Deletes the release branch locally and remotely
   - Pushes everything

## Patch release (hotfix flow)

Patch releases use bflow's hotfix flow:

1. Switch to `main` if not already there
2. Run `bflow start hotfix-fix --name version-bump`
   - If no hotfix branch exists, bflow auto-creates `hotfix/X.Y.Z` (bumps patch from latest tag)
   - Creates `hotfix-fix/X.Y.Z/version-bump` branch
3. Read the version from the hotfix branch name (visible in `hotfix-fix/X.Y.Z/version-bump`)
4. Update version files (same as Phase 2 above)
5. Run tests
6. Commit and run `bflow finish --breaking=false` → creates PR to hotfix branch
7. After PR is merged, switch to the hotfix branch: `git switch hotfix/X.Y.Z`
8. Run `bflow finish` → merges to main + develop, creates `vX.Y.Z` tag, deletes branch

**Note:** The hotfix flow requires a PR merge step. Guide the user through it rather than attempting full automation.

## What happens automatically

The CI pipeline (`.github/workflows/ci.yml`) responds to tags:

### RC tags (e.g., `v2.1.0-rc.2`) — staging validation
- Runs tests on Linux, macOS, Windows
- Does NOT create a GitHub Release or publish packages

### Clean tags (e.g., `v2.1.0`) — production release
- Runs tests on Linux, macOS, Windows
- Builds release binaries: `bflow-macos-aarch64`, `bflow-macos-x86_64`, `bflow-windows-x86_64.exe`
- Creates a GitHub Release at `Beans-BV/beans-gitflow` with auto-generated release notes and binaries attached
- Updates the Homebrew formula at `Beans-BV/homebrew-tap` (requires `HOMEBREW_TAP_TOKEN` secret)
- Publishes to Chocolatey (requires `CHOCOLATEY_API_KEY` secret)

## Summary of bflow commands used

| Step | Command | What it does |
|------|---------|--------------|
| Start release | `bflow start release [--major\|--minor]` | Creates `release/X.Y.0` from develop, tags `vX.Y.0-rc.1` |
| Bump RC | `bflow bump` | Creates next RC tag at HEAD |
| Finish release | `bflow finish` | Merges to main + develop, creates clean `vX.Y.0` tag |
| Start hotfix | `bflow start hotfix-fix --name <name>` | Creates hotfix branch + fix branch |
| Finish hotfix-fix | `bflow finish --breaking=false` | Creates PR to hotfix branch |
| Finish hotfix | `bflow finish` | Merges to main + develop, creates `vX.Y.Z` tag |
