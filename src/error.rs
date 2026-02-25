// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Error types for FSM operations.

use thiserror::Error;

/// Errors returned by FSM handlers, actions, and construction.
#[derive(Error, Debug)]
pub enum FsmError {
    /// A transition was requested for a `(state, event)` pair with no handler.
    ///
    /// Note: most users will see [`FsmError::UnhandledEvent`] instead; this variant exists for
    /// compatibility with earlier designs and for callers that want to distinguish the cases.
    #[error("Invalid transition from state '{from}' on event '{event}'")]
    InvalidTransition { from: String, event: String },

    /// Duplicate transition handler registered for the same `(state, event)`.
    #[error("Duplicate handler for state '{state}' and event '{event}'")]
    DuplicateHandler { state: String, event: String },

    /// An FSM handler or action returned an error.
    #[error("Handler error: {0}")]
    HandlerError(String),

    /// A timeout fired for a given state.
    #[error("Timeout in state '{state}'")]
    Timeout { state: String },

    /// An event was not handled and no `when_unhandled` hook was provided.
    #[error("Unhandled event '{event}' in state '{state}'")]
    UnhandledEvent { state: String, event: String },

    /// Generic builder or configuration error.
    #[error("Builder error: {0}")]
    BuilderError(String),
}
