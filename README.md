# ObzenFlow FSM

`obzenflow-fsm` is an async-first finite state machine library for Rust, built around a Mealy-machine core and explicit actions.

* Deterministic transitions: `(State, Event, Context) -> (State', Actions)`
* Single-owner mutable context API (`&mut Context`)
* Typed DSL (`fsm!`) plus optional derive helpers
* Timeouts, entry/exit hooks, wildcard/unhandled handling
* Structured errors (`FsmError`) and strict builder validation

## Why this exists

ObzenFlow’s architecture leans heavily on event-sourced finite state machines: keep state evolution deterministic, make effects explicit, and make “what happened” auditable.

This crate was extracted as a standalone library so the FSM engine can be reused independently (it has no dependencies on other ObzenFlow crates).

## Install

```toml
[dependencies]
obzenflow-fsm = "0.3.1"
```

You’ll typically also want a Tokio runtime (timeouts use `tokio::time`) and `async-trait` for implementing actions.

## Quick start

A tiny “door” FSM with explicit actions.

Note: `fsm!` stores handlers behind trait objects, so each handler closure returns a boxed pinned future
(`Box::pin(async move { ... })`).

```rust
use obzenflow_fsm::{fsm, types::FsmResult, FsmAction, FsmContext, Transition};

#[derive(Clone, Debug, PartialEq, obzenflow_fsm::StateVariant)]
enum DoorState {
    Closed,
    Open,
}

#[derive(Clone, Debug, obzenflow_fsm::EventVariant)]
enum DoorEvent {
    Open,
    Close,
}

#[derive(Clone, Debug, PartialEq)]
enum DoorAction {
    Ring,
    Log(String),
}

#[derive(Default)]
struct DoorContext {
    log: Vec<String>,
}

impl FsmContext for DoorContext {}

#[async_trait::async_trait]
impl FsmAction for DoorAction {
    type Context = DoorContext;

    async fn execute(&self, ctx: &mut Self::Context) -> FsmResult<()> {
        match self {
            DoorAction::Ring => ctx.log.push("Ring!".to_string()),
            DoorAction::Log(msg) => ctx.log.push(msg.clone()),
        }
        Ok(())
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> FsmResult<()> {
    let mut door = fsm! {
        state:   DoorState;
        event:   DoorEvent;
        context: DoorContext;
        action:  DoorAction;
        initial: DoorState::Closed;

        state DoorState::Closed {
            on DoorEvent::Open => |_s: &DoorState, _e: &DoorEvent, _ctx: &mut DoorContext| {
                Box::pin(async move {
                    Ok(Transition {
                        next_state: DoorState::Open,
                        actions: vec![
                            DoorAction::Ring,
                            DoorAction::Log("Door opened".into()),
                        ],
                    })
                })
            };
        }

        state DoorState::Open {
            on DoorEvent::Close => |_s: &DoorState, _e: &DoorEvent, _ctx: &mut DoorContext| {
                Box::pin(async move {
                    Ok(Transition {
                        next_state: DoorState::Closed,
                        actions: vec![DoorAction::Log("Door closed".into())],
                    })
                })
            };
        }
    };

    let mut ctx = DoorContext::default();

    let actions = door.handle(DoorEvent::Open, &mut ctx).await?;
    door.execute_actions(actions, &mut ctx).await?;
    assert_eq!(door.state(), &DoorState::Open);

    let actions = door.handle(DoorEvent::Close, &mut ctx).await?;
    door.execute_actions(actions, &mut ctx).await?;
    assert_eq!(door.state(), &DoorState::Closed);

    assert_eq!(
        ctx.log,
        vec![
            "Ring!".to_string(),
            "Door opened".to_string(),
            "Door closed".to_string(),
        ]
    );

    Ok(())
}
```

`obzenflow-fsm` is a Mealy machine: outputs depend on both the current state and the input event.

* Transition handlers are async and return a `Transition { next_state, actions }`.
* Actions are executed explicitly (often by the same host loop) via `StateMachine::execute_actions`.

This keeps decision-making deterministic and makes side effects auditable.

For more examples (timeouts, entry/exit hooks, unhandled handlers, host-loop patterns), see the crate docs on
https://docs.rs/obzenflow-fsm.

## Running in an async runtime (supervisor pattern)

`obzenflow-fsm` is designed to be driven by a host loop (often an actor/supervisor task):

* The host owns the mutable context (`&mut Context`) and controls effect execution.
* `StateMachine::handle(event, &mut ctx)` returns actions; the engine never runs effects implicitly.
* Timeouts are cooperative: call `StateMachine::check_timeout(&mut ctx)` when it makes sense for your runtime.
* Action ordering for a transition is: exit-actions → entry-actions → transition-actions (including self-transitions).

This maps cleanly to “retry actions, map failures into explicit error events, and keep state evolution deterministic”.

## Distributed systems guarantees (the “unholy trinity”)

For outcomes that stay stable under duplicates, interleavings, and reshaping (batching/sharding), the tests are
essentially pointing at the "unholy trinity" of distributed systems failures: fuzzy or broken
idempotence, commutativity, and associativity guarantees. These are sufficient conditions for many dataflow
operators, not universal requirements (some domains are intentionally order-dependent).

In practice:

* **Idempotence** keeps retries/replays from amplifying effects.
* **Commutativity** makes nondeterministic interleavings less scary.
* **Associativity** makes batching/sharding/parallel folding equivalent to sequential application.

## Testing

The test suite is intentionally written as documentation. It tells a story about real failure modes and the
guarantees that keep an async FSM correct under distributed-systems pressure (the “unholy trials”).

It’s also loosely modeled after Dante’s *Divine Comedy*. Think of these failure modes as “circles of hell”.

### The circles of hell (the unholy trials)

* Circle 1: race conditions, shared-state correctness: `tests/circle_1_race_condition.rs`
* Circle 2: async coordination across multiple FSMs: `tests/circle_2_async_coordination.rs`
* Circle 3: journals, subscriptions, causality under concurrency: `tests/circle_3_journal_subscription.rs`
* Circle 4: the unholy trinity vs at-least-once delivery: `tests/circle_4_mathematical_properties.rs`
* Circle 5: timeouts, cancellation, and “never drop data”: `tests/circle_5_timeout_cancellation.rs`
* Circle 6: leaks/cycles/self-reference (the memory corruption gauntlet): `tests/circle_6_memory_corruption.rs`

```bash
cargo test
```

Run one “circle” with output:

```bash
cargo test circle_4 -- --nocapture
```

Other feature-focused tests worth skimming:

* Typed DSL basics and features: `tests/test_dsl_basic.rs`, `tests/test_dsl_features.rs`
* Builder-only construction + validation guarantees: `tests/test_builder_enforcement.rs`, `tests/test_builder_only_construction.rs`
* Compile-time safety and edge coverage: `tests/test_compile_safety.rs`, `tests/test_edge_cases.rs`, `tests/test_comprehensive.rs`

## Project links

* Docs: https://docs.rs/obzenflow-fsm
* Changelog: `CHANGELOG.md`
* Contributing: `CONTRIBUTING.md`

## Project policies

* Security: `SECURITY.md`
* Code of Conduct: `CODE_OF_CONDUCT.md`
* Trademarks: `TRADEMARKS.md`

## License

Dual-licensed under MIT OR Apache-2.0.
