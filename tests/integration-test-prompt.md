# bflow Integration Test Prompt

> **For AI agents with terminal access (mcp-terminator or similar).**
> This prompt exercises every bflow flow end-to-end on a playground repository, then verifies the git history matches expectations.

## Prerequisites

- `bflow` is installed and available in PATH
- `git` and `gh` are installed and authenticated
- You have permission to create repositories in the `Beans-BV` GitHub org (or change the org below)

## Important: How bflow Interacts

bflow uses interactive terminal menus. Here's how to interact:

- **Select menus**: Use arrow keys (↓/↑) to navigate, Enter to select. Default is always the first item (index 0).
- **Text input**: Type the name, press Enter.
- **Auto-dispatch**: On release-fix and hotfix-fix branches, bflow auto-executes the finish action with no menu — just run `bflow` and it goes.
- **Work branches**: On work branches (feature/fix/chore/docs/refactor), bflow shows a menu with finish as default (index 0) plus start options. Pressing Enter finishes the branch. A PR target selection prompt follows.

### Menu indices reference

**On `develop`** (6 options):
| Index | Option |
|-------|--------|
| 0 | start feature |
| 1 | start fix |
| 2 | start chore |
| 3 | start docs |
| 4 | start refactor |
| 5 | start release fix |

**On `release/{v}`** (3 options):
| Index | Option |
|-------|--------|
| 0 | bump version |
| 1 | sync with develop |
| 2 | finish release |

**On `main`** (1 option):
| Index | Option |
|-------|--------|
| 0 | start hotfix fix |

**On work branches** (feature/fix/chore/docs/refactor) (6 options):
| Index | Option |
|-------|--------|
| 0 | finish {type} |
| 1 | start feature |
| 2 | start fix |
| 3 | start chore |
| 4 | start docs |
| 5 | start refactor |

---

## Phase 1: Setup Playground Repository

The playground repo already exists at `Beans-BV/bflow-playground` with its `.mcp.json` and any other config files. Before starting the test, record the initial state so cleanup can restore it exactly.

```bash
# Record the initial commit SHA (the reset target after the test)
INITIAL_SHA=$(git rev-parse HEAD)
echo "Initial SHA: $INITIAL_SHA"

# Record existing tags (so we only delete tags created by the test)
git tag --list > /tmp/bflow-pre-test-tags.txt

# Record existing branches
git branch -a > /tmp/bflow-pre-test-branches.txt

# Make sure we're on main and clean
git checkout main
git pull

# Ensure develop branch exists
git checkout -b develop 2>/dev/null || git checkout develop
git push -u origin develop 2>/dev/null || true

# Create initial tag if it doesn't exist yet
if ! git tag --list | grep -q "^1.0.0$"; then
  git tag -a 1.0.0 -m "chore: initial release 1.0.0"
  git push origin 1.0.0
fi

# Start on develop
git checkout develop
```

**Verify before continuing:**
- On branch `develop`
- Remote `origin` points to `Beans-BV/bflow-playground`
- Tag `1.0.0` exists

---

## Phase 2: Work Branch Flows

Each work branch follows the same pattern:
1. On `develop`: run `bflow` → select the branch type → enter a name
2. Make a dummy commit on the new branch
3. Run `bflow` again → select "finish {type}" (index 0) → select PR target "develop" (index 0)
4. Merge the PR via `gh`
5. Return to `develop` and pull

### Step 2.1: Feature

```
# On develop
bflow
→ Select: "start feature" (index 0, just press Enter)
→ Input: "user-auth"
```

bflow creates `feature/user-auth` and you're now on it.

```bash
echo "auth code" > auth.rs
git add auth.rs
git commit -m "feat: add user authentication"
```

```
bflow
→ Select: "finish feature" (index 0, just press Enter)
→ PR target: select "develop" (index 0, just press Enter)
```

```bash
gh pr merge --squash --delete-branch
git checkout develop
git pull
```

### Step 2.2: Fix

```
bflow
→ Select: "start fix" (index 1, press ↓ once then Enter)
→ Input: "null-pointer"
```

```bash
echo "fix code" > fix.rs
git add fix.rs
git commit -m "fix: resolve null pointer exception"
```

```
bflow
→ Select: "finish fix" (index 0, just press Enter)
→ PR target: select "develop" (index 0, just press Enter)
```

```bash
gh pr merge --squash --delete-branch
git checkout develop
git pull
```

### Step 2.3: Chore

```
bflow
→ Select: "start chore" (index 2, press ↓ twice then Enter)
→ Input: "update-deps"
```

```bash
echo "updated deps" > deps.txt
git add deps.txt
git commit -m "chore: update dependencies"
```

```
bflow
→ Select: "finish chore" (index 0, just press Enter)
→ PR target: select "develop" (index 0, just press Enter)
```

```bash
gh pr merge --squash --delete-branch
git checkout develop
git pull
```

### Step 2.4: Docs

```
bflow
→ Select: "start docs" (index 3, press ↓ three times then Enter)
→ Input: "api-guide"
```

```bash
echo "API documentation" > api-guide.md
git add api-guide.md
git commit -m "docs: add API guide"
```

```
bflow
→ Select: "finish docs" (index 0, just press Enter)
→ PR target: select "develop" (index 0, just press Enter)
```

```bash
gh pr merge --squash --delete-branch
git checkout develop
git pull
```

### Step 2.5: Refactor

```
bflow
→ Select: "start refactor" (index 4, press ↓ four times then Enter)
→ Input: "clean-models"
```

```bash
echo "cleaned models" > models.rs
git add models.rs
git commit -m "refactor: clean up data models"
```

```
bflow
→ Select: "finish refactor" (index 0, just press Enter)
→ PR target: select "develop" (index 0, just press Enter)
```

```bash
gh pr merge --squash --delete-branch
git checkout develop
git pull
```

---

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

---

## Phase 3: Release Flow

### Step 3.1: Start First Release Fix

```
# On develop
bflow
→ Select: "start release fix" (index 5, press ↓ five times then Enter)
→ (bflow auto-creates release/1.1 from develop, tags 1.1.0)
→ Input: "payment-bug"
```

bflow creates `release-fix/1.1/payment-bug`. You're now on it.

```bash
echo "payment fix" > payment.rs
git add payment.rs
git commit -m "fix: resolve payment processing error"
```

```
bflow
→ Auto-dispatches: creates PR "fix: payment-bug" → release/1.1
```

```bash
gh pr merge --squash --delete-branch
git checkout release/1.1
git pull
```

### Step 3.2: Bump Version (first bump)

```
# On release/1.1
bflow
→ Select: "bump version" (index 0, just press Enter)
→ (bflow auto-bumps 1.1.0 → 1.1.1, creates and pushes tag)
```

### Step 3.3: Sync with Develop

```
# Still on release/1.1
bflow
→ Select: "sync with develop" (index 1, press ↓ once then Enter)
→ (bflow merges release/1.1 into develop, pushes, returns to release/1.1)
```

### Step 3.4: Start Second Release Fix

```bash
git checkout develop
git pull
```

```
bflow
→ Select: "start release fix" (index 5, press ↓ five times then Enter)
→ (bflow detects existing release/1.1, uses it)
→ Input: "validation-error"
```

```bash
echo "validation fix" > validation.rs
git add validation.rs
git commit -m "fix: correct input validation"
```

```
bflow
→ Auto-dispatches: creates PR "fix: validation-error" → release/1.1
```

```bash
gh pr merge --squash --delete-branch
git checkout release/1.1
git pull
```

### Step 3.5: Bump Version (second bump)

```
# On release/1.1
bflow
→ Select: "bump version" (index 0, just press Enter)
→ (bflow auto-bumps 1.1.1 → 1.1.2, creates and pushes tag)
```

### Step 3.6: Finish Release

```
# On release/1.1
bflow
→ Select: "finish release" (index 2, press ↓ twice then Enter)
→ (bflow merges into main, merges into develop, deletes release/1.1)
```

You should now be on `develop`.

---

## Phase 4: Hotfix Flow

### Step 4.1: Start Hotfix Fix

```bash
git checkout main
git pull
```

```
bflow
→ Select: "start hotfix fix" (index 0, just press Enter)
→ (bflow auto-creates hotfix/1.1.3 from main, bumps patch from 1.1.2)
→ Input: "critical-crash"
```

bflow creates `hotfix-fix/1.1.3/critical-crash`. You're now on it.

```bash
echo "crash fix" > crash-fix.rs
git add crash-fix.rs
git commit -m "fix: resolve critical crash on startup"
```

```
bflow
→ Auto-dispatches: creates PR "fix: critical-crash" → hotfix/1.1.3
```

```bash
gh pr merge --squash --delete-branch
git checkout hotfix/1.1.3
git pull
```

### Step 4.2: Finish Hotfix

```
# On hotfix/1.1.3
bflow
→ Select: "finish hotfix" (index 0, just press Enter)
→ (bflow merges into main, tags 1.1.3, merges into develop, deletes hotfix/1.1.3)
```

---

## Phase 5: Verification

Run the following checks and verify each one matches the expected state.

### 5.1: Verify Tags

```bash
git tag --list --sort=version:refname
```

**Expected:**
```
1.0.0
1.1.0
1.1.1
1.1.2
1.1.3
```

### 5.2: Verify No Leftover Branches

```bash
git branch -a
```

**Expected branches (only):**
```
* develop  (or main, depending on where finish hotfix left you)
  main
  remotes/origin/develop
  remotes/origin/main
```

No `release/*`, `hotfix/*`, `feature/*`, `fix/*`, `chore/*`, `docs/*`, `refactor/*`, or `release-fix/*` branches should remain.

### 5.3: Verify Main Branch History

```bash
git log main --oneline --graph
```

**Expected merge commits on main (most recent first):**
```
- chore: finish hotfix 1.1.3
- chore: finish release 1.1
- chore: initial commit
```

**Expected tags on main:**
- `1.1.3` (from hotfix finish)
- `1.1.2` (latest release tag, reachable from main)
- `1.0.0` (initial)

### 5.4: Verify Tag Reachability

```bash
# All tags should be reachable from main
git tag --merged main --sort=version:refname
```

**Expected:** All 5 tags (1.0.0, 1.1.0, 1.1.1, 1.1.2, 1.1.3)

### 5.5: Verify Develop Contains All Work

```bash
git log develop --oneline | head -20
```

**Expected:** Develop should contain:
- The hotfix merge (`chore: merge hotfix 1.1.3 into develop`)
- The release merge (`chore: merge release 1.1 into develop`)
- The sync merge (`chore: sync release 1.1 with develop`)
- All 5 work branch PRs (squash-merged) plus child work branch PRs

### 5.6: Verify Conventional Commit Compliance

```bash
# Check that all merge commits follow conventional commits
git log main --merges --format="%s"
```

**Expected:** Every merge commit starts with `chore:`, `feat:`, `fix:`, `docs:`, or `refactor:`.

### 5.7: Verify PR History

```bash
gh pr list --state merged --json number,title,baseRefName --jq '.[] | "\(.number) \(.title) → \(.baseRefName)"'
```

**Expected PRs (10 total):**

| # | Title | Target |
|---|-------|--------|
| 1 | feat: user-auth | develop |
| 2 | fix: null-pointer | develop |
| 3 | chore: update-deps | develop |
| 4 | docs: api-guide | develop |
| 5 | refactor: clean-models | develop |
| 6 | fix: payment-validation | feature/payment-system |
| 7 | feat: payment-system | develop |
| 8 | fix: payment-bug | release/1.1 |
| 9 | fix: validation-error | release/1.1 |
| 10 | fix: critical-crash | hotfix/1.1.3 |

---

## Phase 6: Cleanup

Reset the playground repo to its exact pre-test state. The repo itself stays — only branches, tags, commits, and PRs created by the test are removed.

```bash
# 1. Close any open PRs created by the test
gh pr list --state open --json number --jq '.[].number' | while read pr; do
  gh pr close "$pr" --delete-branch 2>/dev/null || true
done

# 2. Delete all remote branches except main and develop
git branch -r | grep -v 'origin/main' | grep -v 'origin/develop' | grep -v 'HEAD' | sed 's|origin/||' | while read branch; do
  # Only delete branches that weren't there before the test
  if ! grep -q "$branch" /tmp/bflow-pre-test-branches.txt 2>/dev/null; then
    git push origin --delete "$branch" 2>/dev/null || true
  fi
done

# 3. Delete all local branches except main and develop
git checkout main
git branch | grep -v 'main' | grep -v 'develop' | xargs -r git branch -D 2>/dev/null || true

# 4. Delete tags created by the test (remote + local)
git tag --list | while read tag; do
  if ! grep -q "^${tag}$" /tmp/bflow-pre-test-tags.txt 2>/dev/null; then
    git push origin --delete "$tag" 2>/dev/null || true
    git tag -d "$tag" 2>/dev/null || true
  fi
done

# 5. Reset main to the initial commit
git checkout main
git reset --hard "$INITIAL_SHA"
git push --force origin main

# 6. Reset develop to match main
git checkout develop
git reset --hard "$INITIAL_SHA"
git push --force origin develop

# 7. Clean up temp files
rm -f /tmp/bflow-pre-test-tags.txt /tmp/bflow-pre-test-branches.txt
```

**Verify cleanup:**
```bash
# Only main and develop should exist
echo "=== Branches ==="
git branch -a

# Only pre-existing tags should remain
echo "=== Tags ==="
git tag --list

# Commit history should match initial state
echo "=== History ==="
git log --oneline --all

# No open PRs
echo "=== Open PRs ==="
gh pr list --state open
```

**Expected:** The repo looks exactly as it did before Phase 1 — same branches, same tags, same commit history, no open PRs.

---

## Summary of Flows Tested

| # | Flow | Branch | Action |
|---|------|--------|--------|
| 1 | Feature | feature/user-auth | start → commit → finish (PR) → merge |
| 2 | Fix | fix/null-pointer | start → commit → finish (PR) → merge |
| 3 | Chore | chore/update-deps | start → commit → finish (PR) → merge |
| 4 | Docs | docs/api-guide | start → commit → finish (PR) → merge |
| 5 | Refactor | refactor/clean-models | start → commit → finish (PR) → merge |
| 5.5 | Child fix | fix/payment-validation | start from feature/payment-system → commit → finish (PR → feature/payment-system) → merge |
| 5.6 | Parent feature finish | feature/payment-system | finish (PR → develop) → merge |
| 6 | Release fix 1 | release-fix/1.1/payment-bug | start (auto-creates release/1.1 + tag 1.1.0) → commit → finish (PR) → merge |
| 7 | Bump version | release/1.1 | 1.1.0 → 1.1.1 |
| 8 | Sync develop | release/1.1 | merge release into develop |
| 9 | Release fix 2 | release-fix/1.1/validation-error | start (detects existing release/1.1) → commit → finish (PR) → merge |
| 10 | Bump version | release/1.1 | 1.1.1 → 1.1.2 |
| 11 | Finish release | release/1.1 | merge → main + develop, delete branch |
| 12 | Hotfix fix | hotfix-fix/1.1.3/critical-crash | start (auto-creates hotfix/1.1.3) → commit → finish (PR) → merge |
| 13 | Finish hotfix | hotfix/1.1.3 | merge → main + develop, tag 1.1.3, delete branch |

**Total: 15 operations covering 100% of bflow's functionality.**
