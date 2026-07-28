# bflow finish --base <branch> (headless finish)

Goal: `bflow finish --base <branch>` skips the interactive PR-target menu; auto-pick
when detection finds exactly one candidate; error when `--base` is used on branch
types with a fixed target.

## Plan
- [x] RED: cli_test.rs — `--base` parses into `Action::FinishWorkBranch { base }`; `--base` on release branch errors
- [x] GREEN: plumb `base: Option<String>` through `Commands::Finish` → `resolve_action` → `Action::FinishWorkBranch` → `main.rs` → `finish_work_branch`; error on non-work finish
- [x] RED: finish_work_test.rs — explicit base skips detection; unknown base errors; local-only base accepted; single candidate auto-selected (no menu)
- [x] GREEN: validate `--base` via `remote_branch_exists`/`local_branch_exists`; auto-pick single candidate in `detect_parent_branch`
- [x] `cargo test --all` green (199 tests), `cargo clippy --lib --bins` no new warnings
- [x] Docs: README.md (Finish section), .claude/skills/bflow/SKILL.md, CHANGELOG.md
- [x] Commit, dogfood: `finish --base develop --breaking=false`

## Review
Small, KISS change mirroring the existing `--base` on `start`:
- `src/cli.rs`: new optional `--base` on `finish` (conflicts with `--abort` at clap level);
  guard arm rejects it on fixed-target finishes (release/hotfix/release-fix/hotfix-fix).
- `src/menu.rs`: `Action::FinishWorkBranch` carries `base: Option<String>`; interactive
  menu passes `None`.
- `src/flows/finish_work.rs`: explicit base validated via `remote_branch_exists` /
  `local_branch_exists` (detection skipped entirely); `detect_parent_branch` returns a
  single candidate directly instead of a one-item menu. 0-candidate `develop` default
  and multi-candidate menu unchanged.
- Tests: 9 new (5 cli, 4 flow), all watched failing first (TDD). Single-candidate test
  proves the menu is unreached — `show_select` would error without a TTY.
- Verified: 199 tests green, clippy shows only the 2 pre-existing warnings, real-binary
  help/conflict checks pass, branch finished headlessly with the new flag itself.
