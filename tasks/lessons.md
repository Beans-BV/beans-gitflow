# Lessons

Patterns that caused rework, and the rule that prevents each. Keep entries
short and prescriptive — this file is read at session start.

Lifecycle: a lesson lives here until it is promoted to a permanent rule in
CLAUDE.md, then its entry is deleted. Promoted so far: skill loading
("Load these first"), plan intent-not-code (Plan Node Default),
consequence-not-shape and mutation verification (Verification Before Done).

---

## Re-load the principle skills between tasks, not once per session

**Correction (2026-08-05):** loading `kiss-principles` / `dry-principles` /
`solid-principles` / `architecture` + CLAUDE.md once at the start of a long
multi-task run is not enough — as context grows, the earliest material is the
first to stop shaping decisions.

**Rule:** on a run with more than a couple of chunks, re-invoke the four skills
and re-read CLAUDE.md between chunks, and always at a phase boundary. When a
skill is already in context the harness answers "instructions unchanged", so the
cost is near zero; when it is not, that is exactly when the reload was needed.
Do not skip it to save tokens — the user has explicitly traded tokens for
quality here.

## A mock is an implementation of the trait

A mock that ignores an argument or returns values the real impl could never
produce is an LSP violation, and it manufactures false confidence: tests pass on
input production cannot generate. Two bugs in this repo traced to exactly that.

**Rule:** whenever a trait method's doc states a guarantee, the mock must model
it or assert it. If production code exists to correct a mock's output, the
relationship is inverted — fix the mock and delete the production code.

## Vector tests are still red-first tests

**Correction (2026-08-05):** when a task's expected call vectors are fully
specified up front (an exact-strings catalog, a task brief's literal
sequence), it is tempting to write the test and the implementation together
"since the answer is already known" — co-designing tests alongside the
implementation happened once in this run. That forfeits the red signal: a
test that never failed never proved it can catch the bug it exists to catch.

**Rule:** even with the exact vector in hand, write the test first and run it
before touching production code. A having-the-answer test still has to fail
for the right reason once. Mutation checks (revert the fix, confirm the test
fails) compensate for a missed red step but don't replace it — they catch
a test that can't fail, not a test that was never proven to fail correctly.

## Re-validate a change request's premise before designing its fix

**Correction (2026-08-08):** handed a change-request document, this run went
straight to designing the best implementation for every request. The user had
to prompt "did you consider denying a change?" — and re-checking CR-01's
premise against the actual binary showed it was false (clap already prints a
guiding "similar subcommand" tip; the claimed "bare invalid value error" does
not exist). The request was rejected as unnecessary.

**Rule:** before planning any fix from a proposal or bug report, reproduce the
claimed bad behavior first. If the premise does not hold, rejecting the
request is a valid — often the best — architectural outcome. A critical
architect reviews the *whether*, not just the *how*; record rejections with
rationale so they are not re-litigated.

## Verify a mechanism's purpose before citing it as a rationale

**Correction (2026-08-08):** the CR-02 wording change justified `sync` being
one-way with "the staging-tag gate exists so unstaged develop content cannot
ride into a release". The gate does nothing of the sort — it counts commits
past the latest RC/patch tag (`rev_list_count`) and has no notion of
`develop`. The invented reason contradicted the gate's own description 120
lines further down the same README, and would not even have held: merging
develop into a release fires the gate, which then tells the user to `bflow
bump`, after which the content rides in legally.

**Rule:** a *why* added to docs is a factual claim about the code and gets the
same scrutiny as code. Before writing "X exists so that Y", read X's
implementation and check for an existing recorded purpose — `decisions.md`
and the README guard blockquote both already stated this one. Reaching for
the most authoritative-sounding nearby mechanism is the failure mode; the
honest answer here was the mundane one (a release branch freezes scope when
it is cut from develop).

## Derive the surface list from the architecture, not from memory

**Correction (2026-08-08):** CR-02 was planned as a wording fix "in three
places" — narration, `--help`, docs — and shipped missing the interactive
menu label, the one surface whose user never reads `--help`. Two reviewers
caught it independently. The same plan also put the narration inside the
free-mode path, where protected mode's early return skips it.

**Rule:** for any user-facing wording change, enumerate surfaces from
architecture principle 8 ("one behavior, two interfaces") — CLI help, menu
label, narration, README, skill — and then walk each *mode branch* of the
flow to confirm the narration is reachable in all of them. A fix that lands
on some surfaces is not a smaller fix; it is an inconsistency.
