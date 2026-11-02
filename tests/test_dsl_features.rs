// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use std::fmt::Debug;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use obzenflow_fsm::{
    fsm, transitions, EventVariant, FsmAction, FsmContext, StateMachine, StateVariant, Transition,
};
use tokio::sync::RwLock;

#[derive(Clone, Debug, PartialEq, StateVariant)]
enum DslState {
    Idle,
    Working,
    Done,
}

#[derive(Clone, Debug, EventVariant)]
enum DslEvent {
    Start,
    Finish,
    Unknown,
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

#[derive(Clone, Debug, PartialEq, StateVariant)]
enum Slot<P> {
    Empty,
    Holding(P),
    Sealed { count: u32 },
}

#[derive(Clone, Debug, EventVariant)]
enum SlotEvent<P> {
    Put(P),
    Seal { count: u32 },
}

fn next_slot<P: Clone>(event: &SlotEvent<P>) -> Slot<P> {
    match event {
        SlotEvent::Put(value) => Slot::Holding(value.clone()),
        SlotEvent::Seal { count } => Slot::Sealed { count: *count },
    }
}

fn slot_machine<P>() -> StateMachine<Slot<P>, SlotEvent<P>, DslContext, DslAction>
where
    P: Clone + Debug + PartialEq + Send + Sync + 'static,
{
    fsm! {
        state:   Slot<P>;
        event:   SlotEvent<P>;
        context: DslContext;
        action:  DslAction;
        initial: Slot::Empty;

        state Slot::Empty {
            on SlotEvent::Put => |_: &Slot<P>, event: &SlotEvent<P>, _: &mut DslContext| {
                let next = next_slot(event);
                Box::pin(async move { Ok(transitions::goto(next)) })
            };
        }

        state Slot::Holding {
            on SlotEvent::Seal => |_: &Slot<P>, event: &SlotEvent<P>, _: &mut DslContext| {
                let next = next_slot(event);
                Box::pin(async move { Ok(transitions::goto(next)) })
            };
        }

        state Slot::Sealed {
            on_entry |_: &Slot<P>, _: &mut DslContext| {
                Box::pin(async { Ok(vec![DslAction::Log("sealed".into())]) })
            };
        }
    }
}

#[tokio::test]
async fn test_dsl_generic_table() {
    let mut machine = slot_machine::<String>();
    let mut ctx = DslContext {
        log: Arc::new(RwLock::new(vec![])),
    };

    machine
        .handle(SlotEvent::Put("parcel".to_string()), &mut ctx)
        .await
        .expect("put");
    assert_eq!(machine.state(), &Slot::Holding("parcel".to_string()));

    let actions = machine
        .handle(SlotEvent::Seal { count: 1 }, &mut ctx)
        .await
        .expect("seal");
    assert_eq!(machine.state(), &Slot::Sealed { count: 1 });
    assert_eq!(actions, vec![DslAction::Log("sealed".into())]);
}

struct OpaqueHandler;

#[derive(StateVariant)]
enum Stage<H> {
    Created,
    Running { seen: u64 },
    _Phantom(PhantomData<H>),
}

impl<H> Clone for Stage<H> {
    fn clone(&self) -> Self {
        match self {
            Self::Created => Self::Created,
            Self::Running { seen } => Self::Running { seen: *seen },
            Self::_Phantom(_) => Self::_Phantom(PhantomData),
        }
    }
}

impl<H> Debug for Stage<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => f.write_str("Created"),
            Self::Running { seen } => write!(f, "Running {{ seen: {seen} }}"),
            Self::_Phantom(_) => f.write_str("_Phantom"),
        }
    }
}

impl<H: Send + Sync> PartialEq for Stage<H> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Created, Self::Created) => true,
            (Self::Running { seen: a }, Self::Running { seen: b }) => a == b,
            _ => false,
        }
    }
}

#[derive(EventVariant)]
enum StageEvent<H> {
    Start,
    _Phantom(PhantomData<H>),
}

impl<H> Clone for StageEvent<H> {
    fn clone(&self) -> Self {
        match self {
            Self::Start => Self::Start,
            Self::_Phantom(_) => Self::_Phantom(PhantomData),
        }
    }
}

impl<H> Debug for StageEvent<H> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start => f.write_str("Start"),
            Self::_Phantom(_) => f.write_str("_Phantom"),
        }
    }
}

fn stage_machine<H: Send + Sync + 'static>(
) -> StateMachine<Stage<H>, StageEvent<H>, DslContext, DslAction> {
    fsm! {
        state:   Stage<H>;
        event:   StageEvent<H>;
        context: DslContext;
        action:  DslAction;
        initial: Stage::Created;

        state Stage::Created {
            on StageEvent::Start => |_: &Stage<H>, _: &StageEvent<H>, _: &mut DslContext| {
                Box::pin(async { Ok(transitions::goto(Stage::Running { seen: 1 })) })
            };
        }
    }
}

#[tokio::test]
async fn test_dsl_table_generic_over_an_opaque_handler() {
    let mut machine = stage_machine::<OpaqueHandler>();
    let mut ctx = DslContext {
        log: Arc::new(RwLock::new(vec![])),
    };

    machine
        .handle(StageEvent::Start, &mut ctx)
        .await
        .expect("start");
    assert_eq!(machine.state(), &Stage::Running { seen: 1 });
    assert_eq!(machine.state().variant_name(), "Running");
}
