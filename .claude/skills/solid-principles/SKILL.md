---
name: solid-principles
description: "SOLID principles — SRP single responsibility, OCP open/closed, LSP Liskov substitution, ISP interface segregation, DIP dependency inversion, class design, refactoring toward SOLID"
---

# SOLID Principles

Authoritative reference for SOLID principles as applied in this project. Defines each principle, violation signals, and pragmatic deviations.

**Code examples:** `examples/violations.md`, `examples/srp-examples.dart`, `examples/ocp-examples.dart`, `examples/lsp-examples.dart`, `examples/isp-examples.dart`, `examples/dip-examples.dart`

**Related principles:** `kiss-principle` (counterbalance to SOLID ceremony), `dry-principles` (DRY-SRP tension).

## S — Single Responsibility Principle

**"A class should have only one reason to change."** — Robert C. Martin

SRP is NOT "a class should do one thing." It means a class should serve one actor or stakeholder. When two different stakeholders could request changes to the same class, that class has two responsibilities.

### Where SRP Applies Per Layer

| Layer | SRP Means |
|-------|-----------|
| **Domain** | One aggregate/entity = one business concept. A `Payment` doesn't manage `Wallet` state. |
| **Application** | One service = one domain aggregate's operations. `PaymentAppService` doesn't handle notifications. |
| **Interface Adapters** | One adapter = one external system. `StripePaymentAdapter` doesn't talk to SendGrid. |
| **Frameworks** | One configuration class = one concern. DB config separate from auth config. |

### Violation Signals

- **Class name contains "Manager", "Helper", "Utils", "Processor"** — vague names hide multiple responsibilities
- **Too many constructor dependencies** — see `kiss-principle` skill for thresholds
- **>7 public methods serving different workflows** — the class is an orchestration hub
- **Changes to unrelated features require modifying the same class**
- **Test file for the class tests unrelated behaviors**

### Relationship to The Helper Trap

Static helpers that disguise orchestration as domain logic are an SRP violation: the class mixes domain state transitions with workflow orchestration (two different reasons to change).

## O — Open/Closed Principle

**"Software entities should be open for extension, closed for modification."** — Bertrand Meyer

When new requirements arrive, you should be able to add new behavior by writing NEW code (a new class, a new implementation) rather than modifying EXISTING code.

### Primary Mechanism: Polymorphism

```
IPaymentProvider  ← interface (closed for modification)
├── StripePaymentProvider   (extension)
├── KulipaPaymentProvider   (extension)
└── NewProvider             (add this — no existing code changes)
```

### Violation Signals

- **Growing switch/if-else chains** that check type or status to determine behavior
- **Modifying existing classes every time a new variant is added**
- **"Shotgun surgery"** — adding one feature requires touching many files in the same way
- **Enum-based dispatch** where each enum value triggers different logic inline

### When NOT to Apply

- **Simple CRUD with no realistic extension points** — don't create an interface hierarchy for a single implementation
- **Stable, well-understood logic** that genuinely won't change
- **Knowledge-based extraction applies** — see the `kiss-principles` skill for extraction timing and premature-abstraction detection

## L — Liskov Substitution Principle

**"Subtypes must be substitutable for their base types."** — Barbara Liskov

If code works with `IPaymentRepository`, it must work identically with ANY implementation. No implementation may surprise its caller.

### Violation Signals

- **`NotImplementedException` or `UnsupportedOperationException`** — the implementation doesn't fulfill the contract
- **Type-checking the implementation** (`if (repo is SqlRepository)`) — code shouldn't care which implementation it has
- **Different error semantics per implementation** — one throws, another returns null, a third returns default
- **Preconditions stricter than the interface promises**
- **Postconditions weaker than the interface promises**

### The Test: Behavioral Substitutability

Write interface contracts as tests. Every implementation must pass the same contract tests. If an implementation needs special handling, either:
1. The interface contract is too broad (split it — ISP)
2. The implementation doesn't belong behind this interface

## I — Interface Segregation Principle

**"No client should be forced to depend on methods it does not use."** — Robert C. Martin

### Violation Signals

- **Interface with >7 methods** serving different client groups
- **Implementations that throw `NotImplementedException`** for some methods
- **"I only use 2 of the 10 methods"** — the client is coupled to 8 irrelevant methods
- **Interface changes break unrelated implementations**
- **Read/write split ignored** — query-only clients forced to depend on mutation methods

### Practical Application

```
// BAD: One fat interface
IUserService { GetUser, UpdateUser, DeleteUser, GetUserPreferences, SendNotification, ResetPassword }

// GOOD: Segregated interfaces
IUserReader { GetUser, GetUserPreferences }
IUserWriter { UpdateUser, DeleteUser }
IUserAuth { ResetPassword }
IUserNotifier { SendNotification }
```

Split by client need, not by arbitrary grouping.

## D — Dependency Inversion Principle

**"Depend on abstractions, not concretions."** — Robert C. Martin

### Class-Level DIP

- Constructor parameters should be interfaces, not concrete classes
- A `PaymentAppService` depends on `IPaymentRepository`, not `SqlPaymentRepository`
- The concrete implementation is resolved by the DI container at runtime

For layer-level DIP (boundary crossing), ensure inner layers never depend on outer layers.

### Violation Signals

- **`new ConcreteClass()` for services** inside application or domain code
- **Constructor parameters typed as concrete classes** instead of interfaces
- **Static method calls to infrastructure** (`Database.Query()`, `HttpClient.Get()`)
- **Service locator pattern** — `ServiceLocator.Get<T>()` hides dependencies

### When a Concrete Dependency Is Acceptable

- **Value objects and DTOs** — `new Money(100, "EUR")` is fine; these are data, not services
- **Domain entities** — `new Payment(...)` is fine; the domain creates its own objects
- **Pure utility types** — `DateTime.UtcNow` (though consider `ITimeProvider` for testability)

## SOLID Code Review Checklist

| # | Principle | Check | Severity |
|---|-----------|-------|----------|
| 1 | SRP | Too many constructor dependencies (see `kiss-principle` for thresholds) | Medium |
| 2 | SRP | Class name contains "Manager", "Helper", "Utils" | Medium |
| 3 | OCP | Growing switch/if-else chain that checks type or variant | Medium |
| 4 | OCP | Adding a variant requires modifying existing classes | High |
| 5 | LSP | Interface implementation with `NotImplementedException` | High |
| 6 | LSP | Type-checking implementations (`is`, `as`, `typeof`) | High |
| 7 | ISP | Interface with >7 methods serving different clients | Medium |
| 8 | ISP | Implementation provides dummy methods it doesn't need | Medium |
| 9 | DIP | `new ConcreteService()` in application/domain code | High |
| 10 | DIP | Static calls to infrastructure from inner layers | Critical |

## When Pragmatic Deviation Is Acceptable

- **Simple value objects** don't need interfaces — `Money`, `Address`, `DateRange` are data, not services
- **Single-implementation interfaces** are justified when they exist for the Dependency Rule (boundary crossing), even if no second implementation is planned
- **Simple CRUD** — a basic GET endpoint that reads and returns data doesn't need full OCP ceremony
- **Prototyping phase** — knowingly violating SOLID during rapid prototyping is fine IF you schedule a cleanup pass
- For over-engineering detection and complexity calibration, see the `kiss-principle` skill

Document deviations: `// SOLID-DEVIATION: {reason}`

## Principles Work Together

SOLID violations cluster. When you find one, look for its siblings:

| Symptom | Primary Violation | Usually Also |
|---------|-------------------|-------------|
| God Service (does everything) | SRP | ISP — fat interface serving all clients |
| Switch/if-else selector | OCP | DIP — depending on concretions to dispatch |
| `NotImplementedException` | LSP | ISP — interface too broad for this implementation |
| Utility class with 15 methods | SRP | ISP — clients use different subsets |
| Service locator pattern | DIP | SRP — hidden dependencies obscure responsibilities |
| Modifying existing code for every new feature | OCP | SRP — class has multiple reasons to change |

When reviewing code, don't just fix the surface symptom. Trace it to the root principle violation, then check for clustered violations.
