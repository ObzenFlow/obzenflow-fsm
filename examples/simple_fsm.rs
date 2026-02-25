// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Simple FSM example using the `fsm!` DSL.

use obzenflow_fsm::{fsm, EventVariant, FsmAction, FsmContext, StateVariant, Transition};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq)]
enum DoorState {
    Closed,
    Open,
}

impl StateVariant for DoorState {
    fn variant_name(&self) -> &str {
        match self {
            DoorState::Closed => "Closed",
            DoorState::Open => "Open",
        }
    }
}

#[derive(Clone, Debug)]
enum DoorEvent {
    Open,
    Close,
}

impl EventVariant for DoorEvent {
    fn variant_name(&self) -> &str {
        match self {
            DoorEvent::Open => "Open",
            DoorEvent::Close => "Close",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum DoorAction {
    Ring,
    Log(String),
}

#[derive(Clone)]
struct DoorContext {
    log: Arc<RwLock<Vec<String>>>,
}

impl FsmContext for DoorContext {}

#[async_trait::async_trait]
impl FsmAction for DoorAction {
    type Context = DoorContext;

    async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
        match self {
            DoorAction::Ring => {
                ctx.log.write().await.push("Ring!".to_string());
                Ok(())
            }
            DoorAction::Log(msg) => {
                ctx.log.write().await.push(msg.clone());
                Ok(())
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut door = fsm! {
        state:   DoorState;
        event:   DoorEvent;
        context: DoorContext;
        action:  DoorAction;
        initial: DoorState::Closed;

        state DoorState::Closed {
            on DoorEvent::Open => |_state: &DoorState, _event: &DoorEvent, ctx: &mut DoorContext| {
                Box::pin(async move {
                    ctx.log.write().await.push("Opening door".to_string());
                    Ok(Transition {
                        next_state: DoorState::Open,
                        actions: vec![DoorAction::Ring],
                    })
                })
            };
        }

        state DoorState::Open {
            on DoorEvent::Close => |_state: &DoorState, _event: &DoorEvent, ctx: &mut DoorContext| {
                Box::pin(async move {
                    ctx.log.write().await.push("Closing door".to_string());
                    Ok(Transition {
                        next_state: DoorState::Closed,
                        actions: vec![DoorAction::Log("Door closed".to_string())],
                    })
                })
            };
        }
    };

    let mut ctx = DoorContext {
        log: Arc::new(RwLock::new(vec![])),
    };

    // Open the door
    let actions = door.handle(DoorEvent::Open, &mut ctx).await.unwrap();
    println!("Actions: {actions:?}");
    println!("State: {:?}", door.state());

    // Close the door
    let actions = door.handle(DoorEvent::Close, &mut ctx).await.unwrap();
    println!("Actions: {actions:?}");
    println!("State: {:?}", door.state());

    // Print log
    let log = ctx.log.read().await;
    println!("Log: {:?}", *log);
}
