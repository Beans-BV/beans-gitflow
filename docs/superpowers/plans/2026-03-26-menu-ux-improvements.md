# Menu UX Improvements Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace dialoguer with crossterm to add number-key instant menu selection and live space-to-hyphen conversion in branch name input.

**Architecture:** Two custom terminal widgets (`show_select` and `prompt_name`) built on crossterm's raw mode and event system. Both functions keep their existing signatures so no callers need to change. The `show_menu` function and all option enums remain untouched.

**Tech Stack:** Rust, crossterm 0.28 (raw terminal input, cursor control, styled output)

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `Cargo.toml` | Modify | Swap `dialoguer` for `crossterm` |
| `src/menu.rs` | Modify | Replace `show_select` and `prompt_name` internals; add `render_menu` helper |
| `tests/integration-test-prompt.md` | Modify | Update interaction docs for number keys and space-to-hyphen |

No new files. No changes to `src/main.rs`, `src/flows/*`, `src/git/*`, `src/hosting/*`, or existing test files.

---

### Task 1: Swap dependencies in Cargo.toml

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Update Cargo.toml**

Replace the `dialoguer` dependency with `crossterm`:

```toml
[package]
name = "bflow"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
crossterm = "0.28"
```

- [ ] **Step 2: Verify it compiles (expect errors)**

Run: `cargo check 2>&1 | head -20`

Expected: Compilation errors in `src/menu.rs` because `dialoguer` imports no longer resolve. This confirms the dependency swap worked and we need to update menu.rs next.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: replace dialoguer with crossterm dependency"
```

---

### Task 2: Implement custom `show_select` with number-key support

**Files:**
- Modify: `src/menu.rs:1` (replace import line)
- Modify: `src/menu.rs:83-90` (replace `show_select` function)

- [ ] **Step 1: Replace imports at top of menu.rs**

Replace line 1:
```rust
use dialoguer::{Select, Input, theme::ColorfulTheme};
```

With:
```rust
use std::io::{self, Write};
use crossterm::{
    cursor, event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute, queue,
    style::{self, Stylize},
    terminal,
};
```

- [ ] **Step 2: Add render_menu helper function**

Add this function right before `show_select` (before line 83):

```rust
fn render_menu(out: &mut io::Stderr, items: &[&str], selected: usize) -> io::Result<()> {
    for (i, item) in items.iter().enumerate() {
        let number = i + 1;
        if i == selected {
            queue!(
                out,
                style::PrintStyledContent(format!("> {number}) {item}").cyan().bold()),
            )?;
        } else {
            queue!(
                out,
                style::PrintStyledContent(format!("  {number}) {item}").dim()),
            )?;
        }
        if i < items.len() - 1 {
            queue!(out, cursor::MoveToNextLine(1))?;
        }
    }
    out.flush()?;
    Ok(())
}
```

- [ ] **Step 3: Replace show_select function**

Replace the entire `show_select` function (lines 83-90) with:

```rust
pub fn show_select(prompt: &str, items: &[&str]) -> Result<usize, String> {
    let mut out = io::stderr();
    let mut selected: usize = 0;

    // Print prompt
    execute!(
        out,
        style::PrintStyledContent("? ".green().bold()),
        style::Print(prompt),
        cursor::MoveToNextLine(1),
    ).map_err(|e| format!("Menu error: {e}"))?;

    terminal::enable_raw_mode().map_err(|e| format!("Menu error: {e}"))?;

    // Hide cursor during selection
    execute!(out, cursor::Hide).map_err(|e| {
        let _ = terminal::disable_raw_mode();
        format!("Menu error: {e}")
    })?;

    // Initial render
    render_menu(&mut out, items, selected).map_err(|e| {
        let _ = execute!(out, cursor::Show);
        let _ = terminal::disable_raw_mode();
        format!("Menu error: {e}")
    })?;

    let result = loop {
        let ev = event::read().map_err(|e| {
            let _ = execute!(out, cursor::Show);
            let _ = terminal::disable_raw_mode();
            format!("Menu error: {e}")
        })?;

        match ev {
            Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. }) |
            Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                let _ = execute!(out, cursor::Show);
                let _ = terminal::disable_raw_mode();
                return Err("Aborted".to_string());
            }
            Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                break selected;
            }
            Event::Key(KeyEvent { code: KeyCode::Up, .. }) => {
                if selected > 0 {
                    selected -= 1;
                }
            }
            Event::Key(KeyEvent { code: KeyCode::Down, .. }) => {
                if selected < items.len() - 1 {
                    selected += 1;
                }
            }
            Event::Key(KeyEvent { code: KeyCode::Char(c), modifiers: KeyModifiers::NONE, .. }) => {
                if let Some(digit) = c.to_digit(10) {
                    let idx = digit as usize;
                    if idx >= 1 && idx <= items.len() && idx <= 9 {
                        selected = idx - 1;
                        break selected;
                    }
                }
            }
            _ => {}
        }

        // Redraw: move cursor up to start of menu, then re-render
        if items.len() > 1 {
            let _ = execute!(out, cursor::MoveUp((items.len() - 1) as u16));
        }
        let _ = execute!(out, cursor::MoveToColumn(0));
        render_menu(&mut out, items, selected).map_err(|e| {
            let _ = execute!(out, cursor::Show);
            let _ = terminal::disable_raw_mode();
            format!("Menu error: {e}")
        })?;
    };

    // Cleanup: show cursor, disable raw mode, move past menu
    let _ = execute!(out, cursor::Show, cursor::MoveToNextLine(1));
    let _ = terminal::disable_raw_mode();

    Ok(result)
}
```

- [ ] **Step 4: Verify show_select compiles**

Run: `cargo check 2>&1 | head -20`

Expected: Errors only in `prompt_name` (still uses `dialoguer::Input`). `show_select` should compile cleanly.

- [ ] **Step 5: Commit**

```bash
git add src/menu.rs
git commit -m "feat: add custom select widget with number-key instant selection"
```

---

### Task 3: Implement custom `prompt_name` with space-to-hyphen

**Files:**
- Modify: `src/menu.rs:92-107` (replace `prompt_name` function)

- [ ] **Step 1: Add validate_branch_name helper**

Add this function right before `prompt_name`:

```rust
fn validate_branch_name(input: &str) -> Result<(), String> {
    if input.is_empty() {
        return Err("Name cannot be empty".to_string());
    }
    if input.contains("..") || input.contains('~') || input.contains('^') || input.contains(':') || input.contains('\\') {
        return Err("Invalid branch name. Avoid special characters (.. ~ ^ : \\)".to_string());
    }
    Ok(())
}
```

- [ ] **Step 2: Replace prompt_name function**

Replace the entire `prompt_name` function (lines 92-107) with:

```rust
pub fn prompt_name(prompt: &str) -> Result<String, String> {
    loop {
        let mut out = io::stderr();
        let mut input = String::new();

        // Print prompt
        execute!(
            out,
            style::PrintStyledContent("? ".green().bold()),
            style::Print(format!("{prompt}: ")),
        ).map_err(|e| format!("Input error: {e}"))?;

        terminal::enable_raw_mode().map_err(|e| format!("Input error: {e}"))?;

        let result = loop {
            let ev = event::read().map_err(|e| {
                let _ = terminal::disable_raw_mode();
                format!("Input error: {e}")
            })?;

            match ev {
                Event::Key(KeyEvent { code: KeyCode::Char('c'), modifiers: KeyModifiers::CONTROL, .. }) |
                Event::Key(KeyEvent { code: KeyCode::Esc, .. }) => {
                    let _ = terminal::disable_raw_mode();
                    return Err("Aborted".to_string());
                }
                Event::Key(KeyEvent { code: KeyCode::Enter, .. }) => {
                    break input.clone();
                }
                Event::Key(KeyEvent { code: KeyCode::Backspace, .. }) => {
                    if input.pop().is_some() {
                        let _ = execute!(
                            out,
                            cursor::MoveLeft(1),
                            style::Print(" "),
                            cursor::MoveLeft(1),
                        );
                    }
                }
                Event::Key(KeyEvent { code: KeyCode::Char(c), .. }) => {
                    let ch = if c == ' ' { '-' } else { c };
                    // Collapse consecutive hyphens: skip if last char is already '-' and new char is '-'
                    if ch == '-' && input.ends_with('-') {
                        continue;
                    }
                    input.push(ch);
                    let _ = execute!(out, style::Print(ch));
                }
                _ => {}
            }
        };

        let _ = execute!(out, cursor::MoveToNextLine(1));
        let _ = terminal::disable_raw_mode();

        // Trim leading/trailing hyphens
        let trimmed = result.trim_matches('-').to_string();

        match validate_branch_name(&trimmed) {
            Ok(()) => return Ok(trimmed),
            Err(e) => {
                let _ = execute!(
                    out,
                    style::PrintStyledContent(format!("  {e}").red()),
                    cursor::MoveToNextLine(1),
                );
                // Loop to re-prompt
            }
        }
    }
}
```

- [ ] **Step 3: Verify full compilation**

Run: `cargo check`

Expected: Clean compilation with no errors and no warnings about unused imports.

- [ ] **Step 4: Run existing tests**

Run: `cargo test`

Expected: All existing tests in `tests/branch_test.rs` and `tests/version_test.rs` pass. These tests don't touch menu code, so they should be unaffected.

- [ ] **Step 5: Commit**

```bash
git add src/menu.rs
git commit -m "feat: add custom text input with live space-to-hyphen conversion"
```

---

### Task 4: Manual smoke test

**Files:** None (verification only)

- [ ] **Step 1: Build release binary**

Run: `cargo build`

Expected: Clean build, binary at `target/debug/bflow`.

- [ ] **Step 2: Smoke test select widget**

Navigate to a git repo on `develop` (or `main`) branch and run:

```bash
cargo run
```

Verify:
- Menu displays with numbered items (e.g., `> 1) start feature`)
- Arrow keys move the `>` cursor
- Pressing a number key (e.g., `2`) instantly selects that option
- Enter confirms the highlighted option
- Esc aborts cleanly

- [ ] **Step 3: Smoke test text input**

After selecting a branch type, verify the name prompt:
- Typing spaces renders hyphens immediately
- Consecutive spaces produce only one hyphen
- Backspace works correctly
- Enter submits
- Esc aborts cleanly

- [ ] **Step 4: Commit (if any fixes needed)**

If smoke testing reveals issues, fix them and commit:

```bash
git add src/menu.rs
git commit -m "fix: address smoke test findings in menu widgets"
```

---

### Task 5: Update integration test documentation

**Files:**
- Modify: `tests/integration-test-prompt.md`

- [ ] **Step 1: Update the "How bflow Interacts" section**

Replace lines 14-19 of `tests/integration-test-prompt.md`:

```markdown
bflow uses interactive terminal menus. Here's how to interact:

- **Select menus**: Use arrow keys (↓/↑) to navigate, Enter to select. Default is always the first item (index 0).
- **Text input**: Type the name, press Enter.
- **Auto-dispatch**: On release-fix and hotfix-fix branches, bflow auto-executes the finish action with no menu — just run `bflow` and it goes.
- **Work branches**: On work branches (feature/fix/chore/docs/refactor), bflow shows a menu with finish as default (index 0) plus start options. Pressing Enter finishes the branch. A PR target selection prompt follows.
```

With:

```markdown
bflow uses interactive terminal menus. Here's how to interact:

- **Select menus**: Items are numbered 1-9. Press a number key to instantly select that option (no Enter needed), or use arrow keys (↓/↑) to navigate and Enter to confirm. Default is always the first item (index 0).
- **Text input**: Type the name, press Enter. Spaces are automatically converted to hyphens as you type.
- **Auto-dispatch**: On release-fix and hotfix-fix branches, bflow auto-executes the finish action with no menu — just run `bflow` and it goes.
- **Work branches**: On work branches (feature/fix/chore/docs/refactor), bflow shows a menu with finish as default (index 0) plus start options. Pressing Enter or 1 finishes the branch. A PR target selection prompt follows.
```

- [ ] **Step 2: Update menu indices reference tables**

Replace the four menu tables (lines 23-53) with number-key references:

```markdown
### Menu key reference

**On `develop`** (6 options):
| Key | Option |
|-----|--------|
| 1 | start feature |
| 2 | start fix |
| 3 | start chore |
| 4 | start docs |
| 5 | start refactor |
| 6 | start release fix |

**On `release/{v}`** (3 options):
| Key | Option |
|-----|--------|
| 1 | bump version |
| 2 | sync with develop |
| 3 | finish release |

**On `main`** (1 option):
| Key | Option |
|-----|--------|
| 1 | start hotfix fix |

**On work branches** (feature/fix/chore/docs/refactor) (6 options):
| Key | Option |
|-----|--------|
| 1 | finish {type} |
| 2 | start feature |
| 3 | start fix |
| 4 | start chore |
| 5 | start docs |
| 6 | start refactor |
```

- [ ] **Step 3: Update Phase 2 interaction instructions**

Update all menu interaction blocks throughout Phase 2-4. Here are the replacements:

**Step 2.1 (Feature) — start:**
```
# On develop
bflow
→ Press 1 (instant select: "start feature")
→ Input: "user-auth"
```

**Step 2.1 (Feature) — finish:**
```
bflow
→ Press 1 (instant select: "finish feature")
→ PR target: press 1 (instant select: "develop")
```

**Step 2.2 (Fix) — start:**
```
bflow
→ Press 2 (instant select: "start fix")
→ Input: "null-pointer"
```

**Step 2.2 (Fix) — finish:**
```
bflow
→ Press 1 (instant select: "finish fix")
→ PR target: press 1 (instant select: "develop")
```

**Step 2.3 (Chore) — start:**
```
bflow
→ Press 3 (instant select: "start chore")
→ Input: "update-deps"
```

**Step 2.3 (Chore) — finish:**
```
bflow
→ Press 1 (instant select: "finish chore")
→ PR target: press 1 (instant select: "develop")
```

**Step 2.4 (Docs) — start:**
```
bflow
→ Press 4 (instant select: "start docs")
→ Input: "api-guide"
```

**Step 2.4 (Docs) — finish:**
```
bflow
→ Press 1 (instant select: "finish docs")
→ PR target: press 1 (instant select: "develop")
```

**Step 2.5 (Refactor) — start:**
```
bflow
→ Press 5 (instant select: "start refactor")
→ Input: "clean-models"
```

**Step 2.5 (Refactor) — finish:**
```
bflow
→ Press 1 (instant select: "finish refactor")
→ PR target: press 1 (instant select: "develop")
```

**Step 2.5.1 (Parent feature) — start:**
```
# On develop
bflow
→ Press 1 (instant select: "start feature")
→ Input: "payment-system"
```

**Step 2.5.2 (Child fix from parent) — start:**
```
# On feature/payment-system
bflow
→ Press 3 (instant select: "start fix")
→ Input: "payment-validation"
```

**Step 2.5.3 (Child fix) — finish:**
```
bflow
→ Press 1 (instant select: "finish fix")
→ PR target: verify "feature/payment-system" is option 1, press 1
```

**Step 2.5.3 (Parent feature) — finish:**
```
bflow
→ Press 1 (instant select: "finish feature")
→ PR target: verify "develop" is option 1, press 1
```

**Step 3.1 (Release fix 1) — start:**
```
# On develop
bflow
→ Press 6 (instant select: "start release fix")
→ (bflow auto-creates release/1.1 from develop, tags 1.1.0)
→ Input: "payment-bug"
```

**Step 3.2 (Bump version):**
```
# On release/1.1
bflow
→ Press 1 (instant select: "bump version")
→ (bflow auto-bumps 1.1.0 → 1.1.1, creates and pushes tag)
```

**Step 3.3 (Sync with develop):**
```
# Still on release/1.1
bflow
→ Press 2 (instant select: "sync with develop")
→ (bflow merges release/1.1 into develop, pushes, returns to release/1.1)
```

**Step 3.4 (Release fix 2) — start:**
```
bflow
→ Press 6 (instant select: "start release fix")
→ (bflow detects existing release/1.1, uses it)
→ Input: "validation-error"
```

**Step 3.5 (Bump version 2):**
```
# On release/1.1
bflow
→ Press 1 (instant select: "bump version")
→ (bflow auto-bumps 1.1.1 → 1.1.2, creates and pushes tag)
```

**Step 3.6 (Finish release):**
```
# On release/1.1
bflow
→ Press 3 (instant select: "finish release")
→ (bflow merges into main, merges into develop, deletes release/1.1)
```

**Step 4.1 (Hotfix fix) — start:**
```
bflow
→ Press 1 (instant select: "start hotfix fix")
→ (bflow auto-creates hotfix/1.1.3 from main, bumps patch from 1.1.2)
→ Input: "critical-crash"
```

**Step 4.2 (Finish hotfix):**
```
# On hotfix/1.1.3
bflow
→ Press 1 (instant select: "finish hotfix")
→ (bflow merges into main, tags 1.1.3, merges into develop, deletes hotfix/1.1.3)
```

- [ ] **Step 4: Verify the integration test doc is coherent**

Read through the full `tests/integration-test-prompt.md` to make sure all number references match the key tables and no old "index N, press ↓ N times" references remain.

- [ ] **Step 5: Commit**

```bash
git add tests/integration-test-prompt.md
git commit -m "docs: update integration test for number-key selection and space-to-hyphen"
```

---

## Self-Review

**Spec coverage:**
- Number key instant selection (1-9) — Task 2
- Space-to-hyphen live replacement — Task 3
- Consecutive space collapse — Task 3 (in the `Char` handler)
- Leading/trailing hyphen trim — Task 3 (after `result` is collected)
- Dependency swap — Task 1
- Integration test updates — Task 5
- All existing callers unchanged — verified: signatures preserved, `show_menu` untouched

**Placeholder scan:** No TBDs, TODOs, or vague steps. All code blocks are complete.

**Type consistency:** `show_select` returns `Result<usize, String>` everywhere. `prompt_name` returns `Result<String, String>` everywhere. `render_menu` takes `&mut io::Stderr` consistently. `validate_branch_name` takes `&str` and returns `Result<(), String>`.
