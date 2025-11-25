//! StateMachine implementation

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tracing::debug;

use crate::handlers::{TransitionHandler, StateHandler, TimeoutHandler};
use crate::types::{FsmAction, FsmContext, FsmResult, StateVariant, EventVariant, Transition};
use crate::error::FsmError;

/// The concrete FSM implementation
pub struct StateMachine<S, E, C, A> {
    current_state: S,
    transitions: Arc<HashMap<(String, String), TransitionHandler<S, E, C, A>>>,
    entry_handlers: Arc<HashMap<String, StateHandler<S, C, A>>>,
    exit_handlers: Arc<HashMap<String, StateHandler<S, C, A>>>,
    timeout_handlers: Arc<HashMap<String, (Duration, TimeoutHandler<S, C, A>)>>,
    unhandled_handler: Option<
        Arc<dyn for<'a> Fn(&'a S, &'a E, &'a mut C) -> crate::types::BoxFuture<'a, FsmResult<()>> + Send + Sync>,
    >,
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
    /// Create a new state machine
    /// This method is intentionally pub(crate) to enforce builder-only construction
    pub(crate) fn new(
        initial_state: S,
        transitions: HashMap<(String, String), TransitionHandler<S, E, C, A>>,
        entry_handlers: HashMap<String, StateHandler<S, C, A>>,
        exit_handlers: HashMap<String, StateHandler<S, C, A>>,
        timeout_handlers: HashMap<String, (Duration, TimeoutHandler<S, C, A>)>,
        unhandled_handler: Option<
            Arc<dyn for<'a> Fn(&'a S, &'a E, &'a mut C) -> crate::types::BoxFuture<'a, FsmResult<()>> + Send + Sync>,
        >,
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

    /// Get the current state
    pub fn state(&self) -> &S {
        &self.current_state
    }

    /// Check if a timeout has occurred for the current state
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

    /// Handle an event and potentially transition to a new state
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
                    Err(FsmError::UnhandledEvent { state: state_name, event: event_name })
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

        debug!("FSM transitioned from {} to {}", old_state_name, new_state_name);

        // Add transition actions
        all_actions.extend(transition.actions);

        Ok(all_actions)
    }
    
    /// Execute a list of actions with the given context
    pub async fn execute_actions(
        &self,
        actions: Vec<A>,
        context: &mut C,
    ) -> FsmResult<()> {
        for action in actions {
            action.execute(context).await?;
        }
        Ok(())
    }
}
