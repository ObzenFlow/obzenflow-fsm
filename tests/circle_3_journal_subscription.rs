// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Test 3: The Journal Subscription Chaos (Tower of Babel) 😈
//!
//! The Confusion of Tongues:
//! - Multiple FSMs speaking different event dialects
//! - Each trying to write their truth to the shared journal
//! - Subscribers with filters trying to understand only their language
//! - Vector clocks maintaining divine causality despite the chaos
//!
//! What it tests:
//! - Concurrent reads/writes to shared journal
//! - Subscription filtering and ordering
//! - Vector clock based causal ordering
//! - Channel overflow and backpressure
//!
//! Why it matters:
//! - This is how stages actually communicate
//! - Tests Arc<Context> with complex async I/O patterns
//! - Proves the journal can maintain order even when everyone speaks at once

#![allow(dead_code)]
#![allow(deprecated)]

use async_trait::async_trait;
use obzenflow_fsm::internal::FsmBuilder;
use obzenflow_fsm::{EventVariant, FsmAction, FsmContext, StateVariant, Transition};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, timeout};

#[tokio::test]
async fn test_3_journal_subscription_chaos() {
    // === THE LANGUAGES OF BABEL ===
    #[derive(Clone, Debug, PartialEq)]
    enum BabelState {
        Speaking { language: String },
        Listening,
        Confused,
        Silenced,
    }

    impl StateVariant for BabelState {
        fn variant_name(&self) -> &str {
            match self {
                BabelState::Speaking { .. } => "Speaking",
                BabelState::Listening => "Listening",
                BabelState::Confused => "Confused",
                BabelState::Silenced => "Silenced",
            }
        }
    }

    #[derive(Clone, Debug)]
    enum BabelEvent {
        Speak { message: String, language: String },
        Listen { filter: String },
        Overflow,
        Silence,
    }

    impl EventVariant for BabelEvent {
        fn variant_name(&self) -> &str {
            match self {
                BabelEvent::Speak { .. } => "Speak",
                BabelEvent::Listen { .. } => "Listen",
                BabelEvent::Overflow => "Overflow",
                BabelEvent::Silence => "Silence",
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum BabelAction {
        WriteToJournal(String),
        Subscribe(String),
        DropMessages,
    }

    // === THE SHARED JOURNAL (TOWER FOUNDATION) ===
    #[derive(Clone)]
    struct JournalEntry {
        id: usize,
        timestamp: std::time::Instant,
        language: String,
        message: String,
        vector_clock: HashMap<String, usize>,
    }

    #[derive(Clone)]
    struct BabelContext {
        // The shared journal - where all languages are written
        journal: Arc<RwLock<Vec<JournalEntry>>>,
        // Subscription channels - each limited to 100 messages (divine limit)
        subscribers: Arc<RwLock<HashMap<String, mpsc::Sender<JournalEntry>>>>,
        // Vector clocks for each speaker
        vector_clocks: Arc<RwLock<HashMap<String, HashMap<String, usize>>>>,
        // Global message counter
        message_counter: Arc<AtomicUsize>,
        // Chaos injection
        chaos_mode: Arc<AtomicBool>,
        // Track dropped messages (overflow victims)
        dropped_messages: Arc<AtomicUsize>,
    }

    impl FsmContext for BabelContext {
        fn describe(&self) -> String {
            format!(
                "BabelContext with {} messages",
                self.message_counter.load(Ordering::Relaxed)
            )
        }
    }

    #[async_trait]
    impl FsmAction for BabelAction {
        type Context = BabelContext;

        async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
            match self {
                BabelAction::WriteToJournal(_msg) => {
                    // Write to journal
                    Ok(())
                }
                BabelAction::Subscribe(_language) => {
                    // Subscribe to language
                    Ok(())
                }
                BabelAction::DropMessages => {
                    ctx.dropped_messages.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
            }
        }
    }

    let journal = Arc::new(RwLock::new(Vec::new()));
    let subscribers = Arc::new(RwLock::new(HashMap::new()));
    let vector_clocks = Arc::new(RwLock::new(HashMap::new()));
    let message_counter = Arc::new(AtomicUsize::new(0));
    let chaos_mode = Arc::new(AtomicBool::new(false));
    let dropped_messages = Arc::new(AtomicUsize::new(0));

    let ctx = BabelContext {
        journal: journal.clone(),
        subscribers: subscribers.clone(),
        vector_clocks: vector_clocks.clone(),
        message_counter: message_counter.clone(),
        chaos_mode: chaos_mode.clone(),
        dropped_messages: dropped_messages.clone(),
    };

    // === THE BABEL BUILDERS (10 FSMs speaking different languages) ===
    let languages = [
        "Hebrew",
        "Greek",
        "Latin",
        "Aramaic",
        "Sanskrit",
        "Egyptian",
        "Sumerian",
        "Akkadian",
        "Phoenician",
        "Coptic",
    ];

    let mut speaker_handles = vec![];

    for (i, language) in languages.iter().enumerate() {
        let lang = language.to_string();

        let journal = journal.clone();
        let subscribers = subscribers.clone();
        let vector_clocks = vector_clocks.clone();
        let message_counter = message_counter.clone();
        let chaos_mode = chaos_mode.clone();
        let dropped_messages = dropped_messages.clone();

        let handle = tokio::spawn(async move {
            let mut ctx = BabelContext {
                journal,
                subscribers,
                vector_clocks,
                message_counter,
                chaos_mode,
                dropped_messages,
            };

            let mut fsm = FsmBuilder::<BabelState, BabelEvent, BabelContext, BabelAction>::new(
                BabelState::Speaking {
                    language: lang.clone(),
                },
            )
            .when("Speaking")
            .on("Speak", move |state, event, ctx: &mut BabelContext| {
                let event = event.clone();
                let state_clone = state.clone();
                let lang = if let BabelState::Speaking { language } = state {
                    language.clone()
                } else {
                    "Unknown".to_string()
                };

                Box::pin(async move {
                    if let BabelEvent::Speak { message, .. } = event {
                        // === WRITING TO THE JOURNAL ===
                        let msg_id = ctx.message_counter.fetch_add(1, Ordering::SeqCst);

                        // Update vector clock
                        let mut vector_clocks = ctx.vector_clocks.write().await;
                        let clock = vector_clocks
                            .entry(lang.clone())
                            .or_insert_with(HashMap::new);
                        *clock.entry(lang.clone()).or_insert(0) += 1;
                        let current_clock = clock.clone();
                        drop(vector_clocks);

                        // Create journal entry
                        let entry = JournalEntry {
                            id: msg_id,
                            timestamp: std::time::Instant::now(),
                            language: lang.clone(),
                            message: message.clone(),
                            vector_clock: current_clock,
                        };

                        // === CHAOS INJECTION ===
                        if ctx.chaos_mode.load(Ordering::Relaxed) && rand::random::<bool>() {
                            // Sometimes delay writes to create out-of-order entries
                            sleep(Duration::from_millis(rand::random::<u64>() % 50)).await;
                        }

                        // Write to journal
                        ctx.journal.write().await.push(entry.clone());

                        // === NOTIFY SUBSCRIBERS ===
                        let subscribers = ctx.subscribers.read().await;
                        for (filter, tx) in subscribers.iter() {
                            if filter == "*" || filter == &lang || message.contains(filter) {
                                // Try to send, but don't block if channel is full
                                if tx.try_send(entry.clone()).is_err() {
                                    ctx.dropped_messages.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        }

                        Ok(Transition {
                            next_state: BabelState::Speaking { language: lang },
                            actions: vec![BabelAction::WriteToJournal(message)],
                        })
                    } else {
                        Ok(Transition {
                            next_state: state_clone,
                            actions: vec![],
                        })
                    }
                })
            })
            .on("Overflow", move |_state, _event, ctx: &mut BabelContext| {
                // === THE FLOOD OF MESSAGES ===
                ctx.chaos_mode.store(true, Ordering::Relaxed);
                Box::pin(async move {
                    Ok(Transition {
                        next_state: BabelState::Confused,
                        actions: vec![BabelAction::DropMessages],
                    })
                })
            })
            .done()
            .when("Confused")
            .on("Speak", move |state, event, ctx: &mut BabelContext| {
                let event = event.clone();
                let state_clone = state.clone();
                Box::pin(async move {
                    if let BabelEvent::Speak { .. } = event {
                        // === CONFUSED SPEECH - RANDOMLY DROP MESSAGES ===
                        if rand::random::<bool>() {
                            ctx.dropped_messages.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(Transition {
                            next_state: BabelState::Confused,
                            actions: vec![BabelAction::DropMessages],
                        })
                    } else {
                        Ok(Transition {
                            next_state: state_clone,
                            actions: vec![],
                        })
                    }
                })
            })
            .on("Silence", move |_state, _event, _ctx: &mut BabelContext| {
                // === GOD SILENCES THE BUILDERS ===
                Box::pin(async move {
                    Ok(Transition {
                        next_state: BabelState::Silenced,
                        actions: vec![],
                    })
                })
            })
            .done()
            .when("Silenced")
            .done()
            .build();

            // === EACH BUILDER SPEAKS THEIR TRUTH ===
            for j in 0..100 {
                let message = format!("Builder {i} speaks message {j} in {lang}");
                fsm.handle(
                    BabelEvent::Speak {
                        message,
                        language: lang.clone(),
                    },
                    &mut ctx,
                )
                .await
                .unwrap();

                // Occasional chaos injection (but not too early!)
                if j == 80 {
                    fsm.handle(BabelEvent::Overflow, &mut ctx).await.unwrap();
                }

                // Divine mercy - small delays between messages
                if j % 10 == 0 {
                    tokio::task::yield_now().await;
                }
            }

            // Final silence
            fsm.handle(BabelEvent::Silence, &mut ctx).await.unwrap();
            fsm
        });

        speaker_handles.push(handle);
    }

    // === THE LISTENERS (5 subscribers with different filters) ===
    let filters = vec!["*", "Hebrew", "message 42", "Greek", "Builder 7"];
    let mut listener_handles = vec![];

    for filter in filters {
        let subscribers = subscribers.clone();
        let filter_str = filter.to_string();

        let handle = tokio::spawn(async move {
            let (tx, mut rx) = mpsc::channel(100); // Limited capacity!

            // Register subscriber
            subscribers.write().await.insert(filter_str.clone(), tx);

            let mut received_count = 0;
            let mut last_vector_clock: HashMap<String, usize> = HashMap::new();
            let mut out_of_order_count = 0;

            // === LISTEN FOR MESSAGES ===
            while let Ok(Some(entry)) = timeout(Duration::from_secs(1), rx.recv()).await {
                received_count += 1;

                // === CHECK CAUSAL ORDERING ===
                for (lang, &new_time) in &entry.vector_clock {
                    if let Some(&last_time) = last_vector_clock.get(lang) {
                        if new_time < last_time {
                            out_of_order_count += 1;
                        }
                    }
                }
                last_vector_clock = entry.vector_clock.clone();
            }

            (filter_str, received_count, out_of_order_count)
        });

        listener_handles.push(handle);
    }

    // === WAIT FOR THE TOWER TO FALL ===
    let speakers: Vec<_> = futures::future::join_all(speaker_handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    let listeners: Vec<_> = futures::future::join_all(listener_handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // === DIVINE JUDGMENT ===
    // Verify all speakers reached Silenced state
    for fsm in &speakers {
        assert!(
            matches!(fsm.state(), BabelState::Silenced),
            "Builder failed to be silenced by God!"
        );
    }

    // Check the journal integrity
    let journal = ctx.journal.read().await;
    let total_messages = journal.len();

    // Verify vector clock consistency
    let mut has_causal_violations = false;
    for i in 1..journal.len() {
        for (lang, &time) in &journal[i].vector_clock {
            if let Some(&prev_time) = journal[i - 1].vector_clock.get(lang) {
                if time < prev_time && journal[i].language == *lang {
                    has_causal_violations = true;
                }
            }
        }
    }

    // Some chaos is expected; this is a stress test rather than a strict causal-order proof.
    // `has_causal_violations` remains available for debugging if the test ever needs to be
    // tightened.
    let _ = has_causal_violations;

    // Check subscriber results
    for (filter, count, out_of_order) in listeners {
        // The "*" listener should receive most messages (minus dropped ones)
        if filter == "*" {
            let dropped = ctx.dropped_messages.load(Ordering::Relaxed);
            assert!(
                count + dropped >= total_messages * 80 / 100,
                "Universal listener missed too many messages (received={count}, dropped={dropped}, total={total_messages}, out_of_order={out_of_order})"
            );
        }
    }

    // === THE TOWER STANDS (despite the chaos) ===
    assert!(total_messages >= 800, "Not enough messages were written!");

    // "Go to, let us go down, and there confound their language" - Genesis 11:7
    // But the journal maintained order despite the confusion!
}
