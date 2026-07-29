# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.0] - 2026-07-29

### Added
- Azure DevOps support — the hosting provider is auto-detected from the origin remote URL (`dev.azure.com` / `*.visualstudio.com` → Azure DevOps via the `az` CLI, anything else → GitHub via `gh`), overridable with `git config bflow.hosting.provider github|devops`. PRs are created with `az repos pr create` and open at the canonical `dev.azure.com` URL; preflight verifies the `azure-devops` extension and repository access ([#13](https://github.com/Beans-BV/beans-gitflow/pull/13)).
- `bflow finish` completes an already-merged branch instead of opening a new PR. When a work branch's (or release-fix/hotfix-fix branch's) most recent PR is merged and the local tip is exactly the merged commit, finish deletes the remote branch (if the platform didn't already), deletes the local branch, and — when the branch lives in its own worktree — removes the worktree. New commits after the merge, or a PR closed without merging, still get a fresh PR; cleanup never runs on them ([#15](https://github.com/Beans-BV/beans-gitflow/pull/15)).
- Optional git worktree flow for `bflow start`. When `bflow.worktree.enabled` is set (git config), work-branch starts (feature/fix/chore/docs/refactor, plus release-fix/hotfix-fix) create the branch in a native git worktree and open it in your editor instead of switching the current checkout. Configurable via `bflow.worktree.editor` (default `code`; `none` to skip opening) and `bflow.worktree.path` (base directory, default the repo's parent). Worktree folders are named `<repo-name>-<branch-with-slashes-as-dashes>`. Off by default; `--no-worktree` skips it for a single command.
- `bflow worktree` command to configure the worktree flow without editing git config by hand: an interactive setup wizard (`bflow worktree`) plus non-interactive subcommands `enable`, `disable`, `editor <cmd>`, `path <dir>`, and `status`. Writes global git config by default; `--local` scopes to the current repository.
- `bflow finish --base <branch>` on work branches sets the PR target explicitly, skipping parent-branch detection and its selection menu — closes the last non-interactive gap for AI agents and CI. The branch must exist on origin (PR targets live on the remote) and differ from the branch being finished; branch types with a fixed target (release, hotfix, release-fix, hotfix-fix) reject the flag. Additionally, when detection finds exactly one candidate parent it is now used directly instead of showing a one-item menu.

## [2.4.0] - 2026-06-17

### Changed
- `bflow finish` resume state is now scoped per source branch. An interrupted release/hotfix finish is tracked in its own file under `.git/bflow-finish/` (e.g. `hotfix-2.5.2.state`) and only resumes when you are standing on that source branch. From `develop`, `main`, or any `feature/*` branch, bflow behaves normally — a stalled finish no longer hijacks every command. Two finishes can be in progress at once without colliding, and `bflow finish --abort` is likewise branch-scoped ([#10](https://github.com/Beans-BV/beans-gitflow/pull/10)).
- Every merge step in the release and hotfix finish flows now appends recovery guidance on conflict, naming the exact source branch to `git switch` back to before re-running `bflow finish`.

### Added
- Pre-2.4 global `.git/bflow-finish.state` files are migrated automatically into the per-branch folder on first run.

## [2.3.0] - 2026-06-17

### Fixed
- `release-fix` and `hotfix-fix` PR titles now convert dashes in the branch name to spaces (e.g. `hotfix-fix/2.1.0/null-crash` → `fix: null crash`), keeping squash-merged history readable ([#9](https://github.com/Beans-BV/beans-gitflow/pull/9)).

## [2.2.0] - 2026-06-01

### Added
- Branch-aware PR templates — `bflow finish` selects the PR body by branch type, resolving most-specific first: branch-specific (`.github/pr-templates/bflow-<type>.md`) → group → `bflow-default.md` → the repo's existing git default. The fix family (`fix`, `release-fix`, `hotfix-fix`) shares `bflow-fix.md`, with optional per-branch overrides like `bflow-release-fix.md`. Opt-in: with no `.github/pr-templates/` directory, behavior is unchanged ([#8](https://github.com/Beans-BV/beans-gitflow/pull/8)).

## [2.1.0] - 2026-05-26

### Added
- Idempotent `bflow finish` for release and hotfix flows — re-running after a merge conflict resumes from the first incomplete step. Steps already completed (merges, tags, pushes, branch deletion) are detected from real git state and skipped.
- `bflow finish --abort` discards an in-progress finish state.
- Hotfix propagation into open `release/*` branches — when a hotfix is finished while a release branch exists, the fix is merged into the release too so the upcoming release ships it; the operator runs `bflow bump` afterward to cut a new RC.
- `bflow start release --major` / `--minor` flags to skip the interactive prompt in non-interactive contexts (CI, AI agents, scripts).

### Fixed
- `bflow finish` on a release branch dispatches correctly after a develop-merge conflict — state file consultation now precedes the branch-eligibility check so resume works from any branch.
- Release/hotfix cleanup switches off the source branch before deletion when HEAD is still on it (resume paths that skipped the develop merge).
- Edge cases in the prior release finish flow ([#7](https://github.com/Beans-BV/beans-gitflow/pull/7)).

## [2.0.0] - 2026-04-14

### Changed
- **BREAKING:** Tags now use `v` prefix (e.g., `v1.2.0` instead of `1.2.0`)
- **BREAKING:** Release start creates RC tags (`v1.2.0-rc.1`) instead of clean tags
- **BREAKING:** `bflow bump` increments RC number (`v1.2.0-rc.2`) instead of patch version
- Release finish creates clean production tag (`v1.2.0`) from latest RC
- SemVer parsing supports pre-release identifiers (`-rc.N`)

### Added
- RC-head guard on `bflow finish`: rejects a release branch when HEAD has commits past the latest RC tag, enforcing that every commit merged to `main` was validated on staging via an RC deploy
- Smart major/minor release selection — `bflow start release` scans commits for breaking changes and preselects accordingly
- `bflow finish` now asks about breaking changes on feature/fix/refactor branches and adds `!` to the PR title when confirmed (e.g., `feat!: name`)
- `bflow finish --breaking[=true|false]` flag for non-interactive control over the breaking-change marker
- CI Integration guide in README with GitHub Actions examples for staging and production
- Mobile app (Flutter) CI guidance with version extraction and build number patterns
- Chocolatey package icon

## [1.2.1] - 2026-04-09

### Changed
- Added `iconUrl`, `packageSourceUrl`, `summary`, and `releaseNotes` fields to Chocolatey nuspec for faster package review approval

## [1.2.0] - 2026-04-09

### Added
- `pull` method on `Git` trait using `--ff-only` for silent fast-forward syncs

### Changed
- Remote sync ("pull latest") operations now fast-forward silently instead of creating merge commits
- Merge commit messages use consistent format: `chore: merge <branch> into <target>` for both main and develop
- Gracefully skip pull when remote branch does not exist (local-only branches)

## [1.1.1] - 2026-04-01

### Fixed
- Windows keyboard input handling for interactive prompts

## [1.1.0] - 2026-03-30

### Added
- `--no-checkout` flag for all `bflow start` subcommands (except `release`) — creates and pushes the branch without switching to it, designed for git worktree workflows
- Auto-stash/restore of uncommitted changes during `bflow start` — no more "working tree is not clean" errors
- `--no-checkout` on `release-fix` and `hotfix-fix` skips branch-type validation and discovers the target branch automatically

### Fixed
- Orphaned stash when early return errors occurred during start flows
- Improved error message when base branch does not exist

## [1.0.0] - 2026-03-26

### Changed
- Release process moved from CLI command to skill-based automation
- Version bump to 1.0.0 (stable release)

## [0.2.1] - 2026-03-26

### Fixed
- Patch version bump fix

## [0.2.0] - 2026-03-26

### Added
- Custom terminal UI with crossterm (select menus with number-key instant selection, text input with live space-to-hyphen conversion)
- Parent branch detection via merge-base for accurate PR targeting
- Child work branch support with retarget-and-merge workflow
- Base branch selection (`--base`) when starting work branches
- Auto-bump version before finishing release if commits exist since last tag
- Homebrew and Chocolatey package distribution
- Unit tests with mock Git/Hosting implementations

### Changed
- Split release and release-fix into separate menu actions
- Improved menu rendering and terminal artifact cleanup

## [0.1.0] - 2026-03-25

### Added
- Initial release of bflow CLI
- Core Git and HostingPlatform traits with GitHub implementation via `gh` CLI
- SemVer parsing, branch name generation, and branch type detection
- Interactive menu system for all branch types
- Start flows: feature, fix, chore, docs, refactor, release, release-fix, hotfix-fix
- Finish flows: work branches (PR creation), release (merge + tag), hotfix (merge + tag)
- Release management: bump version, sync with develop
- Cross-platform CI/CD with GitHub Actions (macOS x86_64, macOS ARM64, Windows)
- README with mermaid diagrams documenting the branch model and workflows

[3.0.0]: https://github.com/Beans-BV/beans-gitflow/compare/v2.4.0...v3.0.0
[2.4.0]: https://github.com/Beans-BV/beans-gitflow/compare/v2.3.0...v2.4.0
[2.3.0]: https://github.com/Beans-BV/beans-gitflow/compare/v2.2.0...v2.3.0
[2.2.0]: https://github.com/Beans-BV/beans-gitflow/compare/v2.1.0...v2.2.0
[2.1.0]: https://github.com/Beans-BV/beans-gitflow/compare/v2.0.0...v2.1.0
[2.0.0]: https://github.com/Beans-BV/beans-gitflow/compare/v1.2.1...v2.0.0
[1.2.1]: https://github.com/Beans-BV/beans-gitflow/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/Beans-BV/beans-gitflow/compare/v1.1.1...v1.2.0
[1.1.1]: https://github.com/Beans-BV/beans-gitflow/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/Beans-BV/beans-gitflow/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/Beans-BV/beans-gitflow/compare/v0.2.1...v1.0.0
[0.2.1]: https://github.com/Beans-BV/beans-gitflow/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Beans-BV/beans-gitflow/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Beans-BV/beans-gitflow/releases/tag/v0.1.0
