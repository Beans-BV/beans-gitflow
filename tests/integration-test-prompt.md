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
- **Auto-dispatch**: On work branches (feature/fix/chore/docs/refactor), release-fix, and hotfix-fix branches, bflow auto-executes the finish action with no menu — just run `bflow` and it goes.

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

---

## Phase 1: Setup Playground Repository

```bash
# Create a temporary playground repo
cd /tmp
mkdir bflow-playground && cd bflow-playground
git init
git checkout -b main

# Create initial structure
echo "# bflow playground" > README.md
git add README.md
git commit -m "chore: initial commit"

# Create a GitHub repo (change org as needed)
gh repo create Beans-BV/bflow-playground --private --source=. --push

# Tag initial release
git tag -a 1.0.0 -m "chore: initial release 1.0.0"
git push origin 1.0.0

# Create develop branch
git checkout -b develop
git push -u origin develop
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
3. Run `bflow` again (auto-finishes, creates PR)
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
→ Auto-dispatches: creates PR "feat: user-auth" → develop
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
→ Auto-dispatches: creates PR "fix: null-pointer" → develop
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
→ Auto-dispatches: creates PR "chore: update-deps" → develop
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
→ Auto-dispatches: creates PR "docs: api-guide" → develop
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
→ Auto-dispatches: creates PR "refactor: clean-models" → develop
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
- All 5 work branch PRs (squash-merged)

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

**Expected PRs (7 total):**

| # | Title | Target |
|---|-------|--------|
| 1 | feat: user-auth | develop |
| 2 | fix: null-pointer | develop |
| 3 | chore: update-deps | develop |
| 4 | docs: api-guide | develop |
| 5 | refactor: clean-models | develop |
| 6 | fix: payment-bug | release/1.1 |
| 7 | fix: validation-error | release/1.1 |
| 8 | fix: critical-crash | hotfix/1.1.3 |

---

## Phase 6: Cleanup

The playground repository must be completely destroyed — no traces left locally or on GitHub.

```bash
# 1. Delete the GitHub repository (removes all remote branches, tags, PRs)
gh repo delete Beans-BV/bflow-playground --yes

# 2. Remove the local clone
cd /tmp
rm -rf bflow-playground
```

**Verify cleanup:**
```bash
# Repo should no longer exist
gh repo view Beans-BV/bflow-playground 2>&1 | grep -q "Could not resolve" && echo "✅ Remote repo deleted" || echo "❌ Remote repo still exists"

# Local directory should be gone
[ ! -d /tmp/bflow-playground ] && echo "✅ Local directory deleted" || echo "❌ Local directory still exists"
```

After cleanup, the state of your machine and GitHub org should be **identical to before the test started** — no leftover repos, branches, tags, or PRs.

---

## Summary of Flows Tested

| # | Flow | Branch | Action |
|---|------|--------|--------|
| 1 | Feature | feature/user-auth | start → commit → finish (PR) → merge |
| 2 | Fix | fix/null-pointer | start → commit → finish (PR) → merge |
| 3 | Chore | chore/update-deps | start → commit → finish (PR) → merge |
| 4 | Docs | docs/api-guide | start → commit → finish (PR) → merge |
| 5 | Refactor | refactor/clean-models | start → commit → finish (PR) → merge |
| 6 | Release fix 1 | release-fix/1.1/payment-bug | start (auto-creates release/1.1 + tag 1.1.0) → commit → finish (PR) → merge |
| 7 | Bump version | release/1.1 | 1.1.0 → 1.1.1 |
| 8 | Sync develop | release/1.1 | merge release into develop |
| 9 | Release fix 2 | release-fix/1.1/validation-error | start (detects existing release/1.1) → commit → finish (PR) → merge |
| 10 | Bump version | release/1.1 | 1.1.1 → 1.1.2 |
| 11 | Finish release | release/1.1 | merge → main + develop, delete branch |
| 12 | Hotfix fix | hotfix-fix/1.1.3/critical-crash | start (auto-creates hotfix/1.1.3) → commit → finish (PR) → merge |
| 13 | Finish hotfix | hotfix/1.1.3 | merge → main + develop, tag 1.1.3, delete branch |

**Total: 13 operations covering 100% of bflow's functionality.**
