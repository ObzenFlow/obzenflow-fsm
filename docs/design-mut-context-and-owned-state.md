## Design: Single Mutable-Context API in `obzenflow_fsm`

### Overview

`obzenflow_fsm` currently uses an `Arc<C>`-based API for contexts:

- `StateMachine::handle(&mut self, event, Arc<C>)`
- Transition / entry / exit / timeout / unhandled handlers all receive `Arc<C>`
- `FsmAction::execute(&self, &Context)`

For FlowState RS and FLOWIP‑080p/080p‑part‑2, the ideal model is:

- Each supervisor **owns** its FSM context.
- The FSM and actions mutate that context directly via `&mut Context`.
- Concurrency/sharing is explicit at the supervisor layer, not implicit inside `obzenflow_fsm`.

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
  - The supervisor owns `(StateMachine, Context)` and drives it step by step.
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

### Current API (simplified)

Relevant APIs today:

- `StateMachine` (`src/machine.rs`):
  - `pub async fn handle(&mut self, event: E, context: Arc<C>) -> Result<Vec<A>, String>`
  - `pub async fn check_timeout(&mut self, context: Arc<C>) -> Result<Vec<A>, String>`
  - `async fn apply_transition(&mut self, transition: Transition<S, A>, context: Arc<C>) -> Result<Vec<A>, String>`
  - `pub async fn execute_actions(&self, actions: Vec<A>, context: &C) -> Result<(), String>`
- `FsmBuilder` (`src/builder.rs`):
  - Transition handlers: `Fn(&S, &E, Arc<C>) -> Fut`
  - Entry/exit handlers: `Fn(&S, Arc<C>) -> Fut`
  - Timeout handlers: `Fn(&S, Arc<C>) -> Fut`
  - Unhandled handler: `Fn(&S, &E, Arc<C>) -> Fut`
- Traits (`src/types.rs`):
  - `pub trait FsmContext { fn describe(&self) -> String { … } }`
  - `pub trait FsmAction { type Context: FsmContext; async fn execute(&self, ctx: &Self::Context) -> Result<(), String>; }`

Runtimes (e.g., FlowState RS) wrap contexts in `Arc` before passing them into the FSM.

---

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

All existing action types (e.g., `StatefulAction`, `JoinAction`, `JournalSinkAction`, `PipelineAction`) will update their implementations accordingly to mutate the context via `&mut Self::Context` instead of going through `Arc<RwLock<…>>`.

#### 2. `StateMachine` methods take `&mut Context`

In `src/machine.rs`, migrate to `&mut C`:

- Signature changes:

```rust
impl<S, E, C, A> StateMachine<S, E, C, A>
where
    S: StateVariant,
    E: EventVariant,
    C: FsmContext,
    A: FsmAction<Context = C>,
{
    pub async fn check_timeout(&mut self, context: &mut C) -> Result<Vec<A>, String> { … }

    pub async fn handle(&mut self, event: E, context: &mut C) -> Result<Vec<A>, String> { … }

    async fn apply_transition(
        &mut self,
        transition: Transition<S, A>,
        context: &mut C,
    ) -> Result<Vec<A>, String> { … }

    pub async fn execute_actions(
        &self,
        actions: Vec<A>,
        context: &mut C,
    ) -> Result<(), String> {
        for action in actions {
            action.execute(context).await?;
        }
        Ok(())
    }
}
```

- Internal handler calls (transition, entry, exit, timeout, unhandled) will also be changed to receive `&mut C` instead of `Arc<C>`.

#### 3. `FsmBuilder` handlers use `&mut Context`

In `src/builder.rs`, update handler types:

- Transition handlers from:

```rust
F: Fn(&S, &E, Arc<C>) -> Fut
```

to:

```rust
F: Fn(&S, &E, &mut C) -> Fut
```

- Entry/exit handlers from:

```rust
F: Fn(&S, Arc<C>) -> Fut
```

to:

```rust
F: Fn(&S, &mut C) -> Fut
```

- Timeout handlers from:

```rust
F: Fn(&S, Arc<C>) -> Fut
```

to:

```rust
F: Fn(&S, &mut C) -> Fut
```

- Unhandled handler from:

```rust
F: Fn(&S, &E, Arc<C>) -> Fut
```

to:

```rust
F: Fn(&S, &E, &mut C) -> Fut
```

`FsmBuilder::build` continues to construct a single `StateMachine<S, E, C, A>`; no new types are introduced.

#### 4. Remove `Arc<C>` from FSM internals

- `StateMachine` internal fields (transition maps, handlers) no longer require `Arc<C>` in their function signatures.
- The only remaining `Arc` usage inside the FSM library will be around handler *closures* themselves (to allow cloning, as today), not around the context.

---

### Impact on FlowState RS

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

- `HandlerSupervisedExt::run` in FlowState RS:
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

- **In FlowState RS**
  - Update supervisors and contexts to use the new API.
  - Remove now-unnecessary `Arc<RwLock<…>>` wrappers for purely local state where possible, relying on 080p‑part‑2 for lock-safety guidance.

This yields a single, coherent FSM API aligned with FLOWIP‑080p’s “FSM owns and mutates its state” design, without a parallel Arc-based path.
