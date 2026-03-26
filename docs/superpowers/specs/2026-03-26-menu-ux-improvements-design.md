# Menu UX Improvements: Number Key Selection & Space-to-Hyphen

**Date:** 2026-03-26
**Status:** Approved

## Summary

Two UX improvements to bflow's interactive menus:

1. **Number key instant selection** — press 1-9 to immediately select a menu option
2. **Space-to-hyphen live replacement** — spaces typed in branch name inputs convert to hyphens as you type

## Dependency Changes

- **Remove:** `dialoguer = "0.11"`
- **Add:** `crossterm = "0.28"`

`dialoguer` doesn't support number-key selection or live character transformation. `crossterm` provides raw terminal input, cursor control, and styling — everything needed for both features.

## Custom Select Widget — `show_select`

Replaces `dialoguer::Select`.

### Display

```
? What would you like to do?
> 1) start feature
  2) start fix
  3) start chore
  4) start docs
  5) start refactor
  6) start release fix
```

The `>` cursor is inline on the currently highlighted item. When navigating with arrows, the cursor moves:

```
? What would you like to do?
  1) start feature
> 2) start fix
  3) start chore
  4) start docs
  5) start refactor
  6) start release fix
```

### Behavior

- Items displayed with 1-based number prefixes
- `>` cursor highlights current selection (starts at item 1)
- **Number keys 1-9:** Instantly selects and returns that item (no Enter needed)
- **Arrow keys up/down:** Move the cursor, Enter to confirm
- **Enter:** Confirms the currently highlighted item
- **Ctrl+C / Esc:** Returns an error (abort)
- Numbers beyond the item count are ignored
- Only single-digit keys (1-9) supported; items 10+ require arrow keys
- Styling: green `?` prompt, current item shown in bold/cyan with `>` prefix, other items dimmed with two-space prefix

### Signature

```rust
pub fn show_select(prompt: &str, items: &[&str]) -> Result<usize, String>
```

Returns 0-based index. All existing callers work unchanged.

## Custom Text Input — `prompt_name`

Replaces `dialoguer::Input`.

### Display

```
? Name for feature branch: my-cool-feature█
```

### Behavior

- Each space keystroke renders as `-` immediately (live replacement)
- Consecutive spaces collapse to a single hyphen (e.g., `my  feature` -> `my-feature`)
- Leading/trailing hyphens from converted spaces are trimmed
- Backspace deletes characters as expected
- **Enter:** Submits the current value
- **Ctrl+C / Esc:** Aborts with error
- After submit, validation runs (same rules as today minus the space check):
  - Not empty
  - No `..`, `~`, `^`, `:`, `\`
  - If invalid, show error message and re-prompt

### Signature

```rust
pub fn prompt_name(prompt: &str) -> Result<String, String>
```

Unchanged signature. All existing callers work unchanged.

## File Changes

### Modified

- **Cargo.toml** — swap `dialoguer` for `crossterm`
- **src/menu.rs** — replace `show_select` and `prompt_name` internals with crossterm-based implementations. Add helper `render_menu()` for drawing/redrawing menu items. All option enums (`DevelopOption`, `WorkBranchOption`, `ReleaseOption`) and `show_menu()` unchanged.
- **tests/integration-test-prompt.md** — update interaction documentation (see below)

### Unchanged

- `src/main.rs`, `src/flows/*`, `src/git/*`, `src/hosting/*` — no changes needed since function signatures are preserved

## Integration Test Updates

### "How bflow Interacts" section

Add documentation for:
- **Number key shortcuts:** Press 1-9 to instantly select a menu option (no Enter needed)
- **Space-to-hyphen:** Spaces typed in branch name inputs are automatically converted to hyphens as you type

### Menu interaction examples

Update from arrow-key style:
```
bflow
-> Select: "start fix" (index 1, press down once then Enter)
-> Input: "null-pointer"
```

To number-key style:
```
bflow
-> Press 2 (instant select: "start fix")
-> Input: "null-pointer"
```

Branch name inputs can optionally show space input:
```
-> Input: "null pointer" (spaces auto-convert to "null-pointer")
```
