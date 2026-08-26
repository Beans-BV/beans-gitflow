---
name: decision-matrix
description: Evaluate technical coding decisions with a structured comparison matrix. Use this skill ONLY for code-level or architectural solutions — comparing implementation approaches, package structure, state management, API design, or similar engineering choices. The criteria (SOLID, precedent, architecture fit, performance, migration effort, etc.) are optimized for code; do NOT use it for everyday comparisons, prose/prompt wording, process questions, or when the user asks for clarification about a question you posed — answer those in plain prose. Can be used standalone or as part of the brainstorming skill workflow. Use proactively when you're about to recommend one coding approach over alternatives.
---

# Decision Matrix

Structured evaluation of technical solutions against engineering principles and project context.

## When to Use

- Comparing 2+ approaches to a technical coding problem
- Making architectural or structural decisions in code
- Evaluating trade-offs between implementation solutions
- During brainstorming when multiple valid approaches exist
- Before recommending one coding approach over alternatives

## When NOT to Use

- Everyday comparisons that aren't code (tools, wording, process, planning docs)
- The user asks for clarification or more info about a question you posed — answer in plain prose
- Choices with an obvious conventional answer — just state it
- The criteria are code-specific; if most rows would be "— Equal" or "N/A", the decision doesn't fit this skill

## Process

### 1. Understand the Problem

State the decision clearly in one sentence. What are we choosing between and why?

### 2. Describe Each Option

For each option (label A, B, C, etc.):
- **One-line summary** of the approach
- **How it works** in 2-3 sentences
- Include enough detail that someone unfamiliar could understand the trade-off

Aim for 3-6 options. Fewer than 3 means you haven't explored enough. More than 6 means some can be eliminated upfront.

### 3. Check Project Context

Before evaluating, read existing architectural decisions and plans to ensure your analysis is grounded in the project's established patterns:

- Check `docs/superpowers/plans/` for implementation plans
- Check `docs/superpowers/specs/` for design specs
- Check `.claude/skills/architecture/SKILL.md` (Principles, Layer Map) and `.claude/skills/architecture/decisions.md` for layering and dependency rules
- Check `CLAUDE.md` (Architectural Decisions) for the required output of any design choice
- Note which patterns have already been chosen and why

Flag if any option conflicts with or aligns with established decisions. These findings feed the **Precedent** and **Architecture fit** rows in the matrix — collect concrete references (package/file for existing patterns, the specific rule for boundary conflicts) so those cells can cite them.

### 4. Architecture Gate

Hard boundaries are constraints, not trade-offs — an option that violates one must not compete on points. Before building the matrix, screen every option against `.claude/skills/architecture/SKILL.md` and `decisions.md` (ports & adapters, no subprocess calls outside adapters, business logic only in `flows/`, dependency budget, state-before-mutation):

- **Passes** — fully compliant, or allowed-but-discouraged (widening a port trait for one call site, logic in `lifecycle.rs` rather than a flow) → enters the matrix.
- **Violates a hard boundary** → does not enter the matrix. Note the option and the violated rule in one line, then **restore the option count**:
  1. First try a **compliant variant** — restructure the option to respect the boundary while keeping its core idea (move the logic into a `flows/` function taking `&dyn` ports, push the subprocess call down into an adapter, add the primitive to the port trait instead of reaching around it). The variant enters the matrix as its own option.
  2. If no compliant variant exists, **replace it with a fresh approach from a different angle**, so the matrix still compares the same number of genuinely different solutions.

**Exception:** if the decision under evaluation is explicitly about changing the boundary itself, the violating option may enter the matrix, clearly marked as requiring a rule change.

Never leave the pool smaller than it entered — losing an option to the gate must not mean losing an angle on the problem.

### 5. Build the Evaluation Matrix

Evaluate each option against these criteria using emoji indicators:

| Indicator | Meaning |
|-----------|---------|
| ✅ | Good — clear advantage |
| ⚠️ | Caution — trade-off or minor concern, explain briefly |
| ❌ | Bad — significant drawback |
| — | Not applicable or equal across all options |

**Criteria definitions** (evaluate in this order):

| Criterion | What to evaluate |
|-----------|-----------------|
| **KISS** | How simple is it to understand and implement? Could a new developer figure it out quickly? |
| **DRY** | Does it avoid knowledge duplication? (Not code duplication — knowledge duplication) |
| **Precedent** | Is this pattern already used in the codebase? ✅ established — cite an existing example (package/file). ⚠️ new pattern, but nothing existing covers this and the benefit justifies a second way of doing things. ❌ new pattern that duplicates an existing one, or divergence the benefit doesn't justify |
| **Architecture fit** | Records the step 4 gate result and weighs soft boundary-bending. ✅ fully within boundaries; ⚠️ allowed but discouraged (widening a port trait for one call site, logic in `lifecycle.rs` rather than a flow) — a legitimate trade-off, cite the relevant rule; ❌ appears only when the decision is explicitly about changing the boundary itself — hard violations were already gated out in step 4 |
| **SRP** | Does each component have one clear reason to change? |
| **OCP** | Can it be extended without modifying existing code? |
| **LSP** | Can implementations be substituted without breaking consumers? |
| **ISP** | Are interfaces narrow and focused? Do consumers see only what they need? |
| **DIP** | Do high-level modules depend on abstractions, not details? (Evaluate based on project rules, not clean architecture dogma) |
| **Performance** | Runtime cost, memory, rendering speed, cache efficiency |
| **Migration effort** | How much existing code changes? How many consumers affected? Count ONLY code/data that actually exists — including released state files and published tags, which cannot be migrated retroactively. Operational sequencing costs (new runtime/deploy target) go under KISS/Risk instead |
| **Testability** | Can it be tested in isolation? How easy to mock/stub? |
| **Reversibility** | If this turns out wrong, how hard is it to undo or change course? |
| **Risk** | What could go wrong? How likely? How severe? |

**Important:** Add a brief explanation after each emoji — don't just put the emoji alone. The explanation is what makes the matrix useful.

### Matrix Format

```markdown
### Full Evaluation Matrix

| Criterion | A: [name] | B: [name] | C: [name] |
|-----------|-----------|-----------|-----------|
| **KISS** | ✅ Simple, no new concepts | ⚠️ Adds indirection layer | ❌ Complex lifecycle |
| **DRY** | ✅ Single source of truth | — Equal | ⚠️ Two code paths |
| **Precedent** | ✅ Matches `hosting/detect.rs` pure-core/thin-shell split | ⚠️ New pattern, justified: no existing equivalent | ❌ Second way to do what `lifecycle::run` already does |
| **Architecture fit** | ✅ Flow depends only on `&dyn` ports | ✅ Adapter owns the subprocess call | ⚠️ Widens the `Git` trait for one call site |
| **SRP** | ✅ Clear single purpose | ⚠️ Mixed concerns | ✅ Well separated |
...
```

### 6. Recommend

After the matrix, provide three sections:

**Recommendation:** State which option you recommend and the primary reason in one sentence.

**Why [recommended] over [runner-up]:** 
Explain the key differentiator. Focus on the criteria where they diverge most. Reference the project's established patterns if relevant.

**When I'd pick [runner-up]:**
Describe the specific conditions under which the runner-up becomes the better choice. Be concrete — "if X happens" or "when Y is true". This demonstrates you've genuinely considered both sides, not just advocated for your pick.

## Quality Checks

Before presenting the matrix, verify:

1. **Every cell has an explanation** — not just an emoji
2. **N/A is honest** — don't mark things N/A to avoid hard evaluation
3. **Project context checked** — you've read the relevant plans/specs
4. **Options are genuinely different** — if two options are nearly identical, merge them
5. **Recommendation follows from the matrix** — don't recommend something the matrix doesn't support
6. **Precedent and Architecture fit cite specifics** — an existing package/file for ✅ Precedent, the violated rule for ❌ Architecture fit; no vibes-based cells
7. **Gate ran and the pool survived** — every option was screened against hard boundaries in step 4, no ungated violation appears in the matrix, and every gated option was replaced (compliant variant or fresh approach) so the number of angles didn't shrink

## Relationship to planning workflows

This skill is the **method** for one contested decision. Whatever planning skill is driving the work — `superpowers:brainstorming`, `superpowers:writing-plans`, or any successor to them — is the **container** for a change that holds many decisions.

**Nested, never run before.** The matrix consumes the scope, non-goals, and constraints that planning establishes; running it first gates options against constraints that were never set.

- In brainstorming, the nesting point is *"Propose 2-3 approaches"* — that skill identifies the approaches, this one evaluates them, and the recommendation feeds back into its design phase.
- In writing-plans, it is the *"File Structure"* section, where decomposition gets locked in.

Use `decision-matrix` when the decision is structural, hard to reverse, and genuinely contested — all three. Everything else gets a stated trade-off inline.

The standard for what any design choice must output — options, separating axis, one recommendation, reversibility — is owned by the **Architectural Decisions** section of `CLAUDE.md`, not by this skill or any planning skill. This matrix is the heavyweight form of that standard; it does not replace it, and the standard still applies when no matrix is warranted.

## Anti-Patterns

- **Emoji-only cells** — "✅" without explanation is useless
- **All-green recommended option** — if your pick has no downsides, you're not being honest
- **Strawman options** — including obviously bad options just to make your pick look good
- **Shrinking the pool** — gating out a violating option and moving on with fewer candidates; every gated option gets a compliant variant or a replacement approach
- **Ignoring project context** — recommending patterns that contradict established decisions without acknowledging the conflict
- **DIP dogma** — evaluate dependency direction based on the project's actual rules, not theoretical clean architecture. Some projects allow core-to-feature dependencies; check before flagging
