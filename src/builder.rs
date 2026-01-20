//! Fluent builder API for creating FSMs (inspired by Akka Classic FSM)

#![allow(deprecated)]

use crate::handlers::{StateHandler, TimeoutHandler, TransitionHandler};
use crate::machine::StateMachine;
use crate::types::{
    BoxFuture, EventVariant, FsmAction, FsmContext, FsmResult, StateVariant, Transition,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;
use tokio::time::Duration;

type UnhandledHandler<S, E, C> = Arc<
    dyn for<'a> Fn(&'a S, &'a E, &'a mut C) -> BoxFuture<'a, FsmResult<()>> + Send + Sync,
>;

/// Main builder for creating FSMs.
///
/// This type is now considered an internal implementation detail of the
/// `fsm!` macro. New code should use the typed DSL (`fsm!`) instead of
/// constructing FSMs directly via `FsmBuilder`.
#[deprecated(
    since = "0.3.0",
    note = "Use the `fsm!` macro from obzenflow_fsm instead; FsmBuilder is now an internal implementation detail used by the macro expansion."
)]
pub struct FsmBuilder<S, E, C, A> {
    initial_state: S,
    transitions: HashMap<(String, String), TransitionHandler<S, E, C, A>>,
    entry_handlers: HashMap<String, StateHandler<S, C, A>>,
    exit_handlers: HashMap<String, StateHandler<S, C, A>>,
    timeout_handlers: HashMap<String, (Duration, TimeoutHandler<S, C, A>)>,
    /// Duplicate (state, event) handlers detected during configuration
    duplicate_handlers: Vec<(String, String)>,
    /// When true, apply additional validation at build time
    strict_validation: bool,
    unhandled_handler: Option<UnhandledHandler<S, E, C>>,
    _phantom: PhantomData<(E, C)>,
}

impl<S, E, C, A> FsmBuilder<S, E, C, A>
where
    S: StateVariant,
    E: EventVariant,
    C: FsmContext + 'static,
    A: FsmAction<Context = C> + 'static,
{
    /// Create a new FSM builder with an initial state
    pub fn new(initial_state: S) -> Self {
        Self {
            initial_state,
            transitions: HashMap::new(),
            entry_handlers: HashMap::new(),
            exit_handlers: HashMap::new(),
            timeout_handlers: HashMap::new(),
            duplicate_handlers: Vec::new(),
            // FLOWIP-FSM: Strict validation is enabled by default.
            // This means all FSMs created through FsmBuilder will:
            // - Reject duplicate (state, event) handlers (always-on),
            // - Require the initial state to have at least one transition
            //   or timeout configured.
            //
            // The `strict()` method remains for readability and tests, but
            // calling `new()` alone is now equivalent to `new().strict()`.
            strict_validation: true,
            unhandled_handler: None,
            _phantom: PhantomData,
        }
    }

    /// Enable strict validation at build time.
    ///
    /// In strict mode, `try_build` will:
    /// - Fail on any detected duplicate `(state, event)` handler, and
    /// - Require that the initial state has at least one transition or timeout.
    pub fn strict(mut self) -> Self {
        self.strict_validation = true;
        self
    }

    /// Start defining behavior for a specific state
    #[deprecated(
        since = "0.3.0",
        note = "Use the typed fsm! DSL instead of string-based state names. This method will become crate-private in obzenflow-fsm 0.3.0."
    )]
    pub fn when(self, state_name: &str) -> WhenBuilder<S, E, C, A> {
        WhenBuilder {
            builder: self,
            state_name: state_name.to_string(),
        }
    }

    /// Define behavior for any state (wildcard)
    #[deprecated(
        since = "0.3.0",
        note = "Use the typed fsm! DSL and an explicit wildcard state instead of from_any(). This method will become crate-private in obzenflow-fsm 0.3.0."
    )]
    pub fn from_any(self) -> WhenBuilder<S, E, C, A> {
        WhenBuilder {
            builder: self,
            state_name: "_".to_string(),
        }
    }

    /// Define handler for unhandled events
    pub fn when_unhandled<F>(mut self, handler: F) -> Self
    where
        F: for<'a> Fn(&'a S, &'a E, &'a mut C) -> BoxFuture<'a, FsmResult<()>>
            + Send
            + Sync
            + 'static,
    {
        self.unhandled_handler = Some(Arc::new(move |s, e, c| handler(s, e, c)));
        self
    }

    /// Define entry handler for a state
    #[deprecated(
        since = "0.3.0",
        note = "Use the typed fsm! DSL with on_entry blocks instead of string-based on_entry(). This method will become crate-private in obzenflow-fsm 0.3.0."
    )]
    pub fn on_entry<F>(mut self, state_name: &str, handler: F) -> Self
    where
        F: for<'a> Fn(&'a S, &'a mut C) -> BoxFuture<'a, FsmResult<Vec<A>>> + Send + Sync + 'static,
    {
        self.entry_handlers
            .insert(state_name.to_string(), Arc::new(move |s, c| handler(s, c)));
        self
    }

    /// Define exit handler for a state
    #[deprecated(
        since = "0.3.0",
        note = "Use the typed fsm! DSL with on_exit blocks instead of string-based on_exit(). This method will become crate-private in obzenflow-fsm 0.3.0."
    )]
    pub fn on_exit<F>(mut self, state_name: &str, handler: F) -> Self
    where
        F: for<'a> Fn(&'a S, &'a mut C) -> BoxFuture<'a, FsmResult<Vec<A>>> + Send + Sync + 'static,
    {
        self.exit_handlers
            .insert(state_name.to_string(), Arc::new(move |s, c| handler(s, c)));
        self
    }

    /// Build the final state machine, panicking on builder errors.
    ///
    /// For fallible construction, use `try_build` instead.
    pub fn build(self) -> StateMachine<S, E, C, A> {
        match self.try_build() {
            Ok(machine) => machine,
            Err(err) => panic!("FsmBuilder::build failed: {err}"),
        }
    }

    /// Build the final state machine, returning a `FsmResult`.
    pub fn try_build(self) -> FsmResult<StateMachine<S, E, C, A>> {
        // 1) Duplicate handler validation
        if let Some((state, event)) = self.duplicate_handlers.first() {
            return Err(crate::error::FsmError::DuplicateHandler {
                state: state.clone(),
                event: event.clone(),
            });
        }

        // 2) Strict-mode structural checks
        if self.strict_validation {
            let initial_state_name = self.initial_state.variant_name().to_string();

            let has_transition = self
                .transitions
                .keys()
                .any(|(state_name, _)| state_name == &initial_state_name);
            let has_timeout = self.timeout_handlers.contains_key(&initial_state_name);

            if !has_transition && !has_timeout {
                return Err(crate::error::FsmError::BuilderError(format!(
                    "No transitions or timeout configured for initial state '{initial_state_name}'"
                )));
            }
        }

        Ok(StateMachine::new(
            self.initial_state,
            self.transitions,
            self.entry_handlers,
            self.exit_handlers,
            self.timeout_handlers,
            self.unhandled_handler,
        ))
    }
}

/// Builder for defining state-specific behavior
pub struct WhenBuilder<S, E, C, A> {
    builder: FsmBuilder<S, E, C, A>,
    state_name: String,
}

impl<S, E, C, A> WhenBuilder<S, E, C, A>
where
    S: StateVariant,
    E: EventVariant,
    C: FsmContext + 'static,
    A: FsmAction<Context = C> + 'static,
{
    /// Define a transition for a specific event
    #[deprecated(
        since = "0.3.0",
        note = "Use the typed fsm! DSL with `on Event::Variant => { … }` instead of string-based event names. This method will become crate-private in obzenflow-fsm 0.3.0."
    )]
    pub fn on<F>(mut self, event_name: &str, handler: F) -> Self
    where
        F: for<'a> Fn(&'a S, &'a E, &'a mut C) -> BoxFuture<'a, FsmResult<Transition<S, A>>>
            + Send
            + Sync
            + 'static,
    {
        let key = (self.state_name.clone(), event_name.to_string());
        if self.builder.transitions.contains_key(&key) {
            self.builder
                .duplicate_handlers
                .push((key.0.clone(), key.1.clone()));
        }
        self.builder
            .transitions
            .insert(key, Arc::new(move |s, e, c| handler(s, e, c)));
        self
    }

    /// Define a timeout for this state
    pub fn timeout<F>(self, duration: Duration, handler: F) -> TimeoutBuilder<S, E, C, A>
    where
        F: for<'a> Fn(&'a S, &'a mut C) -> BoxFuture<'a, FsmResult<Transition<S, A>>>
            + Send
            + Sync
            + 'static,
    {
        let mut builder = self.builder;
        builder.timeout_handlers.insert(
            self.state_name.clone(),
            (duration, Arc::new(move |s, c| handler(s, c))),
        );
        TimeoutBuilder {
            builder,
            state_name: self.state_name,
        }
    }

    /// Continue with the builder (ends this state's configuration)
    pub fn done(self) -> FsmBuilder<S, E, C, A> {
        self.builder
    }
}

/// Builder for states with timeouts
pub struct TimeoutBuilder<S, E, C, A> {
    builder: FsmBuilder<S, E, C, A>,
    state_name: String,
}

impl<S, E, C, A> TimeoutBuilder<S, E, C, A>
where
    S: StateVariant,
    E: EventVariant,
    C: FsmContext + 'static,
    A: FsmAction<Context = C> + 'static,
{
    /// Define a transition for a specific event
    #[deprecated(
        since = "0.3.0",
        note = "Use the typed fsm! DSL with `timeout` blocks instead of string-based event names. This method will become crate-private in obzenflow-fsm 0.3.0."
    )]
    pub fn on<F>(mut self, event_name: &str, handler: F) -> Self
    where
        F: for<'a> Fn(&'a S, &'a E, &'a mut C) -> BoxFuture<'a, FsmResult<Transition<S, A>>>
            + Send
            + Sync
            + 'static,
    {
        self.builder.transitions.insert(
            (self.state_name.clone(), event_name.to_string()),
            Arc::new(move |s, e, c| handler(s, e, c)),
        );
        self
    }

    /// Return to the main builder
    pub fn done(self) -> FsmBuilder<S, E, C, A> {
        self.builder
    }
}

/// Convenience functions for creating transitions
pub mod transitions {
    use crate::Transition;

    /// Transition to a new state
    pub fn goto<S, A>(state: S) -> Transition<S, A> {
        Transition {
            next_state: state,
            actions: vec![],
        }
    }

    /// Transition to a new state with actions
    pub fn goto_with_actions<S, A>(state: S, actions: Vec<A>) -> Transition<S, A> {
        Transition {
            next_state: state,
            actions,
        }
    }

    /// Stay in the current state
    pub fn stay<S: Clone, A>(current_state: &S) -> Transition<S, A> {
        Transition {
            next_state: current_state.clone(),
            actions: vec![],
        }
    }

    /// Stay in the current state with actions
    pub fn stay_with_actions<S: Clone, A>(current_state: &S, actions: Vec<A>) -> Transition<S, A> {
        Transition {
            next_state: current_state.clone(),
            actions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    enum DoorState {
        Closed,
        Open { since: std::time::Instant },
        Locked,
    }

    impl StateVariant for DoorState {
        fn variant_name(&self) -> &str {
            match self {
                DoorState::Closed => "Closed",
                DoorState::Open { .. } => "Open",
                DoorState::Locked => "Locked",
            }
        }
    }

    #[derive(Clone, Debug)]
    enum DoorEvent {
        Open,
        Close,
        Lock,
        Unlock,
    }

    impl EventVariant for DoorEvent {
        fn variant_name(&self) -> &str {
            match self {
                DoorEvent::Open => "Open",
                DoorEvent::Close => "Close",
                DoorEvent::Lock => "Lock",
                DoorEvent::Unlock => "Unlock",
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum DoorAction {
        Ring,
        Log(String),
    }

    struct DoorContext;

    impl crate::FsmContext for DoorContext {
        fn describe(&self) -> String {
            "Door FSM context".to_string()
        }
    }

    #[async_trait::async_trait]
    impl crate::FsmAction for DoorAction {
        type Context = DoorContext;

        async fn execute(&self, _ctx: &mut Self::Context) -> crate::types::FsmResult<()> {
            match self {
                DoorAction::Ring => {
                    println!("Ring!");
                    Ok(())
                }
                DoorAction::Log(msg) => {
                    println!("Log: {msg}");
                    Ok(())
                }
            }
        }
    }

    #[tokio::test]
    async fn test_door_fsm() {
        let mut door = crate::fsm! {
            state:   DoorState;
            event:   DoorEvent;
            context: DoorContext;
            action:  DoorAction;
            initial: DoorState::Closed;

            state DoorState::Closed {
                on DoorEvent::Open => |_state: &DoorState, _event: &DoorEvent, _ctx: &mut DoorContext| {
                    Box::pin(async move {
                        Ok(Transition {
                            next_state: DoorState::Open {
                                since: std::time::Instant::now(),
                            },
                            actions: vec![DoorAction::Ring, DoorAction::Log("Door opened".into())],
                        })
                    })
                };

                on DoorEvent::Lock => |_state: &DoorState, _event: &DoorEvent, _ctx: &mut DoorContext| {
                    Box::pin(async move {
                        Ok(Transition {
                            next_state: DoorState::Locked,
                            actions: vec![DoorAction::Log("Door locked".into())],
                        })
                    })
                };
            }

            state DoorState::Open {
                // Model the timeout via a special event; concrete timeout wiring is
                // exercised elsewhere, here we only care about the resulting transition.
                on DoorEvent::Close => |_state: &DoorState, _event: &DoorEvent, _ctx: &mut DoorContext| {
                    Box::pin(async move {
                        Ok(Transition {
                            next_state: DoorState::Closed,
                            actions: vec![DoorAction::Log("Door closed".into())],
                        })
                    })
                };
            }

            state DoorState::Locked {
                on DoorEvent::Unlock => |_state: &DoorState, _event: &DoorEvent, _ctx: &mut DoorContext| {
                    Box::pin(async move {
                        Ok(Transition {
                            next_state: DoorState::Closed,
                            actions: vec![DoorAction::Log("Door unlocked".into())],
                        })
                    })
                };
            }
        };
        let mut ctx = DoorContext;

        // Test basic transition
        assert!(matches!(door.state(), DoorState::Closed));

        let actions = door.handle(DoorEvent::Open, &mut ctx).await.unwrap();
        // We expect two actions from the transition: Ring + Log("Door opened")
        assert_eq!(actions.len(), 2);
        assert!(matches!(actions[0], DoorAction::Ring));
        assert!(matches!(actions[1], DoorAction::Log(_)));
        assert!(matches!(door.state(), DoorState::Open { .. }));

        // Test another transition - this should work
        let actions = door.handle(DoorEvent::Close, &mut ctx).await.unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], DoorAction::Log(_)));
        assert!(matches!(door.state(), DoorState::Closed));

        // Exercise Lock/Unlock so the enum variants are constructed.
        let actions = door.handle(DoorEvent::Lock, &mut ctx).await.unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(door.state(), DoorState::Locked));

        let actions = door.handle(DoorEvent::Unlock, &mut ctx).await.unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(door.state(), DoorState::Closed));
    }
}
