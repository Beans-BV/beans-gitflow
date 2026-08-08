# bflow — Feature & Fix Request Proposal

| | |
|---|---|
| **Document** | Change request proposal — CLI UX improvements |
| **Date** | 2026-08-06 |
| **Source** | Full command-surface review of bflow from the end-user perspective (every command, flag, and configuration variation) |
| **Status** | Draft — awaiting approval |
| **Supersedes** | The interim plan in `tasks/todo.md` (same content, this document is the authoritative form) |

---

## 1. Summary

The review found the core model sound: branch model, tag strategy, landing
modes, and crash-safe state design all hold up under user-perspective
scrutiny. The findings that remain are edge-of-tool communication gaps —
bflow knows things it does not tell the user — plus one case where a flag
quietly downgrades an enforced invariant to a manual step. This proposal
contains **three change requests** (one fix, one enhancement, one behavior
completion) and records **six considered-and-declined items** with
rationale, so they are not re-litigated later. (A fourth request, CR-01, was
rejected at planning review — its premise did not survive verification; see
§4.)

## 2. Requests at a glance

| ID | Type | Title | Priority | Size | Proposed branch |
|----|------|-------|----------|------|-----------------|
| CR-02 | Fix (UX) | `bflow sync` states its direction | Low | S (copy-level) | `chore/sync-direction-wording` |
| CR-03 | Enhancement | Protected mode prints the full landing plan every run | Medium | M (2–3 chunks) | `feature/protected-landing-plan` |
| CR-04 | Fix (behavior) | Checkout-less hotfix creation must not skip the version script | High | L (3–4 chunks) | `feature/hotfix-version-script-worktree` |

Recommended delivery order: CR-02 → CR-03 → CR-04. Quick win first; CR-04
last because it changes a pinned specification and deserves an unhurried
review.

---

## 3. Detailed requests

### CR-02 — `bflow sync` states its direction

**Type:** Fix (UX) · **Priority:** Low · **Size:** S

**Problem.** "Sync" reads as bidirectional. Users will reach for it hoping
to pull `develop` into the release — the one thing it never does, by design
(the RC gate exists so unstaged develop content cannot ride into a release).
The operation is strictly release → develop, and nothing at the CLI level
says so.

**Proposed change.** Wording only, in three places:
- Command narration: `Merging release/X.Y.Z into develop (one-way — develop
  is never merged into a release)`.
- clap `--help` text for the subcommand.
- README + bflow skill: state the direction and the *why* (RC gate).

**Alternatives considered.**
1. *Do nothing.* Declined — the ambiguity costs a docs round-trip per user.
2. *Rename* (`sync-develop`, `backmerge`) — breaks scripts and muscle
   memory on published surface, a one-way door already walked through.
   Declined.

**Decision axis:** clarity vs. CLI surface stability.

**Implementation plan.**
- [ ] 2.1 Narration + clap help text updated. If the narration line is
      asserted anywhere, adjust that test red-first; otherwise this is a
      copy-level change (no new test — CLAUDE.md's "does NOT apply" list).
- [ ] 2.2 Docs (same change): README + bflow skill sentence — one-way,
      release → develop, and why the reverse doesn't exist.

**Risk / reversibility.** Copy-level; trivially reversible.

---

### CR-03 — Protected mode prints the full landing plan every run

**Type:** Enhancement · **Priority:** Medium · **Size:** M

**Problem.** In `mode=protected`, one hotfix with two open release branches
means up to four re-runs of `bflow finish` spread over hours. Each run
prints only the single PR it opened or reused. The user's actual question on
every run — *"where am I in this?"* — is information bflow already re-derives
internally from PR and tag state on every run, and then discards.

**Proposed change.** Every protected `finish` / `bump` / `sync` run prints
the full derived landing sequence with per-step status before acting:

```
Landing plan for hotfix/2.6.2:
  ✓ main            merged (v2.6.2 tagged)
  → develop         PR open: <url> — merge it, then re-run bflow finish
  ○ release/2.7.0   pending
```

Implementation shape: a pure derivation function (e.g.
`landing_plan(...) -> Vec<LandingStatus>` beside the existing
protected-progress derivation in `flows/`), unit-tested; printing stays a
thin caller — the same logic/narration split used everywhere else. No state
written, no behavior change, no new CLI surface.

**Alternatives considered.**
1. *Do nothing* — the stateless one-PR-per-run design is deliberate, but it
   leaves the user reconstructing progress by hand. Declined.
2. *`bflow status` command* — new public surface AND a second copy of the
   derivation knowledge (a DRY violation by construction), for information
   the user gets for free from the command they must re-run anyway.
   Declined.
3. *Watch/poll mode* (`--watch` until PRs merge) — a long-running process
   contradicts the stateless exit-0 design; TTY/CI complications. Declined.

**Decision axis:** where the "where am I" answer lives — inside the command
the user already re-runs, or in new surface duplicating its knowledge.
**Precedent:** the `↷ skipped:` narration convention for idempotent steps
(`flows/mod.rs`, documented in `decisions.md`) — the same idea lifted from
step level to plan level.

**Implementation plan (TDD).**
- [ ] 3.1 Red first: unit tests for the pure derivation covering fresh
      start, mid-sequence, tag-deferred bump, and `keep-release-branches`.
- [ ] 3.2 Wire into the protected paths of `finish` (release + hotfix),
      `bump`, `sync`. Existing protected call-sequence tests must stay
      green **byte-for-byte** — they pin the no-local-mutation guarantee.
- [ ] 3.3 Docs (same change): README protected-mode section shows one
      sample plan block; bflow skill gets one sentence (keep it concise).

**Risk / reversibility.** Output-only; cheap to change or remove.

---

### CR-04 — Checkout-less hotfix creation must not skip the version script

**Type:** Fix (behavior completion) · **Priority:** High · **Size:** L

**Problem.** With worktree mode enabled (or `--no-checkout`), creating the
hotfix container skips the version script and prints manual recovery steps.
One configuration flag downgrades an enforced invariant — *release/hotfix
branches carry their version* — to a human TODO, at the most urgent moment
the tool serves. This contradicts architecture principle 10 (process is
enforced in code, not documentation).

**Spec-change notice.** Today's skip is deliberate and mutation-pinned:
`hotfix_no_checkout_skips_script` (mutation audit Trap 9, "the script must
never run without a checkout"). This request **changes that specification**
explicitly — it does not work around it. The pinned test is replaced first.

**Proposed change.** After creating and pushing `hotfix/X.Y.Z`, bflow adds a
temporary git worktree for it, runs the version script there, commits,
pushes, and removes the worktree. The invariant then holds in every mode.
If the script itself fails inside the temp worktree, fall back to today's
warning — a broken script must never block the hotfix (same stance as the
M2 develop-bump warn-and-continue in `start.rs`).

**Alternatives considered.**
1. *Do nothing* — warn + recovery steps (shipped 2026-08-05). Keeps a
   manual step inside incident response; easy to forget until a wrong
   version ships. Declined.
2. *Hard error* when a script exists and creation is checkout-less — repos
   with worktree mode **and** a version script could no longer start
   hotfixes at all without `--no-worktree` plus a checkout switch. Strictly
   worse than the warning. Declined.

**Decision axis:** who completes the version commit — the operator (1),
nobody (2), or bflow (proposed).
**Precedent:** the worktree machinery and its `Git` primitives already
exist (`worktree.rs`); this reuses them privately rather than adding
surface.

**Implementation plan (TDD).**
- [ ] 4.1 Spec change first: replace `hotfix_no_checkout_skips_script` with
      the new spec test — script runs via temp worktree, the version commit
      lands on `hotfix/X.Y.Z`, the worktree is removed afterwards. Watch it
      fail red.
- [ ] 4.2 Implement in `flows/start.rs`'s hotfix-creation path using the
      existing `Git` worktree primitives. Every failure path removes the
      temp worktree; script failure produces the existing warning text
      (pinned by its own test).
- [ ] 4.3 Same treatment for the *release*-creation checkout-less path only
      if one exists — verify first; `start release` currently always checks
      out, so likely N/A (confirm, don't assume).
- [ ] 4.4 Docs (same change): `decisions.md` version-script section updated
      (the four-moments table + the no-checkout caveat becomes "runs via
      ephemeral worktree; warns only if the script itself fails"); the
      manual-recovery caveat removed from README and the bflow skill; add
      the `--no-checkout` positioning sentence from §4 below.

**Risk / reversibility.** Internal behavior; no CLI surface, no persisted
state — cheap to revert (the warn path remains as the fallback arm). Main
risks: temp-worktree cleanup on failure paths and Windows path handling,
both covered by the acceptance tests above.

---

## 4. Considered and declined

Recorded so these are not re-opened without new evidence.

| Item | Decision | Rationale |
|------|----------|-----------|
| CR-01: guiding error for `bflow start hotfix` | Rejected (2026-08-08 planning review) | Premise falsified: clap already answers `bflow start hotfix` with `unrecognized subcommand 'hotfix'` + `tip: a similar subcommand exists: 'hotfix-fix'`, and a follow-up bare `start hotfix-fix` names the missing `--name`. The proposed fix requires a hidden parse-accepted `StartKind` variant — permanent CLI surface for marginal gain over the built-in tip; Necessity test fails. Revisit only with evidence users still stumble despite the tip. |
| Deprecate `--no-checkout` (overlaps worktree mode) | Keep, for now | Deletion test fails to justify removal: published surface, and an escape hatch for users with their own worktree tooling. Add one README sentence positioning it ("prefer worktree mode; `--no-checkout` is the low-level escape hatch") — folded into CR-04 step 4.4. Revisit at the next major (the product rename is the natural moment). |
| Restrict `--breaking` on chore/docs branches | Keep as-is | The uniform rule ("the flag works on any work branch type") is a simpler contract than a per-type reject table; `chore!:` is legal Conventional Commits. Nothing to do — one README line already covers it. |
| One-shot `--worktree` opt-in flag | Not yet | KISS red flag #8: a flag with no demonstrated caller is configuration ceremony. Additive surface is the cheap direction — trivially addable on first real request. |
| Protected watch/poll mode | No | See CR-03 alternative 3. |
| Renames (`sync`, product rename) | Out of scope | `sync` rename breaks published surface for a wording problem CR-02 solves; the product rename (gflow) is separately decided and explicitly not to be started unprompted. |

## 5. Delivery constraints (apply to every CR)

- Load `kiss-principles`, `dry-principles`, `solid-principles`,
  `architecture` (+ `decisions.md`) and
  `superpowers:test-driven-development` before any code; re-load between
  chunks on longer runs (see `tasks/lessons.md`).
- Red → green → refactor; watch every red test fail before making it pass.
- One chunk = one commit-sized change. After every chunk: `cargo test`
  green, coverage ≥ `.claude/hooks/coverage-baseline.txt` (the Stop hook
  enforces this; the baseline is never lowered).
- Each CR on its own branch via bflow (names in §2).
- Docs sync (README + `.claude/skills/bflow/skill.md`, plus `decisions.md`
  for CR-04) lands in the same change as the behavior it describes.
- CR-04 replaces a mutation-pinned test: record the spec change in the
  commit body and re-verify the replacement test by mutation (revert the
  fix, confirm the new test fails).
