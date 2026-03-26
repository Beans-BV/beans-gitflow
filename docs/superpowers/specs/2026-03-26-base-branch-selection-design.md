# Base Branch Selection When Starting from Work Branches

**Date:** 2026-03-26
**Status:** Approved

## Summary

When starting a new work branch from an existing work branch, prompt the user to choose the base branch: the current work branch or `develop`. On `develop`, no prompt is shown — it always uses `develop`.

## User Flow

On `feature/a`, starting a new fix:

```
? What would you like to do?
> 2) start fix

? Name for fix branch: payment-validation

? Base branch:
> 1) feature/a (current)
  2) develop
```

- **Option 1 (default):** Creates `fix/payment-validation` from `feature/a` (child branch pattern)
- **Option 2:** Creates `fix/payment-validation` from `develop` (independent branch)

On `develop`, the flow is unchanged — no base branch prompt.

## Code Changes

### Modified: `src/menu.rs`

Only the work branch arm of `show_menu` changes (lines ~297-317). After `prompt_name` returns and before constructing `Action::StartWorkBranch`, add a base branch selection:

```rust
let from = {
    let options = &[
        &format!("{current_branch} (current)") as &str,
        "develop",
    ];
    let idx = show_select("Base branch", options)?;
    if idx == 0 { current_branch.to_string() } else { "develop".to_string() }
};
```

The `from` value is passed to `Action::StartWorkBranch { prefix, name, from }`.

### No other files change

- `src/flows/start.rs` — `start_work_branch` already accepts `from` as a parameter. `git checkout -b <branch> <from>` works with any branch name regardless of current checkout.
- `src/git/mod.rs` — No changes needed.
- `src/main.rs` — No changes needed.

### Modified: `tests/integration-test-prompt.md`

Update the child work branch section (Phase 2.5) to document the base branch prompt. When creating a child branch from a parent work branch, the test should select option 1 (current branch). Add a note that option 2 would create from develop instead.

## Edge Cases

- **Single option:** If somehow on develop (shouldn't happen — `show_menu` routes develop separately), the prompt would still show two options. This is harmless.
- **Number key selection:** Works automatically since we use the existing `show_select` widget which already supports 1-9 instant selection.
