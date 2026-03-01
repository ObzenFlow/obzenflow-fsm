# ObzenFlow FSM

`obzenflow-fsm` is an async-first finite state machine library for Rust, built around deterministic transitions and explicit actions.

* Deterministic transitions: `(State, Event, Context) -> (State', Actions)`
* Single-owner mutable context API (`&mut Context`)
* Typed DSL (`fsm!`) plus optional derive helpers
* Timeouts, entry/exit hooks, wildcard/unhandled handling
* Structured errors (`FsmError`) and strict builder validation

This crate is inspired by Akka / Apache Pekko (Classic) FSM patterns and the [`edfsm`](https://docs.rs/edfsm) crate. If you need a `no_std` event-driven FSM (and an attribute-macro DSL), `edfsm` is an excellent choice. If you need an open source FSM solution on the JVM, [`Apache Pekko`](https://github.com/apache/pekko) is worth considering. Both are more mature, general-purpose options; `obzenflow-fsm` is intentionally niche and shaped around async Tokio host loops and explicit effect execution.

## Why this exists

ObzenFlow’s architecture leans heavily on event-sourced finite state machines that keeps state evolution deterministic, make effects explicit, and make “what happened” auditable.

This crate was extracted as a standalone library so the FSM engine can be reused independently (it has no dependencies on other ObzenFlow crates).

## Install

```toml
[dependencies]
obzenflow-fsm = "0.3.2"
```

You’ll typically also want a Tokio runtime (timeouts use `tokio::time`) and `async-trait` for implementing actions.

## Quick start

A tiny “door” FSM with explicit actions.

Note: `fsm!` stores handlers behind trait objects, so each handler closure returns a boxed pinned future (`Box::pin(async move { ... })`).

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

## Design

In ObzenFlow ([obzenflow.dev](https://obzenflow.dev)), `obzenflow-fsm` powers effect execution and state management by keeping state evolution deterministic and making effects explicit via actions.

* Transition handlers are async and return a `Transition { next_state, actions }`.
* `StateMachine::handle(event, &mut ctx)` returns actions; the engine never runs effects implicitly.
* Actions are executed explicitly (often by the same host loop) via `StateMachine::execute_actions`.
* Timeouts are cooperative: call `StateMachine::check_timeout(&mut ctx)` when it makes sense for your runtime.
* Action ordering for a transition is: exit-actions → entry-actions → transition-actions (including self-transitions).

For more examples (timeouts, entry/exit hooks, unhandled handlers, host-loop patterns), see the crate docs on https://docs.rs/obzenflow-fsm.

## Testing

The test suite is organised around the “circles of distributed systems hell”, six numbered integration tests that target real failure modes in async, event-sourced state machines. Each circle maps to a dimension in ObzenFlow’s [CHAIN maturity model](https://obzenflow.dev/philosophy/chain/).

See [`tests/README.md`](tests/README.md) for the full breakdown.

```bash
cargo test
```

Run one circle with output:

```bash
cargo test circle_4 -- --nocapture
```

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
