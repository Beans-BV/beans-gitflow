---
name: bflow
description: Use when performing ANY branch operation (create, merge, finish, tag, PR creation) — all branch management MUST go through the bflow CLI, never raw git branch/merge or gh CLI
user-invocable: false
disable-model-invocation: false
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
bflow start feature --name <name> # from develop (default)
bflow start fix --name <name> # from develop
bflow start chore --name <name> # from develop
bflow start docs --name <name> # from develop
bflow start refactor --name <name> # from develop
bflow start feature --name <name> --base <branch> # from custom base for work branch stacking (see below)
bflow start release # from develop, auto-versions
bflow start release-fix --name <name> # must be on release/* branch
bflow start hotfix-fix --name <name> # must be on main or hotfix/* branch
```

### Finish current branch

```bash
bflow finish # infers action from current branch type
```

- **Work branches** (feature/fix/chore/docs/refactor) → creates PR to base branch
- **Release-fix / hotfix-fix** → creates PR to parent release/hotfix branch
- **Release** → merges to main + develop, tags, cleans up
- **Hotfix** → merges to main + develop, tags, cleans up

### Release-only commands

```bash
bflow bump # bump patch version and retag (on release/* only)
bflow sync # merge release into develop (on release/* only)
```

### When to use `--base`

All work branch types (feature/fix/chore/docs/refactor) default to branching from `develop`. Use `--base <branch>` only when the work depends on changes that are not yet in `develop`:

- **Stacking on another work branch** — e.g. `feature/login` depends on `feature/auth` which hasn't been merged yet
- **Branching from a release branch** — e.g. work that should target a specific release

```bash
bflow start feature --name login --base feature/auth
```

`--base` is **not available** for `release`, `release-fix`, or `hotfix-fix` — those have fixed base branches.

## Branch Model Quick Reference

| Branch | Created from | Merges into |
|--------|-------------|-------------|
| `feature/{name}` | develop | develop (PR) |
| `fix/{name}` | develop | develop (PR) |
| `chore/{name}` | develop | develop (PR) |
| `docs/{name}` | develop | develop (PR) |
| `refactor/{name}` | develop | develop (PR) |
| `release/{major}.{minor}` | develop | main + develop |
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
- Working tree must be clean (commit or stash first)
