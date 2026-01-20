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
obzenflow-fsm = "0.3"
```

You’ll typically also want a Tokio runtime (timeouts use `tokio::time`) and `async-trait` for implementing actions.

## Quick start

```rust
use obzenflow_fsm::{fsm, EventVariant, FsmAction, FsmContext, StateVariant, Transition};

#[derive(Clone, Debug, PartialEq)]
enum State {
    Idle,
    Running,
}

impl StateVariant for State {
    fn variant_name(&self) -> &str {
        match self {
            State::Idle => "Idle",
            State::Running => "Running",
        }
    }
}

#[derive(Clone, Debug)]
enum Event {
    Start,
}

impl EventVariant for Event {
    fn variant_name(&self) -> &str {
        match self {
            Event::Start => "Start",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Action {
    MarkStarted,
}

struct Context {
    started: bool,
}

impl FsmContext for Context {}

#[async_trait::async_trait]
impl FsmAction for Action {
    type Context = Context;

    async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
        match self {
            Action::MarkStarted => {
                ctx.started = true;
                Ok(())
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), obzenflow_fsm::FsmError> {
    let mut machine = fsm! {
        state:   State;
        event:   Event;
        context: Context;
        action:  Action;
        initial: State::Idle;

        state State::Idle {
            on Event::Start => |_s: &State, _e: &Event, _ctx: &mut Context| {
                Box::pin(async move {
                    Ok(Transition {
                        next_state: State::Running,
                        actions: vec![Action::MarkStarted],
                    })
                })
            };
        }
    };

    let mut ctx = Context { started: false };

    let actions = machine.handle(Event::Start, &mut ctx).await?;
    machine.execute_actions(actions, &mut ctx).await?;

    Ok(())
}
```

## Core model (Mealy machine)

`obzenflow-fsm` is a Mealy machine: outputs depend on both the current state and the input event.

* Transition handlers are async and return a `Transition { next_state, actions }`.
* Actions are executed explicitly (often by the same host loop) via `StateMachine::execute_actions`.

This keeps decision-making deterministic and makes side effects auditable.

## Design background (FlowIPs)

The longer “why” and design rationale live in the ObzenFlow improvement proposals:

* `proposals/p0/complete/FLOWIP-057f-fsm-extraction.md`
* `proposals/p0/complete/flowip-fsm-001-mutable-context-timeouts-errors.md`
* `proposals/p0/complete/flowip-fsm-002-builder-dsl.md`

## Testing

```bash
cargo test
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
