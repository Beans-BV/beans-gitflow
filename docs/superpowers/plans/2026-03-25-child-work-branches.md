# Child Work Branches Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Allow creating work branches from other work branches, with automatic parent detection for PR targeting.

**Architecture:** Add three Git trait methods for merge-base analysis. Change work branch menu from auto-dispatch to a menu with finish (default) + start options. Detect the PR target at finish time using commit distance, confirmed via select menu.

**Tech Stack:** Rust, clap, dialoguer, git CLI, gh CLI

**Spec:** `docs/superpowers/specs/2026-03-25-child-work-branches-design.md`

---

### Task 1: Add Git trait methods

**Files:**
- Modify: `src/git/mod.rs`

- [ ] **Step 1: Add three new methods to the `Git` trait**

In `src/git/mod.rs`, add these methods to the `Git` trait (after `tags_on_branch` on line 21):

```rust
fn list_remote_branches(&self) -> Result<Vec<String>>;
fn merge_base(&self, a: &str, b: &str) -> Result<String>;
fn rev_list_count(&self, from: &str, to: &str) -> Result<u32>;
```

- [ ] **Step 2: Implement the three methods on `GitCli`**

In the `impl Git for GitCli` block (after `tags_on_branch` impl ending at line 78), add:

```rust
fn list_remote_branches(&self) -> Result<Vec<String>> {
    let output = self.run(&["for-each-ref", "--format=%(refname:short)", "refs/remotes/origin/"])?;
    Ok(output
        .lines()
        .map(|s| s.trim_start_matches("origin/").to_string())
        .filter(|s| !s.is_empty() && s != "HEAD")
        .collect())
}

fn merge_base(&self, a: &str, b: &str) -> Result<String> {
    self.run(&["merge-base", a, b])
}

fn rev_list_count(&self, from: &str, to: &str) -> Result<u32> {
    let range = format!("{from}..{to}");
    let output = self.run(&["rev-list", "--count", &range])?;
    output.parse::<u32>().map_err(|e| format!("Failed to parse rev-list count: {e}"))
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo build 2>&1`
Expected: compiles successfully

- [ ] **Step 4: Commit**

```bash
git add src/git/mod.rs
git commit -m "feat: add list_remote_branches, merge_base, rev_list_count to Git trait"
```

---

### Task 2: Add parent detection logic

**Files:**
- Modify: `src/flows/finish_work.rs`

- [ ] **Step 1: Add `use crate::menu;` import**

In `src/flows/finish_work.rs`, add the import after the existing imports (line 3):

```rust
use crate::menu;
```

The imports should now be:

```rust
use crate::git::Git;
use crate::git::branch::BranchType;
use crate::hosting::HostingPlatform;
use crate::menu;
```

- [ ] **Step 2: Add the `detect_parent_branch` function**

Add this function to `src/flows/finish_work.rs` (after the `push_and_create_pr` function, before `finish_work_branch`):

```rust
fn detect_parent_branch(git: &dyn Git, current: &str) -> Result<String, String> {
    let work_prefixes = ["feature/", "fix/", "chore/", "docs/", "refactor/"];

    let remote_branches = git.list_remote_branches()?;
    let mut candidates: Vec<(String, u32)> = Vec::new();

    for branch in &remote_branches {
        if branch == current {
            continue;
        }
        let is_work = work_prefixes.iter().any(|p| branch.starts_with(p));
        let is_develop = branch == "develop";
        if !is_work && !is_develop {
            continue;
        }
        let base = match git.merge_base(current, branch) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let count = match git.rev_list_count(&base, current) {
            Ok(c) => c,
            Err(_) => continue,
        };
        candidates.push((branch.clone(), count));
    }

    if candidates.is_empty() {
        return Ok("develop".to_string());
    }

    // Sort by distance ascending; on ties prefer develop, then alphabetical
    candidates.sort_by(|a, b| {
        a.1.cmp(&b.1)
            .then_with(|| {
                let a_is_develop = a.0 == "develop";
                let b_is_develop = b.0 == "develop";
                b_is_develop.cmp(&a_is_develop)
            })
            .then_with(|| a.0.cmp(&b.0))
    });

    let labels: Vec<&str> = candidates.iter().map(|(name, _)| name.as_str()).collect();
    let idx = menu::show_select("PR target branch", &labels)?;
    Ok(candidates[idx].0.clone())
}
```

- [ ] **Step 3: Update `finish_work_branch` to use `detect_parent_branch`**

Replace the current `finish_work_branch` function (lines 19-23):

```rust
pub fn finish_work_branch(git: &dyn Git, hosting: &dyn HostingPlatform, branch_type: &BranchType) -> Result<(), String> {
    let commit_type = branch_type.commit_type().ok_or("Cannot finish: not on a work branch")?;
    let name = branch_type.name().ok_or("Cannot finish: branch has no name")?;
    let current = git.current_branch()?;
    let base = detect_parent_branch(git, &current)?;
    push_and_create_pr(git, hosting, &base, &format!("{commit_type}: {name}"))
}
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo build 2>&1`
Expected: compiles successfully

- [ ] **Step 5: Commit**

```bash
git add src/flows/finish_work.rs
git commit -m "feat: detect parent branch via merge-base for PR targeting"
```

---

### Task 3: Add work branch menu, update `start_work_branch`, and wire up `main.rs`

This task adds the work branch menu, the `from` field to `Action::StartWorkBranch`, updates `show_menu` to accept the current branch name, updates `start_work_branch` to accept `from`, and updates `main.rs` — all in one task so every commit compiles.

**Files:**
- Modify: `src/menu.rs`
- Modify: `src/flows/start.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Add `WorkBranchOption` enum to `src/menu.rs`**

Add after the `DevelopOption` impl block (after line 33):

```rust
#[derive(Debug, Clone, Copy)]
pub enum WorkBranchOption {
    Finish, StartFeature, StartFix, StartChore, StartDocs, StartRefactor,
}

impl WorkBranchOption {
    pub fn label(&self, branch_type: &str) -> String {
        match self {
            Self::Finish => format!("finish {branch_type}"),
            Self::StartFeature => "start feature".to_string(),
            Self::StartFix => "start fix".to_string(),
            Self::StartChore => "start chore".to_string(),
            Self::StartDocs => "start docs".to_string(),
            Self::StartRefactor => "start refactor".to_string(),
        }
    }

    pub fn branch_prefix(&self) -> &'static str {
        match self {
            Self::StartFeature => "feature",
            Self::StartFix => "fix",
            Self::StartChore => "chore",
            Self::StartDocs => "docs",
            Self::StartRefactor => "refactor",
            Self::Finish => unreachable!(),
        }
    }

    const ALL: [Self; 6] = [Self::Finish, Self::StartFeature, Self::StartFix, Self::StartChore, Self::StartDocs, Self::StartRefactor];
}
```

- [ ] **Step 2: Add `from` field to `Action::StartWorkBranch`**

Change the `Action` enum (currently at line 117-129):

```rust
#[derive(Debug)]
pub enum Action {
    StartWorkBranch { prefix: String, name: String, from: String },
    StartReleaseFix,
    StartHotfixFix,
    FinishWorkBranch,
    FinishReleaseFix,
    FinishRelease,
    FinishHotfix,
    FinishHotfixFix,
    BumpVersion,
    SyncWithDevelop,
}
```

- [ ] **Step 3: Update `show_menu` signature and all match arms**

Change the `show_menu` signature (currently at line 78) to accept the current branch name:

```rust
pub fn show_menu(branch_type: &BranchType, current_branch: &str) -> Result<Action, String> {
```

Update the `BranchType::Develop` arm to pass `from: "develop"`:

```rust
BranchType::Develop => {
    let labels: Vec<&str> = DevelopOption::ALL.iter().map(|o| o.label()).collect();
    let idx = show_select("What would you like to do?", &labels)?;
    let option = DevelopOption::ALL[idx];
    match option {
        DevelopOption::StartReleaseFix => Ok(Action::StartReleaseFix),
        other => {
            let name = prompt_name(&format!("Name for {} branch", other.branch_prefix()))?;
            Ok(Action::StartWorkBranch { prefix: other.branch_prefix().to_string(), name, from: "develop".to_string() })
        }
    }
}
```

Replace the work branch auto-dispatch arm (currently lines 97-100) with:

```rust
BranchType::Feature { .. } | BranchType::Fix { .. } | BranchType::Chore { .. }
| BranchType::Docs { .. } | BranchType::Refactor { .. } => {
    let branch_type_label = match branch_type {
        BranchType::Feature { .. } => "feature",
        BranchType::Fix { .. } => "fix",
        BranchType::Chore { .. } => "chore",
        BranchType::Docs { .. } => "docs",
        BranchType::Refactor { .. } => "refactor",
        _ => unreachable!(),
    };
    let labels: Vec<String> = WorkBranchOption::ALL.iter().map(|o| o.label(branch_type_label)).collect();
    let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
    let idx = show_select("What would you like to do?", &label_refs)?;
    let option = WorkBranchOption::ALL[idx];
    match option {
        WorkBranchOption::Finish => Ok(Action::FinishWorkBranch),
        other => {
            let name = prompt_name(&format!("Name for {} branch", other.branch_prefix()))?;
            Ok(Action::StartWorkBranch { prefix: other.branch_prefix().to_string(), name, from: current_branch.to_string() })
        }
    }
}
```

- [ ] **Step 4: Update `start_work_branch` in `src/flows/start.rs`**

Change the function signature (line 5) to accept `from`:

```rust
pub fn start_work_branch(git: &dyn Git, prefix: &str, name: &str, from: &str) -> Result<(), String> {
    let branch = format!("{prefix}/{name}");
    println!("Creating branch: {branch}");
    git.create_branch(&branch, from)?;
    git.push(&branch)?;
    println!("Branch '{branch}' created and pushed.");
    Ok(())
}
```

- [ ] **Step 5: Update `main.rs`**

Update the `show_menu` call (line 51):

```rust
    let action = menu::show_menu(&branch_type, &branch_name)?;
```

Update the `StartWorkBranch` handler (lines 54-56):

```rust
        Action::StartWorkBranch { prefix, name, from } => {
            start::start_work_branch(&git, &prefix, &name, &from)?;
        }
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo build 2>&1`
Expected: compiles successfully

- [ ] **Step 7: Run all tests**

Run: `cargo test --all 2>&1`
Expected: all existing tests pass (branch parsing and version tests are unaffected)

- [ ] **Step 8: Commit**

```bash
git add src/menu.rs src/flows/start.rs src/main.rs
git commit -m "feat: add work branch menu with child branch support"
```

---

### Task 4: Update integration test prompt

**Files:**
- Modify: `tests/integration-test-prompt.md`

- [ ] **Step 1: Update the auto-dispatch note and menu reference**

In `tests/integration-test-prompt.md`, change the auto-dispatch note (line 18):

From:
```
- **Auto-dispatch**: On work branches (feature/fix/chore/docs/refactor), release-fix, and hotfix-fix branches, bflow auto-executes the finish action with no menu — just run `bflow` and it goes.
```

To:
```
- **Auto-dispatch**: On release-fix and hotfix-fix branches, bflow auto-executes the finish action with no menu — just run `bflow` and it goes.
- **Work branches**: On work branches (feature/fix/chore/docs/refactor), bflow shows a menu with finish as default (index 0) plus start options. Pressing Enter finishes the branch. A PR target selection prompt follows.
```

Add a new menu index table after the **On `main`** table (after line 43):

```markdown
**On work branches** (feature/fix/chore/docs/refactor) (6 options):
| Index | Option |
|-------|--------|
| 0 | finish {type} |
| 1 | start feature |
| 2 | start fix |
| 3 | start chore |
| 4 | start docs |
| 5 | start refactor |
```

- [ ] **Step 2: Update Phase 2 work branch finish steps**

Update all five work branch finish steps (2.1-2.5) to reflect the new menu and PR target prompt. Each `bflow` auto-dispatch comment should change. For example, Step 2.1 (feature) changes from:

```
bflow
→ Auto-dispatches: creates PR "feat: user-auth" → develop
```

To:

```
bflow
→ Select: "finish feature" (index 0, just press Enter)
→ PR target: select "develop" (index 0, just press Enter)
```

Apply the same pattern for Steps 2.2 (fix), 2.3 (chore), 2.4 (docs), and 2.5 (refactor), adjusting the branch type label accordingly.

- [ ] **Step 3: Add child work branch test scenario**

Add a new section after Phase 2 (Work Branch Flows), before Phase 3 (Release Flow). Insert as **Phase 2.5: Child Work Branch Flow**:

````markdown
## Phase 2.5: Child Work Branch Flow

This tests creating a work branch from another work branch and verifying the PR targets the parent.

### Step 2.5.1: Create Parent Feature

```
# On develop
bflow
→ Select: "start feature" (index 0, just press Enter)
→ Input: "payment-system"
```

```bash
echo "payment code" > payment.rs
git add payment.rs
git commit -m "feat: add payment system base"
```

### Step 2.5.2: Create Child Fix from Parent Feature

```
# On feature/payment-system
bflow
→ Select: "start fix" (index 2, press ↓ twice then Enter)
→ Input: "payment-validation"
```

bflow creates `fix/payment-validation` from `feature/payment-system`.

```bash
echo "validation fix" > payment-validation.rs
git add payment-validation.rs
git commit -m "fix: add payment validation"
```

### Step 2.5.3: Finish Child Branch

```
bflow
→ Select: "finish fix" (index 0, just press Enter)
→ PR target: verify "feature/payment-system" is the default (index 0), press Enter
```

**Verify:** The PR targets `feature/payment-system`, not `develop`.

```bash
gh pr view --json baseRefName --jq '.baseRefName'
```

**Expected:** `feature/payment-system`

```bash
gh pr merge --squash --delete-branch
git checkout feature/payment-system
git pull
```

### Step 2.5.4: Finish Parent Branch

```
bflow
→ Select: "finish feature" (index 0, just press Enter)
→ PR target: verify "develop" is the default (index 0), press Enter
```

```bash
gh pr merge --squash --delete-branch
git checkout develop
git pull
```
````

- [ ] **Step 4: Update the Phase 5.7 PR verification table**

Add the two new PRs to the expected PR list. The table should now include (adjust PR numbers to account for the two new PRs):

```markdown
| ... | fix: payment-validation | feature/payment-system |
| ... | feat: payment-system | develop |
```

- [ ] **Step 5: Update the Summary table**

Add the child work branch flow entries to the summary table at the bottom:

```markdown
| ... | Child fix | fix/payment-validation | start from feature/payment-system → commit → finish (PR → feature/payment-system) → merge |
| ... | Parent feature | feature/payment-system | finish (PR → develop) → merge |
```

Update the total count at the bottom accordingly.

- [ ] **Step 6: Commit**

```bash
git add tests/integration-test-prompt.md
git commit -m "docs: add child work branch scenario to integration tests"
```

---

## Task dependency order

```
Task 1 (Git trait methods)
    ↓
Task 2 (parent detection logic)
    ↓
Task 3 (menu + start + main wiring) ← depends on Tasks 1-2
    ↓
Task 4 (integration test update) ← depends on Task 3
```

All tasks are sequential. Each commit compiles and passes tests.
