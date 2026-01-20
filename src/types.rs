//! Core types and traits for `obzenflow-fsm`.
//!
//! Most users will interact with:
//! - [`Transition`], which is returned from transition/timeout handlers.
//! - [`StateVariant`] and [`EventVariant`], which provide stable names for dispatch.
//! - [`FsmContext`] and [`FsmAction`], which model the host-owned context and executable effects.

use std::fmt::Debug;

use crate::error::FsmError;

/// The result of handling an event (or timeout): next state plus actions.
///
/// A `Transition` is produced by a transition handler and interpreted by [`crate::StateMachine`].
/// The engine updates the current state and returns the actions to the host for execution.
///
/// # Example
///
/// ```rust
/// use obzenflow_fsm::Transition;
///
/// #[derive(Clone, Debug, PartialEq)]
/// enum State {
///     A,
///     B,
/// }
///
/// #[derive(Clone, Debug, PartialEq)]
/// enum Action {
///     Log,
/// }
///
/// let t: Transition<State, Action> = Transition {
///     next_state: State::B,
///     actions: vec![Action::Log],
/// };
/// assert_eq!(t.next_state, State::B);
/// ```
#[derive(Clone, Debug)]
pub struct Transition<S, A> {
    /// The next state to transition to.
    pub next_state: S,
    /// Actions to perform as part of the transition.
    pub actions: Vec<A>,
}

impl<S, A> Transition<S, A> {
    /// Start building a transition to `next_state`.
    ///
    /// This is a convenience builder for ergonomic construction inside handlers.
    ///
    /// ```rust
    /// use obzenflow_fsm::Transition;
    ///
    /// #[derive(Clone, Debug, PartialEq)]
    /// enum State {
    ///     Idle,
    ///     Running,
    /// }
    ///
    /// #[derive(Clone, Debug, PartialEq)]
    /// enum Action {
    ///     Start,
    /// }
    ///
    /// let t = Transition::<State, Action>::to(State::Running)
    ///     .with_action(Action::Start)
    ///     .build();
    /// assert_eq!(t.next_state, State::Running);
    /// ```
    pub fn to(next_state: S) -> TransitionBuilder<S, A> {
        TransitionBuilder {
            next_state,
            actions: vec![],
        }
    }

    /// Start building a transition that stays in the current state.
    ///
    /// Because [`Transition`] always carries a concrete `next_state`, the current state must be
    /// supplied at build time via [`StayBuilder::in_state`].
    pub fn stay() -> StayBuilder<S, A> {
        StayBuilder {
            actions: vec![],
            _phantom: std::marker::PhantomData,
        }
    }
}

/// Builder for transitions to a new state.
pub struct TransitionBuilder<S, A> {
    next_state: S,
    actions: Vec<A>,
}

impl<S, A> TransitionBuilder<S, A> {
    /// Replace the action list.
    pub fn with_actions(mut self, actions: Vec<A>) -> Self {
        self.actions = actions;
        self
    }

    /// Append a single action.
    pub fn with_action(mut self, action: A) -> Self {
        self.actions.push(action);
        self
    }

    /// Build the transition.
    pub fn build(self) -> Transition<S, A> {
        Transition {
            next_state: self.next_state,
            actions: self.actions,
        }
    }
}

/// Builder for transitions that stay in the current state.
pub struct StayBuilder<S, A> {
    actions: Vec<A>,
    _phantom: std::marker::PhantomData<S>,
}

impl<S: Clone, A> StayBuilder<S, A> {
    /// Replace the action list.
    pub fn with_actions(mut self, actions: Vec<A>) -> Self {
        self.actions = actions;
        self
    }

    /// Append a single action.
    pub fn with_action(mut self, action: A) -> Self {
        self.actions.push(action);
        self
    }

    /// Build the transition by supplying the current state.
    pub fn in_state(self, current_state: S) -> Transition<S, A> {
        Transition {
            next_state: current_state,
            actions: self.actions,
        }
    }
}

/// Trait for state types that can report a stable "variant name".
///
/// `obzenflow-fsm` uses `variant_name()` as the dispatch key for transition tables.
/// For enum-based states, the name should reflect the enum *variant* (ignoring any payload).
///
/// The recommended approach is to derive it:
/// `#[derive(obzenflow_fsm::StateVariant)]`.
pub trait StateVariant: Clone + Debug + PartialEq + Send + Sync + 'static {
    /// Returns the state variant name (without data).
    ///
    /// For example, `State::Running { count: 5 }` should return `"Running"`.
    fn variant_name(&self) -> &str;
}

/// Trait for event types that can report a stable "variant name".
///
/// `obzenflow-fsm` uses `variant_name()` as the dispatch key for transition tables.
///
/// The recommended approach is to derive it:
/// `#[derive(obzenflow_fsm::EventVariant)]`.
pub trait EventVariant: Clone + Debug + Send + Sync + 'static {
    /// Returns the event variant name (without data).
    fn variant_name(&self) -> &str;
}

/// Trait for FSM context types.
///
/// The context is owned by the host loop and passed into handlers/actions as `&mut Context`.
/// In ObzenFlow, the context commonly holds runtime services (journals, buses, metrics handles)
/// plus mutable supervision state (counters, cached handles, throttling state).
pub trait FsmContext: Send + Sync + 'static {
    /// Returns a human-readable description of this context for debugging.
    fn describe(&self) -> String {
        "FSM Context".to_string()
    }
}

/// Trait for FSM action types that can be executed by the host.
///
/// Actions represent side effects (I/O, notifications, spawning tasks, writing to a journal, ...).
/// The FSM returns actions; the host decides *when* and *how* to execute them.
///
/// In many supervisor-style designs, action failures are mapped into an explicit error event and
/// fed back into the state machine so it can transition into a domain-specific failure state.
#[async_trait::async_trait]
pub trait FsmAction: Clone + Debug + Send + Sync + 'static {
    /// The context type this action operates on.
    type Context: FsmContext;

    /// Execute this action with the given context.
    async fn execute(&self, ctx: &mut Self::Context) -> FsmResult<()>;

    /// Returns a human-readable description of what this action does.
    fn describe(&self) -> String {
        format!("{self:?}")
    }
}

/// A boxed `Send` future.
///
/// This is primarily used by the legacy builder API and the `fsm!` macro expansion.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;

/// Result type used by handlers and actions.
pub type FsmResult<T> = Result<T, FsmError>;
