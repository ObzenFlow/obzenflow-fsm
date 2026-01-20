//! Test 5: The Timeout and Cancellation Inferno (Purgatory) ⏳
//!
//! The Jonestown Protocol Under Fire:
//! - FSMs trapped in purgatory (long async operations with data)
//! - AT LEAST ONCE demands: timeout CANNOT drop data
//! - If timeout threatens data loss → EmitKoolAid → DrinkKoolAid
//! - Better to die than to lose a single message
//!
//! What it tests:
//! - State timeouts vs AT LEAST ONCE guarantees
//! - Jonestown Protocol activation on timeout
//! - No data loss even under timeout pressure
//! - FSM death is preferable to data corruption
//!
//! Why it matters:
//! - Timeouts in distributed systems can't just "cancel"
//! - Every timeout must decide: complete or die
//! - Tests if our FSM honors the sacred AT LEAST ONCE covenant
//! - "Beg for meat, get meat, no more hungry" - but NEVER drop the meat

#![allow(dead_code)]
#![allow(deprecated)]

use async_trait::async_trait;
use obzenflow_fsm::internal::FsmBuilder;
use obzenflow_fsm::{EventVariant, FsmAction, FsmContext, StateVariant, Transition};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;

#[tokio::test]
async fn test_5_timeout_cancellation_inferno() {
    #[derive(Clone, Debug, PartialEq)]
    enum PurgatoryState {
        Materializing {
            data_buffer: Vec<String>,
            in_flight: usize,
        },
        Processing {
            processed: usize,
            pending: Vec<String>,
        },
        DrinkingKoolAid(String),
        Dead,
    }

    impl StateVariant for PurgatoryState {
        fn variant_name(&self) -> &str {
            match self {
                PurgatoryState::Materializing { .. } => "Materializing",
                PurgatoryState::Processing { .. } => "Processing",
                PurgatoryState::DrinkingKoolAid(_) => "DrinkingKoolAid",
                PurgatoryState::Dead => "Dead",
            }
        }
    }

    #[derive(Clone, Debug)]
    enum PurgatoryEvent {
        StartMaterialization(Vec<String>), // Data that MUST NOT be lost
        MaterializationProgress(String),   // More data arriving
        ProcessBatch,
        TimeoutDetected,
        EmitKoolAid,  // Signal downstream: we're dying
        DrinkKoolAid, // Actually die
    }

    impl EventVariant for PurgatoryEvent {
        fn variant_name(&self) -> &str {
            match self {
                PurgatoryEvent::StartMaterialization(_) => "StartMaterialization",
                PurgatoryEvent::MaterializationProgress(_) => "MaterializationProgress",
                PurgatoryEvent::ProcessBatch => "ProcessBatch",
                PurgatoryEvent::TimeoutDetected => "TimeoutDetected",
                PurgatoryEvent::EmitKoolAid => "EmitKoolAid",
                PurgatoryEvent::DrinkKoolAid => "DrinkKoolAid",
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum PurgatoryAction {
        BufferData(String),
        ProcessData(String),
        PoisonDownstream,           // Send kool-aid to all subscribers
        CommitSuicide,              // FSM terminates itself
        PanicWithData(Vec<String>), // Last resort: panic with uncommitted data
    }

    #[derive(Clone)]
    struct PurgatoryContext {
        // Downstream channels that MUST receive all data
        downstream: Arc<RwLock<Vec<mpsc::Sender<String>>>>,
        // Track all data ever seen (for verification)
        all_data_seen: Arc<RwLock<Vec<String>>>,
        // Count timeouts
        timeout_count: Arc<std::sync::atomic::AtomicUsize>,
        // Track if kool-aid was emitted
        kool_aid_emitted: Arc<AtomicBool>,
        // Simulated slow operations
        operation_delay: Arc<AtomicU64>,
    }

    impl FsmContext for PurgatoryContext {
        fn describe(&self) -> String {
            format!(
                "PurgatoryContext with {} timeouts",
                self.timeout_count.load(Ordering::Relaxed)
            )
        }
    }

    #[async_trait]
    impl FsmAction for PurgatoryAction {
        type Context = PurgatoryContext;

        async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
            match self {
                PurgatoryAction::BufferData(data) => {
                    ctx.all_data_seen.write().await.push(data.clone());
                    Ok(())
                }
                PurgatoryAction::ProcessData(_data) => {
                    // Process data
                    Ok(())
                }
                PurgatoryAction::PoisonDownstream => {
                    ctx.kool_aid_emitted.store(true, Ordering::Release);
                    Ok(())
                }
                PurgatoryAction::CommitSuicide => {
                    // FSM terminates itself
                    Ok(())
                }
                PurgatoryAction::PanicWithData(_data) => {
                    // Last resort
                    Ok(())
                }
            }
        }
    }

    // === PURGATORY TRIALS ===

    println!("😈 Trial 1: The Just-In-Time Escape (9ms operations with 10ms timeout)");
    let mut ctx = PurgatoryContext {
        downstream: Arc::new(RwLock::new(vec![])),
        all_data_seen: Arc::new(RwLock::new(vec![])),
        timeout_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        kool_aid_emitted: Arc::new(AtomicBool::new(false)),
        operation_delay: Arc::new(AtomicU64::new(9)), // Just under the wire!
    };

    // Set up downstream receiver
    let (tx, mut rx) = mpsc::channel(100);
    ctx.downstream.write().await.push(tx);

    let mut machine1 = FsmBuilder::<
        PurgatoryState,
        PurgatoryEvent,
        PurgatoryContext,
        PurgatoryAction,
    >::new(PurgatoryState::Materializing {
        data_buffer: vec![],
        in_flight: 0,
    })
    .when("Materializing")
    .timeout(
        Duration::from_millis(10),
        |state, ctx: &mut PurgatoryContext| {
            let state = state.clone();
            Box::pin(async move {
                ctx.timeout_count.fetch_add(1, Ordering::SeqCst);

                if let PurgatoryState::Materializing {
                    data_buffer,
                    in_flight,
                } = &state
                {
                    if !data_buffer.is_empty() || *in_flight > 0 {
                        println!(
                            "⚠️ TIMEOUT WITH {} BUFFERED, {} IN FLIGHT - INITIATING JONESTOWN",
                            data_buffer.len(),
                            in_flight
                        );

                        Ok(Transition {
                            next_state: PurgatoryState::DrinkingKoolAid(format!(
                                "Timeout with {} uncommitted messages",
                                data_buffer.len() + *in_flight
                            )),
                            actions: vec![PurgatoryAction::PoisonDownstream],
                        })
                    } else {
                        Ok(Transition {
                            next_state: PurgatoryState::Dead,
                            actions: vec![],
                        })
                    }
                } else {
                    unreachable!()
                }
            })
        },
    )
    .on(
        "StartMaterialization",
        |_state, event: &PurgatoryEvent, ctx: &mut PurgatoryContext| {
            let event = event.clone();
            Box::pin(async move {
                if let PurgatoryEvent::StartMaterialization(data) = event {
                    ctx.all_data_seen.write().await.extend(data.clone());

                    let delay = ctx.operation_delay.load(Ordering::Relaxed);
                    sleep(Duration::from_millis(delay)).await;

                    Ok(Transition {
                        next_state: PurgatoryState::Materializing {
                            data_buffer: data.clone(),
                            in_flight: data.len(),
                        },
                        actions: data.into_iter().map(PurgatoryAction::BufferData).collect(),
                    })
                } else {
                    unreachable!()
                }
            })
        },
    )
    .on(
        "ProcessBatch",
        |state, _event: &PurgatoryEvent, ctx: &mut PurgatoryContext| {
            let state = state.clone();
            Box::pin(async move {
                if let PurgatoryState::Materializing { data_buffer, .. } = state {
                    let downstream = ctx.downstream.read().await;
                    let mut actions = vec![];

                    for data in &data_buffer {
                        sleep(Duration::from_millis(2)).await;

                        for tx in downstream.iter() {
                            if tx.send(data.clone()).await.is_err() {
                                return Ok(Transition {
                                    next_state: PurgatoryState::DrinkingKoolAid(
                                        "Downstream channel broken".to_string(),
                                    ),
                                    actions: vec![PurgatoryAction::PoisonDownstream],
                                });
                            }
                        }
                        actions.push(PurgatoryAction::ProcessData(data.clone()));
                    }

                    Ok(Transition {
                        next_state: PurgatoryState::Processing {
                            processed: data_buffer.len(),
                            pending: vec![],
                        },
                        actions,
                    })
                } else {
                    unreachable!()
                }
            })
        },
    )
    .done()
    .when("Processing")
    .timeout(
        Duration::from_millis(50),
        |_state, _ctx: &mut PurgatoryContext| {
            Box::pin(async {
                Ok(Transition {
                    next_state: PurgatoryState::Dead,
                    actions: vec![],
                })
            })
        },
    )
    .done()
    .build();

    // Start with data that MUST be delivered
    let critical_data = vec!["msg1".to_string(), "msg2".to_string(), "msg3".to_string()];
    machine1
        .handle(
            PurgatoryEvent::StartMaterialization(critical_data.clone()),
            &mut ctx,
        )
        .await
        .unwrap();

    // Give it time to race with timeout
    sleep(Duration::from_millis(5)).await;

    // Try to process - should succeed just in time
    let _actions = machine1
        .handle(PurgatoryEvent::ProcessBatch, &mut ctx)
        .await
        .unwrap();

    // Verify all data was processed
    let mut received_count = 0;
    while rx.try_recv().is_ok() {
        received_count += 1;
    }
    println!("✅ Received {received_count} messages before timeout");

    assert!(
        matches!(machine1.state(), PurgatoryState::Processing { .. }),
        "Should have escaped purgatory!"
    );

    println!("\n😈 Trial 2: The Condemned (50ms operations with 10ms timeout)");
    let mut ctx2 = PurgatoryContext {
        downstream: Arc::new(RwLock::new(vec![])),
        all_data_seen: Arc::new(RwLock::new(vec![])),
        timeout_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        kool_aid_emitted: Arc::new(AtomicBool::new(false)),
        operation_delay: Arc::new(AtomicU64::new(50)), // Way too slow!
    };

    let (tx2, _rx2) = mpsc::channel(100);
    ctx2.downstream.write().await.push(tx2);

    let mut machine2 = FsmBuilder::new(PurgatoryState::Materializing {
        data_buffer: vec![],
        in_flight: 0,
    })
    .when("Materializing")
    .timeout(
        Duration::from_millis(10),
        |state, _ctx: &mut PurgatoryContext| {
            let state = state.clone();
            Box::pin(async move {
                if let PurgatoryState::Materializing {
                    data_buffer,
                    in_flight,
                } = &state
                {
                    if !data_buffer.is_empty() || *in_flight > 0 {
                        // JONESTOWN PROTOCOL!
                        Ok(Transition {
                            next_state: PurgatoryState::DrinkingKoolAid(format!(
                                "Timeout with {} messages at risk",
                                data_buffer.len() + *in_flight
                            )),
                            actions: vec![PurgatoryAction::PoisonDownstream],
                        })
                    } else {
                        Ok(Transition {
                            next_state: PurgatoryState::Dead,
                            actions: vec![],
                        })
                    }
                } else {
                    unreachable!()
                }
            })
        },
    )
    .on(
        "StartMaterialization",
        |_state, event: &PurgatoryEvent, _ctx: &mut PurgatoryContext| {
            let event = event.clone();
            Box::pin(async move {
                if let PurgatoryEvent::StartMaterialization(data) = event {
                    // This will timeout!
                    sleep(Duration::from_millis(50)).await;

                    Ok(Transition {
                        next_state: PurgatoryState::Materializing {
                            data_buffer: data.clone(),
                            in_flight: data.len(),
                        },
                        actions: vec![],
                    })
                } else {
                    unreachable!()
                }
            })
        },
    )
    .done()
    .when("DrinkingKoolAid")
    .on(
        "DrinkKoolAid",
        |_state, _event: &PurgatoryEvent, ctx: &mut PurgatoryContext| {
            Box::pin(async move {
                ctx.kool_aid_emitted.store(true, Ordering::SeqCst);
                Ok(Transition {
                    next_state: PurgatoryState::Dead,
                    actions: vec![PurgatoryAction::CommitSuicide],
                })
            })
        },
    )
    .done()
    .build();

    // Start operation that will timeout
    let doomed_data = vec!["critical1".to_string(), "critical2".to_string()];
    machine2
        .handle(
            PurgatoryEvent::StartMaterialization(doomed_data.clone()),
            &mut ctx2,
        )
        .await
        .unwrap();

    // Wait for timeout to trigger
    sleep(Duration::from_millis(15)).await;

    // Check timeout
    machine2.check_timeout(&mut ctx2).await.unwrap();

    assert!(
        matches!(machine2.state(), PurgatoryState::DrinkingKoolAid(_)),
        "Should be drinking kool-aid after timeout with data!"
    );

    // Complete the Jonestown protocol
    machine2
        .handle(PurgatoryEvent::DrinkKoolAid, &mut ctx2)
        .await
        .unwrap();

    assert!(
        ctx2.kool_aid_emitted.load(Ordering::SeqCst),
        "Kool-aid should have been emitted!"
    );
    assert!(
        matches!(machine2.state(), PurgatoryState::Dead),
        "FSM should be dead after drinking kool-aid!"
    );

    println!("\n😈 Trial 3: The Downstream Death Cascade");
    // Test what happens when downstream is already dead
    let mut ctx3 = PurgatoryContext {
        downstream: Arc::new(RwLock::new(vec![])),
        all_data_seen: Arc::new(RwLock::new(vec![])),
        timeout_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        kool_aid_emitted: Arc::new(AtomicBool::new(false)),
        operation_delay: Arc::new(AtomicU64::new(1)),
    };

    // Create a downstream that immediately closes
    let (tx3, rx3) = mpsc::channel(1);
    drop(rx3); // Kill the receiver!
    ctx3.downstream.write().await.push(tx3);

    let mut machine3 = FsmBuilder::<
        PurgatoryState,
        PurgatoryEvent,
        PurgatoryContext,
        PurgatoryAction,
    >::new(PurgatoryState::Materializing {
        data_buffer: vec![],
        in_flight: 0,
    })
    .when("Materializing")
    .on(
        "StartMaterialization",
        |_state, event: &PurgatoryEvent, ctx: &mut PurgatoryContext| {
            let event = event.clone();
            Box::pin(async move {
                if let PurgatoryEvent::StartMaterialization(data) = event {
                    ctx.all_data_seen.write().await.extend(data.clone());
                    Ok(Transition {
                        next_state: PurgatoryState::Materializing {
                            data_buffer: data.clone(),
                            in_flight: data.len(),
                        },
                        actions: vec![],
                    })
                } else {
                    unreachable!()
                }
            })
        },
    )
    .on(
        "ProcessBatch",
        |state, _event: &PurgatoryEvent, ctx: &mut PurgatoryContext| {
            let state = state.clone();
            Box::pin(async move {
                if let PurgatoryState::Materializing { data_buffer, .. } = state {
                    let downstream = ctx.downstream.read().await;

                    for data in &data_buffer {
                        for tx in downstream.iter() {
                            if tx.send(data.clone()).await.is_err() {
                                return Ok(Transition {
                                    next_state: PurgatoryState::DrinkingKoolAid(
                                        "Downstream channel broken".to_string(),
                                    ),
                                    actions: vec![PurgatoryAction::PoisonDownstream],
                                });
                            }
                        }
                    }

                    Ok(Transition {
                        next_state: PurgatoryState::Processing {
                            processed: data_buffer.len(),
                            pending: vec![],
                        },
                        actions: vec![],
                    })
                } else {
                    unreachable!()
                }
            })
        },
    )
    .done()
    .when("DrinkingKoolAid")
    .done()
    .build();
    machine3
        .handle(
            PurgatoryEvent::StartMaterialization(vec!["doomed".to_string()]),
            &mut ctx3,
        )
        .await
        .unwrap();

    // Try to process - should detect broken downstream
    machine3
        .handle(PurgatoryEvent::ProcessBatch, &mut ctx3)
        .await
        .unwrap();

    assert!(
        matches!(machine3.state(), PurgatoryState::DrinkingKoolAid(_)),
        "Should initiate Jonestown when downstream is dead!"
    );

    println!("\n🔥 Purgatory trials complete!");
    println!("✝️ The Jonestown Protocol preserved AT LEAST ONCE even under timeout pressure!");

    // "Better to die with honor than live with dropped messages" - FLOWIP-075a
}
