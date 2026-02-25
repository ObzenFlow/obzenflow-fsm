// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use std::sync::Arc;
use std::time::Duration;

use obzenflow_fsm::{
    fsm, EventVariant, FsmAction, FsmContext, StateMachine, StateVariant, Transition,
};
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq)]
enum DslState {
    Idle,
    Working,
    Done,
}

impl StateVariant for DslState {
    fn variant_name(&self) -> &str {
        match self {
            DslState::Idle => "Idle",
            DslState::Working => "Working",
            DslState::Done => "Done",
        }
    }
}

#[derive(Clone, Debug)]
enum DslEvent {
    Start,
    Finish,
    Unknown,
}

impl EventVariant for DslEvent {
    fn variant_name(&self) -> &str {
        match self {
            DslEvent::Start => "Start",
            DslEvent::Finish => "Finish",
            DslEvent::Unknown => "Unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum DslAction {
    Log(String),
}

#[derive(Clone)]
struct DslContext {
    log: Arc<RwLock<Vec<String>>>,
}

impl FsmContext for DslContext {}

#[async_trait::async_trait]
impl FsmAction for DslAction {
    type Context = DslContext;

    async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
        match self {
            DslAction::Log(msg) => {
                ctx.log.write().await.push(msg.clone());
                Ok(())
            }
        }
    }
}

#[tokio::test]
async fn test_dsl_entry_exit_and_unhandled() {
    let machine: StateMachine<DslState, DslEvent, DslContext, DslAction> = fsm! {
        state:   DslState;
        event:   DslEvent;
        context: DslContext;
        action:  DslAction;
        initial: DslState::Idle;

        unhandled => |state: &DslState, event: &DslEvent, ctx: &mut DslContext| {
            let state_name = state.variant_name().to_string();
            let event_name = event.variant_name().to_string();
            Box::pin(async move {
                ctx.log
                    .write()
                    .await
                    .push(format!("Unhandled {event_name} in {state_name}"));
                Ok(())
            })
        };

        state DslState::Idle {
            on DslEvent::Start => |_s: &DslState, _e: &DslEvent, _ctx: &mut DslContext| {
                Box::pin(async move {
                    Ok(Transition {
                        next_state: DslState::Working,
                        actions: vec![],
                    })
                })
            };
        }

        state DslState::Working {
            on_entry |_s: &DslState, ctx: &mut DslContext| {
                Box::pin(async move {
                    ctx.log.write().await.push("enter Working".into());
                    Ok(vec![DslAction::Log("entry".into())])
                })
            };

            on_exit |_s: &DslState, ctx: &mut DslContext| {
                Box::pin(async move {
                    ctx.log.write().await.push("exit Working".into());
                    Ok(vec![DslAction::Log("exit".into())])
                })
            };

            on DslEvent::Finish => |_s: &DslState, _e: &DslEvent, _ctx: &mut DslContext| {
                Box::pin(async move {
                    Ok(Transition {
                        next_state: DslState::Done,
                        actions: vec![],
                    })
                })
            };
        }
    };

    let mut ctx = DslContext {
        log: Arc::new(RwLock::new(vec![])),
    };
    let mut machine = machine;

    // Idle -> Working should trigger entry actions.
    let actions = machine
        .handle(DslEvent::Start, &mut ctx)
        .await
        .expect("transition");
    assert_eq!(actions.len(), 1);
    actions[0].clone().execute(&mut ctx).await.unwrap();

    // Working -> Done should trigger exit actions.
    let actions = machine
        .handle(DslEvent::Finish, &mut ctx)
        .await
        .expect("transition");
    assert_eq!(actions.len(), 1);
    actions[0].clone().execute(&mut ctx).await.unwrap();

    // Send an unhandled event in Done; should hit unhandled handler.
    let actions = machine
        .handle(DslEvent::Unknown, &mut ctx)
        .await
        .expect("unhandled handler");
    assert!(actions.is_empty());

    let log = ctx.log.read().await.clone();
    assert_eq!(
        log,
        vec![
            "enter Working".to_string(),
            "entry".to_string(),
            "exit Working".to_string(),
            "exit".to_string(),
            "Unhandled Unknown in Done".to_string(),
        ]
    );
}

#[tokio::test]
async fn test_dsl_timeout() {
    use tokio::time::sleep;

    let machine: StateMachine<DslState, DslEvent, DslContext, DslAction> = fsm! {
        state:   DslState;
        event:   DslEvent;
        context: DslContext;
        action:  DslAction;
        initial: DslState::Idle;

        state DslState::Idle {
            on DslEvent::Start => |_s: &DslState, _e: &DslEvent, _ctx: &mut DslContext| {
                Box::pin(async move {
                    Ok(Transition {
                        next_state: DslState::Working,
                        actions: vec![],
                    })
                })
            };
        }

        state DslState::Working {
            timeout Duration::from_millis(30) => |_s: &DslState, _ctx: &mut DslContext| {
                Box::pin(async move {
                    Ok(Transition {
                        next_state: DslState::Done,
                        actions: vec![DslAction::Log("timed out".into())],
                    })
                })
            };
        }
    };

    let mut ctx = DslContext {
        log: Arc::new(RwLock::new(vec![])),
    };
    let mut machine = machine;

    // Idle -> Working
    machine
        .handle(DslEvent::Start, &mut ctx)
        .await
        .expect("start");
    assert!(matches!(machine.state(), DslState::Working));

    // Wait long enough for timeout to fire.
    sleep(Duration::from_millis(40)).await;

    let actions = machine.check_timeout(&mut ctx).await.expect("timeout");
    assert_eq!(actions.len(), 1);
    actions[0].clone().execute(&mut ctx).await.unwrap();
    assert!(matches!(machine.state(), DslState::Done));
}
