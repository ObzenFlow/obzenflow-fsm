## FLOWIP-FSM-001: Mutable-Context FSM API, Timeouts, and Errors

**Version target**: obzenflow-fsm `0.2.x`  
**Status**: In progress (0.2.x implementation underway)

### Single Mutable-Context API in `obzenflow_fsm`

### Overview

`obzenflow_fsm` currently uses an `Arc<C>`-based API for contexts:

- `StateMachine::handle(&mut self, event, Arc<C>)`
- Transition / entry / exit / timeout / unhandled handlers all receive `Arc<C>`
- `FsmAction::execute(&self, &Context)`

For ObzenFlow and FLOWIP‑080p/080p‑part‑2, the ideal model is:

- Each *host* of the FSM (for example, an ObzenFlow “supervisor” or similar integration component) **owns** its FSM context.
- The FSM and actions mutate that context directly via `&mut Context`.
- Concurrency/sharing is explicit in the host layer, not implicit inside `obzenflow_fsm`.

There are no external users of `obzenflow_fsm`, so we can evolve the library to a **single, `&mut Context` API** and drop the `Arc<C>` context usage entirely.

This document describes that migration.

---

### Semantics: Mealy Machines, Event Sourcing, and FP

Although this design changes *how* state is represented (moving from `Arc<C>` to `&mut Context`), it does **not** change the underlying FSM semantics. The intent remains close to an Erlang/Akka‐style persistent FSM with an event-sourced flavor.

**Mealy machine view**

- Classic Mealy machine:
  - State `S`, input `I`, output `O`
  - Transition: `(S, I) → (S', O)`
- In an event-sourced / persistent FSM:
  - We treat “output” as **events**, and state as the fold of those events.
  - Conceptually:
    - Command handler: `(S, Command) → (Events, S')`
    - Folding events: `(S, Events) → S''` (often `S' == S''` if you apply immediately)

For `obzenflow_fsm`, think of:

- `S` = FSM state enum (e.g., `StatefulState`, `JoinState`, `PipelineState`).
- `C` = context (e.g., `StatefulContext`, `JoinContext`, `PipelineContext`).
- `E` = input event (FSM event enum).
- `A` = actions that produce side effects (journal events, metrics updates, contract checks).

Each FSM step is still a deterministic function:

- `(S, C, E) → (S', C', A*)`

The `&mut Context` API is simply an in-place encoding of this function (Rust lets us update `S` and `C` in place), but the *semantics* remain “immutable state in, new state out” at the logical level.

**Why `&mut Context` is not a semantic regression**

- With `&mut Context`:
  - There is exactly one mutable reference to the context at a time (enforced by Rust).
  - The host/caller owns `(StateMachine, Context)` and drives it step by step.
  - Each call to `handle(event, &mut context)` + subsequent `execute_actions(actions, &mut context)` is a single-threaded Mealy step.
- Under the hood, Akka/Erlang actors also mutate their “current state” variable, even if the state *type* is immutable. The persistent FSM model is about **single-writer, deterministic evolution**, not about never using in-place writes in the runtime.

From a functional and event-sourcing perspective:

- Journals remain the source of truth.
- FSM state is a projection built by folding over journals.
- The “command + state = events + state'” story still holds; we are simply implementing that fold via `&mut` instead of by allocating new values.

**Why the old `Arc<C>` model was less “pure”**

- The `Arc<C>` + `Arc<RwLock<…>>` style encouraged:
  - Shared mutable state (multiple tasks could, in principle, hold `Arc<C>` and mutate `Arc<RwLock<…>>`).
  - Long-lived lock guards across `await` and potential deadlocks.
  - Ambiguity about ownership: is `C` a single FSM state or a shared runtime object?
- The singleton‐context, `&mut` model does the opposite:
  - Enforces **one logical writer** for FSM state.
  - Forces any true sharing to be explicit (via `Arc` around *specific* resources or via channels).
  - Makes it easier to reason about replay and determinism: there is one machine, one state, one event at a time.

In that sense, `&mut Context` is closer to the functional ideal than “shared `Arc<RwLock<Context>>` everywhere”.

### Guardrails to Stay “Functional in Spirit”

To keep the new `&mut Context` API aligned with the persistent FSM / event-sourcing model, we should follow some discipline in how we structure FSM code:

- **Separate decision from effects**
  - Transition handlers (`when(...).on(...)`) should:
    - Look at `(state, event, context)` and return a `Transition<S, A>` deterministically.
    - Avoid direct I/O where possible; push side effects into actions.
  - Actions (`FsmAction::execute(&mut Context)`) perform side effects (journal writes, metrics, contract checks) and update `Context` in response.

- **Single writer, single step**
  - Each call to `StateMachine::handle(event, &mut ctx)` + `execute_actions(actions, &mut ctx)` is the *only* place the FSM state is advanced for that event.
  - Callers should not mutate `Context` behind the FSM’s back in ways that affect semantics.

- **Determinism**
  - Given the same initial `(S, C)` and the same sequence of events, running the FSM should:
    - Visit the same sequence of states.
    - Produce the same actions and journal events.
  - Any non-determinism (e.g., wall-clock timestamps for observability) must be confined to fields that do not affect control flow or contracts.

- **No hidden side-effect channels**
  - Avoid “backdoor” shared state that bypasses the FSM (e.g., global singletons, ad-hoc `Arc<RwLock<…>>` mutated from other tasks).
  - If a resource must be shared (e.g., a journal, message bus, metrics exporter), make that explicit in the context and clearly document its role.

If we follow these guardrails, the Mealy-machine / event-sourcing interpretation remains sound even though we are using `&mut Context` at the implementation level.

---

### Goals

1. **Single, mutable-context FSM API**
   - `StateMachine::handle(&mut self, event, &mut Context)`
   - Handlers and actions use `&mut Context`, not `Arc<Context>`.
2. **Simpler ownership story**
   - FSM contexts are owned by the caller (e.g., supervisors) and not shared implicitly.
3. **No parallel APIs**
   - Remove the `Arc<C>`-based context path from the FSM library; all internal usage moves to `&mut C`.

---

### Proposed API Changes

#### 1. `FsmAction` becomes mutable-context

In `src/types.rs`, change `FsmAction` to operate on `&mut Context`:

```rust
#[async_trait::async_trait]
pub trait FsmAction: Clone + Debug + Send + Sync + 'static {
    type Context: FsmContext;

    async fn execute(&self, ctx: &mut Self::Context) -> Result<(), String>;

    fn describe(&self) -> String {
        format!("{:?}", self)
    }
}
```

All existing action types (e.g., `StatefulAction`, `JoinAction`, `JournalSinkAction`, `PipelineAction`) are being updated to mutate the context via `&mut Self::Context` instead of going through `Arc<RwLock<…>>`. `StateMachine` and `FsmBuilder` have been migrated in code to the `&mut Context` API; remaining work is primarily in tests and builder-time validation (described below).

---

### Timeouts, Self-Transitions, Errors, and Validation

#### Overview

- Clarify runtime semantics for initial-state timeouts and self-transitions in the dynamic-dispatch FSM.
- Adopt structured errors (`FsmError`) across the public API.
- Add builder-time validation to catch configuration mistakes early.
- Improve observability with explicit handling of unhandled events and tracing.

#### Context / Background

- “Initial state” is simply the starting state provided to `FsmBuilder::new(...)`. Any state (including the initial one) can have a timeout if configured via `timeout(...)` in the builder.
- Current behavior leaves `state_timeout = None` on construction, so an initial-state timeout never starts counting down until after some transition occurs.
- Self-transitions are currently treated as no-ops for hooks and timeout refresh.
- Unhandled events without a `when_unhandled` hook are often logged but otherwise treated as success, and errors are surfaced as plain `String`s.

#### Current Pain Points

- **Initial state timeout ignored**: `state_timeout` is never scheduled in `new`, so a configured initial-state timeout never fires until after a transition.
- **Self-transitions are no-ops**: when `next_state == current_state`, entry/exit hooks and timeout refresh are skipped, leading to stale timers and surprising behavior.
- **Silent overwrites**: the builder uses string keys and overwrites duplicate `(state, event)` handlers with no warning.
- **Unstructured errors**: APIs return `Result<_, String>` even though `FsmError` exists, losing context about which handler and which state/event failed.
- **Unhandled events are easy to miss**: an event with no handler and no `when_unhandled` hook does not reliably fail the call, which can hide configuration mistakes.

#### Decisions

1. **Initial-state timeout scheduling**
   - Schedule timeouts in `StateMachine::new` when the initial state has a timeout handler by setting `state_timeout = Instant::now() + duration`.
2. **Self-transition semantics**
   - Treat self-transitions as full transitions (run exit→entry hooks and refresh timeout), not as silent no-ops.
3. **Timeout refreshing**
   - Refresh `state_timeout` on every transition (including self-transitions) based on the resulting state; clear when no timeout is configured.
4. **Builder validation**
   - Error on duplicate `(state, event)` registrations (e.g., `FsmError::DuplicateHandler { state, event }`).
   - Optionally support a “strict mode” to also validate state/event names against allowlists when provided by the caller.
5. **Error surfacing and unhandled events**
   - Public APIs (`handle`, `check_timeout`, `execute_actions`, `build`) return `Result<_, FsmError>`.
   - Map handler/timeout failures to `FsmError::HandlerError`; add variants for `UnhandledEvent`, `Timeout`, `DuplicateHandler`, etc.
   - For unhandled events, prefer failing with `FsmError::UnhandledEvent { state, event }` unless the user installs an explicit `when_unhandled` handler that chooses to return `Ok(())`.
   - Keep precedence: specific handler > wildcard > `when_unhandled` hook.
6. **Telemetry and tracing**
   - Emit structured tracing events for transitions, timeouts firing, handler failures, and unhandled events to aid production debugging.

#### Technical Plan (Timeouts, Errors, Validation)

- **Error plumbing**
  - Update signatures and internal calls to return `FsmError` instead of `String`.
  - Ensure `when_unhandled` failures propagate via a `HandlerError` variant.
- **Initial timeout**
  - In `StateMachine::new`, set `state_timeout` when the initial state has a timeout handler; use `Instant::now() + duration`.
- **Self-transition handling**
  - Implement full hooks + timeout refresh semantics in `apply_transition`, including for helper methods that encode “stay” or “goto” semantics.
- **Builder validation**
  - Track and reject duplicate `(state, event)` inserts at build time with structured errors.
  - Add optional strict mode in the builder for validating known state/event names, while allowing dynamic names in lax mode.
- **Docs & tests**
  - Add tests for: initial-state timeout triggering, self-transition hooks + timeout refresh, duplicate handler rejection, unhandled events with `FsmError`.
  - Update README/CHANGELOG to document semantics, migration notes, and error type changes.

#### Compatibility & Migration Notes (Timeouts/Errors)

- Changing timeout/self-transition semantics may require test updates for existing users; document clearly.
- Switching to `FsmError` changes public signatures; callers need to handle the enum instead of `String`.
- Validation errors at build time will fail fast for previously silent duplicates; highlight in release notes.
- Unhandled events without a `when_unhandled` hook will now fail the call rather than returning `Ok(vec![])`, making configuration mistakes visible.

---

### Impact on ObzenFlow

For each supervisor (stateful, join, sink, pipeline, sources, transforms):

- Replace:

  ```rust
  let context = Arc::new(Context::new(...));
  let actions = machine.handle(event, context.clone()).await?;
  machine.execute_actions(actions, &*context).await?;
  ```

  with:

  ```rust
  let mut context = Context::new(...);
  let actions = machine.handle(event, &mut context).await?;
  machine.execute_actions(actions, &mut context).await?;
  ```

- `HandlerSupervisedExt::run` in ObzenFlow:
  - Stop wrapping the context in `Arc`.
  - Hold `Context` by value in the run loop.
  - Pass `&mut Context` into `StateMachine::handle` and `execute_actions`.

- `FsmAction` implementations:
  - Update `execute(&self, ctx: &mut Context)` bodies to mutate fields directly instead of going through `Arc<RwLock<…>>`.

Supervisors that need genuine cross-task sharing of specific resources will wrap only those resources in `Arc` (or channels); FSM contexts remain single-owner values.

---

### Migration and Validation

- **Internal to `obzenflow_fsm`**
  - Update all tests to use the new signatures:
    - Builder examples (`lib.rs` tests, `builder.rs` tests).
    - `StateMachine` unit tests.
  - Ensure timeouts, entry/exit hooks, wildcard/unhandled behaviors all work with `&mut Context`.

- **In ObzenFlow**
  - Update supervisors and contexts to use the new API.
  - Remove now-unnecessary `Arc<RwLock<…>>` wrappers for purely local state where possible, relying on 080p‑part‑2 for lock-safety guidance.

This yields a single, coherent ObzenFlow FSM API aligned with FLOWIP‑080p’s “FSM owns and mutates its state” design, without a parallel Arc-based path.
