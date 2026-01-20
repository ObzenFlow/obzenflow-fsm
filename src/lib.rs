//! Async-first Finite State Machine library inspired by Akka FSM
//!
//! Core principle: State(S) × Event(E) → Actions(A), State(S')
//!
//! "If we are in state S and the event E occurs, we should perform the actions A
//! and make a transition to the state S'."

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
//   #[derive(StateVariant, EventVariant)]
//   let fsm = fsm! { ... };
pub use obzenflow_fsm_macros::{
    fsm, EventVariant as EventVariantDerive, StateVariant as StateVariantDerive,
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
