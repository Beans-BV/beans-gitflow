---
name: dry-principles
description: "DRY (Don't Repeat Yourself) — knowledge duplication vs coincidental similarity, when duplication is correct, Wrong Abstraction anti-pattern, cross-service contract DRY, DAMP testing"
---

# DRY Principles

Authoritative reference for the DRY principle as applied in this project. Defines knowledge duplication vs coincidental similarity, when duplication is correct, the Wrong Abstraction lifecycle, and DAMP testing.

**Code examples:** `examples/real-vs-coincidental.dart`, `examples/wrong-abstraction.dart`, `examples/violations.md`, `examples/damp-tests.dart`

**Related principles:** `solid-principles` (SRP-DRY tension when actors differ), `kiss-principles` (counterforce to premature extraction — see `kiss-principles` for the knowledge-based extraction guards and timing).

## The DRY Principle — Correctly Defined

> "Every piece of **knowledge** must have a single, unambiguous, authoritative representation within a system." — *The Pragmatic Programmer*, Hunt & Thomas

DRY is about **knowledge**, not **code**. Two identical-looking code blocks are not necessarily a DRY violation. Two different-looking code blocks can be a DRY violation if they encode the same business rule.

The key question: **"If this business rule changes, how many places do I need to update?"** If the answer is more than one, you have a DRY violation — regardless of whether the code looks similar.

## Three Types of Similarity

### 1. Knowledge Duplication (Semantic — Extract)

The same business rule or decision encoded in multiple places. When the rule changes, all copies must change together.

**Heuristic:** These change for the **same reason** at the **same time**.

**Example:** Fee calculation logic in both `PaymentApplicationService` and `TransferApplicationService`. When the fee structure changes, both must update — this is one piece of knowledge in two places.

### 2. Coincidental Similarity (Syntactic — Keep Separate)

Code that looks similar today but represents different business concepts. They will diverge as requirements evolve.

**Heuristic:** These change for **different reasons** or at **different times**.

**Example:** A `CreatePaymentInput` and `CreateTransferInput` both have `Amount`, `Currency`, and `Description` fields. They look identical, but payments and transfers are different business concepts with different validation rules that will evolve independently.

### 3. Structural Similarity (Pattern — Document)

Repeated code structure that follows a convention or pattern (e.g., all Application Services have similar constructor injection). This is intentional consistency, not duplication.

**Heuristic:** These change when the **pattern itself** changes, not when individual business rules change.

### The Decision Heuristic

> **"Do these change for the same reason?"**

| Answer | Action |
|--------|--------|
| Yes, same business rule | Extract — this is knowledge duplication |
| No, different business concepts | Keep separate — this is coincidental similarity |
| Same pattern/convention | Document the pattern, don't abstract the instances |

See `examples/real-vs-coincidental.dart` for concrete scenarios.

## When Duplication Is Correct

Duplication is not always wrong. In these cases, the coupling cost of sharing exceeds the duplication cost:

### Across Architectural Layers

DTOs, entities, and view models may have similar fields but serve different layers. Merging them creates tight coupling between layers.

**Example:** `PaymentEntity` (Domain), `PaymentDto` (Application), `PaymentResponseDto` (API), `PaymentState` (Flutter) — four representations of "payment" is correct, not duplication.

**Mapper code between these representations is also structural, not duplicative.** Don't extract a generic mapper — each boundary has its own evolution path.

### Across Bounded Contexts

Different bounded contexts own their own models. Sharing models between contexts creates tight coupling that's worse than duplication.

**Example:** Two separately deployed services are separate products. Duplicating a shared model in both is correct.

### When Coupling Cost Exceeds Duplication Cost

Small-scale duplication within a bounded context is sometimes preferable to a shared abstraction that creates coupling between unrelated features. See the `kiss-principles` skill for extraction timing (knowledge-based, with wrong-abstraction guards) and the KISS-DRY decision table.

### Naming the Concept (Before Extracting)

If you can't name the shared concept better than "SharedHelper", "CommonUtils", or "BaseProcessor", you haven't found the abstraction yet. A good extraction has a name that describes the **knowledge** being shared, not the **code structure**.

## The Wrong Abstraction

> "Duplication is far cheaper than the wrong abstraction." — Sandi Metz

### The Lifecycle of Abstraction Decay

1. **Two similar cases appear.** Developer extracts shared code. Feels good.
2. **Third case is slightly different.** Add a boolean parameter. Manageable.
3. **Fourth case needs another variation.** Add another parameter. Getting complex.
4. **Fifth case is an edge case.** Add a conditional branch. Now the shared code is harder to understand than the duplication was.
5. **Nobody dares touch it.** The abstraction has become load-bearing — everyone depends on it, nobody understands it.

### Red Flags of a Wrong Abstraction

| Signal | What It Means |
|--------|---------------|
| Boolean parameters controlling behavior | The abstraction serves multiple concepts |
| Growing conditional chains inside shared code | Cases are diverging, not converging |
| Callers passing `null` or empty values for unused parameters | Interface is too broad for some callers |
| "I need to understand all callers to change this" | Coupling exceeds the value of sharing |
| Shared base class where subclasses override most methods | Inheritance serving code reuse, not "is-a" |
| Comments like "// only used by X" inside shared code | The sharing is no longer symmetric |

### The Fix: Inline and Re-Extract

1. **Inline** the shared code back into each caller
2. **Accept the temporary duplication** — this is healthy intermediate state
3. **Let the natural groupings emerge** — with all code visible, the real abstractions become clear
4. **Re-extract** only the genuinely shared knowledge (if any exists)

Do NOT try to "fix" a wrong abstraction by adding more parameters or conditionals.

See `examples/wrong-abstraction.dart` for detailed lifecycle examples.

## DRY Violations to Watch For

### Shotgun Surgery
A single business rule change requires modifying multiple files. The knowledge is scattered.

### Business Rules in Multiple Places
The same validation, calculation, or business decision implemented in more than one location.

### Magic Numbers
Literal values scattered through the codebase representing a single business decision. `3600` (token expiry), `0.015m` (fee percentage), `3` (max retry) appearing in multiple files without a named constant.

### Copy-Paste with Minor Variations
Nearly identical blocks of code where the differences are incidental, not intentional.

### Parallel Hierarchies
Two class hierarchies that mirror each other and must be updated in lockstep.

## DRY vs SRP Tension

When DRY and SRP conflict, **SRP wins when actors differ**.

The same logic serving different stakeholders is coincidental similarity, not knowledge duplication. Even if the code is identical today, different stakeholders will drive divergence.

**Example:** Fee calculation for customer-facing payments vs fee calculation for internal reconciliation reports. Same formula today, but the customer-facing calculation might add promotional discounts while reconciliation stays on raw fees.

Cross-reference: `solid-principles` skill, SRP section.

## DRY vs Loose Coupling

| Scope | Action | Rationale |
|-------|--------|-----------|
| Within a class | Extract method | Zero coupling cost |
| Within a module/feature | Extract to shared class in same module | Low coupling cost |
| Across features in same product | Shared module with clear ownership | Medium coupling cost — worth it for business rules |
| Across separately deployed services | **Keep duplicated** | High coupling cost — shared library creates deployment dependency |
| Across bounded contexts | **Keep duplicated** | Very high coupling cost — different models, different evolution |

## Cross-Service DRY (Enterprise)

### Separate Products: Accept Duplication

Separately deployed services with independent bounded contexts should accept duplication. Sharing models between them creates deployment coupling and version synchronization burden.

### Within One Product: Single Source of Truth

Within a single product, shared constants and business rules should have one authoritative source.

### Frontend-Backend Contract Sync

The mobile app, portal, and backend are one product. The contract is the **single source of truth**: database types come from `supabase gen types typescript` into `packages/api-types`, and Zod schemas defined once in that package serve backend validation (authoritative) and client-side UX validation (advisory). Clients never hand-write their own model or validation of a server concept — when client and server rules differ, that's a DRY violation.

See `examples/real-vs-coincidental.dart` for further examples (code samples predate this project's stack; the principles transfer).

## DAMP in Tests

> DAMP: Descriptive And Meaningful Phrases

In test code, **DRY the "how" (infrastructure), allow duplication in the "what" (scenarios)**.

### What to DRY in Tests
- Test infrastructure: builders, factories, fixtures, setup helpers
- Assertion helpers: custom matchers for domain-specific checks
- Mock configuration: shared mock setups for common dependencies

### What to Allow Duplication In
- Test scenarios: each test should tell a complete story
- Arrange sections: explicit setup makes preconditions visible
- Assert sections: explicit assertions make expected outcome visible

Test code optimizes for **readability at the individual test level**, not for minimizing total lines.

See `examples/damp-tests.dart` for DAMP testing examples.

## DRY Code Review Checklist

| # | Check | Severity |
|---|-------|----------|
| 1 | Same business rule in multiple places | High |
| 2 | Magic numbers without named constants | Medium |
| 3 | Copy-paste with minor variations | Medium |
| 4 | Growing conditionals in shared code | High |
| 5 | Boolean parameters on shared methods | Medium |
| 6 | Frontend/backend validation drift | High |
| 7 | Shared library between separately deployed services | Medium |

## Documentation Convention

**`// DRY:` — applying the principle** (explaining why duplication is kept):
```
// DRY: Coincidental similarity — payments and transfers evolve independently
// DRY: Separate bounded contexts — duplication accepted over shared library coupling
```

**`// DRY-DEVIATION:` — knowingly violating** (duplicating knowledge):
```
// DRY-DEVIATION: Fee calculation duplicated in reconciliation job — scheduled for extraction
// DRY-DEVIATION: Validation rules duplicated client/server — API contract generation not yet available
```
