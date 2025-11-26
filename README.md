# ObzenFlow FSM

Finite State Machines are at the heart of how ObzenFlow works. This is our own custom implementation of a **Mealy machine**, that makes intention tradeoffs to support async closures for the right balance between correctness and architectural flexibility. Flexibility is required due to ObzenFlow's extremely expressive middleware and monitoring systems. 

## Project Naming

- **ObzenFlow** is the main workflow/orchestration project and overall system.
- **ObzenFlow FSM** is this reusable finite state machine library that powers ObzenFlow, published as the Rust crate `obzenflow-fsm`.

## Context Ownership Model (`0.2.x`)

Starting in `0.2.0`, this library uses a **single-owner mutable context** API:

- The FSM does **not** require `Arc<Context>` anymore.
- Instead, the caller (typically a supervisor) owns the context by value and passes `&mut Context` into the FSM.

At a high level you use it like this:

```rust
let mut context = MyContext::new(...);
let mut fsm = FsmBuilder::new(MyState::Idle)
    .when("Idle")
        .on("Start", |state, event, ctx: &mut MyContext| async move {
            // ctx is &mut MyContext; mutate it directly
            ctx.counters.starts += 1;
            Ok(Transition {
                next_state: MyState::Running,
                actions: vec![MyAction::Log("started".into())],
            })
        })
        .done()
    .build();

let actions = fsm.handle(MyEvent::Start, &mut context).await?;
fsm.execute_actions(actions, &mut context).await?;
```

### Why this is still “functional in spirit”

Semantically, each step is still a Mealy-style transition:

- `(state, context, event) → (next_state, updated_context, actions)`

The `&mut Context` API is just an in-place encoding of that function:

- Rust’s borrow checker guarantees there is only **one mutable reference** to the context at a time.
- The supervisor owns `(StateMachine, Context)` and drives it step by step.

This aligns better with the “single writer per FSM” model and avoids the accidental shared mutability that `Arc<RwLock<…>>` encouraged in earlier versions.

### What about sharing?

If you truly need to share something across tasks (journals, message bus handles, metrics exporters, etc.), you should:

- Wrap just those resources in `Arc` (or channels) inside your context, and
- Continue to pass `&mut Context` into the FSM.

The FSM library itself no longer imposes `Arc<Context>`; sharing becomes an explicit design choice at the application level.

## Design Decisions: Why Not Typestate?

You might be wondering, why use runtime state machines when Rust has such powerful type system features? We actually considered the typestate pattern, which would give us compile-time guarantees about valid state transitions. Here's why we stuck with the enum-based approach:

### The Storage Dilemma

In a real async system, you need to store your state machine somewhere - typically as a field in a supervisor or actor. With typestate, the machine's type changes with every transition. This means you'd either need:
- Trait objects (goodbye type safety!)
- An enum wrapper (wait... that's just our current design with extra steps)
- A complete architectural overhaul

### Dynamic vs Static Dispatch

Our FSM thrives on runtime flexibility:
```rust
// This is what we want - configure behavior at runtime
.when("Running")
  .on("UserEvent", handle_user_event)
  .on("SystemEvent", handle_system_event)
  .on_any(log_all_events)
```

Typestate needs everything resolved at compile time. Different philosophies for different needs.

### The Features We'd Lose

Some of our most useful features just don't translate well to typestate:
- **Wildcard transitions** - "From any state, on ErrorEvent, go to Failed"
- **Unhandled event callbacks** - "Log any event we don't have a handler for"
- **Dynamic timeout handlers** - "If we're in Pending for 30s, transition to Timeout"
- **Entry/exit handlers** - Running cleanup code when leaving states

We're building for async, event-driven systems where events come from external sources (network, timers, user input). The typestate pattern shines when state transitions are driven by method calls in your own code. Different tools for different jobs!

### Getting Safety Without Typestate

Want similar guarantees? Here's what we recommend:
1. **State-specific event loops** - Only poll certain futures in certain states
2. **Invariant assertions** - `debug_assert!(matches!(self.state, State::Running))` 
3. **Builder patterns for complex states** - Make invalid states harder to construct
4. **Comprehensive tests** - Our test utilities make it easy to verify all transitions

The bottom line: our enum-based FSM is purpose-built for the realities of async Rust services. It's not a compromise - it's the right tool for this particular job.

## Theoretical Foundation: Mealy Machine with Modern Extensions

You might be curious how ObzenFlow FSM relates to traditional state machine theory. At its core, this implementation is closest to a **Mealy machine** with some practical adaptations for async Rust.

### Core Mealy Machine Characteristics

1. **Outputs depend on both state AND input**: Our transition handlers take both the current state and event as parameters to produce actions:
   ```rust
   pub type TransitionHandler<S, E, C, A> = Arc<
       dyn Fn(&S, &E, Arc<C>) -> Pin<Box<dyn Future<Output = Result<Transition<S, A>, String>> + Send>>
   ```

2. **Actions occur during transitions**: Actions are produced as part of the transition result, not just from being in a state:
   ```rust
   pub struct Transition<S, A> {
       pub next_state: S,
       pub actions: Vec<A>,  // Actions are part of the transition
   }
   ```

3. **The fundamental equation matches Mealy**: As stated in our design docs:
   > **State(S) × Event(E) → Actions(A), State(S')**
   
   This is exactly the Mealy machine model where outputs (actions) are determined by the combination of current state and input event.

### Why Not a Moore Machine?

In a Moore machine, outputs depend only on the current state, not the input. But in ObzenFlow FSM, the transition handlers explicitly receive both state AND event to determine actions. This gives us more flexibility for real-world use cases.

### Modern Extensions Beyond Classical Theory

1. **Entry/Exit actions**: We support state entry and exit handlers, which is more like a UML state machine feature:
   ```rust
   .on_entry("Running", |state, ctx| async {
       // Initialize resources when entering Running state
   })
   .on_exit("Running", |state, ctx| async {
       // Cleanup when leaving Running state
   })
   ```

2. **Async operations**: All handlers are async, which is a practical adaptation for modern systems but not part of classical FSM theory.

3. **Context parameter**: The addition of a context parameter provides shared state across the FSM, enabling complex stateful operations while keeping the FSM itself pure.

4. **Timeout handlers**: State timeouts are supported, allowing automatic transitions based on time:
   ```rust
   .timeout("Pending", Duration::from_secs(30), |state, ctx| async {
       // Transition to Timeout state after 30 seconds
   })
   ```

### The Akka Heritage

Our design was inspired by Akka's classic FSM, which also follows the Mealy machine model. This heritage shows through in:
- The builder API design
- Support for unhandled event handlers
- The focus on actor-like isolation and message passing
- The emphasis on making state transitions explicit and auditable

### Practical Implications

Understanding that this is a Mealy machine helps explain certain design decisions:
- Why actions are returned from transitions (not just state entry)
- Why we pass both state and event to handlers
- Why the same event can produce different actions in different states
- Why we model side effects as explicit actions rather than implicit operations

This theoretical foundation, combined with our practical extensions for async Rust, creates a powerful abstraction for modeling complex stateful systems in a type-safe, testable way.
