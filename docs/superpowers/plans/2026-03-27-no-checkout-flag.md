# `--no-checkout` Flag Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--no-checkout` flag to `bflow start` subcommands so branches are created and pushed without checking them out, supporting worktree workflows.

**Architecture:** Add a shared `StartOptions` struct (clap flatten) to 7 `StartKind` variants, thread `no_checkout: bool` through `Action` → flow functions, add `create_branch_no_checkout` to `Git` trait, and skip stash/merge in `main.rs` when the flag is set.

**Tech Stack:** Rust, clap v4 (derive), standard Git CLI

---

### Task 1: Add `create_branch_no_checkout` to Git trait and MockGit

**Files:**
- Modify: `src/git/mod.rs:7-27` (Git trait)
- Modify: `src/git/mod.rs:46-102` (GitCli impl)
- Modify: `tests/common/mod.rs:35-130` (MockGit impl)

- [ ] **Step 1: Write the failing test**

Add to `tests/stash_test.rs` (reusing as a simple git method test file):

```rust
#[test]
fn create_branch_no_checkout_records_call() {
    let git = MockGit::new();
    git.create_branch_no_checkout("feature/test", "develop").unwrap();
    assert_eq!(git.calls(), vec!["create_branch_no_checkout:feature/test:develop"]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test create_branch_no_checkout_records_call`
Expected: compile error — `create_branch_no_checkout` not found on `Git` trait

- [ ] **Step 3: Add method to Git trait**

In `src/git/mod.rs`, add to the `Git` trait (after `create_branch`):

```rust
fn create_branch_no_checkout(&self, branch: &str, from: &str) -> Result<()>;
```

- [ ] **Step 4: Add GitCli implementation**

In `src/git/mod.rs`, add to `impl Git for GitCli` (after `create_branch`):

```rust
fn create_branch_no_checkout(&self, branch: &str, from: &str) -> Result<()> {
    self.run(&["branch", branch, from]).map(|_| ())
}
```

- [ ] **Step 5: Add MockGit implementation**

In `tests/common/mod.rs`, add to `impl Git for MockGit` (after `create_branch`):

```rust
fn create_branch_no_checkout(&self, branch: &str, from: &str) -> Result<(), String> {
    self.calls.borrow_mut().push(format!("create_branch_no_checkout:{branch}:{from}"));
    Ok(())
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cargo test create_branch_no_checkout_records_call`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/git/mod.rs tests/common/mod.rs tests/stash_test.rs
git commit -m "feat: add create_branch_no_checkout to Git trait"
```

---

### Task 2: Add `no_checkout` to Action enum and helpers

**Files:**
- Modify: `src/menu.rs:373-398` (Action enum and is_start)

- [ ] **Step 1: Write the failing test**

Add to `tests/action_test.rs`:

```rust
#[test]
fn no_checkout_returns_true_for_start_work_branch() {
    let action = Action::StartWorkBranch {
        prefix: "feature".into(),
        name: "x".into(),
        from: "develop".into(),
        no_checkout: true,
    };
    assert!(action.no_checkout());
}

#[test]
fn no_checkout_returns_false_for_start_work_branch_default() {
    let action = Action::StartWorkBranch {
        prefix: "feature".into(),
        name: "x".into(),
        from: "develop".into(),
        no_checkout: false,
    };
    assert!(!action.no_checkout());
}

#[test]
fn no_checkout_returns_true_for_start_release_fix() {
    let action = Action::StartReleaseFix { name: "x".into(), no_checkout: true };
    assert!(action.no_checkout());
}

#[test]
fn no_checkout_returns_true_for_start_hotfix_fix() {
    let action = Action::StartHotfixFix { name: "x".into(), no_checkout: true };
    assert!(action.no_checkout());
}

#[test]
fn no_checkout_returns_false_for_non_start_actions() {
    let actions: Vec<Action> = vec![
        Action::StartRelease,
        Action::FinishWorkBranch,
        Action::FinishReleaseFix,
        Action::FinishRelease,
        Action::FinishHotfix,
        Action::FinishHotfixFix,
        Action::BumpVersion,
        Action::SyncWithDevelop,
    ];
    for action in actions {
        assert!(!action.no_checkout(), "Expected no_checkout() == false for {:?}", action);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test no_checkout_returns`
Expected: compile errors — `no_checkout` field doesn't exist, `no_checkout()` method doesn't exist

- [ ] **Step 3: Update Action enum variants**

In `src/menu.rs`, replace the Action enum:

```rust
#[derive(Debug, PartialEq)]
pub enum Action {
    StartWorkBranch { prefix: String, name: String, from: String, no_checkout: bool },
    StartRelease,
    StartReleaseFix { name: String, no_checkout: bool },
    StartHotfixFix { name: String, no_checkout: bool },
    FinishWorkBranch,
    FinishReleaseFix,
    FinishRelease,
    FinishHotfix,
    FinishHotfixFix,
    BumpVersion,
    SyncWithDevelop,
}
```

- [ ] **Step 4: Add `no_checkout()` method**

In `src/menu.rs`, add below `is_start()` in the `impl Action` block:

```rust
pub fn no_checkout(&self) -> bool {
    match self {
        Action::StartWorkBranch { no_checkout, .. } => *no_checkout,
        Action::StartReleaseFix { no_checkout, .. } => *no_checkout,
        Action::StartHotfixFix { no_checkout, .. } => *no_checkout,
        _ => false,
    }
}
```

- [ ] **Step 5: Fix all compile errors from changed Action variants**

Every place that constructs an `Action` variant with the new field needs updating. Add `no_checkout: false` to all existing construction sites:

In `src/menu.rs` `show_menu()` — 3 sites:
- `Action::StartWorkBranch` at line ~314: add `no_checkout: false`
- `Action::StartWorkBranch` at line ~340: add `no_checkout: false`
- `Action::StartReleaseFix` at line ~354: add `no_checkout: false`
- `Action::StartHotfixFix` at line ~304: add `no_checkout: false`

In `src/cli.rs` `start_work_branch()` at line 73:
```rust
Ok(Action::StartWorkBranch { prefix: prefix.to_string(), name, from: base, no_checkout: false })
```

In `src/cli.rs` `resolve_action()`:
- `Action::StartReleaseFix` at line ~95: add `no_checkout: false`
- `Action::StartHotfixFix` at line ~102: add `no_checkout: false`

In `tests/action_test.rs` `start_actions_return_true` — update existing test:
```rust
Action::StartWorkBranch { prefix: "feature".into(), name: "x".into(), from: "develop".into(), no_checkout: false },
Action::StartReleaseFix { name: "x".into(), no_checkout: false },
Action::StartHotfixFix { name: "x".into(), no_checkout: false },
```

- [ ] **Step 6: Run all tests to verify everything compiles and passes**

Run: `cargo test`
Expected: all tests PASS (including new `no_checkout_returns_*` tests)

- [ ] **Step 7: Commit**

```bash
git add src/menu.rs src/cli.rs tests/action_test.rs
git commit -m "feat: add no_checkout field to Action enum with helper method"
```

---

### Task 3: Add `StartOptions` to CLI and thread through `resolve_action`

**Files:**
- Modify: `src/cli.rs` (StartKind enum, resolve_action)

- [ ] **Step 1: Write the failing test**

Add to `tests/cli_test.rs`:

```rust
#[test]
fn start_feature_with_no_checkout_flag() {
    let cmd = Commands::Start { kind: StartKind::Feature {
        name: "login".to_string(),
        base: "develop".to_string(),
        opts: StartOptions { no_checkout: true },
    }};
    let branch_type = BranchType::Develop;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartWorkBranch { no_checkout: true, .. }));
}

#[test]
fn start_release_fix_with_no_checkout_flag() {
    let cmd = Commands::Start { kind: StartKind::ReleaseFix {
        name: "broken-login".to_string(),
        opts: StartOptions { no_checkout: true },
    }};
    let branch_type = BranchType::Release { major: 1, minor: 2 };
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartReleaseFix { no_checkout: true, .. }));
}

#[test]
fn start_hotfix_fix_with_no_checkout_flag() {
    let cmd = Commands::Start { kind: StartKind::HotfixFix {
        name: "urgent".to_string(),
        opts: StartOptions { no_checkout: true },
    }};
    let branch_type = BranchType::Main;
    let action = resolve_action(cmd, &branch_type).unwrap();
    assert!(matches!(action, Action::StartHotfixFix { no_checkout: true, .. }));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test start_feature_with_no_checkout_flag start_release_fix_with_no_checkout_flag start_hotfix_fix_with_no_checkout_flag`
Expected: compile error — `StartOptions` doesn't exist, `opts` field doesn't exist

- [ ] **Step 3: Add `StartOptions` struct and flatten into `StartKind`**

In `src/cli.rs`, add the struct and update imports:

```rust
use clap::{Args, Subcommand};
use crate::git::branch::BranchType;
use crate::menu::{self, Action};

#[derive(Args, Debug, Clone)]
pub struct StartOptions {
    /// Create and push the branch without checking it out
    #[arg(long)]
    pub no_checkout: bool,
}
```

Update `StartKind` enum:

```rust
#[derive(Subcommand)]
pub enum StartKind {
    /// Start a new feature branch
    Feature {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a new fix branch
    Fix {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a new chore branch
    Chore {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a new docs branch
    Docs {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a new refactor branch
    Refactor {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "develop")]
        base: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a new release branch (or resume existing)
    Release,
    /// Start a release fix branch (must be on a release branch)
    ReleaseFix {
        #[arg(long)]
        name: String,
        #[command(flatten)]
        opts: StartOptions,
    },
    /// Start a hotfix fix branch (must be on main or hotfix branch)
    HotfixFix {
        #[arg(long)]
        name: String,
        #[command(flatten)]
        opts: StartOptions,
    },
}
```

- [ ] **Step 4: Update `start_work_branch` helper and `resolve_action`**

Update the helper function:

```rust
fn start_work_branch(prefix: &str, name: String, base: String, no_checkout: bool) -> Result<Action, String> {
    menu::validate_branch_name(&name)?;
    Ok(Action::StartWorkBranch { prefix: prefix.to_string(), name, from: base, no_checkout })
}
```

Update `resolve_action` match arms:

```rust
pub fn resolve_action(command: Commands, branch_type: &BranchType) -> Result<Action, String> {
    match command {
        Commands::Start { kind } => match kind {
            StartKind::Feature { name, base, opts } => start_work_branch("feature", name, base, opts.no_checkout),
            StartKind::Fix { name, base, opts } => start_work_branch("fix", name, base, opts.no_checkout),
            StartKind::Chore { name, base, opts } => start_work_branch("chore", name, base, opts.no_checkout),
            StartKind::Docs { name, base, opts } => start_work_branch("docs", name, base, opts.no_checkout),
            StartKind::Refactor { name, base, opts } => start_work_branch("refactor", name, base, opts.no_checkout),
            StartKind::Release => Ok(Action::StartRelease),
            StartKind::ReleaseFix { name, opts } => {
                menu::validate_branch_name(&name)?;
                require_release_branch(branch_type)?;
                Ok(Action::StartReleaseFix { name, no_checkout: opts.no_checkout })
            }
            StartKind::HotfixFix { name, opts } => {
                menu::validate_branch_name(&name)?;
                if !matches!(branch_type, BranchType::Main | BranchType::Hotfix { .. }) {
                    return Err("This command is only valid on a main or hotfix branch.".to_string());
                }
                Ok(Action::StartHotfixFix { name, no_checkout: opts.no_checkout })
            }
        },
        // ... rest unchanged
```

- [ ] **Step 5: Fix existing cli_test.rs tests that construct StartKind**

Update all existing `StartKind` constructions in `tests/cli_test.rs` to include `opts: StartOptions { no_checkout: false }`. Add the import:

```rust
use bflow::cli::{Commands, StartKind, StartOptions, resolve_action};
```

Update every `StartKind::Feature { name, base }` to `StartKind::Feature { name, base, opts: StartOptions { no_checkout: false } }`. Same for Fix, Chore, Docs, Refactor, ReleaseFix, HotfixFix.

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 7: Commit**

```bash
git add src/cli.rs tests/cli_test.rs
git commit -m "feat: add StartOptions with --no-checkout flag to CLI"
```

---

### Task 4: Thread `no_checkout` through flow functions

**Files:**
- Modify: `src/flows/start.rs` (all 3 public functions + 1 private helper)
- Modify: `src/main.rs:92-104` (dispatch)

- [ ] **Step 1: Write failing tests for `start_work_branch` with no_checkout**

Add to `tests/start_test.rs`:

```rust
#[test]
fn start_work_branch_no_checkout_creates_without_switching() {
    let git = MockGit::new();
    start_work_branch(&git, "feature", "login-page", "develop", true).unwrap();

    assert_eq!(git.calls(), vec![
        "create_branch_no_checkout:feature/login-page:develop",
        "push:feature/login-page",
    ]);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test start_work_branch_no_checkout`
Expected: compile error — `start_work_branch` takes 4 args, not 5

- [ ] **Step 3: Update `start_work_branch` signature and implementation**

In `src/flows/start.rs`:

```rust
pub fn start_work_branch(git: &dyn Git, prefix: &str, name: &str, from: &str, no_checkout: bool) -> Result<(), String> {
    let branch = format!("{prefix}/{name}");
    println!("Creating branch: {branch}");
    if no_checkout {
        git.create_branch_no_checkout(&branch, from).map_err(|e| {
            if e.contains("not a commit") {
                format!("Branch '{from}' does not exist. Use --base to specify a different base branch.")
            } else {
                e
            }
        })?;
    } else {
        git.create_branch(&branch, from).map_err(|e| {
            if e.contains("not a commit") {
                format!("Branch '{from}' does not exist. Use --base to specify a different base branch.")
            } else {
                e
            }
        })?;
    }
    git.push(&branch)?;
    println!("Branch '{branch}' created and pushed.");
    Ok(())
}
```

- [ ] **Step 4: Fix existing `start_work_branch` call sites**

In `src/main.rs` dispatch (line ~94):

```rust
Action::StartWorkBranch { prefix, name, from, no_checkout } => {
    start::start_work_branch(git, &prefix, &name, &from, no_checkout)?;
}
```

In `tests/start_test.rs`, update existing tests to pass `false`:

```rust
start_work_branch(&git, "feature", "login-page", "develop", false).unwrap();
```

```rust
start_work_branch(&git, "fix", "broken-auth", "main", false).unwrap();
```

- [ ] **Step 5: Run tests**

Run: `cargo test start_work_branch`
Expected: all PASS including new `start_work_branch_no_checkout_creates_without_switching`

- [ ] **Step 6: Commit**

```bash
git add src/flows/start.rs src/main.rs tests/start_test.rs
git commit -m "feat: add no_checkout support to start_work_branch"
```

---

### Task 5: Add `no_checkout` to `start_release_fix`

**Files:**
- Modify: `src/flows/start.rs:24-34` (start_release_fix)
- Modify: `src/main.rs:99-101` (dispatch)

- [ ] **Step 1: Write failing tests**

Add to `tests/start_test.rs`:

```rust
#[test]
fn start_release_fix_no_checkout_discovers_release_branch() {
    let mut git = MockGit::new();
    git.current_branch = "develop".to_string();
    git.branches_matching = vec!["release/1.2".to_string()];

    start_release_fix(&git, "broken-login", true).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:release/*",
        "create_branch_no_checkout:release-fix/1.2/broken-login:release/1.2",
        "push:release-fix/1.2/broken-login",
    ]);
}

#[test]
fn start_release_fix_no_checkout_errors_when_no_release_branch() {
    let mut git = MockGit::new();
    git.branches_matching = vec![];

    let result = start_release_fix(&git, "broken-login", true);
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test start_release_fix_no_checkout`
Expected: compile error — `start_release_fix` takes 2 args, not 3

- [ ] **Step 3: Update `start_release_fix`**

In `src/flows/start.rs`:

```rust
pub fn start_release_fix(git: &dyn Git, name: &str, no_checkout: bool) -> Result<(), String> {
    let release_branch = if no_checkout {
        let branches = git.list_branches_matching("release/*")?;
        let release_branches: Vec<&String> = branches.iter()
            .filter(|b| b.starts_with("release/") && !b.starts_with("release-fix/"))
            .collect();
        release_branches.first()
            .ok_or("No release branch found. Create one with 'bflow start release' first.")?
            .to_string()
    } else {
        let current = git.current_branch()?;
        if current.strip_prefix("release/").is_none() {
            return Err("Not on a release branch".to_string());
        }
        current
    };

    let version = release_branch.strip_prefix("release/").unwrap();
    let branch = format!("release-fix/{version}/{name}");
    println!("Creating branch: {branch}");
    if no_checkout {
        git.create_branch_no_checkout(&branch, &release_branch)?;
    } else {
        git.create_branch(&branch, &release_branch)?;
    }
    git.push(&branch)?;
    println!("Branch '{branch}' created and pushed.");
    Ok(())
}
```

- [ ] **Step 4: Fix call sites**

In `src/main.rs` dispatch:

```rust
Action::StartReleaseFix { name, no_checkout } => {
    start::start_release_fix(git, &name, no_checkout)?;
}
```

In `tests/start_test.rs`, update existing test:

```rust
start_release_fix(&git, "broken-login", false).unwrap();
```

- [ ] **Step 5: Run tests**

Run: `cargo test start_release_fix`
Expected: all PASS

- [ ] **Step 6: Commit**

```bash
git add src/flows/start.rs src/main.rs tests/start_test.rs
git commit -m "feat: add no_checkout support to start_release_fix"
```

---

### Task 6: Add `no_checkout` to `start_hotfix_fix` and `resolve_or_create_hotfix`

**Files:**
- Modify: `src/flows/start.rs:36-98` (start_hotfix_fix, resolve_or_create_hotfix)
- Modify: `src/main.rs:102-104` (dispatch)

- [ ] **Step 1: Write failing tests**

Add to `tests/start_test.rs`:

```rust
#[test]
fn start_hotfix_fix_no_checkout_existing_hotfix() {
    let mut git = MockGit::new();
    git.branches_matching = vec!["hotfix/1.0.1".to_string()];

    start_hotfix_fix(&git, "urgent-crash", true).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:hotfix/*",
        "create_branch_no_checkout:hotfix-fix/1.0.1/urgent-crash:hotfix/1.0.1",
        "push:hotfix-fix/1.0.1/urgent-crash",
    ]);
}

#[test]
fn start_hotfix_fix_no_checkout_creates_hotfix_branch_when_none_exists() {
    let mut git = MockGit::new();
    git.branches_matching = vec![];
    git.tags = vec!["1.0.0".to_string()];

    start_hotfix_fix(&git, "urgent-crash", true).unwrap();

    assert_eq!(git.calls(), vec![
        "list_branches_matching:hotfix/*",
        "list_tags",
        "create_branch_no_checkout:hotfix/1.0.1:main",
        "push:hotfix/1.0.1",
        "create_branch_no_checkout:hotfix-fix/1.0.1/urgent-crash:hotfix/1.0.1",
        "push:hotfix-fix/1.0.1/urgent-crash",
    ]);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test start_hotfix_fix_no_checkout`
Expected: compile error — `start_hotfix_fix` takes 2 args, not 3

- [ ] **Step 3: Update `resolve_or_create_hotfix`**

In `src/flows/start.rs`:

```rust
fn resolve_or_create_hotfix(git: &dyn Git, no_checkout: bool) -> Result<String, String> {
    let branches = git.list_branches_matching("hotfix/*")?;
    let hotfix_branches: Vec<&String> = branches.iter()
        .filter(|b| b.starts_with("hotfix/") && !b.starts_with("hotfix-fix/"))
        .collect();

    if let Some(branch) = hotfix_branches.first() {
        println!("Using existing hotfix branch: {branch}");
        if !no_checkout {
            git.checkout(branch)?;
        }
        return Ok(branch.to_string());
    }

    let latest = find_latest_tag(git)?;
    let next = latest.bump_patch();
    let branch = next.hotfix_branch();

    println!("Creating hotfix branch: {branch}");
    if no_checkout {
        git.create_branch_no_checkout(&branch, "main")?;
    } else {
        git.checkout("main")?;
        git.create_branch(&branch, "main")?;
    }
    git.push(&branch)?;

    Ok(branch)
}
```

- [ ] **Step 4: Update `start_hotfix_fix`**

In `src/flows/start.rs`:

```rust
pub fn start_hotfix_fix(git: &dyn Git, name: &str, no_checkout: bool) -> Result<(), String> {
    let hotfix_branch = resolve_or_create_hotfix(git, no_checkout)?;
    let version = hotfix_branch.strip_prefix("hotfix/").unwrap();
    let branch = format!("hotfix-fix/{version}/{name}");
    println!("Creating branch: {branch}");
    if no_checkout {
        git.create_branch_no_checkout(&branch, &hotfix_branch)?;
    } else {
        git.create_branch(&branch, &hotfix_branch)?;
    }
    git.push(&branch)?;
    println!("Branch '{branch}' created and pushed.");
    Ok(())
}
```

- [ ] **Step 5: Fix call sites**

In `src/main.rs` dispatch:

```rust
Action::StartHotfixFix { name, no_checkout } => {
    start::start_hotfix_fix(git, &name, no_checkout)?;
}
```

In `tests/start_test.rs`, update existing tests to pass `false`:

```rust
start_hotfix_fix(&git, "urgent-crash", false).unwrap();
```

(Both `start_hotfix_fix_creates_and_pushes_existing_hotfix` and `start_hotfix_fix_creates_hotfix_branch_when_none_exists`)

- [ ] **Step 6: Run all tests**

Run: `cargo test`
Expected: all PASS

- [ ] **Step 7: Commit**

```bash
git add src/flows/start.rs src/main.rs tests/start_test.rs
git commit -m "feat: add no_checkout support to start_hotfix_fix"
```

---

### Task 7: Skip stash and merge in main.rs when no_checkout is true

**Files:**
- Modify: `src/main.rs:31-90` (run and run_flow functions)

- [ ] **Step 1: Update `run()` to check no_checkout before stashing**

The challenge: `no_checkout` lives inside the `Action`, which is resolved inside `run_flow()`. We need to restructure slightly so the action is resolved before the stash decision. Move action resolution earlier.

Refactor `run()` in `src/main.rs`:

```rust
fn run(command: Option<Commands>) -> Result<(), String> {
    check_command_exists("git")?;
    check_command_exists("gh")?;

    let git = GitCli::new();
    let hosting = GitHub::new();

    hosting.check_auth().map_err(|e| {
        format!("GitHub CLI is not authenticated. Run 'gh auth login' first.\n{e}")
    })?;

    let branch_name = git.current_branch().map_err(|_| {
        "Not in a git repository.".to_string()
    })?;

    println!("Fetching latest...");
    git.fetch()?;

    let branch_type = BranchType::parse(&branch_name);

    let action = match command {
        None => menu::show_menu(&branch_type, &branch_name)?,
        Some(cmd) => resolve_action(cmd, &branch_type)?,
    };

    let no_checkout = action.no_checkout();

    let stashed = if !no_checkout && branch_type != BranchType::Other && !git.is_working_tree_clean()? {
        println!("Stashing uncommitted changes...");
        git.stash_push()?;
        true
    } else {
        false
    };

    let result = run_flow(&git, &hosting, &branch_type, &branch_name, action, stashed, no_checkout);

    if stashed {
        println!("Restoring uncommitted changes...");
        if let Err(e) = git.stash_pop() {
            eprintln!("Warning: Failed to restore stashed changes: {e}");
            eprintln!("Your changes are saved in git stash. Run 'git stash pop' to restore them.");
        }
    }

    result
}
```

- [ ] **Step 2: Update `run_flow()` signature and skip merge when no_checkout**

```rust
fn run_flow(
    git: &GitCli,
    hosting: &GitHub,
    branch_type: &BranchType,
    branch_name: &str,
    action: Action,
    stashed: bool,
    no_checkout: bool,
) -> Result<(), String> {
    if !no_checkout {
        git.merge(&format!("origin/{branch_name}"), &format!("chore: pull latest {branch_name}"))?;
    }

    if stashed && !action.is_start() {
        return Err("Working tree is not clean. Commit your changes before finishing.".to_string());
    }

    match action {
        Action::StartWorkBranch { prefix, name, from, no_checkout } => {
            start::start_work_branch(git, &prefix, &name, &from, no_checkout)?;
        }
        Action::StartRelease => {
            start::start_release(git)?;
        }
        Action::StartReleaseFix { name, no_checkout } => {
            start::start_release_fix(git, &name, no_checkout)?;
        }
        Action::StartHotfixFix { name, no_checkout } => {
            start::start_hotfix_fix(git, &name, no_checkout)?;
        }
        Action::FinishWorkBranch => {
            finish_work::finish_work_branch(git, hosting, branch_type)?;
        }
        Action::FinishReleaseFix => {
            let BranchType::ReleaseFix { major, minor, name, .. } = branch_type else {
                unreachable!("FinishReleaseFix action only from ReleaseFix branch");
            };
            finish_work::finish_release_fix(git, hosting, *major, *minor, name)?;
        }
        Action::FinishHotfixFix => {
            let BranchType::HotfixFix { major, minor, patch, name, .. } = branch_type else {
                unreachable!("FinishHotfixFix action only from HotfixFix branch");
            };
            finish_work::finish_hotfix_fix(git, hosting, *major, *minor, *patch, name)?;
        }
        Action::BumpVersion => {
            let BranchType::Release { major, minor } = branch_type else {
                unreachable!("BumpVersion action only from Release branch");
            };
            finish_release::bump_version(git, *major, *minor)?;
        }
        Action::SyncWithDevelop => {
            let BranchType::Release { major, minor } = branch_type else {
                unreachable!("SyncWithDevelop action only from Release branch");
            };
            finish_release::sync_with_develop(git, *major, *minor)?;
        }
        Action::FinishRelease => {
            let BranchType::Release { major, minor } = branch_type else {
                unreachable!("FinishRelease action only from Release branch");
            };
            finish_release::finish_release(git, *major, *minor)?;
        }
        Action::FinishHotfix => {
            let BranchType::Hotfix { major, minor, patch } = branch_type else {
                unreachable!("FinishHotfix action only from Hotfix branch");
            };
            finish_hotfix::finish_hotfix(git, *major, *minor, *patch)?;
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: all PASS

- [ ] **Step 4: Manually verify CLI parsing**

Run: `cargo run -- start feature --name test-branch --no-checkout --help` (should show `--no-checkout` in help)
Run: `cargo run -- start release --help` (should NOT show `--no-checkout`)

- [ ] **Step 5: Commit**

```bash
git add src/main.rs
git commit -m "feat: skip stash and merge when --no-checkout is set"
```

---

### Task 8: Update documentation

**Files:**
- Modify: `README.md`
- Modify: `.claude/skills/bflow/skill.md`

- [ ] **Step 1: Read current docs to understand structure**

Read `README.md` and `.claude/skills/bflow/skill.md` to understand what sections need updating.

- [ ] **Step 2: Update README.md**

Add `--no-checkout` to the usage examples for start commands. Add a brief section explaining the flag and its worktree use case.

- [ ] **Step 3: Update skill.md**

Add `--no-checkout` flag documentation to the bflow skill.

- [ ] **Step 4: Commit**

```bash
git add README.md .claude/skills/bflow/skill.md
git commit -m "docs: document --no-checkout flag for bflow start"
```

---

### Task 9: Final verification

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: all tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: no warnings

- [ ] **Step 3: Build release binary**

Run: `cargo build --release`
Expected: builds successfully

- [ ] **Step 4: Verify help output**

Run: `cargo run -- start feature --help`
Expected: shows `--no-checkout` flag in output

Run: `cargo run -- start release --help`
Expected: does NOT show `--no-checkout` flag
