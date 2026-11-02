// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use obzenflow_fsm::{
    fsm, transitions, types::FsmResult, EventVariant, FsmAction, FsmContext, StateVariant,
};

#[derive(Clone, Debug, PartialEq)]
enum Door {
    Closed,
    Open,
}

impl StateVariant for Door {
    fn variant_name(&self) -> &str {
        match self {
            Door::Closed => "Closed",
            Door::Open => "Opne",
        }
    }
}

#[derive(Clone, Debug)]
enum DoorEvent {
    Open,
}

impl EventVariant for DoorEvent {
    fn variant_name(&self) -> &str {
        match self {
            DoorEvent::Open => "Open",
        }
    }
}

#[derive(Clone, Debug)]
enum DoorAction {}

struct DoorContext;

impl FsmContext for DoorContext {}

#[async_trait::async_trait]
impl FsmAction for DoorAction {
    type Context = DoorContext;

    async fn execute(&self, _: &mut DoorContext) -> FsmResult<()> {
        match *self {}
    }
}

fn main() {
    fsm! {
        state:   Door;
        event:   DoorEvent;
        context: DoorContext;
        action:  DoorAction;
        initial: Door::Closed;

        state Door::Closed {
            on DoorEvent::Open => |_: &Door, _: &DoorEvent, _: &mut DoorContext| {
                Box::pin(async { Ok(transitions::goto(Door::Open)) })
            };
        }
    };
}
