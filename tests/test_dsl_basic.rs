// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use obzenflow_fsm::{
    fsm, EventVariant, FsmAction, FsmContext, StateMachine, StateVariant, Transition,
};

#[derive(Clone, Debug, PartialEq)]
enum SimpleState {
    Idle,
    Running,
}

impl StateVariant for SimpleState {
    fn variant_name(&self) -> &str {
        match self {
            SimpleState::Idle => "Idle",
            SimpleState::Running => "Running",
        }
    }
}

#[derive(Clone, Debug)]
enum SimpleEvent {
    Start,
}

impl EventVariant for SimpleEvent {
    fn variant_name(&self) -> &str {
        match self {
            SimpleEvent::Start => "Start",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum SimpleAction {
    MarkStarted,
}

struct SimpleContext {
    started: bool,
}

impl FsmContext for SimpleContext {}

#[async_trait::async_trait]
impl FsmAction for SimpleAction {
    type Context = SimpleContext;

    async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
        match self {
            SimpleAction::MarkStarted => {
                ctx.started = true;
                Ok(())
            }
        }
    }
}

#[tokio::test]
async fn test_fsm_macro_basic() {
    let machine: StateMachine<SimpleState, SimpleEvent, SimpleContext, SimpleAction> = fsm! {
        state:   SimpleState;
        event:   SimpleEvent;
        context: SimpleContext;
        action:  SimpleAction;
        initial: SimpleState::Idle;

        state SimpleState::Idle {
            on SimpleEvent::Start => |_state: &SimpleState, _event: &SimpleEvent, _ctx: &mut SimpleContext| {
                Box::pin(async move {
                    Ok(Transition {
                        next_state: SimpleState::Running,
                        actions: vec![SimpleAction::MarkStarted],
                    })
                })
            };
        }
    };

    let mut ctx = SimpleContext { started: false };
    let mut machine = machine;

    assert!(matches!(machine.state(), SimpleState::Idle));
    let actions = machine
        .handle(SimpleEvent::Start, &mut ctx)
        .await
        .expect("transition should succeed");
    assert_eq!(actions, vec![SimpleAction::MarkStarted]);
    for action in &actions {
        action.execute(&mut ctx).await.expect("action executes");
    }
    assert!(matches!(machine.state(), SimpleState::Running));
    assert!(ctx.started);
}
