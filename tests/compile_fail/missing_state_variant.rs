// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use obzenflow_fsm::{fsm, transitions, types::FsmResult, FsmAction, FsmContext};

#[derive(Clone, Debug, PartialEq, obzenflow_fsm::StateVariant)]
enum Door {
    Closed,
    Open,
}

#[derive(Clone, Debug, obzenflow_fsm::EventVariant)]
enum DoorEvent {
    Open,
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

        state Door::Clsoed {
            on DoorEvent::Open => |_: &Door, _: &DoorEvent, _: &mut DoorContext| {
                Box::pin(async { Ok(transitions::goto(Door::Open)) })
            };
        }
    };
}
