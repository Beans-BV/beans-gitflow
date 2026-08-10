# CR-03 — Protected `finish` prints the landing plan

Branch: `feature/protected-landing-plan`
Source: `tasks/proposal-ux-improvements.md` §3 CR-03 — **accepted, but rescoped and
re-shaped**. Read "Corrections to the proposal" before planning; the proposal's
implementation shape costs network round-trips it does not need to.
Priority: Medium · Size: M

---

## The prompt

In `mode=protected`, one hotfix with two open release branches means up to four
re-runs of `bflow finish`, spread over hours. Each run prints only the single PR it
opened or reused. The user's actual question on every run — *"where am I in this?"*
— is something bflow already knows and then discards.

Make a protected `bflow finish` (release **and** hotfix) print the full landing
sequence with per-step status, using only information the run already gathered.

Follow the repo's TDD policy and the "Load these first" table in `CLAUDE.md`
(`kiss-principles`, `dry-principles`, `solid-principles`, `architecture` +
`decisions.md`, `superpowers:test-driven-development`). Red → green → refactor.

Target output, roughly (exact glyphs and wording are yours to justify):

```
Landing plan for hotfix/2.6.2:
  ✓ main            merged (v2.6.2 tagged)
  → develop         PR open: <url> — merge it, then re-run bflow finish
  ○ release/2.7.0   pending
```

---

## Corrections to the proposal (these change the design)

### 1. Scope: `finish` only. Drop `bump` and `sync`.

The proposal says "every protected `finish` / `bump` / `sync` run". Verified
against the code, that is scope inflation:

- `bflow sync` (`sync_with_develop_protected`) has **exactly one** leg,
  release → develop. A "plan" with one row is ceremony, not information — and the
  existing two lines already say the PR is open and to re-run after the merge.
- `bflow bump` (`bump_protected`) has **no landing sequence at all**. It opens a
  version PR into the release branch; it is not a leg of the landing walk.
  `announce_deferred` already states exactly what to do next.

Applying the plan there fails the KISS Necessity test. **Scope: `finish_release_protected`
and `finish_hotfix_protected` only.**

### 2. Shape: derive from what the run already knows. Zero extra hosting calls.

The proposal asks for *"a pure derivation function that re-derives the sequence from
PR and tag state, printed before acting"*. Re-deriving means calling
`HostingPlatform::merged_pr_to` for **every** leg on **every** run — up to four `gh`
/ `az` subprocess round-trips added to a command that today stops at the first
unlanded leg. That is a real latency cost, and it creates a second encoding of the
leg order (the derivation *and* the sequential walk) — a DRY violation by
construction, the exact objection the proposal itself raised against a
`bflow status` command.

**The flow is sequential, and that is what makes this free.** Both protected finish
functions walk legs in a fixed order and return at the first unlanded one:

- `finish_release_protected` (`src/flows/finish_release.rs`): `main` → `develop`.
- `finish_hotfix_protected` (`src/flows/finish_hotfix.rs`): `main` → `develop` →
  each open `release/*` in sorted order.

Therefore, at the moment the run stops:

- every leg **before** the stop point is landed — already queried, result in hand;
- the stop leg is the PR just opened or reused — its URL is in hand;
- every leg **after** the stop point is pending **by definition** — no query can
  tell you anything a correct implementation doesn't already know.

So the plan is derivable with **zero** additional `HostingPlatform` calls. Build it
that way. Printing it at the *end* of the run (right after the PR line, or in place
of it) is acceptable and arguably better — it leaves "where am I" as the last thing
on screen.

The one genuine cost: the hotfix flow must know its full leg list to name the
pending rows, and `open_versioned_branches` currently runs only *after* the develop
leg lands. Moving that enumeration earlier adds one `list_branches_matching` (plus
the shipped-branch filter) on early-return runs — and under
`bump-strategy=patch` + protected, that filter itself calls `leg_landed` per release
branch (`flows/mod.rs::open_versioned_branches`). **Measure that against the
benefit and state the call.** If it is judged too expensive, the fallback is a
plan whose tail is un-enumerated ("remaining: open release branches") — weaker, but
honest and free. Pick one, justify it.

### 3. The pinned sequences

The proposal requires existing protected call-sequence tests stay green
**byte-for-byte** — they pin the no-local-mutation guarantee. With shape (2) that is
achievable for `finish_release_protected` (fixed legs, no new calls). For
`finish_hotfix_protected`, moving the enumeration earlier **will** reorder recorded
git calls on early-return paths. That is a legitimate change, not a regression —
but it must be a **deliberate, explained** test update, never a quiet one. Any
sequence edit needs a commit-body sentence saying which call moved and why, and the
"no local mutation in protected mode" property must still be asserted (no
`checkout:`/`merge:`/`push:main` in the log).

---

## Design constraints

- **Logic/narration split**, as everywhere else: a pure function returning the plan
  (e.g. `Vec<LandingStep>` with a status enum), unit-tested directly; printing is a
  thin caller. No state written, no behavior change, no new CLI surface.
- Where it lives: `flows/mod.rs` alongside the other protected-mode landing helpers
  (`leg_landed`, `tip_landed_somewhere`, `report_commits_past_landing`) is the
  obvious home — confirm against the Layer Map rather than assuming.
- **Precedent to follow:** the `↷ skipped:` narration convention for idempotent
  steps (`flows/mod.rs`, documented in `decisions.md`) — the same idea lifted from
  step level to plan level. Match its voice.
- stdout = narration of what happened (`decisions.md`, CLI/UX Conventions). The plan
  is narration → stdout, not stderr.
- Watch the KISS red flags: this must not become a general "landing plan engine".
  If the plan type needs a mode flag or an "and" in its name at birth, it is wrong.

## Acceptance criteria

- [ ] Unit tests for the pure derivation, written red first: fresh start,
      mid-sequence (main landed, develop pending), fully landed, and the
      hotfix fan-out with 2+ open release branches.
- [ ] `keep-release-branches=true` and the tag-deferred/patch-strategy paths do not
      produce a misleading plan (both bump strategies covered).
- [ ] Protected `finish_release` and `finish_hotfix` print the plan; free mode is
      untouched.
- [ ] No new `HostingPlatform` calls on any path (assert it — `MockHosting` records
      calls; a test that pins the hosting call count is the guard against this
      change quietly becoming the expensive version).
- [ ] `bump` and `sync` unchanged.
- [ ] Existing protected call-sequence tests green; any change to them is
      deliberate, explained in the commit body, and preserves the
      no-local-mutation assertions.
- [ ] `cargo test` green; coverage ≥ `.claude/hooks/coverage-baseline.txt`
      (never lower the baseline).

## Docs sync (same change — `CLAUDE.md` requires it)

- [ ] `README.md` — protected-mode section shows one sample plan block.
- [ ] `.claude/skills/bflow/skill.md` — one sentence, concise.
- [ ] `.claude/skills/architecture/decisions.md` — extend the
      "Protected-mode landing progress is derived, never stored" entry: the plan is
      derived from the walk's own knowledge, deliberately **not** re-queried, and
      why (round-trips + a second encoding of leg order).

## Review focus (for the later code review)

1. Count the `HostingPlatform` calls per run, before and after. If the number went
   up, the change took the shape this brief rejects.
2. Is the leg order encoded **once**, or twice (derivation + walk)?
3. Is the plan honest when the run *errors* mid-walk (e.g. `staging_gate` rejects,
   or `tag_at_if_missing` finds a mismatched tag)? A plan printed next to a failure
   must not claim progress that did not happen — or must not print at all.
4. Does the free-mode path stay byte-for-byte identical?
