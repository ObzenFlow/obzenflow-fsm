// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Test 4: The Mark of the Beast - AT LEAST ONCE vs Mathematical Properties 😈
//!
//! Satan's Mathematical Trickery:
//! - 666 duplicate events (AT LEAST ONCE delivery guarantee)
//! - Non-idempotent operations (balance += amount)
//! - Non-commutative operations (append to list)
//! - Non-associative operations ((a-b)-c ≠ a-(b-c))
//!
//! The Unholy Trinity of Distributed Systems:
//! - Idempotent × Associative × Commutative = Correct
//! - But AT LEAST ONCE breaks this trinity!
//!
//! What it tests:
//! - Duplicate event handling (same event delivered multiple times)
//! - Order-dependent operations (A then B ≠ B then A)
//! - State accumulation under duplicates
//! - Mathematical properties vs real-world guarantees
//!
//! Why it matters:
//! - ObzenFlow guarantees AT LEAST ONCE delivery
//! - FSMs must handle duplicate events correctly
//! - Some operations can't be made idempotent
//! - Tests if our FSM design exposes or hides these issues

#![allow(dead_code)]
#![allow(deprecated)]

use async_trait::async_trait;
use obzenflow_fsm::internal::FsmBuilder;
use obzenflow_fsm::{EventVariant, FsmAction, FsmContext, StateVariant, Transition};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_4_mark_of_the_beast_mathematical_properties() {
    #[derive(Clone, Debug, PartialEq)]
    enum BeastState {
        Counting {
            balance: i64,
            operations: Vec<String>,
            operation_ids: std::collections::HashSet<String>,
        },
        Overflowed,
        Corrupted(String),
    }

    impl StateVariant for BeastState {
        fn variant_name(&self) -> &str {
            match self {
                BeastState::Counting { .. } => "Counting",
                BeastState::Overflowed => "Overflowed",
                BeastState::Corrupted(_) => "Corrupted",
            }
        }
    }

    #[derive(Clone, Debug)]
    enum BeastEvent {
        // Non-idempotent: balance += amount
        Credit { id: String, amount: i64 },
        // Non-idempotent: balance -= amount
        Debit { id: String, amount: i64 },
        // Non-commutative: order matters
        Append { id: String, value: String },
        // Non-associative: (a-b)-c ≠ a-(b-c)
        Subtract { id: String, value: i64 },
        // The mark
        MarkOfBeast,
    }

    impl EventVariant for BeastEvent {
        fn variant_name(&self) -> &str {
            match self {
                BeastEvent::Credit { .. } => "Credit",
                BeastEvent::Debit { .. } => "Debit",
                BeastEvent::Append { .. } => "Append",
                BeastEvent::Subtract { .. } => "Subtract",
                BeastEvent::MarkOfBeast => "MarkOfBeast",
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum BeastAction {
        RecordOperation(String),
        AlertDuplicate(String),
        ApocalypseNow,
    }

    #[derive(Clone)]
    struct BeastContext {
        // Track all events seen (for duplicate detection)
        seen_events: Arc<RwLock<std::collections::HashSet<String>>>,
        // Count duplicates
        duplicate_count: Arc<AtomicUsize>,
        // Track operation order for non-commutative ops
        operation_log: Arc<RwLock<Vec<(String, String)>>>,
    }

    impl FsmContext for BeastContext {
        fn describe(&self) -> String {
            format!(
                "BeastContext with {} duplicates",
                self.duplicate_count.load(Ordering::Relaxed)
            )
        }
    }

    #[async_trait]
    impl FsmAction for BeastAction {
        type Context = BeastContext;

        async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
            match self {
                BeastAction::RecordOperation(op) => {
                    ctx.operation_log
                        .write()
                        .await
                        .push((op.clone(), "executed".to_string()));
                    Ok(())
                }
                BeastAction::AlertDuplicate(_event_id) => {
                    ctx.duplicate_count.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
                BeastAction::ApocalypseNow => {
                    // The end times have come
                    Ok(())
                }
            }
        }
    }

    let mut ctx = BeastContext {
        seen_events: Arc::new(RwLock::new(std::collections::HashSet::new())),
        duplicate_count: Arc::new(AtomicUsize::new(0)),
        operation_log: Arc::new(RwLock::new(Vec::new())),
    };

    // === BUILD THE BEAST'S FSM ===
    let fsm = FsmBuilder::new(BeastState::Counting {
        balance: 0,
        operations: vec![],
        operation_ids: std::collections::HashSet::new(),
    })
    .when("Counting")
    .on(
        "Credit",
        |state, event: &BeastEvent, ctx: &mut BeastContext| {
            let state = state.clone();
            let event = event.clone();
            Box::pin(async move {
                if let (
                    BeastState::Counting {
                        balance,
                        mut operations,
                        mut operation_ids,
                    },
                    BeastEvent::Credit { id, amount },
                ) = (state, event)
                {
                    // === DUPLICATE DETECTION ===
                    let is_duplicate = {
                        let mut seen = ctx.seen_events.write().await;
                        !seen.insert(id.clone())
                    };

                    if is_duplicate {
                        ctx.duplicate_count.fetch_add(1, Ordering::Relaxed);

                        // CHOICE 1: Make it idempotent (check operation_ids)
                        if operation_ids.contains(&id) {
                            // Already processed, ignore
                            return Ok(Transition {
                                next_state: BeastState::Counting {
                                    balance,
                                    operations,
                                    operation_ids,
                                },
                                actions: vec![BeastAction::AlertDuplicate(id)],
                            });
                        }
                    }

                    // CHOICE 2: Process anyway (AT LEAST ONCE breaks idempotency!)
                    operation_ids.insert(id.clone());
                    operations.push(format!("Credit {id} by {amount}"));
                    ctx.operation_log
                        .write()
                        .await
                        .push((id.clone(), format!("Credit:{amount}")));

                    // Non-idempotent operation!
                    let new_balance = balance.saturating_add(amount);

                    Ok(Transition {
                        next_state: BeastState::Counting {
                            balance: new_balance,
                            operations,
                            operation_ids,
                        },
                        actions: vec![BeastAction::RecordOperation(format!("Credit:{amount}"))],
                    })
                } else {
                    unreachable!()
                }
            })
        },
    )
    .on(
        "Debit",
        |state, event: &BeastEvent, ctx: &mut BeastContext| {
            let state = state.clone();
            let event = event.clone();
            Box::pin(async move {
                if let (
                    BeastState::Counting {
                        balance,
                        mut operations,
                        operation_ids,
                    },
                    BeastEvent::Debit { id, amount },
                ) = (state, event)
                {
                    // Debit is also non-idempotent!
                    operations.push(format!("Debit {id} by {amount}"));
                    ctx.operation_log
                        .write()
                        .await
                        .push((id, format!("Debit:{amount}")));

                    let new_balance = balance.saturating_sub(amount);

                    Ok(Transition {
                        next_state: BeastState::Counting {
                            balance: new_balance,
                            operations,
                            operation_ids,
                        },
                        actions: vec![BeastAction::RecordOperation(format!("Debit:{amount}"))],
                    })
                } else {
                    unreachable!()
                }
            })
        },
    )
    .on(
        "Append",
        |state, event: &BeastEvent, ctx: &mut BeastContext| {
            let state = state.clone();
            let event = event.clone();
            Box::pin(async move {
                if let (
                    BeastState::Counting {
                        balance,
                        mut operations,
                        operation_ids,
                    },
                    BeastEvent::Append { id, value },
                ) = (state, event)
                {
                    // Non-commutative: order matters!
                    operations.push(value.clone());
                    ctx.operation_log
                        .write()
                        .await
                        .push((id, format!("Append:{value}")));

                    Ok(Transition {
                        next_state: BeastState::Counting {
                            balance,
                            operations,
                            operation_ids,
                        },
                        actions: vec![BeastAction::RecordOperation(format!("Append:{value}"))],
                    })
                } else {
                    unreachable!()
                }
            })
        },
    )
    .on(
        "MarkOfBeast",
        |state, _event: &BeastEvent, ctx: &mut BeastContext| {
            let state = state.clone();
            Box::pin(async move {
                if let BeastState::Counting {
                    balance,
                    operations,
                    operation_ids,
                } = state
                {
                    let duplicates = ctx.duplicate_count.load(Ordering::Relaxed);
                    if balance == 666 || duplicates == 666 || operations.len() == 666 {
                        Ok(Transition {
                            next_state: BeastState::Corrupted(
                                "The number of the beast!".to_string(),
                            ),
                            actions: vec![BeastAction::ApocalypseNow],
                        })
                    } else {
                        Ok(Transition {
                            next_state: BeastState::Counting {
                                balance,
                                operations,
                                operation_ids,
                            },
                            actions: vec![],
                        })
                    }
                } else {
                    unreachable!()
                }
            })
        },
    )
    .done()
    .build();

    let mut machine = fsm;

    // === THE BEAST'S TRIALS ===

    // Trial 1: Duplicate Credits (testing idempotency)
    for i in 0..10 {
        let event = BeastEvent::Credit {
            id: format!("credit_{i}"),
            amount: 100,
        };

        // Send the same event 3 times (AT LEAST ONCE!)
        for _ in 0..3 {
            machine.handle(event.clone(), &mut ctx).await.unwrap();
        }
    }

    if let BeastState::Counting { balance, .. } = machine.state() {
        assert_eq!(
            balance, &1000,
            "Idempotency failed! Duplicate credits were processed"
        );
    }

    // Trial 2: Non-commutative operations
    let append_events = vec![
        BeastEvent::Append {
            id: "1".to_string(),
            value: "First".to_string(),
        },
        BeastEvent::Append {
            id: "2".to_string(),
            value: "Second".to_string(),
        },
        BeastEvent::Append {
            id: "3".to_string(),
            value: "Third".to_string(),
        },
    ];

    // Process in order
    for event in &append_events {
        machine.handle(event.clone(), &mut ctx).await.unwrap();
    }

    let ordered_ops = if let BeastState::Counting { operations, .. } = machine.state() {
        operations.clone()
    } else {
        vec![]
    };

    // Create another FSM and process in reverse order
    let mut machine2 = FsmBuilder::<BeastState, BeastEvent, BeastContext, BeastAction>::new(
        BeastState::Counting {
            balance: 0,
            operations: vec![],
            operation_ids: std::collections::HashSet::new(),
        },
    )
    .when("Counting")
    .on(
        "Append",
        |state, event: &BeastEvent, _ctx: &mut BeastContext| {
            let state = state.clone();
            let event = event.clone();
            Box::pin(async move {
                if let (
                    BeastState::Counting {
                        balance,
                        mut operations,
                        operation_ids,
                    },
                    BeastEvent::Append { value, .. },
                ) = (state, event)
                {
                    operations.push(value);
                    Ok(Transition {
                        next_state: BeastState::Counting {
                            balance,
                            operations,
                            operation_ids,
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
    .build();

    // Process in reverse order
    for event in append_events.iter().rev() {
        machine2.handle(event.clone(), &mut ctx).await.unwrap();
    }

    let reversed_ops = if let BeastState::Counting { operations, .. } = machine2.state() {
        operations.clone()
    } else {
        vec![]
    };

    assert_ne!(
        ordered_ops, reversed_ops,
        "Operations are commutative when they shouldn't be!"
    );

    // Trial 3: The Number of the Beast

    // Send exactly 566 more credits to reach 666 from 1000
    for i in 0..566 {
        let event = BeastEvent::Debit {
            id: format!("debit_{i}"),
            amount: 1,
        };
        // Only handle if still in Counting state (might transition to Corrupted)
        if matches!(machine.state(), BeastState::Counting { .. }) {
            machine.handle(event, &mut ctx).await.unwrap();
        }

        if i == 333 {
            // Check for the mark mid-way
            if matches!(machine.state(), BeastState::Counting { .. }) {
                machine
                    .handle(BeastEvent::MarkOfBeast, &mut ctx)
                    .await
                    .unwrap();
            }
        }
    }

    // Check if we're still in Counting state or already Corrupted
    let already_corrupted = matches!(machine.state(), BeastState::Corrupted(_));

    if !already_corrupted {
        // Final balance should be 1000 - 566 = 434
        // Now credit to reach exactly 666
        machine
            .handle(
                BeastEvent::Credit {
                    id: "beast".to_string(),
                    amount: 232, // 434 + 232 = 666
                },
                &mut ctx,
            )
            .await
            .unwrap();

        // Check for the mark
        let _actions = machine
            .handle(BeastEvent::MarkOfBeast, &mut ctx)
            .await
            .unwrap();
    }

    assert!(
        matches!(machine.state(), BeastState::Corrupted(_)),
        "expected Corrupted(_) after reaching 666, got {:?}",
        machine.state()
    );

    let total_duplicates = ctx.duplicate_count.load(Ordering::Relaxed);
    assert!(total_duplicates > 0, "expected to detect duplicate events");

    // "Here is wisdom. Let him that hath understanding count the number of the beast" - Revelation 13:18
}
