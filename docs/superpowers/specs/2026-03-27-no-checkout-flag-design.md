# Design: `--no-checkout` flag for `bflow start`

## Goal

Add a `--no-checkout` flag to `bflow start` subcommands so the branch is created and pushed but not checked out. This supports worktree workflows where the branch will be opened in a separate worktree after creation.

## Scope

**Applies to:** feature, fix, chore, docs, refactor, release-fix, hotfix-fix
**Does not apply to:** release (infrastructure setup, not a work branch)
**CLI-only** — interactive menu always uses default behavior (checkout).

## Example Usage

```bash
# Current behavior (unchanged):
bflow start feature --name login
# → git checkout -b feature/login develop
# → git push -u origin feature/login
# → you are now on feature/login

# New behavior:
bflow start feature --name login --no-checkout
# → git branch feature/login develop
# → git push -u origin feature/login
# → you stay on your current branch
```

## Design

### 1. CLI Layer (`cli.rs`)

Shared struct via `clap(flatten)`:

```rust
#[derive(Args, Debug, Clone)]
struct StartOptions {
    /// Create and push the branch without checking it out
    #[arg(long)]
    no_checkout: bool,
}
```

Flattened into the 7 relevant `StartKind` variants:
- `Feature { name, base, opts: StartOptions }`
- `Fix { name, base, opts: StartOptions }`
- `Chore { name, base, opts: StartOptions }`
- `Docs { name, base, opts: StartOptions }`
- `Refactor { name, base, opts: StartOptions }`
- `ReleaseFix { name, opts: StartOptions }`
- `HotfixFix { name, opts: StartOptions }`

`Release` stays untouched.

### 2. Action Enum (`menu.rs`)

Add `no_checkout: bool` to relevant variants:
- `StartWorkBranch { prefix, name, from, no_checkout }`
- `StartReleaseFix { name, no_checkout }`
- `StartHotfixFix { name, no_checkout }`

`StartRelease` unchanged. Interactive menu always passes `no_checkout: false`.

Add `Action::no_checkout()` helper — returns `true` if the variant carries `no_checkout: true`, `false` for all other variants.

### 3. Git Trait (`git/mod.rs`)

New method:

```rust
fn create_branch_no_checkout(&self, branch: &str, from: &str) -> Result<(), String>;
```

`GitCli` implementation: `git branch {branch} {from}` (creates branch without switching).

Existing `create_branch()` (`git checkout -b`) unchanged.

### 4. Flow Functions (`flows/start.rs`)

**`start_work_branch()`** — add `no_checkout: bool` param. When `true`, use `create_branch_no_checkout()` instead of `create_branch()`. Push unchanged.

**`start_release_fix()`** — add `no_checkout: bool` param. When `true`:
- Discover the release branch via `list_branches_matching("release/*")` instead of relying on the current branch
- Create fix branch with `create_branch_no_checkout()`
- Push unchanged

**`start_hotfix_fix()`** — add `no_checkout: bool` param. When `true`:
- `resolve_or_create_hotfix()` uses `git branch` instead of `git checkout -b` for intermediate branches (no checkout of `main`)
- Fix branch created with `create_branch_no_checkout()`
- All pushes unchanged

### 5. Main Orchestration (`main.rs`)

When `action.no_checkout()` is `true`:
- **Skip stash push/pop** — user stays on current branch, no dirty-tree risk
- **Skip merge of current branch** — not switching away, syncing isn't our concern

### 6. Tests

**New tests:**
- `start_work_branch` with `no_checkout: true` — verify `create_branch_no_checkout` called, push still happens
- `start_release_fix` with `no_checkout: true` — verify release branch discovered via `list_branches_matching`, fix branch created without checkout
- `start_hotfix_fix` with `no_checkout: true` (no existing hotfix) — both branches created without checkout
- `start_hotfix_fix` with `no_checkout: true` (existing hotfix) — only fix branch created without checkout
- CLI parsing — `--no-checkout` accepted on all 7 variants, not available on `start release`
- `Action::no_checkout()` helper returns correct values
- `MockGit` — add `create_branch_no_checkout` call tracking

**Existing tests:** Update to pass `no_checkout: false` for backward compat.
