# CR-04 — Checkout-less hotfix creation must run the version script

Branch: `feature/hotfix-version-script-worktree`
Source: `tasks/proposal-ux-improvements.md` §3 CR-04 (accepted, with a corrected cost model — see "Correction to the proposal" below).
Priority: High · Size: L

---

## The prompt

Make the invariant *"a `hotfix/X.Y.Z` branch carries its version"* hold in **every**
creation mode, including checkout-less creation (`--no-checkout`, or worktree mode).
Today one configuration flag downgrades that invariant to a printed human TODO, at
the most urgent moment the tool serves.

Implement it by running the version script inside an **ephemeral git worktree** for
the freshly created hotfix branch, committing there, then removing the worktree.
If the script itself fails, fall back to today's warning — a broken script must
never block a hotfix.

Follow the repo's TDD policy and the "Load these first" table in `CLAUDE.md`
(`kiss-principles`, `dry-principles`, `solid-principles`, `architecture` +
`decisions.md`, `superpowers:test-driven-development`). Red → green → refactor.

---

## Current behavior (the thing being changed)

`src/flows/start.rs::resolve_or_create_hotfix`, the `no_checkout` arm:

```rust
if no_checkout {
    git.create_branch_no_checkout(&branch, main_branch)?;
    if let Some(script) = script {
        eprintln!("⚠ Version script not run: {branch} was created without checkout, …");
        eprintln!("  Recover manually: git switch {branch}, run {} {next}, commit, and push.", script.display_name());
    }
} else {
    git.checkout(main_branch)?;
    git.create_branch(&branch, main_branch)?;
    if let Some(script) = script { run_version_script(git, script, &next)?; }
}
git.push(&branch)?;
```

`no_checkout` here is `effective_no_checkout` — `--no-checkout` **or** an active
worktree context (`src/flows/start.rs::effective_no_checkout`). So this fires for
every worktree-mode user with a version script.

Note the asymmetry this creates: the checkout path runs `require_clean_tree` first
(trap 2 — the script must never see pre-existing local changes); the no-checkout
path needs no such guard, because a fresh worktree of a fresh branch is clean by
construction.

## Spec change (explicit, not a workaround)

`tests/start_test.rs::hotfix_no_checkout_skips_script` currently **pins** today's
skip (mutation audit Trap 9: "the script must never run without a checkout"). This
change replaces that specification. Replace the pinned test *first* and watch it
fail red before implementing. Record the spec change in the commit body, and
re-verify the replacement by mutation (revert the fix, confirm the new test fails).

Current pinned expectation:

```rust
assert_eq!(git.calls(), vec![
    "list_branches_matching:hotfix/*",
    "list_tags",
    "create_branch_no_checkout:hotfix/1.0.1:main",
    "push:hotfix/1.0.1",
    "create_branch_no_checkout:hotfix-fix/1.0.1/urgent-crash:hotfix/1.0.1",
    "push:hotfix-fix/1.0.1/urgent-crash",
]);
assert!(script.calls().is_empty());
```

## Correction to the proposal — the real cost

The proposal claims *"the worktree machinery and its `Git` primitives already
exist (`worktree.rs`); this reuses them privately rather than adding surface."*
**That is false, and the plan must not be written as if it were true.** Verified
against the code:

- `Git::add_worktree(path, branch)` exists. `Git::remove_worktree(path)` **does
  not** — only `remove_current_worktree()`, which removes the worktree the process
  is *standing in* and must be a flow's last git call (it deletes the process cwd).
  It cannot remove an ephemeral one.
- **Every other `Git` primitive is cwd-relative** (`decisions.md`, Boundaries:
  one deliberate `-C` exception). `run_version_script`'s
  `is_working_tree_clean` / `stage_all` / `commit` would all act on the *main*
  tree, not the new worktree.
- `VersionScript::run(version)` spawns with `current_dir(self.repo_root)` fixed at
  construction (`src/version_script.rs`), and `ScriptCli` is built in `main.rs` —
  flows cannot construct adapters (composition root rule).

So this change **does widen two ports**. Own that in the design write-up rather
than discovering it mid-implementation.

### The surface floor (justify anything beyond it)

The key fact that keeps this small: **linked worktrees share the repository's
refs**. Only the *file-touching* steps need the worktree; `push` from the main tree
pushes the branch fine, and no `checkout` is involved anywhere.

Minimum viable additions — each must have a present-tense caller:

1. `VersionScript::run_in(&self, version: &str, dir: &Path)` — a second method
   rather than widening `run`, so every existing call site and its pinned tests
   stay untouched. (If you widen `run` instead, say why; the trait is a port with
   one production impl, so ISP is not the deciding axis — churn is.)
2. `Git::commit_all_in(dir, message) -> Result<bool>` — status + stage + commit
   inside `dir`, returning whether anything was committed (mirrors
   `run_version_script`'s `Ok(true)` = committed contract).
   This is **deliberately coarser** than the "48 fine-grained primitives"
   convention. That convention exists so *ordering logic lives in flows and mocks
   can record exact sequences* (`decisions.md`, Boundaries). Here there is no
   ordering choice to make — status → add → commit is fixed and already
   encapsulated by `run_version_script`. Three `*_in(dir)` primitives would buy
   zero testable ordering. **If you accept this, add a `decisions.md` entry
   recording the narrowing and its rationale; if you reject it, add the three
   fine-grained primitives instead and say why the convention wins here.**
3. `Git::remove_worktree(path)` — the ephemeral counterpart to
   `remove_current_worktree`. Document on the trait why both exist.

Do **not** add a `require_clean_tree`-in-dir primitive: a fresh worktree of a
freshly created branch is clean by construction, so the check is vacuous there.
Say so in a comment where the checkout path's `require_clean_tree` has no sibling.

### Options considered and rejected (don't re-litigate)

| Option | Why rejected |
|---|---|
| Do nothing (today's warning) | Leaves a manual step inside incident response; easy to forget until a wrong version ships. Contradicts architecture principle 10. |
| Hard error when a script exists and creation is checkout-less | Repos with worktree mode **and** a version script could not start hotfixes at all. Strictly worse than the warning. |
| `git checkout` the mainline in place, run the script, switch back (precedent: `bump_develop` in `start.rs`) | Fails in worktree mode: the mainline is usually checked out in the main tree already, and git refuses a second checkout of the same branch. Also directly violates what `--no-checkout` promises. |
| Let the version commit ride on the `hotfix-fix/*` branch bflow creates next (its worktree already exists) | Weakens the invariant: the container is unversioned until the first fix PR lands, and a second concurrent fix branches off an unversioned container. Also mixes a machine commit into the user's fix PR. |

## Required behavior

After `git.create_branch_no_checkout(&branch, main_branch)` and **before**
`git.push(&branch)`, when a script is present:

1. Add an ephemeral worktree for `hotfix/X.Y.Z`.
2. Run the version script there with the hotfix version — reuse the
   already-resolved `ScriptCli` (only the cwd changes; do not re-resolve).
3. Commit whatever it changed, on that branch.
4. Remove the worktree — **on every exit path**, success or failure.
5. Push the branch (unchanged; the commit is on the shared ref).

Failure policy, matching the M2 `bump_develop` precedent in the same file
(warn-and-continue when the primary work already succeeded):

- **Script fails** → keep today's warning text (it is pinned by its own test) and
  continue; the hotfix must still exist.
- **Worktree add/remove fails** → decide and state the call: a failed *add* has the
  same "the hotfix must not be blocked" logic as a failed script; a failed *remove*
  leaves a stray directory, so it should at minimum warn with the exact
  `git worktree remove` command (error model: every message names the next command).

Where to put the ephemeral worktree: derive it, don't invent a second scheme —
`worktree::worktree_path` already owns folder naming. Decide whether the ephemeral
one reuses it or gets its own suffix, and say why. It must not collide with a
user-facing worktree the flow may create moments later for `hotfix-fix/*`.

Also check (**verify, do not assume**): does a checkout-less *release* creation
path exist? `resolve_or_create_release` currently always checks out, so this is
likely N/A — confirm and state the finding either way.

## Acceptance criteria

- [ ] `hotfix_no_checkout_skips_script` replaced by a spec test: the script runs
      via an ephemeral worktree, the version commit lands on `hotfix/X.Y.Z`, and the
      worktree is removed afterwards. Watched fail red first.
- [ ] A test proving the worktree is removed when the **script fails**, and that
      the run still succeeds with the existing warning.
- [ ] The `--no-checkout` (non-worktree-mode) path is covered too — it is the same
      code path via `effective_no_checkout`, but pin it.
- [ ] No-script repos are byte-for-byte unchanged (existing sequences stay green).
- [ ] New `Git` methods added to `MockGit` (`tests/common/mod.rs`) **and** to the
      flags table in `tests/git_cli_test.rs` (extension recipe, `decisions.md`).
- [ ] `cargo test` green; coverage ≥ `.claude/hooks/coverage-baseline.txt`
      (never lower the baseline).
- [ ] Windows: the ephemeral worktree path handling is exercised by the tests, not
      by hope.

## Docs sync (same change — `CLAUDE.md` requires it)

- [ ] `.claude/skills/architecture/decisions.md` — version-script section: the
      four-moments table and the no-checkout caveat become "runs via an ephemeral
      worktree; warns only if the script itself fails". Add the `commit_all_in`
      narrowing entry (or the rejection of it) under Boundaries.
- [ ] `README.md` — remove the manual-recovery caveat; add the one-line
      `--no-checkout` positioning sentence: *"prefer worktree mode; `--no-checkout`
      is the low-level escape hatch"* (proposal §4, deliberately kept rather than
      deprecated).
- [ ] `.claude/skills/bflow/skill.md` — same caveat removal, concise.

## Review focus (for the later code review)

1. Is the port widening the honest floor, or did it grow past the three additions?
2. Is worktree cleanup guaranteed on **every** failure path, including a panic-free
   early return between add and remove?
3. Does the spec-change commit body say what specification changed and why?
4. Does anything in `flows/` spawn a process directly? (It must not — principle 1.)
