# FLOWIP-FSM-002: Typed Builder DSL for `obzenflow_fsm`

Status: Draft (post‑0.2 direction)  
Authors: obzenflow  
Depends on: `flowip-fsm-001-mutable-context-timeouts-errors.md`

## 1. Problem Statement

The current `obzenflow_fsm` builder API is string‑driven:

- `FsmBuilder::when("StateName")` selects a state by a string.
- `.on("EventName", handler)` selects an event by a string.
- Internally, we key handlers as `HashMap<(String, String), TransitionHandler>`.

The connection between these strings and the actual FSM types is via:

- `StateVariant::variant_name(&self) -> &str`
- `EventVariant::variant_name(&self) -> &str`

This works and aligns with Akka‑style FSMs, but it has drawbacks:

- Typos are runtime bugs (e.g., `"Conected"` silently becomes an unhandled transition).
- Enum refactors are not refactor‑safe; `variant_name()` and string literals must be kept in sync.
- Tools can’t “jump to” string keys; FSM definitions are less discoverable and harder to maintain.

Now that `FLOWIP-FSM-001` has stabilized a single mutable‑context API and clarified Mealy/event‑sourcing semantics, the stringly builder API is the weakest remaining part of the design.

Goal for 002:

- Move toward a **typed, macro‑backed builder DSL** that lets users define FSMs in terms of **enums and patterns**, not bare strings, while preserving the internal semantics and logging/journaling behavior established in 0.2.

## 2. Constraints and Non‑Goals

Constraints:

- Preserve the core semantic model from 001:
  - Mealy‑style transitions `(S, E, C) → (S', C', A*)`.
  - Journals and logs remain string‑named (state/event variant names).
  - `StateMachine` remains single‑writer over `&mut Context`; no reintroduction of shared mutable state in the FSM core.
- Do **not** introduce a second FSM engine; this is a **front‑end** on top of the existing `FsmBuilder`/`StateMachine`.
- Keep existing code workable during transition; we should not break all current users at once.

Non‑goals (for 002):

- No attempt to encode the full FSM type‑state graph at the type level (e.g., compile‑time guarantees that only certain events are valid in certain states).
- No change to the internal handler storage model (still maps keyed by state/event “names”).
- No requirement to eliminate all `when("…").on("…")` usages immediately; they can continue to exist, especially in tests.

## 3. Problem Analysis: Strings vs Types

### 3.1 What strings buy us

- **Simplicity**: The builder API is very lightweight; any `StateVariant`/`EventVariant` can be wired up with `when("Name").on("Name")`.
- **Stability**: Logs, metrics, and journal entries carry human‑readable `variant_name()` strings that are stable even if enum internals change.
- **Flexibility**: We can change the data payloads of states/events without touching the FSM wiring, as long as `variant_name()` stays the same.

### 3.2 What strings cost us

- **Compile‑time safety**:
  - Typos in `"StateName"` or `"EventName"` only show up as runtime unhandled events.
  - Refactoring enum variant names requires manually updating string literals and `variant_name()` implementations.
- **Tooling**:
  - IDEs can’t reliably navigate from strings to enum definitions.
  - Search‑and‑replace refactors are brittle.
- **Cognitive load**:
  - Readers must mentally correlate string names back to enum variants.
  - FSM specs feel less “Rusty” and less in line with the rest of obzenflow’s DSL style.

Given how the rest of obzenflow is evolving (DSLs, macros, schema‑like definitions), these costs are increasingly out of place.

## 4. Design Direction: Macro‑Backed Typed Builder DSL

We choose **Option 3** from previous discussions:

> Keep the internal string‑based handler map, but introduce a **macro‑based, strongly typed front‑end** that lets users define FSMs in terms of enum variants and pattern‑matching, not raw strings.

### 4.1 High‑level shape

At a high level, we introduce a macro (names TBD; examples use `fsm!` and `define_fsm!`) that:

- Accepts:
  - The concrete `State`, `Event`, `Context`, and `Action` types.
  - A set of **states**, each with their **event handlers** and timeout definitions.
- Expands to:
  - Calls to `FsmBuilder::new`, `.when(state_name)`, `.on(event_name, handler)`, `.timeout(duration, handler)`, `.on_entry`, `.on_exit`, and `.when_unhandled` using `variant_name()` internally.

Pseudo‑example:

```rust
fsm! {
    builder: FsmBuilder<PipelineState, PipelineEvent, PipelineContext, PipelineAction>;
    initial: PipelineState::Initializing;

    state PipelineState::Initializing {
        on PipelineEvent::Start => |state, event, ctx| async move {
            // …
            Transition::to(PipelineState::WaitingForStages)
                .with_action(PipelineAction::Log("Starting".into()))
                .build()
        };
    }

    state PipelineState::WaitingForStages {
        on PipelineEvent::StageReady => |state, event, ctx| async move {
            // …
        };

        on PipelineEvent::AllStagesReady => |_, _, ctx| async move {
            // …
        };
    }
}
```

This expands into the existing builder API:

- `when("Initializing")`
- `on("Start", |state, event, ctx| Box::pin(async move { … }))`
- etc.

### 4.2 Type safety gains

With the macro front‑end:

- **State and event names are enum variants**, not strings:
  - `state PipelineState::WaitingForStages` instead of `when("WaitingForStages")`.
  - `on PipelineEvent::BeginShutdown` instead of `on("BeginShutdown", …)`.
- Typos become **compile‑time errors**:
  - `PipelineState::WaitngForStages` will not compile.
  - `PipelineEvent::BeginShutdow` will not compile.
- Refactors are safer:
  - Renaming an enum variant is caught where it’s referenced in the macro, rather than silently turning into a different string.

Internally, `obzenflow_fsm` still operates on string keys derived from `StateVariant::variant_name()` / `EventVariant::variant_name()`, so existing logs/journals remain readable and stable.

## 5. Detailed Design Sketch

### 5.1 Macro responsibilities

The macro (or macros) will:

- Accept a **builder spec**:
  - Selected `State`, `Event`, `Context`, and `Action` types.
  - Initial state.
  - A set of state blocks.
- For each `state` block:
  - Call `builder.when(state_name)` where `state_name` is derived from the variant (via `variant_name()`).
  - Within that, generate `.on(event_name, handler)` for each `on` clause.
  - Optionally generate `.timeout(duration, handler)`, `.on_entry`, and `.on_exit` as needed.
- For unhandled events:
  - Provide an optional `unhandled` handler block that maps to `when_unhandled(...)`.

The macros should be purely syntactic sugar over the existing API:

- No new runtime types.
- No change to `StateMachine` behavior.

### 5.2 Relationship to `StateVariant` / `EventVariant`

To keep the mapping consistent:

- We will continue to rely on `StateVariant::variant_name()` and `EventVariant::variant_name()` to obtain string keys.
- To reduce drift, we should introduce or encourage a **derive**:

```rust
#[derive(StateVariant)]
enum PipelineState {
    Initializing,
    WaitingForStages,
    // …
}

#[derive(EventVariant)]
enum PipelineEvent {
    Start,
    StageReady { stage_id: usize },
    // …
}
```

The derive can implement `variant_name()` by returning a stable string (likely the variant identifier).

The builder DSL macro would then:

- Use `State::variant_name()` on representative values to produce the state name string.
- Use `Event::variant_name()` similarly for event names.

This lets us keep **string names as public API** (for logs, metrics, journals) while letting **user code work in terms of enum variants**.

### 5.3 Concrete `fsm!` syntax sketch

This section sketches a realistic `fsm!` macro for a simple FSM. The goal is to show:

- How users write enum‑centric FSM definitions.
- How that maps to today’s builder API internally.

#### 5.3.1 Example: Door FSM

Current style:

```rust
let fsm = FsmBuilder::<DoorState, DoorEvent, DoorContext, DoorAction>::new(DoorState::Closed)
    .when("Closed")
        .on("Open", |_state, _event, ctx: &mut DoorContext| {
            Box::pin(async move {
                ctx.log.push("Opening".into());
                Ok(Transition {
                    next_state: DoorState::Open,
                    actions: vec![DoorAction::Ring],
                })
            })
        })
        .done()
    .when("Open")
        .on("Close", |_state, _event, ctx: &mut DoorContext| {
            Box::pin(async move {
                ctx.log.push("Closing".into());
                Ok(Transition {
                    next_state: DoorState::Closed,
                    actions: vec![DoorAction::Log("Closed".into())],
                })
            })
        })
        .done()
    .build();
```

Proposed `fsm!` syntax:

```rust
fsm! {
    builder: FsmBuilder<DoorState, DoorEvent, DoorContext, DoorAction>;
    initial: DoorState::Closed;

    state DoorState::Closed {
        on DoorEvent::Open => |state, event, ctx| async move {
            // `state` is &DoorState::Closed, `event` is &DoorEvent::Open
            ctx.log.push("Opening".into());
            Transition::to(DoorState::Open)
                .with_action(DoorAction::Ring)
                .build()
        };
    }

    state DoorState::Open {
        on DoorEvent::Close => |_, _, ctx| async move {
            ctx.log.push("Closing".into());
            Transition::to(DoorState::Closed)
                .with_action(DoorAction::Log("Closed".into()))
                .build()
        };
    }
}
```

Conceptual expansion (simplified):

```rust
{
    let builder = FsmBuilder::<DoorState, DoorEvent, DoorContext, DoorAction>::new(DoorState::Closed);

    let builder = {
        let state_name = DoorState::Closed.variant_name(); // "Closed"
        builder
            .when(state_name)
            .on("Open", |state: &DoorState, event: &DoorEvent, ctx: &mut DoorContext| {
                Box::pin(async move {
                    if let DoorEvent::Open = event {
                        ctx.log.push("Opening".into());
                        Ok(Transition {
                            next_state: DoorState::Open,
                            actions: vec![DoorAction::Ring],
                        })
                    } else {
                        unreachable!()
                    }
                })
            })
            .done()
    };

    let builder = {
        let state_name = DoorState::Open.variant_name(); // "Open"
        builder
            .when(state_name)
            .on("Close", |state: &DoorState, event: &DoorEvent, ctx: &mut DoorContext| {
                Box::pin(async move {
                    if let DoorEvent::Close = event {
                        ctx.log.push("Closing".into());
                        Ok(Transition {
                            next_state: DoorState::Closed,
                            actions: vec![DoorAction::Log("Closed".into())],
                        })
                    } else {
                        unreachable!()
                    }
                })
            })
            .done()
    };

    builder.build()
}
```

Notes:

- The macro remains responsible for:
  - Resolving `DoorState::X` and `DoorEvent::Y` into the correct `variant_name()` strings.
  - Inserting the `Box::pin(async move { … })` wrapper.
  - Providing the typed `(state, event, ctx)` handler parameters.
- The user’s handler body remains plain async Rust with pattern‑matching and access to `&mut Context`.

#### 5.3.2 Example: timeouts and entry/exit hooks

We also want to support timeouts and hooks in the DSL:

```rust
fsm! {
    builder: FsmBuilder<DoorState, DoorEvent, DoorContext, DoorAction>;
    initial: DoorState::Closed;

    state DoorState::Open {
        timeout 5.seconds() => |state, ctx| async move {
            // Auto‑close after 5s
            Transition::to(DoorState::Closed)
                .with_action(DoorAction::Log("Auto‑closed".into()))
                .build()
        };

        on DoorEvent::Close => |_, _, ctx| async move {
            Transition::to(DoorState::Closed)
                .with_action(DoorAction::Log("Closed".into()))
                .build()
        };

        on_entry |state, ctx| async move {
            Ok(vec![DoorAction::Log("Entering Open".into())])
        };

        on_exit |state, ctx| async move {
            Ok(vec![DoorAction::Log("Exiting Open".into())])
        };
    }
}
```

This would expand to:

- `.when("Open").timeout(duration, handler)…`
- `.on_entry("Open", handler)…`
- `.on_exit("Open", handler)…`

using the existing builder APIs.

### 5.4 Gradual adoption

We will **not** remove the stringy API immediately. Instead:

- The macro DSL becomes the **recommended** way to define FSMs for:
  - `obzenflow_runtime_services` pipeline/stage FSMs.
  - New user FSMs.
- The existing builder API (`when("…")`, `on("…")`) remains public and stable:
  - Test code can continue to use it.
  - Power users can still build FSMs dynamically at runtime if needed.

## 6. Alternatives Considered

We explicitly **do not** pursue these options for 002:

1. **Full type‑state FSM encoding**:
   - Encoding allowed transitions in types (e.g., `Fsm<Initialized>` → `Fsm<Active>`).
   - Rejected for now as too heavy and not aligned with dynamic, journal‑driven semantics.

2. **Replacing string keys with numeric IDs internally**:
   - E.g., `StateId(u16)`, `EventId(u16)`.
   - Might be a future optimization, but not necessary to solve the ergonomics and safety problems.
   - Adds complexity around ID generation/versioning and doesn’t fundamentally change the DSL story.

3. **Keeping strings but adding runtime validation only**:
   - E.g., detect when `when("Foo")` is never actually reachable given the enum.
   - Helpful, but still stringly; does not align with obzenflow’s macro/DSL style as well as a proper typed front‑end.

Option 3 (macro DSL) gives us the ergonomics and safety we want with minimal disturbance to the 0.2 core.

## 7. Migration Plan

This FlowIP is explicitly **post‑001**; we assume the mutable‑context API and error model from 0.2 are already in place.

### 7.1 Phase 0 – 001 stabilization (current)

Already done as part of 0.2:

- Single `&mut Context` API.
- Error type `FsmError` and `FsmResult<T>`.
- Tests migrated and passing.

002 starts **after** this baseline is stable.

### 7.2 Phase 1 – Macro + derive prototype

1. Implement `#[derive(StateVariant)]` and `#[derive(EventVariant)]` in a small proc‑macro crate (or in `obzenflow_fsm` if we keep macros local).
2. Implement a first version of the `fsm!`/`define_fsm!` macro:
   - Support a minimal subset: `state`, `on`, and initial state.
   - Expand to current builder usage with `Box::pin` async handlers.
3. Migrate one non‑trivial FSM (e.g., the pipeline FSM in `obzenflow_runtime_services`) to the macro DSL:
   - Keep the old builder usage nearby for reference and potential back‑compat testing.

### 7.3 Phase 2 – Broader adoption in runtime

1. Convert the other core runtime FSMs (stage lifecycle, join, stateful handlers) to the macro DSL.
2. Ensure:
   - Journals and logs show the same state/event names as before.
   - No behavior changes in timeouts, entry/exit hooks, or unhandled handling.
3. Update documentation and examples:
   - Add a macro‑based FSM example alongside the simple builder example.
   - Recommend the macro DSL for all new FSMs.

### 7.4 Phase 3 – Hardening and Lints

1. Add guidance or lints:
   - Consider a feature flag that warns on `when("…")` / `on("…")` in production code (excluding tests).
   - Or provide a `#[deprecated]` style hint on string overloads once all internal users are migrated.
2. Optionally:
   - Add debug‑time checks that the string keys used in builder code correspond to actual enum `variant_name()` values.

At the end of 002, we expect:

- Most real FSMs (runtime, library examples) to be defined with the macro DSL.
- String literals mostly relegated to tests, low‑level adapters, or very dynamic use cases.

## 8. Impact and Compatibility

Backward compatibility:

- Existing string‑based code continues to compile and behave the same.
- No changes to serialized formats or journal entries are required.
- The macro DSL is additive; users can adopt it incrementally.

Forward compatibility:

- Having a single macro front‑end makes it easier to:
  - Enforce style and conventions across FSMs.
  - Layer additional tooling (e.g., static analysis, visualizers) on top of FSM definitions.
  - Potentially introduce typed IDs or more advanced guarantees later, without changing end‑user syntax.

## 9. Summary

`FLOWIP-FSM-001` brought `obzenflow_fsm` to a clean, single‑path mutable‑context Mealy model. The remaining rough edge is the stringly builder API for states and events.

`FLOWIP-FSM-002` proposes:

- A **macro‑backed, strongly typed builder DSL** as the primary way to define FSM behavior, while:
  - Keeping the existing string‑keyed internals and semantics intact.
  - Preserving log/journal friendliness.
  - Aligning with obzenflow’s overall DSL/macro style.

This gives us a path to:

- Better compile‑time safety (no more silent string typos).
- Safer refactors.
- More idiomatic, discoverable, and maintainable FSM definitions.

Implementation is intentionally staged and incremental; we can adopt the DSL in the core runtime first, validate it, and then expand usage across the codebase without disrupting the 0.2 semantics.
