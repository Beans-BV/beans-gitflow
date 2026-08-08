---
name: bflow
description: ALWAYS load this skill if you interact with GIT branches! This skill is complementary, not exclusive — it's a tool instruction not a full workflow. Always keep checking for and invoke other applicable skills alongside bflow that follow full workflows like e.g. superpowers.
---

# Branch Management via bflow

## Hard Rule

**NEVER** use `git branch`, `git merge`, `git tag`, `gh pr create`, `az repos pr create`, or any other raw git/gh/az command for branch lifecycle operations. **ALL** branch creation, merging, tagging, PR creation, and version bumping MUST go through `bflow`.

**ONLY EXCEPTION:** The user explicitly asks you to bypass bflow.

## Allowed Without bflow

- `git switch` / `git checkout` — switching between existing branches
- `git branch -d` / `git branch -D` — deleting a local branch that's no longer needed

Everything else branch-related → use `bflow`.

## Command Reference

### Start a branch

```bash
bflow start feature --name <name> [--base <branch>] [--no-checkout] [--no-worktree]
bflow start fix --name <name> [--base <branch>] [--no-checkout] [--no-worktree]
bflow start chore --name <name> [--base <branch>] [--no-checkout] [--no-worktree]
bflow start docs --name <name> [--base <branch>] [--no-checkout] [--no-worktree]
bflow start refactor --name <name> [--base <branch>] [--no-checkout] [--no-worktree]
bflow start release [--major | --minor] # from develop, prompts if no flag given
bflow start release-fix --name <name> [--no-checkout] [--no-worktree]
bflow start hotfix-fix --name <name> [--no-checkout] [--no-worktree]  # from the mainline or a hotfix branch
```

#### Mainline branch (`main` or `master`)

Either name works. On its first run in a repo bflow detects which exists and
saves it to `bflow.branch.main` (**local** scope — the mainline belongs to the
repo, unlike `bflow.worktree.*`), announcing the save. Set it yourself with
`git config bflow.branch.main master`. Only `main` and `master` are accepted;
anything else is a hard error. Everything below that says "main" means this
resolved branch.

#### Worktree integration (optional)

When `bflow.worktree.enabled=true` (git config), `start` (work branches + release-fix/hotfix-fix, not `release`) creates the branch in a native git worktree and opens it in an editor instead of switching the current checkout. Config keys: `bflow.worktree.enabled` (bool, default false), `bflow.worktree.editor` (default `code`; `none` skips opening), `bflow.worktree.path` (base dir, default repo's parent, `~` expanded). Folder name: `<repo-name>-<branch-with-slashes-as-dashes>`. `--no-worktree` skips it for one command. Like `--no-checkout`, active worktree mode relaxes the branch-type check for `release-fix`/`hotfix-fix` (target branch is discovered automatically).

Configure it with the `bflow worktree` command (writes global git config; `--local` for one repo):

```bash
bflow worktree                     # interactive setup (enable / editor / location)
bflow worktree enable | disable
bflow worktree editor <cmd>        # code | cursor | windsurf | zed | pycharm | none | any command
bflow worktree path <dir>
bflow worktree status
```

### Finish current branch

```bash
bflow finish [--breaking] [--base <branch>]  # infers action from current branch type
bflow finish --abort       # discard an in-progress release/hotfix finish
```

- **Work branches** (feature/fix/chore/docs/refactor) → asks about breaking changes (feat/fix/refactor only), creates PR to base branch. PR title converts dashes to spaces (e.g. `feature/foo-bar` → `feat: foo bar`). If breaking, PR title gets `!` (e.g., `feat!: foo bar`). Use `--breaking` flag in non-interactive mode to skip the prompt.
  - PR target: detected from branch topology; a single candidate is used directly, multiple candidates show a menu. `--base <branch>` sets the target explicitly and skips both — **AI agents/CI must pass `--base` (plus `--breaking`) so finish never needs a TTY** (e.g. `bflow finish --base develop --breaking=false`). The branch must exist on origin (push or fetch first) and differ from the current branch. Not valid on release/hotfix/release-fix/hotfix-fix (fixed target).
  - **PR already merged** → re-running `bflow finish` completes the finish instead of opening a new PR: deletes remote + local branch and, when the branch is in its own worktree, removes the worktree (close the editor window afterwards). Only when the local tip equals the merged commit — new commits since the merge get a fresh PR instead. Applies to work branches and release-fix/hotfix-fix.
- **Release-fix / hotfix-fix** → creates PR to parent release/hotfix branch, title `fix: {name}` (dashes → spaces, e.g. `null-crash` → `fix: null crash`)
- **Release** → merges to main + develop, tags, cleans up
- **Hotfix** → merges to main + develop + every open `release/*`, tags, cleans up. If a release branch already exists, the hotfix is propagated into it so the upcoming release ships the fix; the operator must then run `bflow bump` to cut a new RC for staging validation.

### Resuming after a merge conflict

`bflow finish` for **release** and **hotfix** branches is **idempotent**: re-running it after a merge conflict resumes from the first incomplete step. Recovery procedure:

1. Resolve the conflict in your editor.
2. `git add` the resolved files and `git commit` to complete the merge.
3. **Switch back to the source branch** (`git switch <release|hotfix branch>`) — a conflict usually leaves HEAD on the target branch (e.g. develop). The conflict message names the branch.
4. Re-run `bflow finish` — already-done steps (merges, tags, pushes, branch deletion) are detected and skipped.

**Resume is branch-scoped.** Each in-progress finish is tracked in its own file under `.git/bflow-finish/` (e.g. `hotfix-2.5.2.state`), keyed by the source branch. bflow resumes **only when you are on that source branch** — from develop/main/feature it behaves normally, so a stalled finish never blocks other work. Two finishes (release + hotfix) can be in progress at once. Run `bflow finish --abort` from the source branch to discard its state. Legacy pre-2.4 `.git/bflow-finish.state` files are migrated automatically.

### PR templates

PR bodies resolve from `.github/pr-templates/bflow-<key>.md`, most-specific first:

1. Branch-specific: `bflow-<type>.md` (e.g. `bflow-release-fix.md`)
2. Group: the fix family (`fix`, `release-fix`, `hotfix-fix`) shares `bflow-fix.md`; other types' group == their own name
3. `bflow-default.md`
4. Repo's git default (`.github/PULL_REQUEST_TEMPLATE.md` etc. on GitHub; `.azuredevops/pull_request_template.md` etc. on Azure DevOps), else empty body

Opt-in: with no `.github/pr-templates/`, behavior is unchanged.

### Release-only commands

```bash
bflow bump # create next RC tag (on release/* only)
bflow sync # merge release into develop (on release/* only; one-way — develop is never merged into a release, the staging-tag gate depends on it)
```

### Landing modes & version script

`.bflow/config` (committed file, not git config — repo policy, not per-clone): `mode=free|protected` (default `free` = today's behavior), `keep-release-branches=true|false` (default `false`; skips deleting `release/*`/`hotfix/*` on finish, work branches unaffected), `bump-strategy=rc|patch` (default `rc`; see Tag Strategy).

**`mode=protected`** — `main`/`develop` require PRs. `finish`/`bump`/`sync` open (or reuse) a PR for every landing instead of merging directly, print its URL, and **exit 0** — bflow never merges a PR. **Re-run the same command after a human merges it**; it resumes even if resolving a PR conflict (e.g. a version-file conflict landing into develop) added a commit to the source branch — an already-landed leg stays landed. One PR per run, in order (main → develop → each open release branch for hotfixes); only the last landing deletes the branch — unless its tip isn't part of any landed PR, in which case bflow keeps it, warns why, and prints the manual delete command (a guard, not a failure — the run still completed). Nothing is stored on disk to resume — progress is re-derived from PR/tag state each run. Don't add new, unrelated work to a release branch once its `main` PR has merged (the clean tag is already placed) — ship fixes as a hotfix instead.

`bump` may print `Version PR: {url}` and defer the RC tag when a version-script commit needs its own PR on an already-pushed release branch — re-run `bflow bump` after that PR merges; it tags the **PR's merge commit**, it does not re-run the script.

**Version script** (`.bflow/set-version.sh` / `.bflow/set-version.cmd`, platform-picked; both missing = feature off): runs with the clean `X.Y.Z` at release/hotfix branch creation, the post-release develop bump, and `bump`; commits `chore: set version {v}` only if it changed files. Requires a clean tree first. Under `bump-strategy=patch` each `bump` passes the **newly incremented** `X.Y.Z` (so the script commits every bump); under `rc` it gets the constant `X.Y.0`.

`chore/set-version-{v}` and `release-chore/{v}/set-version` are **bflow-created** — merge their PR, never commit to them. `chore/set-version-*` is also excluded from `finish`'s PR-target candidates (it gets merged and deleted out from under a PR that targeted it). A `release-chore/*` branch finishes like `release-fix` (PR into its release branch, no `--base`).

**Papercut**: version files can conflict on merge (hotfix vs. develop, major release vs. develop) — resolve manually, bflow does not auto-resolve.

`--no-checkout` hotfix creation skips the script (HEAD isn't on the new branch) and warns with the manual recovery steps.

### Tag Strategy

Selected per repo via `bump-strategy` in `.bflow/config`. All tags use the `v` prefix.

**`rc` (default)** — SemVer pre-release tags for CI integration:

- **Release start** → `v{X}.{Y}.0-rc.1` (RC tag — triggers staging deploy)
- **Bump version** → `v{X}.{Y}.0-rc.{N+1}` (next RC — triggers staging deploy)
- **Finish release** → `v{X}.{Y}.0` (clean tag — triggers production deploy)

CI systems filter on `v*-rc.*` for staging and clean `v*` (no hyphen) for production.

**`patch`** — every staged build carries a real, incrementing version (for projects that can't consume pre-release tags):

- **Release start** → `v{X}.{Y}.0` (clean tag)
- **Bump version** → next patch, e.g. `v{X}.{Y}.1`, `v{X}.{Y}.2` (clean tags)
- **Finish release** → merge only — the last bump tag is already final (finish re-pushes it if origin lacks it)
- **Hotfix version** derives from *shipped* tags only — an open release's staging tags are skipped

No tag-shape staging/production split exists under `patch`: CI must gate production on something else (branch, merge to main, manual promotion).

**Both strategies:**

- **Finish hotfix** → `v{X}.{Y}.{Z}` (clean tag at finish, both strategies)
- **Guard:** `bflow finish` rejects a release branch if HEAD is past the latest RC/patch tag — prevents promoting unstaged commits to production. Fix by running `bflow bump` and waiting for staging.

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
| `release-chore/{v}/set-version` | release/{v} (bflow-created) | release/{v} (PR) |
| `hotfix/{major}.{minor}.{patch}` | main | main + develop + open `release/*` |
| `hotfix-fix/{v}/{name}` | hotfix/{v} | hotfix/{v} (PR) |

## Non-Interactive Environments (AI agents, CI, scripts)

`bflow` without arguments is **interactive** and requires a TTY for its menu prompts. However, when all required arguments are provided (as shown in the Command Reference above), `bflow` runs **non-interactively** and is safe to use from AI agents, CI pipelines, and scripts.

**Rule:** Always provide all required arguments so the command runs non-interactively. Never run bare `bflow` without arguments from a non-interactive context.

## Prerequisites

bflow runs preflight checks automatically:
- `git` must be installed
- The hosting provider is auto-detected from the origin remote URL (`dev.azure.com` / `*.visualstudio.com` → Azure DevOps, else GitHub); override with `git config bflow.hosting.provider github|devops`
- GitHub repos: `gh` installed + `gh auth login` completed
- Azure DevOps repos: `az` installed + `az extension add --name azure-devops` + authenticated (`az login`, or a PAT via `az devops login`)
- Uncommitted changes are auto-stashed and restored after the operation
