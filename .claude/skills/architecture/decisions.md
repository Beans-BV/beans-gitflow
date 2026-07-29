# bflow Design Decisions

Catalog of deliberate choices — each entry: the choice, why, and the rejected alternative. Preserve these when changing code; if you must break one, say so explicitly in the PR. (Broad architecture principles live in SKILL.md; this file is the detail behind them.)

## Extension Recipes

- **New git operation:** add method to `Git` trait → impl in `GitCli` → add to `MockGit` in `tests/common/mod.rs`.
- **New hosting provider:** new file in `hosting/` implementing `HostingPlatform` → `Provider` variant + parse arm in `detect.rs` → preflight arm in `main.rs::create_hosting`. Reuse `hosting::run_cli`.
- **New command/flow:** clap variant in `cli.rs` → `Action` variant + menu entry gated by `BranchType` → flow fn in `flows/` taking `&dyn` deps → dispatch arm in `run_flow`.
- **New work-branch type** (e.g. `perf/`): variant on `BranchType` → entry in the `WORK_TYPES` table + arm in `work_kind` in `git/branch.rs` (the compiler forces both — `work_kind` has no wildcard) → clap `StartKind` variant + arms in `cli.rs::resolve_action` → `pr_template_keys` arm. Menus, parent-branch detection, and `commit_type` (kind name, unless special-cased like feature→feat) follow from the table automatically.

## Plug-in Points

| Port (trait) | Adapters | Adapter selected by |
|---|---|---|
| `Git` | `GitCli`; `MockGit` in tests | Built once in `main.rs` |
| `HostingPlatform` | `GitHub` (`gh`), `AzureDevOps` (`az`); `MockHosting` | Auto-detected from origin remote URL (`hosting/detect.rs`) |
| `Editor` | `CommandEditor` (any command); `MockEditor` | `bflow.worktree.editor` git config |

User-configurable without code changes: the worktree flow (`bflow.worktree.*` git config, local/global scope, `bflow worktree` wizard) and PR templates (`.github/pr-templates/bflow-<key>.md`, most-specific-first, falling back to the repo's native template).

## Dependency Budget

- **Exactly two direct dependencies (`clap`, `crossterm`), zero dev-dependencies** (`Cargo.toml`). Everything else is hand-rolled on purpose: the dependency graph stays reviewable in one screen. Rejected: `serde` (state file is a hand-rolled versioned `key=value` format), `anyhow`/`thiserror` (see Error Model), `mockall` (hand-written mocks), `tempfile` (hand-rolled `tmp_dir()` in `state.rs` tests), `dirs` (manual `HOME`/`USERPROFILE` tilde expansion in `worktree.rs`), `regex` (slice-pattern URL parsing in `detect.rs`), `open`/`webbrowser` (15-line cfg dispatch in `hosting/mod.rs`).
- **Shell out to `git`/`gh`/`az` CLIs instead of linking `git2`/`octocrab`/REST clients.** Inherits the user's existing auth (SSH agent, credential helpers, `gh auth`, `az` PAT), hooks, config, and signing; no HTTP/TLS stack; tiny binary. Rejected: libgit2 (own credential callbacks, cross-compile pain) and per-provider REST clients (token storage, API version churn).

## Error Model

- **`Result<T, String>` everywhere; no error enums.** Every error is terminal and user-facing — nothing ever matches on a variant; `main` prints `Error: {e}` once and exits non-zero. Accepted cost: rare string sniffing (`e.contains("not something we can merge")` in `main.rs`).
- **Every error message names the exact next command** — `gh auth login`, `bflow bump`, `git stash pop <ref>`, `git switch <branch>` + `bflow finish` (`resume_hint` in `flows/mod.rs`). Raw git errors are intercepted and rewritten when bflow knows better (`start.rs`: "not a commit" → "Branch does not exist. Use --base…").
- **Warn-and-continue only when the work already succeeded**: failed stash pop, failed editor open ("Worktree is ready at <path>"), corrupt *legacy* state file. Everything protecting in-flight work is a hard error — including an unknown *current* state-file version (with `--abort` as the named remedy). Asymmetry is the rule: report completed work as complete; never misparse or destroy.
- **Exit-code-aware git runners** (`run` / `run_check` for 0/1-as-bool / `run_config` for exit-1-means-unset; exit 5 = already-unset is success): "false" is never conflated with "failed".

## State & Crash-Safety

- **Per-branch state files** (`.git/bflow-finish/<kind>-<version>.state`), identity derived from the branch, not stored — a stalled hotfix finish can never hijack a release finish; lookup is a pure function of where you stand. Under `.git/` so it can't be committed, stashed, or `git clean`ed.
- **Ordering contract in `run()`**: reject (dirty tree, mid-merge) → stash → write state → first side effect. State is written before the first mutation of a release/hotfix finish and records the stash message, so a crash at any point leaves a resumable record that never references a nonexistent stash.
- **Missing state = `Ok(None)`; corrupt state = `Err`.** Absence is normal; malformation must not silently restart a half-done finish. Format has a `version=` field: unknown keys ignored (forward-compatible), unknown version is a hard error.
- **`--abort` short-circuits before every check including the mid-merge preflight** ("abort is itself a recovery action") and succeeds on a clean repo — safe to run speculatively. One-time legacy-state migration runs at startup, deletes the old file unconditionally so it can't loop.

## Stash Policy

- **Stash by unique message (`bflow-finish:{branch}:{ts}`), looked up by message before popping — never blind `stash pop`** (which could pop a stash the user pushed meanwhile). The state's `stash_ref` field stores the *message*, not `stash@{n}` (indices shift).
- **Three-way pop policy**: success → pop; failed release/hotfix finish → keep for resume (announced with recovery commands); any other failure → pop (restore the user's tree). Resume inherits the prior stash instead of stashing again; abort keeps the stash and tells you how to pop it (auto-pop could conflict with the aborted merge's tree).

## Resume & Idempotency

- **Resume is branch-scoped**: only release/hotfix branches carry finish identity; you continue by standing on the source branch. Resume state outranks branch-based dispatch (a develop-merge conflict leaves HEAD on develop, where eligibility checks would wrongly reject) — but an explicit `--base` bypasses the resume shortcut so it gets *rejected*, not silently ignored (regression-tested in `main.rs`).
- **Every finish step is guarded by a real-state predicate** (`is_ancestor`, `tag_exists`, `is_pushed` via SHA compare, `remote_tag_exists`, branch-exists checks) and prints a visible `↷ skipped:` line — a resume audits exactly what it did and didn't repeat. Rejected: relying on git's "already exists" errors, which would abort resumes at the first completed step.
- **Resume-path details that matter**: version comes from state, not the branch you're standing on; checkout to `main` before deleting the source branch (HEAD may still be on it); hotfix push sits *outside* the merge if/else so merged-but-unpushed crashes still push; hotfix propagation targets are filtered (`release-fix/` excluded), sorted, deduped for deterministic replay.

## Release Discipline

- **The RC gate is enforced, not documented**: `finish_release` refuses when HEAD is past the latest RC tag — every commit reaching `main` was staging-validated under an RC deploy. Error names the tag, commit count (singular/plural tested), and the fix (`bflow bump`).
- **Version truth lives in git tags** (`vX.Y.Z`, RCs `vX.Y.Z-rc.N`) — no VERSION file. Fallback chain: highest clean tag → highest RC stripped to release form → `0.0.0` (works on a fresh repo). Non-semver tags are filtered, never errors.
- **`SemVer::Ord` is hand-implemented so a pre-release sorts below its own release** (derived `Ord` on `Option` would rank `-rc.4` above the clean tag). Leading zeros in RC numbers rejected (`rc.01` ≠ `rc.1`). Branch/tag names are always generated from `SemVer` methods, never string-concatenated; bumps clear the pre-release.
- **Breaking-change detection reorders the release-type menu default, it never decides for you**; detection failure degrades to `false` rather than blocking the flow.

## CLI / UX Conventions

- **One `Action` enum is the single currency**: menu and subcommands both resolve into it; nothing downstream knows which interface ran. Menu selections are "the flags at their defaults", not a separate feature set.
- **Branch-type gating with specific messages**: each branch type gets only its legal actions; single-item menus still confirm rather than auto-execute. Interactive and scripted variants of the same error differ on purpose (menu adds "Switch to main or develop first"; CLI stays terse). `--base` rejection derives from `BranchType::has_fixed_finish_target()`, not a hardcoded list.
- **stdout = narration of what happened; stderr = prompts, warnings, resume notices, errors.** The interactive menu renders on stderr so piped stdout stays clean.
- **Input shaping over validation**: branch-name prompts transform as you type (space→`-`, collapse, trim) so invalid names are untypeable; `prompt_line` deliberately does *no* mangling for paths/commands. `validate_branch_name` is shared by CLI and menu — same rules, same message. Ctrl-C/Esc return `Err("Aborted")` through the normal path so terminal cleanup and stash restore still run; every menu early-return restores the terminal (no raw-mode leak).
- **Flag design**: `--breaking` is tri-state (absent=prompt for commonly-breaking types only, `--breaking`, `--breaking=false` for CI); incompatibilities are declarative clap `conflicts_with`; shared `StartOptions` is `#[command(flatten)]`ed; `--local` is `global = true` (position-independent) and *global scope is the default* — worktree preference belongs to the developer, not the repo. Parent-branch detection: 0 candidates→develop, 1→auto + announced, 2+→menu sorted by merge distance; child branches excluded.
- **Config store is git config** (`bflow.*` keys) — free local/global scoping, precedence, and `git config` as an alternate UI. No bflow config file. All reads are trimmed and empty-after-trim falls back to defaults; "use default" *unsets* the key. The worktree wizard is built on the same setter functions as the flags.
- **`bflow worktree` dispatches before all branch/auth machinery** — configuring worktrees must not require `gh`/`az` or a network fetch.

## Boundaries & Extensibility

- **`Git` is ~45 fine-grained primitives** so ordering logic lives in flows and mocks can record exact sequences. Rejected: coarse `finish_release()`-style methods (untestable ordering).
- **`HostingPlatform` stays minimal (3 methods)**: `create_or_get_pr` is deliberately one method ("PR already open" is a normal resume outcome and the check-then-create dance is provider-specific); `open_url` is a default method (provider-agnostic, but on the trait as a test seam); the `template` param is a *path* (only `az` needs the contents read; `gh` takes `--body-file`).
- **Detection: pure core, thin shell.** `resolve`/`parse_remote` are pure functions unit-tested without a repo (same pattern in `template.rs` and `devops.rs` helpers). Unknown host → GitHub (preserves pre-detection behavior for GitLab/Enterprise users), but an *explicit* `devops` override with an unparseable remote is a hard error — asymmetric on purpose. `Provider::AzureDevOps` carries `{org, project, repo}` so detection and construction are one step. ADO PR URLs are synthesized from parsed coordinates (`az`'s `webUrl` is unreliable); `validate_pr_id` rejects `az`'s literal `"None"`.
- **Library crate + thin binary exists solely so `tests/` can link the flows.** Consequence: visibility falls exactly on the test boundary — `pub mod` at the top, tight function-level privacy inside (`run_cli`, parsers, runners all private). Orchestration (ordering, stash, state lifecycle) stays in `main.rs`, not the library.
- **PR templates are repo files** (`.github/pr-templates/bflow-<key>.md`, specific → group → default → the repo's own native template → empty). The fix family shares one `bflow-fix.md` group key. Native-template path lists are per-provider knowledge, intentionally not shared.

## Testing Strategy

- **Call-recording mock: one `Vec<String>` of `format!`-encoded calls; assertions are exact sequences** (23-element scripts in `finish_release_test.rs`) — a reorder of merge-before-tag is a test failure. Strings, not a typed `Call` enum: expectations read as a runnable script and diff legibly.
- **One configurable `MockGit` with knob fields (state axes of git: what exists, what's merged, what's pushed), not a mock family.** Targeted failure injection (`fail_nth_merge` distinguishes "main merge conflicted" from "develop merge conflicted"); `RefCell` interior mutability keeps the trait `&self` so test bookkeeping never infects production signatures. Stateful where flows read back their own writes (stash save→find), dumb constants where nothing branches on the value.
- **Negative assertions prove guards fire before damage** (no `checkout:main` in the log after a gate error; `rev_list_count_result = 99` proves the RC gate didn't even run on resume). Idempotent-resume tests build a fully-completed world and assert *zero* mutating calls.
- **No real git anywhere in tests** (flows push/delete/PR — a sandbox would need a fake remote and stubs, and be slow on Windows CI). The one filesystem exception is `state.rs` round-trip tests. Tests split by *reachability*, not dogma: public behavior in `tests/` (one file per flow), private logic inline `#[cfg(test)]`. Scenario builders stay local per test file (DAMP over DRY); the tiny `mock_contract_test.rs` deliberately pins the mock's own recording contract.

## Delivery & CI

- **Tests on 3 OSes; releases for 3 targets excluding Linux** (no distribution channel — Homebrew + Chocolatey; Linux via `cargo install`). Two macOS arch builds, not a universal binary (Homebrew formula needs per-arch sha256).
- **RC tags run tests only; clean tags publish** — enforced by a repeated `if: startsWith(refs/tags/v) && !contains(ref_name, '-')` (Actions tag filters can't express exclusion). Strict `needs:` chain: test → build → create-release → {homebrew, chocolatey} — package managers never point at a release that doesn't exist. Per-job least-privilege permissions; no third-party actions in the publish path (Homebrew formula regenerated from a heredoc each release; push is idempotent for re-runs — same philosophy as `bflow finish`).
- **Chocolatey is a downloader package**: CI substitutes a `__CHECKSUM__` placeholder with the SHA256 of the *actually uploaded* artifact, so the committed script is non-functional and the checksum can't be stale. `$ErrorActionPreference='Stop'` on install, deliberately permissive on uninstall.
- **AI review is a gated workflow**: fires on open/reopen only (not every push), read-only tool allowlist (`gh pr comment` is the only write), review logic pulled from a marketplace plugin so standards update upstream.
- **Total dogfooding**: bflow releases itself via `/release` (release flow for majors/minors, hotfix flow for patches), with a human gate at the RC step. CHANGELOG is hand-curated Keep-a-Changelog narrative; GitHub release notes are auto-generated — two audiences, two artifacts. `Cargo.lock` committed (binary crate).

## Known Gaps (accepted, not endorsed)

- No `clippy`/`fmt --check` in CI; unpinned stable toolchain; no MSRV; no build cache.
- Version triple (`Cargo.toml` / nuspec / tag) synced only by the release-skill checklist (nuspec is overridden at pack time, so drift is bounded).
- `git_dir()` is unresolved/relative and per-worktree in linked worktrees; `state.save` is not write-temp-then-rename (a truncated file fails safe via version/field validation, but noisily).
- `stash_ref` field name is misleading (stores a message); one state test seeds a literal `stash@{0}` that isn't the real-world shape.
