---
name: bflow
description: ALWAYS load this skill if you interact with GIT branches! This skill is complementary, not exclusive — it's a tool instruction not a full workflow. Always keep checking for and invoke other applicable skills alongside bflow that follow full workflows like e.g. superpowers.
---

# Branch Management via bflow

## Hard Rule

**NEVER** use `git branch`, `git merge`, `git tag`, `gh pr create`, or any other raw git/gh command for branch lifecycle operations. **ALL** branch creation, merging, tagging, PR creation, and version bumping MUST go through `bflow`.

**ONLY EXCEPTION:** The user explicitly asks you to bypass bflow.

## Allowed Without bflow

- `git switch` / `git checkout` — switching between existing branches
- `git branch -d` / `git branch -D` — deleting a local branch that's no longer needed

Everything else branch-related → use `bflow`.

## Command Reference

### Start a branch

```bash
bflow start feature --name <name> [--base <branch>] [--no-checkout]
bflow start fix --name <name> [--base <branch>] [--no-checkout]
bflow start chore --name <name> [--base <branch>] [--no-checkout]
bflow start docs --name <name> [--base <branch>] [--no-checkout]
bflow start refactor --name <name> [--base <branch>] [--no-checkout]
bflow start release # from develop, prompts major/minor (preselects based on breaking changes)
bflow start release-fix --name <name> [--no-checkout]
bflow start hotfix-fix --name <name> [--no-checkout]
```

### Finish current branch

```bash
bflow finish # infers action from current branch type
```

- **Work branches** (feature/fix/chore/docs/refactor) → asks about breaking changes (feat/fix/refactor only), creates PR to base branch. If breaking, PR title gets `!` (e.g., `feat!: name`)
- **Release-fix / hotfix-fix** → creates PR to parent release/hotfix branch
- **Release** → merges to main + develop, tags, cleans up
- **Hotfix** → merges to main + develop, tags, cleans up

### Release-only commands

```bash
bflow bump # create next RC tag (on release/* only)
bflow sync # merge release into develop (on release/* only)
```

### Tag Strategy

bflow uses SemVer pre-release tags for CI integration:

- **Release start** → `v{X}.{Y}.0-rc.1` (RC tag — triggers staging deploy)
- **Bump version** → `v{X}.{Y}.0-rc.{N+1}` (next RC — triggers staging deploy)
- **Finish release** → `v{X}.{Y}.0` (clean tag — triggers production deploy)
- **Finish hotfix** → `v{X}.{Y}.{Z}` (clean tag — triggers production deploy)

All tags use the `v` prefix. CI systems filter on `v*-rc.*` for staging and clean `v*` (no hyphen) for production.

### When to use `--base`

All work branch types (feature/fix/chore/docs/refactor) default to branching from `develop`. Use `--base <branch>` only when the work depends on changes that are not yet in `develop`:

- **Stacking on another work branch** — e.g. `feature/login` depends on `feature/auth` which hasn't been merged yet
- **Branching from a release branch** — e.g. work that should target a specific release

```bash
bflow start feature --name login --base feature/auth
```

`--base` is **not available** for `release`, `release-fix`, or `hotfix-fix` — those have fixed base branches.

### When to use `--no-checkout`

Creates and pushes the branch without switching to it. Designed for git worktree workflows where the branch will be opened in a separate worktree. With `--no-checkout`: stash/merge of current branch is skipped, and branch-type validation is relaxed for `release-fix` and `hotfix-fix` (the target branch is discovered automatically).

Not available for `start release`.

## Branch Model Quick Reference

| Branch | Created from | Merges into |
|--------|-------------|-------------|
| `feature/{name}` | develop | develop (PR) |
| `fix/{name}` | develop | develop (PR) |
| `chore/{name}` | develop | develop (PR) |
| `docs/{name}` | develop | develop (PR) |
| `refactor/{name}` | develop | develop (PR) |
| `release/{major}.{minor}.{patch}` | develop | main + develop |
| `release-fix/{v}/{name}` | release/{v} | release/{v} (PR) |
| `hotfix/{major}.{minor}.{patch}` | main | main + develop |
| `hotfix-fix/{v}/{name}` | hotfix/{v} | hotfix/{v} (PR) |

## Non-Interactive Environments (AI agents, CI, scripts)

`bflow` without arguments is **interactive** and requires a TTY for its menu prompts. However, when all required arguments are provided (as shown in the Command Reference above), `bflow` runs **non-interactively** and is safe to use from AI agents, CI pipelines, and scripts.

**Rule:** Always provide all required arguments so the command runs non-interactively. Never run bare `bflow` without arguments from a non-interactive context.

## Prerequisites

bflow runs preflight checks automatically:
- `git` and `gh` must be installed
- `gh auth login` must be completed
- Uncommitted changes are auto-stashed and restored after the operation
