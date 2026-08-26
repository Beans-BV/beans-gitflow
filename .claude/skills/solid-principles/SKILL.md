---
name: solid-principles
description: "SOLID principles — SRP single responsibility, OCP open/closed, LSP Liskov substitution, ISP interface segregation, DIP dependency inversion, class design, refactoring toward SOLID"
---

# SOLID Principles

Authoritative reference for SOLID principles as applied in this project. Defines each principle, violation signals, and pragmatic deviations.

**Precedent lives in this repo, not in example files** — every claim below points
at real code. See also `architecture` and its `decisions.md`.

**Related principles:** `kiss-principles` (counterbalance to SOLID ceremony), `dry-principles` (DRY-SRP tension).

## S — Single Responsibility Principle

**"A class should have only one reason to change."** — Robert C. Martin

SRP is NOT "a class should do one thing." It means a class should serve one actor or stakeholder. When two different stakeholders could request changes to the same class, that class has two responsibilities.

### Where SRP Applies Per Layer

| Layer | SRP Means |
|-------|-----------|
| `flows/` | One function = one workflow. `finish_release` does not know about worktrees. |
| `lifecycle.rs` | Cross-cutting ordering only (stash, state, dispatch) — never a workflow's own steps. |
| Adapters | One adapter = one external CLI. `GitCli` never shells out to `gh`. |
| `cli.rs` / `menu.rs` | Turn input into an `Action`. Neither performs the action. |

### Violation Signals

- **Name contains "Manager", "Helper", "Utils", "Processor"** — vague names hide multiple responsibilities
- **Too many injected `&dyn` ports** — see `kiss-principles` for thresholds
- **A flow reaches for a port it only needs on one branch** — that branch is a different responsibility
- **Changes to unrelated features touch the same function**
- **Business logic inside the subprocess/TTY shell**, which is exempt from testing — this is how the exemption stops being honest

## O — Open/Closed Principle

**"Software entities should be open for extension, closed for modification."** — Bertrand Meyer

When new requirements arrive, you should be able to add new behavior by writing NEW code (a new class, a new implementation) rather than modifying EXISTING code.

### Two Mechanisms Here

**A port, when the variation is an external system:**
```
HostingPlatform          ← trait (closed for modification)
├── GitHub    (gh)
└── AzureDevOps (az)     ← adding a third never touches flows/
```

**A table, when the variation is data.** `WORK_TYPES` in `git/branch.rs` is the single source of truth for work-branch kinds; menus, parent detection, `pr_template_keys` and `commit_type` all derive from it, so a new kind is a table row plus the arms the compiler demands.

### Violation Signals

- **A `match` on `BranchType` that a table lookup could answer** — `pr_template_keys` used to be five hand-written arms restating "the key is the kind name"
- **A wildcard arm that silently swallows a new variant** (`_ => None`) where the compiler could have forced a decision
- **Adding one feature requires the same edit in several files**

### When NOT to Apply

- **A closed set that genuinely will not grow** — `main` | `master` is deliberately closed
- **Knowledge-based extraction applies** — see `kiss-principles` for timing

## L — Liskov Substitution Principle

**"Subtypes must be substitutable for their base types."** — Barbara Liskov

If a flow works with `&dyn Git`, it must work identically with `GitCli` and with `MockGit`. **This cuts both ways: the mock is an implementation too.** A mock with a weaker postcondition than the trait promises lets tests pass input production could never produce — that is an LSP violation that manufactures false confidence.

### Violation Signals

- **`todo!()` / `unimplemented!()` in an impl** — it doesn't fulfil the contract
- **Downcasting or type-checking an impl** — callers must not care which one they have
- **Different error semantics per impl** — one returns `Err`, another returns `Ok(false)`
- **Preconditions stricter than the trait promises**
- **Postconditions weaker than the trait promises** — e.g. `MockGit::list_branches_matching` once ignored its pattern, so flows had to re-filter what git already filters; `MockPrompter::prompt_name` once returned names `validate_branch_name` would reject

### The Test: Behavioral Substitutability

Whenever a trait method's doc comment states a guarantee, the mock must assert it (`MockPrompter::prompt_name`) or model it (`MockGit::list_branches_matching` honouring its glob). If an impl needs special handling, either the contract is too broad, or the impl doesn't belong behind this trait.

## I — Interface Segregation Principle

**"No client should be forced to depend on methods it does not use."** — Robert C. Martin

### Violation Signals

- **A trait method with no production caller** — dead weight on every impl and every mock
- **An impl that must fake a method it has no notion of**
- **A new method added for one caller** that every mock now has to answer

### Deliberate Deviation Here

`Git` has 42 methods and is **not** split into role traits. ISP fires nominally, but every method has a production caller, there is one client cluster (`flows/` + `lifecycle.rs`), and one mock. Splitting would multiply `&dyn` parameters at every call site for zero present-tense benefit — `kiss-principles` vetoes it (see the veto order in CLAUDE.md). Fine granularity is itself the decision: it is what lets mocks record exact call sequences.

Split by client need when a *second* client cluster appears, not by arbitrary grouping.

## D — Dependency Inversion Principle

**"Depend on abstractions, not concretions."** — Robert C. Martin

DIP is the backbone of this repo, and it has exactly one shape: **flows take `&dyn Trait`; `main.rs` is the only place that names a concrete adapter.** There is no DI container and no service locator — the composition root passes references down, and detection results (`hosting/detect.rs`, `mainline::resolve_main_branch`) are resolved once there and threaded as data.

### Violation Signals

- **`Command::new(...)` outside an adapter impl or the composition root**
- **A parameter typed `&GitCli` instead of `&dyn Git`**
- **A flow reading git config or the filesystem directly** instead of receiving the resolved value
- **A global or `lazy_static` standing in for an injected dependency**

### When a Concrete Type Is Fine

- **Value types** — `SemVer`, `BranchType`, `Action`, `FinishState` are data, not services
- **Pure helpers** — `validate_branch_name`, `worktree_path`, `template::resolve`
- **The adapters themselves** — `GitCli` naming `Command` is the whole point of an adapter

## SOLID Code Review Checklist

| # | Principle | Check | Severity |
|---|-----------|-------|----------|
| 1 | SRP | Too many injected `&dyn` ports (see `kiss-principles` for thresholds) | Medium |
| 2 | SRP | Name contains "Manager", "Helper", "Utils" | Medium |
| 3 | SRP | Business logic inside the untested subprocess/TTY shell | High |
| 4 | OCP | A wildcard arm that swallows a new `BranchType` variant | High |
| 5 | OCP | A `match` restating what a table already knows | Medium |
| 6 | LSP | A mock with a weaker postcondition than its trait's doc | High |
| 7 | LSP | `todo!()`/`unimplemented!()` in an impl | High |
| 8 | ISP | A trait method with no production caller | Medium |
| 9 | DIP | A parameter typed as a concrete adapter instead of `&dyn` | High |
| 10 | DIP | `Command::new` outside an adapter impl or the composition root | Critical |

## When Pragmatic Deviation Is Acceptable

- **Value types** don't need traits — `SemVer`, `BranchType` are data
- **Single-impl traits** are justified when they cross the port boundary (`Prompter`, `CommandRunner`), even with no second impl planned
- **ISP on `Git`** is knowingly deviated from — see the ISP section for the full argument
- **`unreachable!` for an invariant the type system can't state** is preferred over a silent fallback (`cli.rs`'s `--abort` arm, `lifecycle`'s finish identity)
- For over-engineering detection and complexity calibration, see `kiss-principles`

Document deviations: `// SOLID-DEVIATION: {reason}`

## Principles Work Together

SOLID violations cluster. When you find one, look for its siblings:

| Symptom | Primary Violation | Usually Also |
|---------|-------------------|-------------|
| One function doing two workflows | SRP | ISP — it needs ports neither half uses alone |
| A `match` that a table could answer | OCP | DRY — the table and the match are one piece of knowledge |
| `todo!()` in an impl | LSP | ISP — the trait is too broad for this impl |
| A mock that fakes rather than models | LSP | Tests that pass on input production can't produce |
| A global standing in for a dependency | DIP | SRP — hidden dependencies obscure responsibilities |
| Every new feature edits the same file | OCP | SRP — that file has several reasons to change |

When reviewing code, don't just fix the surface symptom. Trace it to the root principle violation, then check for clustered violations. When principles collide, the veto order is `kiss-principles` → `dry-principles` → `solid-principles`.
