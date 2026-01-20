//! Async-first finite state machine (FSM) library inspired by Akka (Classic) FSM and
//! [`edfsm`](https://docs.rs/edfsm).
//!
//! `obzenflow-fsm` implements a small **Mealy-machine** core:
//! `State(S) × Event(E) → Actions(A), State(S')`.
//!
//! The engine is intentionally minimal:
//! - Transition handlers are async and return a [`Transition`].
//! - Actions are executed explicitly by the host via [`FsmAction::execute`].
//! - The [`StateMachine`] stores only the current state; the mutable context lives outside.
//!
//! This split makes it easy to build deterministic state evolution while keeping side effects
//! explicit and auditable (e.g. write-to-journal, publish-to-bus, spawn tasks).
//!
//! ## Key Differences vs `edfsm`
//!
//! [`edfsm`](https://docs.rs/edfsm) is an event-driven FSM with a `Command`/`Event` split and a
//! separate effect handler; it's `no_std` and keeps effects synchronous by design.
//!
//! `obzenflow-fsm` makes different trade-offs for async supervisor-style runtimes:
//! - **Async-first**: handlers and actions are async (Tokio-friendly).
//! - **Explicit effects**: transitions return `Vec<Action>`; effects run only when the host calls
//!   [`FsmAction::execute`].
//! - **Single input type**: there is one `Event` type; "commands" can be modeled as event variants.
//! - **Runtime hooks**: entry/exit handlers and cooperative per-state timeouts are built into
//!   [`StateMachine`].
//!
//! If you're coming from `edfsm`, a rough mapping is:
//! - `Command`/`Event` inputs → a single `Event` enum (external commands and internal signals as
//!   variants).
//! - The effect handler (`SE`) → an `Action` enum executed against your runtime `Context`.
//!
//! For replay-style flows, keep transition handlers free of external side effects and execute
//! actions only in "live" mode.
//!
//! If you want a synchronous, event-sourcing-oriented FSM core where `Command -> Event -> State`
//! is the primary model, `edfsm` is an excellent choice. If you want async actions and an explicit
//! supervisor/host-loop integration style, this crate is optimized for that.
//!
//! ## Quick start
//!
//! A tiny "door" FSM with explicit actions:
//!
//! ```rust
//! use obzenflow_fsm::{fsm, FsmAction, FsmContext, Transition};
//!
//! #[derive(Clone, Debug, PartialEq, obzenflow_fsm::StateVariant)]
//! enum DoorState {
//!     Closed,
//!     Open,
//! }
//!
//! #[derive(Clone, Debug, obzenflow_fsm::EventVariant)]
//! enum DoorEvent {
//!     Open,
//!     Close,
//! }
//!
//! #[derive(Clone, Debug, PartialEq)]
//! enum DoorAction {
//!     Ring,
//!     Log(String),
//! }
//!
//! #[derive(Default)]
//! struct DoorContext {
//!     log: Vec<String>,
//! }
//!
//! impl FsmContext for DoorContext {}
//!
//! #[async_trait::async_trait]
//! impl FsmAction for DoorAction {
//!     type Context = DoorContext;
//!
//!     async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
//!         match self {
//!             DoorAction::Ring => ctx.log.push("Ring!".to_string()),
//!             DoorAction::Log(msg) => ctx.log.push(msg.clone()),
//!         }
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<(), obzenflow_fsm::FsmError> {
//!     let mut door = fsm! {
//!         state:   DoorState;
//!         event:   DoorEvent;
//!         context: DoorContext;
//!         action:  DoorAction;
//!         initial: DoorState::Closed;
//!
//!         state DoorState::Closed {
//!             on DoorEvent::Open => |_s: &DoorState, _e: &DoorEvent, _ctx: &mut DoorContext| {
//!                 Box::pin(async move {
//!                     Ok(Transition {
//!                         next_state: DoorState::Open,
//!                         actions: vec![DoorAction::Ring, DoorAction::Log("Door opened".into())],
//!                     })
//!                 })
//!             };
//!         }
//!
//!         state DoorState::Open {
//!             on DoorEvent::Close => |_s: &DoorState, _e: &DoorEvent, _ctx: &mut DoorContext| {
//!                 Box::pin(async move {
//!                     Ok(Transition {
//!                         next_state: DoorState::Closed,
//!                         actions: vec![DoorAction::Log("Door closed".into())],
//!                     })
//!                 })
//!             };
//!         }
//!     };
//!
//!     let mut ctx = DoorContext::default();
//!
//!     let actions = door.handle(DoorEvent::Open, &mut ctx).await?;
//!     door.execute_actions(actions, &mut ctx).await?;
//!     assert_eq!(door.state(), &DoorState::Open);
//!
//!     let actions = door.handle(DoorEvent::Close, &mut ctx).await?;
//!     door.execute_actions(actions, &mut ctx).await?;
//!     assert_eq!(door.state(), &DoorState::Closed);
//!
//!     assert_eq!(
//!         ctx.log,
//!         vec![
//!             "Ring!".to_string(),
//!             "Door opened".to_string(),
//!             "Door closed".to_string()
//!         ]
//!     );
//!     Ok(())
//! }
//! ```
//!
//! ## Supervisor / host loop pattern
//!
//! The FSM is usually embedded in a "host loop" (an actor/supervisor task) that:
//! 1) Receives or derives events from the outside world.
//! 2) Feeds events into the FSM via [`StateMachine::handle`].
//! 3) Executes returned actions (often sequentially).
//! 4) Decides how to handle action failures (retry, escalate, or feed an error event back in).
//!
//! In ObzenFlow's runtime-services, a supervisor typically owns the `StateMachine` plus a context
//! that holds runtime capabilities (journals, message bus handles, metrics emitters). The
//! supervision loop drives the FSM forward and treats actions as explicit, auditable effects.
//!
//! Concretely, supervisors often split into two layers:
//! - A "dispatch" loop (sometimes named `dispatch_state`) that performs state-specific I/O and
//!   decides what should happen next (e.g. `Continue`, `Transition(event)`, `Terminate`).
//! - A single-threaded FSM step that calls `handle(event, &mut context)` and then executes the
//!   returned actions, mapping action failures into explicit error events when needed.
//!
//! ```rust,no_run
//! # use obzenflow_fsm::{FsmAction, StateMachine};
//! # async fn example<S, E, C, A>(
//! #   mut machine: StateMachine<S, E, C, A>,
//! #   mut context: C,
//! # ) -> Result<(), obzenflow_fsm::FsmError>
//! # where
//! #   S: obzenflow_fsm::StateVariant,
//! #   E: obzenflow_fsm::EventVariant,
//! #   C: obzenflow_fsm::FsmContext,
//! #   A: obzenflow_fsm::FsmAction<Context = C>,
//! # {
//! loop {
//!     // Give the current state a chance to time out.
//!     let timeout_actions = machine.check_timeout(&mut context).await?;
//!     for action in timeout_actions {
//!         action.execute(&mut context).await?;
//!     }
//!
//!     // Receive an external event (channel, journal subscription, socket, ...).
//!     let event: E = todo!("receive or derive an event");
//!
//!     let actions = machine.handle(event, &mut context).await?;
//!     for action in actions {
//!         if let Err(e) = action.execute(&mut context).await {
//!             // Many supervisors map action failures into a domain event and feed it back in
//!             // so the FSM can transition into an explicit Failed/Drained state.
//!             let failure_event: E = todo!("map {e} into an error event");
//!             let failure_actions = machine.handle(failure_event, &mut context).await?;
//!             for action in failure_actions {
//!                 action.execute(&mut context).await?;
//!             }
//!         }
//!     }
//! }
//! # }
//! ```
//!
//! ## Timeouts
//!
//! Timeouts are **per-state** and **cooperative**:
//! - A timeout is configured inside a `state` block via `timeout <duration> => handler;`.
//! - The engine does not spawn timers; the host decides when to poll for timeouts by calling
//!   [`StateMachine::check_timeout`].
//! - Timeout handlers return a [`Transition`] just like normal `on Event => ...` handlers.
//!
//! The timeout countdown starts when a state is entered. Any transition into a state (including a
//! self-transition) schedules that state's timeout; transitioning into a state without a timeout
//! clears the timer.
//!
//! ```rust
//! use obzenflow_fsm::{fsm, FsmAction, FsmContext, Transition};
//! use std::time::Duration;
//!
//! #[derive(Clone, Debug, PartialEq, obzenflow_fsm::StateVariant)]
//! enum DoorState {
//!     Closed,
//!     Open,
//! }
//!
//! #[derive(Clone, Debug, obzenflow_fsm::EventVariant)]
//! enum DoorEvent {
//!     Open,
//! }
//!
//! #[derive(Clone, Debug, PartialEq)]
//! enum DoorAction {
//!     Log(String),
//! }
//!
//! #[derive(Default)]
//! struct DoorContext {
//!     log: Vec<String>,
//! }
//!
//! impl FsmContext for DoorContext {}
//!
//! #[async_trait::async_trait]
//! impl FsmAction for DoorAction {
//!     type Context = DoorContext;
//!
//!     async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
//!         match self {
//!             DoorAction::Log(msg) => ctx.log.push(msg.clone()),
//!         }
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> Result<(), obzenflow_fsm::FsmError> {
//!     let mut door = fsm! {
//!         state:   DoorState;
//!         event:   DoorEvent;
//!         context: DoorContext;
//!         action:  DoorAction;
//!         initial: DoorState::Closed;
//!
//!         state DoorState::Closed {
//!             on DoorEvent::Open => |_s: &DoorState, _e: &DoorEvent, _ctx: &mut DoorContext| {
//!                 Box::pin(async move {
//!                     Ok(Transition {
//!                         next_state: DoorState::Open,
//!                         actions: vec![DoorAction::Log("Opened".into())],
//!                     })
//!                 })
//!             };
//!         }
//!
//!         state DoorState::Open {
//!             timeout Duration::from_millis(10) => |_s: &DoorState, _ctx: &mut DoorContext| {
//!                 Box::pin(async move {
//!                     Ok(Transition {
//!                         next_state: DoorState::Closed,
//!                         actions: vec![DoorAction::Log("Auto-closed".into())],
//!                     })
//!                 })
//!             };
//!         }
//!     };
//!
//!     let mut ctx = DoorContext::default();
//!
//!     // Closed -> Open
//!     let actions = door.handle(DoorEvent::Open, &mut ctx).await?;
//!     door.execute_actions(actions, &mut ctx).await?;
//!     assert_eq!(door.state(), &DoorState::Open);
//!
//!     tokio::time::sleep(Duration::from_millis(20)).await;
//!
//!     // Open -> Closed (timeout)
//!     let actions = door.check_timeout(&mut ctx).await?;
//!     door.execute_actions(actions, &mut ctx).await?;
//!     assert_eq!(door.state(), &DoorState::Closed);
//!
//!     assert_eq!(
//!         ctx.log,
//!         vec!["Opened".to_string(), "Auto-closed".to_string()]
//!     );
//!     Ok(())
//! }
//! ```
//!
//! ## Variant names and dispatch
//!
//! Transitions are looked up by `(state.variant_name(), event.variant_name())`. For enums, the
//! recommended approach is to derive these traits:
//! `#[derive(obzenflow_fsm::StateVariant, obzenflow_fsm::EventVariant)]`.

// Allow this crate to refer to itself via `obzenflow_fsm` so that
// proc-macro expansions using `::obzenflow_fsm::...` also work in
// the crate's own tests.
extern crate self as obzenflow_fsm;

// Module declarations
pub mod builder;
pub mod error;
pub mod handlers;
pub mod machine;
pub mod types;

// Re-export main types for convenience
pub use error::FsmError;
pub use machine::StateMachine;
pub use types::{EventVariant, FsmAction, FsmContext, StateVariant, Transition};

// Re-export handler types for advanced usage
pub use handlers::{StateHandler, TimeoutHandler, TransitionHandler};

/// Internal types used by the `fsm!` macro expansion.
///
/// This module is **not** intended for direct use; it exists so that
/// proc-macro expansions can refer to a stable, public path while
/// signalling that the underlying types are implementation details.
#[doc(hidden)]
pub mod internal {
    #[allow(deprecated)]
    pub use crate::builder::FsmBuilder;
}

// Re-export proc-macro helpers (derives + DSL).
// This allows users to write:
//   #[derive(obzenflow_fsm::StateVariant, obzenflow_fsm::EventVariant)]
//   let fsm = fsm! { ... };
pub use obzenflow_fsm_macros::{fsm, EventVariant, StateVariant};
pub use obzenflow_fsm_macros::{
    EventVariant as EventVariantDerive, StateVariant as StateVariantDerive,
};

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    #[allow(dead_code)]
    #[derive(Clone, Debug, PartialEq)]
    enum TestState {
        Idle,
        Active { count: u32 },
        Done,
    }

    impl StateVariant for TestState {
        fn variant_name(&self) -> &str {
            match self {
                TestState::Idle => "Idle",
                TestState::Active { .. } => "Active",
                TestState::Done => "Done",
            }
        }
    }

    #[allow(dead_code)]
    #[derive(Clone, Debug)]
    enum TestEvent {
        Start,
        Increment,
        Finish,
    }

    impl EventVariant for TestEvent {
        fn variant_name(&self) -> &str {
            match self {
                TestEvent::Start => "Start",
                TestEvent::Increment => "Increment",
                TestEvent::Finish => "Finish",
            }
        }
    }

    #[allow(dead_code)]
    #[derive(Clone, Debug, PartialEq)]
    enum TestAction {
        Log(String),
        Notify,
    }

    struct TestContext {
        total: u32,
    }

    impl FsmContext for TestContext {
        fn describe(&self) -> String {
            format!("Test context with total: {}", self.total)
        }
    }

    #[async_trait]
    impl FsmAction for TestAction {
        type Context = TestContext;

        async fn execute(&self, _ctx: &mut Self::Context) -> crate::types::FsmResult<()> {
            match self {
                TestAction::Log(msg) => {
                    println!("Log: {msg}");
                    Ok(())
                }
                TestAction::Notify => {
                    println!("Notify");
                    Ok(())
                }
            }
        }
    }

    #[tokio::test]
    async fn test_basic_fsm() {
        let mut state_machine = crate::fsm! {
            state:   TestState;
            event:   TestEvent;
            context: TestContext;
            action:  TestAction;
            initial: TestState::Idle;

            state TestState::Idle {
                on TestEvent::Start => |_state: &TestState, _event: &TestEvent, _ctx: &mut TestContext| {
                    Box::pin(async move {
                        Ok(Transition {
                            next_state: TestState::Active { count: 0 },
                            actions: vec![TestAction::Log("Starting".into())],
                        })
                    })
                };
            }
        };
        let mut ctx = TestContext { total: 0 };

        // Test transition
        let actions = state_machine
            .handle(TestEvent::Start, &mut ctx)
            .await
            .unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            state_machine.state(),
            TestState::Active { count: 0 }
        ));
    }
}
