// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Constructors for the [`Transition`] a handler returns.

use crate::Transition;

/// Transition to a new state (no actions).
pub fn goto<S, A>(state: S) -> Transition<S, A> {
    Transition {
        next_state: state,
        actions: vec![],
    }
}

/// Transition to a new state with actions.
pub fn goto_with_actions<S, A>(state: S, actions: Vec<A>) -> Transition<S, A> {
    Transition {
        next_state: state,
        actions,
    }
}

/// Stay in the current state (no actions).
pub fn stay<S: Clone, A>(current_state: &S) -> Transition<S, A> {
    Transition {
        next_state: current_state.clone(),
        actions: vec![],
    }
}

/// Stay in the current state with actions.
pub fn stay_with_actions<S: Clone, A>(current_state: &S, actions: Vec<A>) -> Transition<S, A> {
    Transition {
        next_state: current_state.clone(),
        actions,
    }
}
