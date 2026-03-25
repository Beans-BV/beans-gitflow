# bflow — Beans GitFlow CLI

**Date:** 2026-03-25
**Status:** Approved

## Overview

`bflow` is a cross-platform (macOS + Windows) CLI tool that implements the Beans customized gitflow workflow. It detects the current git branch, presents context-appropriate menu options, and executes the selected flow.

**Approach:** Rust binary, shells out to `git` and `gh` CLI tools. This avoids reinventing auth/SSH/credential handling and keeps the implementation simple.

## Dependencies

- **Runtime:** `git`, `gh` (GitHub CLI) must be installed
- **Rust crates:** `clap` (CLI parsing), `dialoguer` (interactive menus)

## Architecture

```
bflow
├── main.rs              — entry point, detect current branch, route to menu
├── git/
│   ├── mod.rs           — Git trait + impl shelling out to `git`
│   └── branch.rs        — branch detection, parsing, classification
├── hosting/
│   ├── mod.rs           — HostingPlatform trait (create_pr, etc.)
│   └── github.rs        — impl via `gh` CLI
├── flows/
│   ├── start.rs         — start feature/fix/release-fix/hotfix-fix
│   ├── finish_work.rs   — finish work branches (PR creation)
│   ├── finish_release.rs — finish release (merge, tag, cleanup)
│   └── finish_hotfix.rs — finish hotfix (merge, tag, cleanup)
├── version.rs           — parse tags, bump minor/patch
└── menu.rs              — interactive menu via dialoguer
```

### Key Abstractions

- **`Git` trait** — all git operations behind an interface (testable, swappable)
- **`HostingPlatform` trait** — `create_pr(head, base, title)` — GitHub impl today, GitLab/Bitbucket can be added later
- **`BranchType` enum** — classifies the current branch

## Branch Classification

| Pattern | BranchType | Extracted data |
|---------|-----------|----------------|
| `main` or `master` | `Main` | — |
| `develop` | `Develop` | — |
| `feature/{name}` | `Feature` | name |
| `fix/{name}` | `Fix` | name |
| `release/{major}.{minor}` | `Release` | version |
| `release-fix/{major}.{minor}/{name}` | `ReleaseFix` | version, name |
| `hotfix/{major}.{minor}.{patch}` | `Hotfix` | version |
| `hotfix-fix/{major}.{minor}.{patch}/{name}` | `HotfixFix` | version, name |
| anything else | `Other` | — |

## Version Resolution

Used when starting release-fix or hotfix-fix and no existing integration branch exists:

1. List all tags matching semver pattern (e.g. `1.2.3`, `v1.2.3`)
2. Sort semantically, take the latest
3. For release: bump minor, zero patch → `release/{major}.{minor}`
4. For hotfix: bump patch → `hotfix/{major}.{minor}.{patch}`
5. Before creating, check if a `release/*` or `hotfix/*` branch already exists (local + remote). If yes, use that version.

## Menus

### On `main`/`master`
- start hotfix fix

### On `develop`
- start feature
- start fix
- start release fix

### On `feature/{name}`
- finish feature

### On `fix/{name}`
- finish fix

### On `release-fix/{v}/{name}`
- finish release fix

### On `release/{v}`
- bump version
- sync with develop
- finish release

### On `hotfix-fix/{v}/{name}`
- finish hotfix fix

### On `hotfix/{v}`
- finish hotfix

### On unrecognized branch
- Display error: "Not on a recognized gitflow branch. Switch to main or develop first."

## Flow Details

### Start Flows

| Flow | Steps |
|------|-------|
| **start feature** | prompt for name → create `feature/{name}` from `develop` → push → checkout |
| **start fix** | prompt for name → create `fix/{name}` from `develop` → push → checkout |
| **start release-fix** | check for existing `release/*` branch, create if needed (bump minor from last tag) → prompt for name → create `release-fix/{v}/{name}` from `release/{v}` → push → checkout |
| **start hotfix-fix** | check for existing `hotfix/*` branch, create if needed (bump patch from last tag) → prompt for name → create `hotfix-fix/{v}/{name}` from `hotfix/{v}` → push → checkout |

### Finish Work Branches (PR-based)

| Flow | Steps | PR Target |
|------|-------|-----------|
| **finish feature** | push branch → create PR via `gh` → open PR URL in browser | `develop` |
| **finish fix** | push branch → create PR via `gh` → open PR URL in browser | `develop` |
| **finish release-fix** | push branch → create PR via `gh` → open PR URL in browser | `release/{v}` |
| **finish hotfix-fix** | push branch → create PR via `gh` → open PR URL in browser | `hotfix/{v}` |

### Release/Hotfix Operations

| Flow | Steps |
|------|-------|
| **bump version** | prompt for tag name (default: `{version}.0`) → create git tag → push tag |
| **sync with develop** | merge `release/{v}` into `develop` → push `develop` |
| **finish release** | merge into `main` → create tag → merge into `develop` → push all → delete `release/{v}` branch (local + remote) |
| **finish hotfix** | merge into `main` → create tag → merge into `develop` → push all → delete `hotfix/{v}` branch (local + remote) |

## Cross-Platform

- Rust compiles natively for macOS (x86_64 + aarch64) and Windows (x86_64)
- CI via GitHub Actions with cross-compilation
- Single codebase, no platform-specific code (all operations go through `git`/`gh` CLI)

## Error Handling

- Verify `git` and `gh` are installed on startup
- Verify working directory is a git repository
- Verify clean working tree before branch operations
- All git/gh command failures surface the stderr output to the user
