//! Test 2: The Async Coordination Nightmare 😈
//!
//! The Beast with Seven Heads:
//! - 1 Pipeline FSM (the beast itself)
//! - 5 Stage FSMs (five of the heads)
//! - 1 Barrier (the sixth head - synchronization)
//! - 1 Broadcast channel (the seventh head - shutdown signal)
//!
//! The Unholy Ritual:
//! 1. Stages materialize in parallel (demons awakening)
//! 2. Pipeline waits for all stages to be ready (gathering the legion)
//! 3. Barrier synchronization for coordinated start (the unholy mass begins)
//! 4. Events flow through the stages (possession spreading)
//! 5. Shutdown broadcast triggers cascading drain (exorcism)
//! 6. Pipeline verifies all stages reached Drained state (demons banished)
//!
//! Divine Patterns Tested:
//! - The holy trinity of channels: mpsc (commands), broadcast (shutdown), barrier (sync)
//! - State machines within state machines (fractals of divine design)
//! - Async message passing without data races (speaking in tongues, but orderly)
//! - Perfect ordering despite concurrent chaos (God's timeline prevails)
//!
//! What it tests:
//! - Pipeline coordinating multiple stage FSMs
//! - Barrier synchronization for startup
//! - Complex shutdown propagation
//! - EOF signal handling with multiple upstreams
//!
//! Why it matters:
//! - This is exactly how LayeredPipelineLifecycle coordinates stages
//! - Tests our ability to handle complex inter-FSM communication
//! - Proves the FSM can maintain order even when demons attack from multiple angles

#![allow(deprecated)]

use async_trait::async_trait;
use obzenflow_fsm::internal::FsmBuilder;
use obzenflow_fsm::{EventVariant, FsmAction, FsmContext, StateVariant, Transition};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::select;
use tokio::sync::{broadcast, mpsc, Barrier, RwLock};
use tokio::time::sleep;

#[tokio::test]
async fn test_2_async_coordination_nightmare() {
    // Pipeline States
    #[derive(Clone, Debug, PartialEq)]
    enum PipelineState {
        Initializing,
        WaitingForStages,
        Running,
        Draining,
        Shutdown,
    }

    impl StateVariant for PipelineState {
        fn variant_name(&self) -> &str {
            match self {
                PipelineState::Initializing => "Initializing",
                PipelineState::WaitingForStages => "WaitingForStages",
                PipelineState::Running => "Running",
                PipelineState::Draining => "Draining",
                PipelineState::Shutdown => "Shutdown",
            }
        }
    }

    // Stage States
    #[derive(Clone, Debug, PartialEq)]
    enum StageState {
        Uninitialized,
        Materializing,
        Materialized,
        Running,
        Draining,
        Drained,
    }

    impl StateVariant for StageState {
        fn variant_name(&self) -> &str {
            match self {
                StageState::Uninitialized => "Uninitialized",
                StageState::Materializing => "Materializing",
                StageState::Materialized => "Materialized",
                StageState::Running => "Running",
                StageState::Draining => "Draining",
                StageState::Drained => "Drained",
            }
        }
    }

    // Pipeline Events
    #[derive(Clone, Debug)]
    enum PipelineEvent {
        Start,
        StageReady { stage_id: usize },
        AllStagesReady,
        BeginShutdown,
        StageDrained { stage_id: usize },
        AllStagesDrained,
    }

    impl EventVariant for PipelineEvent {
        fn variant_name(&self) -> &str {
            match self {
                PipelineEvent::Start => "Start",
                PipelineEvent::StageReady { .. } => "StageReady",
                PipelineEvent::AllStagesReady => "AllStagesReady",
                PipelineEvent::BeginShutdown => "BeginShutdown",
                PipelineEvent::StageDrained { .. } => "StageDrained",
                PipelineEvent::AllStagesDrained => "AllStagesDrained",
            }
        }
    }

    // Stage Events
    #[derive(Clone, Debug)]
    enum StageEvent {
        Initialize,
        MaterializationComplete,
        StartProcessing,
        ProcessEvent { data: String },
        BeginDrain,
        DrainComplete,
    }

    impl EventVariant for StageEvent {
        fn variant_name(&self) -> &str {
            match self {
                StageEvent::Initialize => "Initialize",
                StageEvent::MaterializationComplete => "MaterializationComplete",
                StageEvent::StartProcessing => "StartProcessing",
                StageEvent::ProcessEvent { .. } => "ProcessEvent",
                StageEvent::BeginDrain => "BeginDrain",
                StageEvent::DrainComplete => "DrainComplete",
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum CoordAction {
        NotifyPipeline,
        BroadcastShutdown,
        WaitAtBarrier,
    }

    // Shared coordination context
    #[derive(Clone)]
    struct CoordinationContext {
        // Startup synchronization
        startup_barrier: Arc<Barrier>,
        // Ready stages tracking
        ready_stages: Arc<RwLock<std::collections::HashSet<usize>>>,
        // Drained stages tracking
        drained_stages: Arc<RwLock<std::collections::HashSet<usize>>>,
        // Shutdown broadcast
        shutdown_tx: broadcast::Sender<()>,
        // Pipeline command channel
        pipeline_tx: mpsc::Sender<PipelineEvent>,
        // Stage command channels
        stage_txs: Arc<RwLock<HashMap<usize, mpsc::Sender<StageEvent>>>>,
        // Event log for verification
        event_log: Arc<RwLock<Vec<String>>>,
    }

    impl FsmContext for CoordinationContext {
        fn describe(&self) -> String {
            "CoordinationContext for pipeline and stages".to_string()
        }
    }

    #[async_trait]
    impl FsmAction for CoordAction {
        type Context = CoordinationContext;

        async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
            match self {
                CoordAction::NotifyPipeline => {
                    ctx.event_log
                        .write()
                        .await
                        .push("NotifyPipeline executed".to_string());
                    Ok(())
                }
                CoordAction::BroadcastShutdown => {
                    let _ = ctx.shutdown_tx.send(());
                    ctx.event_log
                        .write()
                        .await
                        .push("BroadcastShutdown executed".to_string());
                    Ok(())
                }
                CoordAction::WaitAtBarrier => {
                    ctx.startup_barrier.wait().await;
                    ctx.event_log
                        .write()
                        .await
                        .push("WaitAtBarrier executed".to_string());
                    Ok(())
                }
            }
        }
    }

    let (shutdown_tx, _) = broadcast::channel(10);
    let (pipeline_tx, mut pipeline_rx) = mpsc::channel(100);

    let ctx = CoordinationContext {
        startup_barrier: Arc::new(Barrier::new(6)), // 1 pipeline + 5 stages
        ready_stages: Arc::new(RwLock::new(std::collections::HashSet::new())),
        drained_stages: Arc::new(RwLock::new(std::collections::HashSet::new())),
        shutdown_tx,
        pipeline_tx,
        stage_txs: Arc::new(RwLock::new(HashMap::new())),
        event_log: Arc::new(RwLock::new(Vec::new())),
    };

    // Create Pipeline FSM
    let pipeline_ctx = CoordinationContext {
        startup_barrier: ctx.startup_barrier.clone(),
        ready_stages: ctx.ready_stages.clone(),
        drained_stages: ctx.drained_stages.clone(),
        shutdown_tx: ctx.shutdown_tx.clone(),
        pipeline_tx: ctx.pipeline_tx.clone(),
        stage_txs: ctx.stage_txs.clone(),
        event_log: ctx.event_log.clone(),
    };
    let pipeline_handle = tokio::spawn(async move {
        let mut ctx = pipeline_ctx;

        let mut fsm =
            FsmBuilder::<PipelineState, PipelineEvent, CoordinationContext, CoordAction>::new(
                PipelineState::Initializing,
            )
            .when("Initializing")
            .on(
                "Start",
                move |_state, _event, ctx: &mut CoordinationContext| {
                    Box::pin(async move {
                        ctx.event_log
                            .write()
                            .await
                            .push("Pipeline: Starting".to_string());
                        Ok(Transition {
                            next_state: PipelineState::WaitingForStages,
                            actions: vec![],
                        })
                    })
                },
            )
            .done()
            .when("WaitingForStages")
            .on(
                "StageReady",
                move |_state, event, ctx: &mut CoordinationContext| {
                    let event = event.clone();
                    Box::pin(async move {
                        if let PipelineEvent::StageReady { stage_id } = event {
                            ctx.ready_stages.write().await.insert(stage_id);
                            ctx.event_log
                                .write()
                                .await
                                .push(format!("Pipeline: Stage {stage_id} ready"));

                            if ctx.ready_stages.read().await.len() == 5 {
                                // All stages ready, notify via event
                                let _ = ctx.pipeline_tx.send(PipelineEvent::AllStagesReady).await;
                            }
                        }
                        Ok(Transition {
                            next_state: PipelineState::WaitingForStages,
                            actions: vec![],
                        })
                    })
                },
            )
            .on(
                "AllStagesReady",
                move |_state, _event, ctx: &mut CoordinationContext| {
                    Box::pin(async move {
                        ctx.event_log
                            .write()
                            .await
                            .push("Pipeline: All stages ready, starting".to_string());

                        // Signal all stages to start
                        let stage_txs = ctx.stage_txs.read().await;
                        for (_, tx) in stage_txs.iter() {
                            let _ = tx.send(StageEvent::StartProcessing).await;
                        }

                        Ok(Transition {
                            next_state: PipelineState::Running,
                            actions: vec![CoordAction::WaitAtBarrier],
                        })
                    })
                },
            )
            .done()
            .when("Running")
            .on(
                "BeginShutdown",
                move |_state, _event, ctx: &mut CoordinationContext| {
                    Box::pin(async move {
                        ctx.event_log
                            .write()
                            .await
                            .push("Pipeline: Beginning shutdown".to_string());

                        // Broadcast shutdown to all stages
                        let _ = ctx.shutdown_tx.send(());

                        Ok(Transition {
                            next_state: PipelineState::Draining,
                            actions: vec![CoordAction::BroadcastShutdown],
                        })
                    })
                },
            )
            .done()
            .when("Draining")
            .on(
                "StageDrained",
                move |_state, event, ctx: &mut CoordinationContext| {
                    let event = event.clone();
                    Box::pin(async move {
                        if let PipelineEvent::StageDrained { stage_id } = event {
                            ctx.drained_stages.write().await.insert(stage_id);
                            ctx.event_log
                                .write()
                                .await
                                .push(format!("Pipeline: Stage {stage_id} drained"));

                            if ctx.drained_stages.read().await.len() == 5 {
                                let _ = ctx.pipeline_tx.send(PipelineEvent::AllStagesDrained).await;
                            }
                        }
                        Ok(Transition {
                            next_state: PipelineState::Draining,
                            actions: vec![],
                        })
                    })
                },
            )
            .on(
                "AllStagesDrained",
                move |_state, _event, ctx: &mut CoordinationContext| {
                    Box::pin(async move {
                        ctx.event_log
                            .write()
                            .await
                            .push("Pipeline: All stages drained".to_string());
                        Ok(Transition {
                            next_state: PipelineState::Shutdown,
                            actions: vec![],
                        })
                    })
                },
            )
            .done()
            .build();

        // Pipeline event loop
        while let Some(event) = pipeline_rx.recv().await {
            let actions = fsm.handle(event, &mut ctx).await.unwrap();

            // Handle barrier synchronization
            for action in actions {
                if action == CoordAction::WaitAtBarrier {
                    ctx.startup_barrier.wait().await;
                }
            }

            if matches!(fsm.state(), PipelineState::Shutdown) {
                break;
            }
        }

        fsm
    });

    // Create 5 Stage FSMs
    let mut stage_handles = vec![];

    for stage_id in 0..5 {
        let (stage_tx, mut stage_rx) = mpsc::channel(100);
        ctx.stage_txs.write().await.insert(stage_id, stage_tx);

        let stage_ctx = CoordinationContext {
            startup_barrier: ctx.startup_barrier.clone(),
            ready_stages: ctx.ready_stages.clone(),
            drained_stages: ctx.drained_stages.clone(),
            shutdown_tx: ctx.shutdown_tx.clone(),
            pipeline_tx: ctx.pipeline_tx.clone(),
            stage_txs: ctx.stage_txs.clone(),
            event_log: ctx.event_log.clone(),
        };
        let mut shutdown_rx = ctx.shutdown_tx.subscribe();

        let handle = tokio::spawn(async move {
            let mut ctx = stage_ctx;

            let mut fsm =
                FsmBuilder::<StageState, StageEvent, CoordinationContext, CoordAction>::new(
                    StageState::Uninitialized,
                )
                .when("Uninitialized")
                .on(
                    "Initialize",
                    move |_state, _event, ctx: &mut CoordinationContext| {
                        Box::pin(async move {
                            ctx.event_log
                                .write()
                                .await
                                .push(format!("Stage {stage_id}: Initializing"));

                            // Simulate materialization delay
                            sleep(Duration::from_millis(10 + stage_id as u64 * 5)).await;

                            // Schedule auto-completion of materialization
                            let ctx_clone = CoordinationContext {
                                startup_barrier: ctx.startup_barrier.clone(),
                                ready_stages: ctx.ready_stages.clone(),
                                drained_stages: ctx.drained_stages.clone(),
                                shutdown_tx: ctx.shutdown_tx.clone(),
                                pipeline_tx: ctx.pipeline_tx.clone(),
                                stage_txs: ctx.stage_txs.clone(),
                                event_log: ctx.event_log.clone(),
                            };
                            tokio::spawn(async move {
                                sleep(Duration::from_millis(20)).await;
                                let stage_txs = ctx_clone.stage_txs.read().await;
                                if let Some(tx) = stage_txs.get(&stage_id) {
                                    let _ = tx.send(StageEvent::MaterializationComplete).await;
                                }
                            });

                            Ok(Transition {
                                next_state: StageState::Materializing,
                                actions: vec![],
                            })
                        })
                    },
                )
                .done()
                .when("Materializing")
                .on(
                    "MaterializationComplete",
                    move |_state, _event, ctx: &mut CoordinationContext| {
                        Box::pin(async move {
                            ctx.event_log
                                .write()
                                .await
                                .push(format!("Stage {stage_id}: Materialized"));

                            // Notify pipeline we're ready
                            let _ = ctx
                                .pipeline_tx
                                .send(PipelineEvent::StageReady { stage_id })
                                .await;

                            Ok(Transition {
                                next_state: StageState::Materialized,
                                actions: vec![CoordAction::NotifyPipeline],
                            })
                        })
                    },
                )
                .done()
                .when("Materialized")
                .on(
                    "StartProcessing",
                    move |_state, _event, ctx: &mut CoordinationContext| {
                        Box::pin(async move {
                            ctx.event_log
                                .write()
                                .await
                                .push(format!("Stage {stage_id}: Starting processing"));

                            // Wait at barrier for coordinated start
                            ctx.startup_barrier.wait().await;

                            Ok(Transition {
                                next_state: StageState::Running,
                                actions: vec![],
                            })
                        })
                    },
                )
                .done()
                .when("Running")
                .on(
                    "ProcessEvent",
                    move |_state, event, ctx: &mut CoordinationContext| {
                        let event = event.clone();
                        Box::pin(async move {
                            if let StageEvent::ProcessEvent { data } = event {
                                ctx.event_log
                                    .write()
                                    .await
                                    .push(format!("Stage {stage_id}: Processing {data}"));
                            }
                            Ok(Transition {
                                next_state: StageState::Running,
                                actions: vec![],
                            })
                        })
                    },
                )
                .on(
                    "BeginDrain",
                    move |_state, _event, ctx: &mut CoordinationContext| {
                        Box::pin(async move {
                            ctx.event_log
                                .write()
                                .await
                                .push(format!("Stage {stage_id}: Beginning drain"));

                            // Simulate drain work
                            sleep(Duration::from_millis(50 - stage_id as u64 * 5)).await;

                            // Schedule auto-completion of drain
                            let ctx_clone = CoordinationContext {
                                startup_barrier: ctx.startup_barrier.clone(),
                                ready_stages: ctx.ready_stages.clone(),
                                drained_stages: ctx.drained_stages.clone(),
                                shutdown_tx: ctx.shutdown_tx.clone(),
                                pipeline_tx: ctx.pipeline_tx.clone(),
                                stage_txs: ctx.stage_txs.clone(),
                                event_log: ctx.event_log.clone(),
                            };
                            tokio::spawn(async move {
                                let stage_txs = ctx_clone.stage_txs.read().await;
                                if let Some(tx) = stage_txs.get(&stage_id) {
                                    let _ = tx.send(StageEvent::DrainComplete).await;
                                }
                            });

                            Ok(Transition {
                                next_state: StageState::Draining,
                                actions: vec![],
                            })
                        })
                    },
                )
                .done()
                .when("Draining")
                .on(
                    "DrainComplete",
                    move |_state, _event, ctx: &mut CoordinationContext| {
                        Box::pin(async move {
                            ctx.event_log
                                .write()
                                .await
                                .push(format!("Stage {stage_id}: Drained"));

                            // Notify pipeline we're drained
                            let _ = ctx
                                .pipeline_tx
                                .send(PipelineEvent::StageDrained { stage_id })
                                .await;

                            Ok(Transition {
                                next_state: StageState::Drained,
                                actions: vec![],
                            })
                        })
                    },
                )
                .done()
                .build();

            // Stage event loop
            loop {
                select! {
                    Some(event) = stage_rx.recv() => {
                        fsm.handle(event, &mut ctx).await.unwrap();

                        if matches!(fsm.state(), StageState::Drained) {
                            break;
                        }
                    }
                    Ok(_) = shutdown_rx.recv() => {
                        // Shutdown signal received
                        fsm.handle(StageEvent::BeginDrain, &mut ctx).await.unwrap();
                    }
                }
            }

            fsm
        });

        stage_handles.push(handle);
    }

    // Initialize all stages
    for i in 0..5 {
        let stage_txs = ctx.stage_txs.read().await;
        if let Some(tx) = stage_txs.get(&i) {
            tx.send(StageEvent::Initialize).await.unwrap();
        }
    }

    // Start pipeline
    ctx.pipeline_tx.send(PipelineEvent::Start).await.unwrap();

    // Wait for system to stabilize
    sleep(Duration::from_millis(200)).await;

    // Send some events to stages
    for i in 0..5 {
        let stage_txs = ctx.stage_txs.read().await;
        if let Some(tx) = stage_txs.get(&i) {
            for j in 0..3 {
                tx.send(StageEvent::ProcessEvent {
                    data: format!("event_{j}"),
                })
                .await
                .unwrap();
            }
        }
    }

    // Begin shutdown
    ctx.pipeline_tx
        .send(PipelineEvent::BeginShutdown)
        .await
        .unwrap();

    // Wait for all components to complete
    let pipeline_fsm = pipeline_handle.await.unwrap();
    let stage_fsms: Vec<_> = futures::future::join_all(stage_handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Verify final states
    assert!(matches!(pipeline_fsm.state(), PipelineState::Shutdown));
    for fsm in &stage_fsms {
        assert!(matches!(fsm.state(), StageState::Drained));
    }

    // Verify event ordering
    let log = ctx.event_log.read().await;

    // All stages should initialize before any materialize
    let first_materialized = log.iter().position(|s| s.contains("Materialized")).unwrap();
    let last_initialized = log
        .iter()
        .rposition(|s| s.contains("Initializing"))
        .unwrap();
    assert!(last_initialized < first_materialized);

    // All stages should be ready before pipeline starts them
    let pipeline_start = log
        .iter()
        .position(|s| s.contains("All stages ready"))
        .unwrap();
    let first_processing = log
        .iter()
        .position(|s| s.contains("Starting processing"))
        .unwrap();
    assert!(pipeline_start < first_processing);

    // Shutdown should happen in order
    let shutdown_start = log
        .iter()
        .position(|s| s.contains("Beginning shutdown"))
        .unwrap();
    let first_drain = log
        .iter()
        .position(|s| s.contains("Beginning drain"))
        .unwrap();
    assert!(shutdown_start < first_drain);
}
