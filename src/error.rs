//! Error types for FSM operations

use thiserror::Error;

/// Error types for FSM operations
#[derive(Error, Debug)]
pub enum FsmError {
    /// Transition was requested for a state/event pair with no handler
    #[error("Invalid transition from state '{from}' on event '{event}'")]
    InvalidTransition { from: String, event: String },

    /// Duplicate transition handler registered for the same (state, event)
    #[error("Duplicate handler for state '{state}' and event '{event}'")]
    DuplicateHandler { state: String, event: String },

    /// An FSM handler returned an error
    #[error("Handler error: {0}")]
    HandlerError(String),

    /// A timeout fired for a given state
    #[error("Timeout in state '{state}'")]
    Timeout { state: String },

    /// An event was not handled and no when_unhandled hook was provided
    #[error("Unhandled event '{event}' in state '{state}'")]
    UnhandledEvent { state: String, event: String },

    /// Generic builder or configuration error
    #[error("Builder error: {0}")]
    BuilderError(String),
}
