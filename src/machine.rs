//! The FSM runtime engine.
//!
//! [`StateMachine`] owns the current state and the configured transition tables (built via the
//! `fsm!` DSL). It does **not** own the mutable context; instead, the host loop passes `&mut C`
//! into [`StateMachine::handle`] and [`StateMachine::check_timeout`].
//!
//! The engine never executes actions automatically. It returns a `Vec<A>` so the host can choose
//! how to run effects (sequentially, with retries, with supervision, etc.).

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tracing::debug;

use crate::error::FsmError;
use crate::handlers::{StateHandler, TimeoutHandler, TransitionHandler};
use crate::types::{EventVariant, FsmAction, FsmContext, FsmResult, StateVariant, Transition};

type TransitionMap<S, E, C, A> = HashMap<(String, String), TransitionHandler<S, E, C, A>>;
type StateHandlerMap<S, C, A> = HashMap<String, StateHandler<S, C, A>>;
type TimeoutHandlerMap<S, C, A> = HashMap<String, (Duration, TimeoutHandler<S, C, A>)>;
type UnhandledHandler<S, E, C> = Arc<
    dyn for<'a> Fn(&'a S, &'a E, &'a mut C) -> crate::types::BoxFuture<'a, FsmResult<()>>
        + Send
        + Sync,
>;

/// The concrete finite state machine runtime.
///
/// Constructed via the `fsm!` DSL (recommended) or the legacy builder API.
///
/// # Action ordering
///
/// When a transition occurs, actions are returned in this order:
/// 1) Exit-handler actions (for the old state), if any.
/// 2) Entry-handler actions (for the new state), if any.
/// 3) Transition actions returned by the transition/timeout handler.
///
/// Entry/exit hooks are executed even for self-transitions to keep timeout and hook behaviour
/// consistent.
pub struct StateMachine<S, E, C, A> {
    current_state: S,
    transitions: Arc<TransitionMap<S, E, C, A>>,
    entry_handlers: Arc<StateHandlerMap<S, C, A>>,
    exit_handlers: Arc<StateHandlerMap<S, C, A>>,
    timeout_handlers: Arc<TimeoutHandlerMap<S, C, A>>,
    unhandled_handler: Option<UnhandledHandler<S, E, C>>,
    state_timeout: Option<Instant>,
    _phantom: PhantomData<(E, C, A)>,
}

impl<S, E, C, A> StateMachine<S, E, C, A>
where
    S: StateVariant,
    E: EventVariant,
    C: FsmContext,
    A: FsmAction<Context = C>,
{
    /// Create a new state machine.
    ///
    /// This method is intentionally `pub(crate)` to enforce construction via `fsm!` (or the
    /// deprecated builder API).
    pub(crate) fn new(
        initial_state: S,
        transitions: TransitionMap<S, E, C, A>,
        entry_handlers: StateHandlerMap<S, C, A>,
        exit_handlers: StateHandlerMap<S, C, A>,
        timeout_handlers: TimeoutHandlerMap<S, C, A>,
        unhandled_handler: Option<UnhandledHandler<S, E, C>>,
    ) -> Self {
        let mut machine = Self {
            current_state: initial_state,
            transitions: Arc::new(transitions),
            entry_handlers: Arc::new(entry_handlers),
            exit_handlers: Arc::new(exit_handlers),
            timeout_handlers: Arc::new(timeout_handlers),
            unhandled_handler,
            state_timeout: None,
            _phantom: PhantomData,
        };

        // Schedule initial timeout for the starting state, if configured
        let state_name = machine.current_state.variant_name().to_string();
        if let Some((duration, _)) = machine.timeout_handlers.get(&state_name) {
            machine.state_timeout = Some(Instant::now() + *duration);
        }

        machine
    }

    /// Returns a reference to the current state.
    pub fn state(&self) -> &S {
        &self.current_state
    }

    /// Checks whether the current state's timeout has elapsed.
    ///
    /// This is a cooperative API: the host decides how frequently to call it.
    ///
    /// Returns:
    /// - `Ok(vec![])` if no timeout is configured or it has not yet elapsed.
    /// - A non-empty action list if the timeout handler fired and produced a transition.
    pub async fn check_timeout(&mut self, context: &mut C) -> FsmResult<Vec<A>> {
        if let Some(timeout_instant) = self.state_timeout {
            if Instant::now() >= timeout_instant {
                let state_name = self.current_state.variant_name().to_string();
                if let Some((_, handler)) = self.timeout_handlers.get(&state_name) {
                    let transition = handler(&self.current_state, context).await?;
                    return self.apply_transition(transition, context).await;
                }
            }
        }
        Ok(vec![])
    }

    /// Handles an event and (optionally) transitions to a new state.
    ///
    /// Dispatch is performed by `(state.variant_name(), event.variant_name())`.
    /// If there is no exact match, the engine will also check for a wildcard state (`"_"`) handler
    /// (used by the legacy builder API). If the event is still unhandled:
    /// - If a `when_unhandled` hook was configured, it is executed immediately and an empty action
    ///   list is returned.
    /// - Otherwise [`FsmError::UnhandledEvent`] is returned.
    pub async fn handle(&mut self, event: E, context: &mut C) -> FsmResult<Vec<A>> {
        let state_name = self.current_state.variant_name().to_string();
        let event_name = event.variant_name().to_string();
        let key = (state_name.clone(), event_name.clone());

        debug!("FSM handling event {} in state {}", event_name, state_name);

        if let Some(handler) = self.transitions.get(&key) {
            let transition = handler(&self.current_state, &event, context).await?;
            self.apply_transition(transition, context).await
        } else {
            // Check for wildcard transitions (from any state)
            let wildcard_key = ("_".to_string(), event_name.clone());
            if let Some(handler) = self.transitions.get(&wildcard_key) {
                let transition = handler(&self.current_state, &event, context).await?;
                self.apply_transition(transition, context).await
            } else {
                // Handle unhandled event
                if let Some(handler) = &self.unhandled_handler {
                    handler(&self.current_state, &event, context).await?;
                    Ok(vec![])
                } else {
                    Err(FsmError::UnhandledEvent {
                        state: state_name,
                        event: event_name,
                    })
                }
            }
        }
    }

    /// Apply a state transition
    async fn apply_transition(
        &mut self,
        transition: Transition<S, A>,
        context: &mut C,
    ) -> FsmResult<Vec<A>> {
        let mut all_actions = vec![];

        let old_state_name = self.current_state.variant_name().to_string();
        let new_state_name = transition.next_state.variant_name().to_string();

        // Always treat as a full transition (including self-transitions)
        // so that entry/exit hooks and timeouts behave consistently.

        // Exit current state
        if let Some(handler) = self.exit_handlers.get(&old_state_name) {
            let exit_actions = handler(&self.current_state, context).await?;
            all_actions.extend(exit_actions);
        }

        // Transition to new state
        self.current_state = transition.next_state;

        // Enter new state
        if let Some(handler) = self.entry_handlers.get(&new_state_name) {
            let entry_actions = handler(&self.current_state, context).await?;
            all_actions.extend(entry_actions);
        }

        // Set up timeout for new state if configured
        if let Some((duration, _)) = self.timeout_handlers.get(&new_state_name) {
            self.state_timeout = Some(Instant::now() + *duration);
        } else {
            self.state_timeout = None;
        }

        debug!(
            "FSM transitioned from {} to {}",
            old_state_name, new_state_name
        );

        // Add transition actions
        all_actions.extend(transition.actions);

        Ok(all_actions)
    }

    /// Convenience helper to execute actions sequentially.
    ///
    /// Supervisors with more advanced needs (retries, compensation, mapping failures into events)
    /// will typically execute actions themselves instead of using this helper.
    pub async fn execute_actions(&self, actions: Vec<A>, context: &mut C) -> FsmResult<()> {
        for action in actions {
            action.execute(context).await?;
        }
        Ok(())
    }
}
