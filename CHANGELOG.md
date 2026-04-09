# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[1.2.1]: https://github.com/Beans-BV/beans-gitflow/compare/v1.2.0...v1.2.1
[1.2.0]: https://github.com/Beans-BV/beans-gitflow/compare/v1.1.1...v1.2.0
[1.1.1]: https://github.com/Beans-BV/beans-gitflow/compare/v1.1.0...v1.1.1
[1.1.0]: https://github.com/Beans-BV/beans-gitflow/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/Beans-BV/beans-gitflow/compare/v0.2.1...v1.0.0
[0.2.1]: https://github.com/Beans-BV/beans-gitflow/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Beans-BV/beans-gitflow/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Beans-BV/beans-gitflow/releases/tag/v0.1.0
