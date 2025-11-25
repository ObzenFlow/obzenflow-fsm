//! Handler type aliases for FSM callbacks

use std::sync::Arc;

use crate::types::{BoxFuture, FsmResult, Transition};

/// Type alias for async transition handlers
///
/// The handler is invoked with the current state, event, and a mutable
/// reference to the FSM context. It returns a future that resolves to
/// a `Transition`.
pub type TransitionHandler<S, E, C, A> = Arc<
    dyn for<'a> Fn(&'a S, &'a E, &'a mut C) -> BoxFuture<'a, FsmResult<Transition<S, A>>>
        + Send
        + Sync,
>;

/// Type alias for async state handlers (entry/exit)
///
/// These are invoked on state entry/exit and may produce additional actions.
pub type StateHandler<S, C, A> = Arc<
    dyn for<'a> Fn(&'a S, &'a mut C) -> BoxFuture<'a, FsmResult<Vec<A>>>
        + Send
        + Sync,
>;

/// Type alias for timeout handlers
///
/// Timeout handlers are invoked when a state's configured timeout elapses
/// and can drive transitions just like normal handlers.
pub type TimeoutHandler<S, C, A> = Arc<
    dyn for<'a> Fn(&'a S, &'a mut C) -> BoxFuture<'a, FsmResult<Transition<S, A>>>
        + Send
        + Sync,
>;
