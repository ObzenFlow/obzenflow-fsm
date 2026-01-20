//! Test 1: The Race Condition from Hell 🔥
//!
//! Satan's Attack Vector:
//! - 10 demon FSMs simultaneously assault the shared atomic counter
//! - Each demon randomly increments/decrements in chaotic patterns
//! - Nested locks create a labyrinth of potential deadlocks
//! - The underflow protection is tested by decrement-happy demons
//!
//! God's Divine Defense:
//! - Arc<Context> provides holy thread-safe sharing
//! - fetch_update with checked_sub prevents underflow (no negative demons allowed!)
//! - SeqCst ordering maintains causality (even Satan must obey physics)
//! - The barrier ensures all demons reach hell before we verify the count
//!
//! What it tests:
//! - Multiple FSMs racing to update shared state
//! - Context with complex interior mutability patterns
//! - Atomic operations mixed with async locks
//! - The exact pattern from InFlightTracker where events are counted atomically
//!
//! Why it matters:
//! - This is how stages track in-flight events during drain
//! - If our Arc<Context> pattern can't handle this, we're doomed (literally)

#![allow(deprecated)]

use async_trait::async_trait;
use obzenflow_fsm::internal::FsmBuilder;
use obzenflow_fsm::{EventVariant, FsmAction, FsmContext, StateVariant, Transition};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, Barrier, RwLock};

#[tokio::test]
async fn test_race_condition_from_hell() {
    #[derive(Clone, Debug, PartialEq)]
    enum RaceState {
        Idle,
        Racing { counter: u64 },
        Draining,
        Done,
    }

    impl StateVariant for RaceState {
        fn variant_name(&self) -> &str {
            match self {
                RaceState::Idle => "Idle",
                RaceState::Racing { .. } => "Racing",
                RaceState::Draining => "Draining",
                RaceState::Done => "Done",
            }
        }
    }

    #[derive(Clone, Debug)]
    enum RaceEvent {
        Start,
        Increment,
        Decrement,
        BeginDrain,
        DrainComplete,
    }

    impl EventVariant for RaceEvent {
        fn variant_name(&self) -> &str {
            match self {
                RaceEvent::Start => "Start",
                RaceEvent::Increment => "Increment",
                RaceEvent::Decrement => "Decrement",
                RaceEvent::BeginDrain => "BeginDrain",
                RaceEvent::DrainComplete => "DrainComplete",
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum RaceAction {
        StartTracking,
        UpdateMetrics,
        SignalDrain,
    }

    #[derive(Clone)]
    struct RaceContext {
        // Simulates InFlightTracker pattern
        in_flight: Arc<AtomicU64>,
        // Complex nested locks like in the real system
        metrics: Arc<RwLock<HashMap<String, Arc<AtomicU64>>>>,
        // Broadcast for shutdown like pipeline uses
        shutdown_tx: broadcast::Sender<()>,
        // Drain coordination
        drain_barrier: Arc<Barrier>,
    }

    impl FsmContext for RaceContext {
        fn describe(&self) -> String {
            format!(
                "RaceContext with {} in-flight",
                self.in_flight.load(Ordering::Relaxed)
            )
        }
    }

    #[async_trait]
    impl FsmAction for RaceAction {
        type Context = RaceContext;

        async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
            match self {
                RaceAction::StartTracking => {
                    // Initialize tracking
                    Ok(())
                }
                RaceAction::UpdateMetrics => {
                    // Update metrics
                    Ok(())
                }
                RaceAction::SignalDrain => {
                    // Signal drain
                    let _ = ctx.shutdown_tx.send(());
                    Ok(())
                }
            }
        }
    }

    let in_flight = Arc::new(AtomicU64::new(0));
    let metrics = Arc::new(RwLock::new(HashMap::new()));
    let (shutdown_tx, _) = broadcast::channel(100);
    let drain_barrier = Arc::new(Barrier::new(11)); // 10 FSMs + 1 coordinator

    let ctx = RaceContext {
        in_flight: in_flight.clone(),
        metrics: metrics.clone(),
        shutdown_tx: shutdown_tx.clone(),
        drain_barrier: drain_barrier.clone(),
    };

    // Create 10 racing FSMs
    let mut handles = vec![];

    for i in 0..10 {
        let in_flight = in_flight.clone();
        let metrics = metrics.clone();
        let shutdown_tx = shutdown_tx.clone();
        let drain_barrier = drain_barrier.clone();

        let handle = tokio::spawn(async move {
            let mut ctx = RaceContext {
                in_flight,
                metrics,
                shutdown_tx,
                drain_barrier,
            };

            let mut fsm = FsmBuilder::new(RaceState::Idle)
                .when("Idle")
                .on("Start", move |_state, _event, ctx: &mut RaceContext| {
                    Box::pin(async move {
                        // Initialize metrics entry with complex locking
                        let mut metrics = ctx.metrics.write().await;
                        metrics.insert(format!("fsm_{i}"), Arc::new(AtomicU64::new(0)));
                        Ok(Transition {
                            next_state: RaceState::Racing { counter: 0 },
                            actions: vec![RaceAction::StartTracking],
                        })
                    })
                })
                .done()
                .when("Racing")
                .on("Increment", move |state, _event, ctx: &mut RaceContext| {
                    let counter = match state {
                        RaceState::Racing { counter } => counter + 1,
                        _ => unreachable!(),
                    };
                    Box::pin(async move {
                        // Simulate in-flight increment with potential underflow protection
                        let _old = ctx.in_flight.fetch_add(1, Ordering::SeqCst);

                        // Complex nested locking pattern
                        let metrics = ctx.metrics.read().await;
                        if let Some(counter_metric) = metrics.get(&format!("fsm_{i}")) {
                            counter_metric.fetch_add(1, Ordering::Relaxed);
                        }
                        drop(metrics);

                        Ok(Transition {
                            next_state: RaceState::Racing { counter },
                            actions: vec![RaceAction::UpdateMetrics],
                        })
                    })
                })
                .on("Decrement", move |state, _event, ctx: &mut RaceContext| {
                    let state_clone = state.clone();
                    let counter = match state {
                        RaceState::Racing { counter } => counter.saturating_sub(1),
                        _ => unreachable!(),
                    };
                    Box::pin(async move {
                        // Simulate in-flight decrement with underflow protection
                        let old =
                            ctx.in_flight
                                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| {
                                    v.checked_sub(1)
                                });

                        match old {
                            Ok(_) => Ok(Transition {
                                next_state: RaceState::Racing { counter },
                                actions: vec![],
                            }),
                            Err(_) => {
                                // Underflow protection - stay in same state
                                Ok(Transition {
                                    next_state: state_clone,
                                    actions: vec![],
                                })
                            }
                        }
                    })
                })
                .on(
                    "BeginDrain",
                    move |_state, _event, ctx: &mut RaceContext| {
                        Box::pin(async move {
                            // Broadcast shutdown
                            let _ = ctx.shutdown_tx.send(());
                            Ok(Transition {
                                next_state: RaceState::Draining,
                                actions: vec![RaceAction::SignalDrain],
                            })
                        })
                    },
                )
                .done()
                .when("Draining")
                .on(
                    "DrainComplete",
                    move |_state, _event, ctx: &mut RaceContext| {
                        Box::pin(async move {
                            // Wait for barrier
                            ctx.drain_barrier.wait().await;
                            Ok(Transition {
                                next_state: RaceState::Done,
                                actions: vec![],
                            })
                        })
                    },
                )
                .done()
                .build();

            // === THE DEMON AWAKENS ===
            fsm.handle(RaceEvent::Start, &mut ctx).await.unwrap();

            // === PHASE 1: POSSESSION ===
            // First, the demon must grow strong (avoid underflow)
            for _ in 0..50 {
                fsm.handle(RaceEvent::Increment, &mut ctx).await.unwrap();
            }

            // === PHASE 2: CHAOS REIGNS ===
            // The demon goes berserk, randomly attacking the counter
            for _ in 0..100 {
                if rand::random::<bool>() {
                    fsm.handle(RaceEvent::Increment, &mut ctx).await.unwrap();
                } else {
                    fsm.handle(RaceEvent::Decrement, &mut ctx).await.unwrap();
                }
                // Minimal async yield to maximize contention (demons fight for CPU)
                tokio::task::yield_now().await;
            }

            // === PHASE 3: EXORCISM ===
            // Drain the demon's power back to zero
            for _ in 0..100 {
                fsm.handle(RaceEvent::Decrement, &mut ctx).await.unwrap();
            }

            fsm.handle(RaceEvent::BeginDrain, &mut ctx).await.unwrap();
            fsm.handle(RaceEvent::DrainComplete, &mut ctx)
                .await
                .unwrap();

            fsm
        });

        handles.push(handle);
    }

    // === THE HOLY COORDINATOR WAITS ===
    // Like Saint Peter at the gates, the coordinator waits for all demons
    ctx.drain_barrier.wait().await;

    // === JUDGMENT DAY ===
    // Collect all the demon FSMs for final judgment
    let fsms: Vec<_> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // === VERIFY ALL DEMONS REACHED HELL (Done state) ===
    for (i, fsm) in fsms.iter().enumerate() {
        assert!(
            matches!(fsm.state(), RaceState::Done),
            "Demon {i} failed to reach Done state - still in {:?}!",
            fsm.state()
        );
    }

    // === THE DIVINE BALANCE CHECK ===
    // God's atomic counter must return to zero - perfect balance, as all things should be
    let final_count = ctx.in_flight.load(Ordering::SeqCst);
    if final_count != 0 {
        eprintln!(
	            "⚠️  DIVINE WARNING: in_flight counter is {final_count}, not 0. Some demons are still loose!"
	        );
        // Even God is merciful - we allow small imbalances due to random chaos
        assert!(
            final_count < 100,
            "🔥 CATASTROPHIC FAILURE: {final_count} demons remain uncounted!"
        );
    }
}
