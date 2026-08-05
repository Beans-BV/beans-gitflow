---
name: kiss-principles
description: "KISS principle — over-engineering detection, simplicity heuristics, complexity anti-patterns, knowledge-based extraction timing, KISS vs DRY tension, KISS vs SOLID balance"
---

# KISS Principle

Authoritative reference for the KISS (Keep It Simple, Stupid) principle as applied in this project. Provides over-engineering detection, simplicity heuristics, and complexity calibration.

**Precedent lives in this repo, not in example files** — every claim below points
at real code. See also `architecture` (Layer Map) and its `decisions.md`.

**Related principles:** `solid-principles` (KISS counterbalances SOLID ceremony), `dry-principles` (KISS prevents premature DRY extraction).

## Core Principle

**The simplest solution that meets current requirements wins.**

Complexity has two dimensions:
1. **Too many parts** — unnecessary classes, interfaces, layers, abstractions
2. **Too many interconnections** — excessive coupling, deep dependency chains, indirect communication paths

## Simplicity Heuristics

Four concrete tests to evaluate whether a solution is too complex:

### Comprehension Test
> "Can someone unfamiliar with this code understand its intent within reasonable onboarding time?"

If the design requires deep context to understand, either the problem genuinely demands that complexity (write the rationale into `decisions.md`) or the solution has unnecessary indirection.

### Necessity Test
> "Is there a simpler way that meets CURRENT requirements?"

Emphasis on current. "We might need it later" is not a current requirement.

### Deletion Test
> "If I deleted this abstraction, what concrete problem reappears?"

If you can't name a specific, present-tense problem, the abstraction isn't earning its keep.

The ports (`Git`, `HostingPlatform`, `Editor`, `Prompter`) pass this test on the spot: delete them and flows can no longer run against mocks, which is principle 9. This test targets everything *else* — extra indirection inside `flows/`, inside an adapter, or between `lifecycle.rs` and a flow.

### Explanation Test
> "Can I explain this design in one sentence without using 'flexible', 'extensible', or 'reusable'?"

If the only justification uses future-tense words, the complexity serves a hypothetical need.

## Over-Engineering Anti-Patterns

### Premature Polymorphism

A new trait with one production impl when nothing needs the seam.

**Justified here:** the port/adapter boundary (a real CLI on one side, a mock on the other). `Prompter` has exactly one production impl and earns its keep because `menu::show_menu` is otherwise untestable without a TTY. `CommandRunner`/`CliRunner` are single-impl for the same reason.

**Detection:** trait + one impl + no mock in `tests/common/mod.rs` = premature polymorphism.

### A Layer That Isn't in the Layer Map

The Layer Map has four rows: composition root, lifecycle, interfaces, flows, ports+adapters. Anything between two of them that only forwards is a layer that doesn't belong — a "service" between `lifecycle::run_flow` and a flow, or a wrapper around `GitCli`.

**Important:** thin is not the same as pointless. `flows::push_if_needed` is four lines and correct: the guard IS the logic.

**Detection:** a function outside the Layer Map whose every call forwards to one other function with the same signature.

### Premature Abstraction (Wrong-Abstraction Guards)

Extracting a shared abstraction from coincidental similarity — cases that share structure without sharing a meaningful concept.

**Rule (this project — knowledge-based, never count-based):** extract at the SECOND occurrence when the cases share knowledge (same rule/contract, same reason to change) AND the extraction passes three guards: a clean name without "and"/"or", zero boolean flags or mode params at birth, and all callers change for the same reason. Coincidental shape-similarity is never extracted, at any count. One-liners never earn the indirection. Decided mechanisms/infrastructure go further: framework-first at the FIRST occurrence.

**Detection:** a shared abstraction whose name contains "and"/"or", or that needed a flag/mode parameter at birth to serve its callers.

### Configuration Ceremony

Making things configurable that will never be configured. Adding options, flags, and parameters for hypothetical flexibility.

**Detection:** Configuration parameters that have only ever had one value. Generic type parameters instantiated at one concrete type.

### Pattern Worship

Applying a pattern where a simpler construct suffices: a trait for two stable variants where a `match` reads better; a builder for a struct with three fields; a params struct where the argument list was the problem.

**Detection:** the pattern's structural overhead exceeds the logic it contains. Precedent: `run_flow` keeps 10 plain parameters and an `#[allow]` rather than growing a params struct.

## The KISS-DRY Tension

When KISS and DRY conflict, **prefer KISS until the duplication becomes a maintenance risk**. For wrong abstraction detection and recovery, see the `dry-principles` skill.

### Decision Table

| Situation | Action |
|-----------|--------|
| 2 similar blocks | Duplicate — too early to know if they share a concept |
| 3+ identical blocks | Abstract — the pattern is established |
| Similar structure, different business reasons | Duplicate — they will diverge (see `dry-principles` for coincidental similarity) |
| Varies by multiple dimensions | Duplicate — shared abstraction becomes configuration nightmare |

## The KISS-SOLID Balance

SOLID ceremony is justified when the problem demands it. It's over-engineering when it exceeds the problem's complexity. See `solid-principles` for when deviation from specific SOLID principles is acceptable.

### The Balance Point
> "Is there a concrete, present-tense reason for this abstraction?"

- **Yes, it crosses the port boundary** (subprocess, network, terminal, clock) → add the trait
- **Yes, a flow cannot be tested without it** → add the trait
- **Yes, 2+ real impls exist** (`GitHub` / `AzureDevOps`) → add the trait
- **No, maybe someday** → don't add it (KISS wins over speculative SOLID)

## KISS Applied to Architecture

### Injected `&dyn` Ports as a Complexity Signal

Counting only the port parameters a function takes:

- **0-3 ports** — healthy (`finish_work_branch` takes 3)
- **4-5 ports** — ask whether the function is orchestrating two things
- **6+ ports** — almost certainly an SRP violation

Plain data parameters are a *separate* smell; `run_flow`'s ten are tolerated because it is the dispatch table itself, and the `#[allow]` says so out loud.

### Choosing the Right Level of Ceremony

Where code goes is decided by the Layer Map. KISS decides how much machinery lives inside it:

| Situation | Shape |
|---|---|
| One git call with a guard | A free function in `flows/mod.rs` (`push_if_needed`) |
| A multi-step branch lifecycle | A flow function in `flows/` taking `&dyn` ports |
| Needs a subprocess or the terminal | A port + adapter, never inline |
| Needs to survive a crash mid-way | State in `state.rs`, written before the first side effect |

### Start Simple

A flow that starts as three git calls in sequence is finished code, not a placeholder. Add structure when a second caller or a resume path actually arrives.

## Red Flags Checklist

| # | Red Flag | Likely Anti-Pattern |
|---|----------|-------------------|
| 1 | Interface with exactly one implementation (no layer boundary) | Premature Polymorphism |
| 2 | Generic type parameter instantiated at one concrete type | Configuration Ceremony |
| 3 | Abstract base class with one subclass | Premature Abstraction |
| 4 | Extra layer where every method delegates to another class | Lasagna Architecture |
| 5 | Design pattern where a conditional or direct call suffices | Pattern Worship |
| 6 | Configuration parameter that has only ever had one value | Configuration Ceremony |
| 7 | Generic base class shared by 2 unrelated concepts | Wrong Abstraction (see `dry-principles`) |
| 8 | A `--flag` or config key with exactly one value ever used | Configuration Ceremony |
| 9 | 6+ `&dyn` port parameters on one function | Complexity signal (see `solid-principles` SRP) |

## Severity Classification

### High (Should fix before merge)
- A forwarding-only layer that is not in the Layer Map
- A subprocess call outside an adapter impl (also an architecture violation)

### Medium (Fix soon)
- Premature abstraction with single usage and no justification
- Design pattern where a conditional suffices
- Wrong abstraction forcing unrelated concerns through shared base

### Low / Suggestions
- Configuration parameter with a single known value
- Opportunity to simplify logic inside an existing flow or adapter

## Documentation Convention

**`// KISS:` — applying simplicity:**
```
// KISS: two stable variants — a match beats a trait here
// KISS: one impl and no mock needed — a plain fn, not a port
```

**`// KISS-DEVIATION:` — knowingly adding complexity:**
```
// KISS-DEVIATION: CommandRunner exists so GitCli's exit-code semantics are testable
```

Anything load-bearing enough to argue about belongs in `decisions.md`, not only in a comment.
