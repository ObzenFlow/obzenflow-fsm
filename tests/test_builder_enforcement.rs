//! This test file demonstrates that StateMachine can only be created via FsmBuilder
//! It includes examples that would fail to compile if uncommented

#![allow(dead_code)]
#![allow(deprecated)]

use obzenflow_fsm::internal::FsmBuilder;
use obzenflow_fsm::{EventVariant, FsmAction, FsmContext, FsmError, StateVariant, Transition};

// Test types
#[derive(Clone, Debug, PartialEq)]
enum DemoState {
    Start,
    End,
}

impl StateVariant for DemoState {
    fn variant_name(&self) -> &str {
        match self {
            DemoState::Start => "Start",
            DemoState::End => "End",
        }
    }
}

#[derive(Clone, Debug)]
enum DemoEvent {
    Finish,
}

impl EventVariant for DemoEvent {
    fn variant_name(&self) -> &str {
        match self {
            DemoEvent::Finish => "Finish",
        }
    }
}

#[derive(Clone, Debug)]
struct DemoAction;

#[derive(Clone)]
struct DemoContext;

impl FsmContext for DemoContext {
    fn describe(&self) -> String {
        "Demo context".to_string()
    }
}

#[async_trait::async_trait]
impl FsmAction for DemoAction {
    type Context = DemoContext;

    async fn execute(&self, _ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
        Ok(())
    }
}

#[test]
fn test_fsm_works_via_builder() {
    // This is the correct way: use `FsmBuilder`.
    let fsm = FsmBuilder::<DemoState, DemoEvent, DemoContext, DemoAction>::new(DemoState::Start)
        .when("Start")
        .on(
            "Finish",
            |_state, _event: &DemoEvent, _ctx: &mut DemoContext| {
                Box::pin(async {
                    Ok(Transition {
                        next_state: DemoState::End,
                        actions: vec![],
                    })
                })
            },
        )
        .done()
        .build();

    // Verify the FSM was created successfully
    assert!(matches!(fsm.state(), DemoState::Start));
}

// The following examples demonstrate what CANNOT be done:

/*
// COMPILE ERROR: Cannot import and use `StateMachine::new` directly.
#[test]
fn test_direct_construction_fails() {
    use obzenflow_fsm::StateMachine;
    use std::collections::HashMap;

    let fsm = StateMachine::<DemoState, DemoEvent, DemoContext, DemoAction>::new(
        DemoState::Start,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        None,
    );
}
*/

/*
// COMPILE ERROR: Cannot access `new()` even with full path.
#[test]
fn test_full_path_construction_fails() {
    use std::collections::HashMap;

    let fsm = obzenflow_fsm::StateMachine::new(
        DemoState::Start,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        None,
    );
}
*/

/*
// COMPILE ERROR: Cannot circumvent by re-exporting.
mod sneaky {
    pub use obzenflow_fsm::*;
}

#[test]
fn test_reexport_construction_fails() {
    use std::collections::HashMap;

    let fsm = sneaky::StateMachine::new(
        DemoState::Start,
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        None,
    );
}
*/

/// Documentation test showing the compilation errors
///
/// To verify builder-only construction is enforced:
/// 1. Uncomment any of the code blocks above
/// 2. Run `cargo test`
/// 3. Observe compilation errors about private functions
///
/// Expected errors:
/// - "function `new` is private"
/// - "private function"
/// - Similar access restriction messages
///
/// This ensures all FSM users must use FsmBuilder, which:
/// - Provides a fluent API for defining transitions
/// - Ensures states and events are properly connected
/// - Prevents creation of invalid or incomplete state machines
#[test]
fn test_builder_enforcement_documentation() {
    // Documentation-only test: see the rustdoc on this function for manual verification steps.
}

#[test]
fn test_duplicate_handler_detection() {
    let builder =
        FsmBuilder::<DemoState, DemoEvent, DemoContext, DemoAction>::new(DemoState::Start)
            .when("Start")
            .on(
                "Finish",
                |_state, _event: &DemoEvent, _ctx: &mut DemoContext| {
                    Box::pin(async {
                        Ok(Transition {
                            next_state: DemoState::End,
                            actions: vec![],
                        })
                    })
                },
            )
            .done()
            // Duplicate handler for the same (state, event) pair
            .when("Start")
            .on(
                "Finish",
                |_state, _event: &DemoEvent, _ctx: &mut DemoContext| {
                    Box::pin(async {
                        Ok(Transition {
                            next_state: DemoState::End,
                            actions: vec![],
                        })
                    })
                },
            )
            .done();

    let result = builder.try_build();
    assert!(matches!(
        result,
        Err(FsmError::DuplicateHandler { state, event })
            if state == "Start" && event == "Finish"
    ));
}

#[test]
fn test_strict_mode_requires_initial_transitions() {
    // No transitions or timeouts configured for the initial state
    let builder =
        FsmBuilder::<DemoState, DemoEvent, DemoContext, DemoAction>::new(DemoState::Start).strict();

    let result = builder.try_build();
    assert!(matches!(
        result,
        Err(FsmError::BuilderError(msg)) if msg.contains("initial state 'Start'")
    ));
}

#[test]
fn test_strict_mode_allows_valid_initial_state() {
    let builder =
        FsmBuilder::<DemoState, DemoEvent, DemoContext, DemoAction>::new(DemoState::Start)
            .strict()
            .when("Start")
            .on(
                "Finish",
                |_state, _event: &DemoEvent, _ctx: &mut DemoContext| {
                    Box::pin(async {
                        Ok(Transition {
                            next_state: DemoState::End,
                            actions: vec![],
                        })
                    })
                },
            )
            .done();

    let result = builder.try_build();
    assert!(result.is_ok());
}

#[test]
fn test_default_builder_requires_initial_transitions() {
    // With strict_validation enabled by default, a builder with no transitions or timeouts
    // for the initial state should fail just like explicit .strict().
    let builder =
        FsmBuilder::<DemoState, DemoEvent, DemoContext, DemoAction>::new(DemoState::Start);

    let result = builder.try_build();
    assert!(matches!(
        result,
        Err(FsmError::BuilderError(msg)) if msg.contains("initial state 'Start'")
    ));
}

#[test]
fn test_default_builder_allows_valid_initial_state() {
    // Default builder (no .strict()) should accept a valid initial state configuration.
    let builder =
        FsmBuilder::<DemoState, DemoEvent, DemoContext, DemoAction>::new(DemoState::Start)
            .when("Start")
            .on(
                "Finish",
                |_state, _event: &DemoEvent, _ctx: &mut DemoContext| {
                    Box::pin(async {
                        Ok(Transition {
                            next_state: DemoState::End,
                            actions: vec![],
                        })
                    })
                },
            )
            .done();

    let result = builder.try_build();
    assert!(result.is_ok());
}

#[test]
#[should_panic(expected = "FsmBuilder::build failed")]
fn test_build_panics_on_duplicate_handler() {
    // Sanity-check that build() panics on duplicate handlers and surfaces the builder error.
    let _fsm = FsmBuilder::<DemoState, DemoEvent, DemoContext, DemoAction>::new(DemoState::Start)
        .when("Start")
        .on(
            "Finish",
            |_state, _event: &DemoEvent, _ctx: &mut DemoContext| {
                Box::pin(async {
                    Ok(Transition {
                        next_state: DemoState::End,
                        actions: vec![],
                    })
                })
            },
        )
        .done()
        // Duplicate handler for the same (state, event) pair
        .when("Start")
        .on(
            "Finish",
            |_state, _event: &DemoEvent, _ctx: &mut DemoContext| {
                Box::pin(async {
                    Ok(Transition {
                        next_state: DemoState::End,
                        actions: vec![],
                    })
                })
            },
        )
        .done()
        .build();
}
