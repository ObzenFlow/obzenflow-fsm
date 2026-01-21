//! Async-first finite state machine (FSM) library inspired by Akka / Apache Pekko (Classic) FSM and
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
//! ## Why This Crate Exists (ObzenFlow Background)
//!
//! ObzenFlow is an async dataflow runtime. Pipeline/stage supervisors coordinate lifecycles,
//! subscriptions, backpressure, and observability while executing **user-defined code** (sources,
//! transforms, stateful operators, sinks) potentially thousands of times per second.
//!
//! That runtime context drives a few non-negotiable constraints:
//! - A "step" often needs to `await` (journals, channels, locks, timers, user code).
//! - Failures must be handled explicitly (emit an error event, drain/cleanup) rather than panic.
//! - The host needs control over effect execution (ordering, retries, batching, instrumentation).
//!
//! ## Philosophy
//!
//! ObzenFlow’s runtime makes *at-least-once* delivery the default. An event may be retried or
//! replayed and therefore observed more than once.
//!
//! When a stage has multiple upstreams, each upstream may be sequential on its own, but the
//! *interleaving across upstreams* is nondeterministic. In practice this means the combined stream
//! can be “reordered”, even if no single upstream violates its own ordering.
//!
//! `obzenflow-fsm` is designed so those realities stay visible. Transition handlers compute the
//! next state and return actions; the host executes (and can retry) actions explicitly.
//!
//! For outcomes that stay stable under duplicates, interleavings, and reshaping (batching/
//! sharding), the tests are essentially pointing at the "unholy trinity" of distributed systems
//! failures: fuzzy or broken idempotence, commutativity, and associativity guarantees. These are
//! sufficient conditions for many dataflow operators, not universal requirements (some domains
//! are intentionally order-dependent).
//!
//! - **Idempotence**: applying the same logical input more than once has the same effect as
//!   applying it once.
//!   In practice, naively doing `credit += amount` is not idempotent under retries, it only becomes
//!   idempotent if you deduplicate by a stable key (event ID, business key, or a durable cursor).
//!   Prefer bounded/durable tracking when possible (sequence numbers, journal offsets, windowed
//!   keys). In ObzenFlow’s runtime-services, stateful stage supervisors already keep bounded per-upstream
//!   progress/lineage metadata (e.g. last seen event ID *and* vector-clock snapshots) alongside the
//!   handler’s accumulator state.
//!
//! - **Commutativity**: swapping the order of two inputs does not change the outcome.
//!   If your update is order-dependent (e.g. appending into a `Vec`), that can be correct, but
//!   then you must model ordering explicitly (total ordering / deterministic tie-breaks) instead of
//!   assuming reordering is harmless. In ObzenFlow, vector clocks capture happened-before vs concurrency, but they do
//!   **not** impose a total order on concurrent events, so they can’t be used as an ordering key by
//!   themselves.
//!
//! - **Associativity**: when you represent updates as deltas/partials with a “combine” operator,
//!   regrouping combinations does not change the result: `(a ⊕ b) ⊕ c == a ⊕ (b ⊕ c)`.
//!   This is what makes batching and parallel folding safe: combine partial aggregates in any
//!   grouping and get the same combined delta. If you can only define a sequential update (or use a
//!   non-associative operator like subtraction), then “batch then reduce” is not equivalent to
//!   “apply one-by-one”.
//!
//! When a property does *not* hold, model that explicitly: carry ordering metadata, detect and
//! surface duplicates, or transition into a domain `Corrupted`/`Failed` state instead of silently
//! producing inconsistent results.
//!
//! Tip: if you care about replay/event-sourcing, keep state evolution deterministic and model
//! external effects only as actions. In replay mode you can ignore actions; in live mode you
//! execute them.
//!
//! Also apply idempotence to control/lifecycle signals where possible (e.g. receiving an `Error`
//! while already in a terminal `Failed` state should be a no-op) so retries don’t amplify failures.
//!
//! ## Quick start
//!
//! A tiny "door" FSM with explicit actions:
//!
//! Note on handler syntax: `fsm!` stores handlers behind trait objects, so each handler closure
//! returns a boxed pinned future. That’s why examples use `Box::pin(async move { ... })`.
//!
//! ```rust
//! use obzenflow_fsm::{fsm, types::FsmResult, FsmAction, FsmContext, Transition};
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
//!     async fn execute(&self, ctx: &mut Self::Context) -> FsmResult<()> {
//!         match self {
//!             DoorAction::Ring => ctx.log.push("Ring!".to_string()),
//!             DoorAction::Log(msg) => ctx.log.push(msg.clone()),
//!         }
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main(flavor = "current_thread")]
//! async fn main() -> FsmResult<()> {
//!     let mut door = fsm! {
//!         state:   DoorState;
//!         event:   DoorEvent;
//!         context: DoorContext;
//!         action:  DoorAction;
//!         initial: DoorState::Closed;
//!
//!         state DoorState::Closed {
//!             on DoorEvent::Open => |
//!                 _s: &DoorState,
//!                 _e: &DoorEvent,
//!                 _ctx: &mut DoorContext,
//!             | {
//!                 // `fsm!` expects a boxed pinned future from each handler.
//!                 Box::pin(async move {
//!                     Ok(Transition {
//!                         next_state: DoorState::Open,
//!                         actions: vec![
//!                             DoorAction::Ring,
//!                             DoorAction::Log("Door opened".into()),
//!                         ],
//!                     })
//!                 })
//!             };
//!         }
//!
//!         state DoorState::Open {
//!             on DoorEvent::Close => |
//!                 _s: &DoorState,
//!                 _e: &DoorEvent,
//!                 _ctx: &mut DoorContext,
//!             | {
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
//! The rest of this page focuses on the patterns used to run FSMs inside an async runtime.
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
//! The "hot path" where user-defined handlers run (process one input, emit zero or more outputs)
//! usually lives in the dispatch layer; the FSM primarily models lifecycle coordination and
//! produces actions like "publish running", "forward EOF", "write completion", "cleanup", etc.
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
//!                 // Timeout handlers are the same idea: return a boxed pinned future.
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
