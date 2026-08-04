---
name: kiss-principle
description: "KISS principle — over-engineering detection, simplicity heuristics, complexity anti-patterns, knowledge-based extraction timing, KISS vs DRY tension, KISS vs SOLID balance"
---

# KISS Principle

Authoritative reference for the KISS (Keep It Simple, Stupid) principle as applied in this project. Provides over-engineering detection, simplicity heuristics, and complexity calibration.

**Code examples:** `examples/over-engineering.dart`

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

If the design requires deep context to understand, either the problem genuinely demands that complexity (document the rationale) or the solution has unnecessary indirection. Note: patterns like CQRS or domain events are not over-engineering when the problem warrants them.

### Necessity Test
> "Is there a simpler way that meets CURRENT requirements?"

Emphasis on current. "We might need it later" is not a current requirement.

### Deletion Test
> "If I deleted this abstraction, what concrete problem reappears?"

If you can't name a specific, present-tense problem, the abstraction isn't earning its keep. Note: standard CA layers serve structural consistency and testability — these are present-tense reasons. This test targets extra abstractions *within* layers.

### Explanation Test
> "Can I explain this design in one sentence without using 'flexible', 'extensible', or 'reusable'?"

If the only justification uses future-tense words, the complexity serves a hypothetical need.

## Over-Engineering Anti-Patterns

### Premature Polymorphism

Creating an interface with a single implementation when no layer boundary or test isolation requires it.

**Justified single-implementation interfaces:** layer boundary (Dependency Rule demands it), test isolation, API contract stability between teams.

**Detection:** Interface + single implementation + no layer boundary + no test mock + no cross-team contract = premature polymorphism.

### Lasagna Architecture

Extra layers *beyond the standard CA layer model* that add no logic — a facade between controller and service, a wrapper around a wrapper.

**Important:** This targets layers that don't belong, NOT thin-but-structurally-correct standard layers. A thin Application Service is the correct shape for simple operations.

**Detection:** A class outside standard CA layers where every method delegates to another class with the same signature.

### Premature Abstraction (Wrong-Abstraction Guards)

Extracting a shared abstraction from coincidental similarity — cases that share structure without sharing a meaningful concept.

**Rule (this project — knowledge-based, never count-based):** extract at the SECOND occurrence when the cases share knowledge (same rule/contract, same reason to change) AND the extraction passes three guards: a clean name without "and"/"or", zero boolean flags or mode params at birth, and all callers change for the same reason. Coincidental shape-similarity is never extracted, at any count. One-liners never earn the indirection. Decided mechanisms/infrastructure go further: framework-first at the FIRST occurrence.

**Detection:** a shared abstraction whose name contains "and"/"or", or that needed a flag/mode parameter at birth to serve its callers.

### Configuration Ceremony

Making things configurable that will never be configured. Adding options, flags, and parameters for hypothetical flexibility.

**Detection:** Configuration parameters that have only ever had one value. Generic type parameters instantiated at one concrete type.

### Pattern Worship

Applying a design pattern where a simpler construct suffices. Strategy pattern for two stable variants. Full AsyncNotifier with freezed state for a simple boolean toggle.

**Detection:** The pattern's structural overhead exceeds the logic it contains.

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

- **Yes, layer boundary** → add the interface (Dependency Rule is non-negotiable)
- **Yes, test isolation** → add the interface
- **Yes, API contract stability** → add the interface
- **Yes, 3+ implementations exist** → add the interface
- **No, maybe someday** → don't add it (KISS wins over speculative SOLID)

## KISS Applied to Architecture

### Constructor Injection as Complexity Signal

- **0-5 dependencies** — healthy
- **6-7 dependencies** — review whether responsibilities should split
- **8+ dependencies** — almost certainly an SRP violation

### Choosing the Right Level of Ceremony

The standard CA layers are always present. KISS applies to how much logic lives *within* each layer:

| Complexity | Application Service Shape |
|-----------|----------|
| Simple CRUD, no business rules | Thin pass-through — delegates to repository. Still present for consistency and as a seam. |
| One business rule | Thin service with the rule inline |
| Orchestration across multiple concerns | Full service with injected dependencies |
| Cross-aggregate coordination | Domain Events |

### Start Simple Within the Layers

Keep implementations simple and add ceremony within each layer as complexity grows. A service method that starts as 3 lines of delegation is fine — it's a placeholder in the right place.

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
| 8 | Full AsyncNotifier with freezed state for a boolean toggle | Pattern Worship (Flutter) |
| 9 | 8+ constructor dependencies | Complexity signal (see `solid-principles` SRP) |

## Severity Classification

### High (Should fix before merge)
- Extra forwarding-only layer outside standard CA model that adds zero logic

### Medium (Fix soon)
- Premature abstraction with single usage and no justification
- Design pattern where a conditional suffices
- Wrong abstraction forcing unrelated concerns through shared base

### Low / Suggestions
- Configuration parameter with a single known value
- Opportunity to simplify logic within an existing layer

## Documentation Convention

**`// KISS:` — applying simplicity:**
```
// KISS: Single implementation, no layer boundary — concrete class sufficient
// KISS: 2 stable variants, switch preferred over Strategy pattern
```

**`// KISS-DEVIATION:` — knowingly adding complexity:**
```
// KISS-DEVIATION: Full Strategy pattern justified — 4 payment providers with different auth flows
// KISS-DEVIATION: Generic base class needed — 5 Notifiers share identical pagination logic
```
