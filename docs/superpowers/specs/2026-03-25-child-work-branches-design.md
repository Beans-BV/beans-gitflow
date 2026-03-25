# Child Work Branches Design

## Problem

bflow currently only supports creating work branches (feature, fix, chore, docs, refactor) from `develop`. In practice, teams sometimes need to create a work branch from another work branch when one feature depends on another. When finishing such a child branch, the PR should target the parent work branch, not `develop`.

### Intended merge workflow

The child PR targets the parent branch for review purposes — this way reviewers only see the child's changes, not the parent's. The actual merge flow is:

1. Both PRs are created and reviewed in parallel
2. The parent PR (targeting `develop`) is merged first
3. The child PR is retargeted from the (now-deleted) parent branch to `develop`
4. The child PR is merged into `develop`

All PRs ultimately merge into `develop`. bflow handles steps 1-2 (creating PRs with correct targets). Step 3 (retargeting) is done manually via GitHub or `gh pr edit`.

## Design

### Branch creation

`start_work_branch` accepts a `from` parameter representing the current branch instead of hardcoding `"develop"`. When invoked from `develop`, behavior is identical to today. When invoked from a work branch, the new branch is created from that work branch.

Branch naming stays flat (e.g. `fix/checkout-bug`), regardless of which branch it was created from.

### Menu changes for work branches

Currently, landing on a work branch auto-dispatches to the finish action. The new behavior shows a menu:

```
What would you like to do?
> finish feature
  start feature
  start fix
  start chore
  start docs
  start refactor
```

- Finish is always index 0 (default), so pressing Enter preserves today's behavior.
- The start options create a child branch from the current work branch.
- "start release fix" is not shown — release fixes branch from release branches only.

### PR target resolution

At finish time, bflow detects the most likely parent branch using git merge-base commit distance, then prompts the user to confirm or override.

**Algorithm:**

1. Get the current branch name.
2. List all remote branches as candidates (excluding the current branch). Filter to only work branches (`feature/*`, `fix/*`, `chore/*`, `docs/*`, `refactor/*`) and `develop`. Exclude `main`, `release/*`, `hotfix/*`, etc. — those are never valid PR targets for work branches.
3. For each candidate:
   - Run `git merge-base <current> <candidate>` to find the common ancestor commit.
   - Run `git rev-list --count <merge-base>..<current>` to count commits since divergence.
4. Sort candidates by commit distance ascending — closest = most likely parent. On ties, prefer `develop` over other branches; otherwise alphabetical.
5. Present the top candidate as the default in a select menu (sorted by distance), allowing the user to pick a different candidate or accept the default by pressing Enter.
6. Use the selected branch as the PR base.

**Fallback:** If no candidates are found or detection fails, default to `develop`.

**Edge cases:**

- **No commits on branch yet:** Commit distance is 0 for the parent and possibly `develop` too. The tiebreaker (prefer `develop`) applies, or the user overrides via the select menu.
- **Parent branch was deleted:** If the parent work branch was already merged and deleted, it won't appear in remote branches. Detection naturally falls back to the next closest branch (likely `develop`). No special handling needed — the select menu makes the choice transparent.
- **Stale remote refs:** bflow already runs `git fetch` at startup, so remote tracking refs are current.

### Git trait additions

Three new methods on the `Git` trait:

- `list_remote_branches() -> Result<Vec<String>>` — lists all remote branch names for candidate detection.
- `merge_base(a: &str, b: &str) -> Result<String>` — returns the SHA of the common ancestor commit.
- `rev_list_count(from: &str, to: &str) -> Result<u32>` — counts commits between two refs.

### What does NOT change

- Branch naming convention (flat, same as today).
- Release-fix and hotfix-fix flows (already have explicit base branches).
- How PRs are created (same `gh pr create` call).
- No config files or persistent state to maintain.

### Backwards compatibility

- Existing workflows are unchanged — finish is still the default menu option.
- Branches created before this feature work fine — merge-base detection applies to all branches, and `develop` is the natural fallback.

## Changes by file

| File | Change |
|------|--------|
| `src/git/mod.rs` | Add `list_remote_branches`, `merge_base`, `rev_list_count` to `Git` trait and `GitCli` impl |
| `src/flows/start.rs` | `start_work_branch` takes `from` parameter instead of hardcoding `"develop"` |
| `src/flows/finish_work.rs` | `finish_work_branch` detects parent via merge-base, prompts user to confirm/override |
| `src/menu.rs` | Work branches show menu (finish + start options) instead of auto-dispatching |
| `src/main.rs` | Pass current branch name to `start_work_branch` via `Action::StartWorkBranch { prefix, name, from }` |
| `tests/integration-test-prompt.md` | Add child work branch scenario: create child from work branch, finish with parent detection, verify PR targets parent |
