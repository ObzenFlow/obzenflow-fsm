// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Test 4: The Mark of the Beast - AT LEAST ONCE vs Mathematical Properties 😈
//!
//! Satan's Mathematical Trickery:
//! - 666 duplicate events (AT LEAST ONCE delivery guarantee)
//! - Non-idempotent operations (balance += amount)
//! - Non-commutative operations (append to list)
//! - Non-associative reductions (naive partial averages without counts)
//!
//! The Unholy Trinity of Distributed Systems:
//! - Idempotent × Associative × Commutative = safe under many retry/reorder/regroup workloads
//! - But AT LEAST ONCE forces those properties to be explicit!
//!
//! What it tests:
//! - Duplicate event handling (same event delivered multiple times)
//! - Order-dependent operations (A then B ≠ B then A)
//! - Correct batching for additive deltas
//! - Regrouped reductions with a genuinely non-associative combine operator
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
async fn circle_4_mark_of_the_beast_mathematical_properties() {
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
        // Additive deltas: correctly batch by summing payloads.
        Subtract { id: String, value: i64 },
        // Non-associative: averaging partial averages without carrying counts.
        NaiveAverage { id: String, value: i64 },
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
                BeastEvent::NaiveAverage { .. } => "NaiveAverage",
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

    let new_context = || BeastContext {
        duplicate_count: Arc::new(AtomicUsize::new(0)),
        operation_log: Arc::new(RwLock::new(Vec::new())),
    };

    fn naive_average(lhs: i64, rhs: i64) -> i64 {
        lhs.saturating_add(rhs) / 2
    }

    let mut ctx = new_context();

    let build_subtract_machine = |initial_balance| {
        FsmBuilder::<BeastState, BeastEvent, BeastContext, BeastAction>::new(BeastState::Counting {
            balance: initial_balance,
            operations: vec![],
            operation_ids: std::collections::HashSet::new(),
        })
        .when("Counting")
        .on(
            "Subtract",
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
                        BeastEvent::Subtract { id, value },
                    ) = (state, event)
                    {
                        operations.push(format!("Subtract {id} by {value}"));
                        Ok(Transition {
                            next_state: BeastState::Counting {
                                balance: balance.saturating_sub(value),
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
        .build()
    };

    let build_append_machine = || {
        FsmBuilder::<BeastState, BeastEvent, BeastContext, BeastAction>::new(BeastState::Counting {
            balance: 0,
            operations: vec![],
            operation_ids: std::collections::HashSet::new(),
        })
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
        .build()
    };

    let build_naive_average_machine = |initial_balance| {
        FsmBuilder::<BeastState, BeastEvent, BeastContext, BeastAction>::new(BeastState::Counting {
            balance: initial_balance,
            operations: vec![],
            operation_ids: std::collections::HashSet::new(),
        })
        .when("Counting")
        .on(
            "NaiveAverage",
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
                        BeastEvent::NaiveAverage { id, value },
                    ) = (state, event)
                    {
                        operations.push(format!("NaiveAverage {id} with {value}"));
                        Ok(Transition {
                            next_state: BeastState::Counting {
                                balance: naive_average(balance, value),
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
        .build()
    };

    let balance_of = |state: &BeastState| {
        if let BeastState::Counting { balance, .. } = state {
            *balance
        } else {
            panic!("expected Counting state, got {state:?}");
        }
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
                    // The FSM state is the source of truth for idempotence. Context may track
                    // metrics, but it must not be required to decide whether a logical event
                    // has already changed state.
                    if operation_ids.contains(&id) {
                        ctx.duplicate_count.fetch_add(1, Ordering::Relaxed);

                        // Already processed, ignore.
                        return Ok(Transition {
                            next_state: BeastState::Counting {
                                balance,
                                operations,
                                operation_ids,
                            },
                            actions: vec![BeastAction::AlertDuplicate(id)],
                        });
                    }

                    operation_ids.insert(id.clone());
                    operations.push(format!("Credit {id} by {amount}"));
                    ctx.operation_log
                        .write()
                        .await
                        .push((id.clone(), format!("Credit:{amount}")));

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
        "Subtract",
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
                    BeastEvent::Subtract { id, value },
                ) = (state, event)
                {
                    // Additive deltas batch by summing payloads; see the regrouping trial below.
                    operations.push(format!("Subtract {id} by {value}"));
                    ctx.operation_log
                        .write()
                        .await
                        .push((id, format!("Subtract:{value}")));

                    let new_balance = balance.saturating_sub(value);

                    Ok(Transition {
                        next_state: BeastState::Counting {
                            balance: new_balance,
                            operations,
                            operation_ids,
                        },
                        actions: vec![BeastAction::RecordOperation(format!("Subtract:{value}"))],
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

    let mut replay_ctx = new_context();
    let replay_actions = machine
        .handle(
            BeastEvent::Credit {
                id: "credit_0".to_string(),
                amount: 100,
            },
            &mut replay_ctx,
        )
        .await
        .unwrap();

    if let BeastState::Counting {
        balance,
        operation_ids,
        ..
    } = machine.state()
    {
        assert_eq!(
            balance, &1000,
            "Idempotency depended on context instead of persisted FSM state"
        );
        assert!(
            operation_ids.contains("credit_0"),
            "expected processed operation ID to remain in FSM state"
        );
    }
    assert_eq!(
        replay_actions,
        vec![BeastAction::AlertDuplicate("credit_0".to_string())]
    );
    assert_eq!(replay_ctx.duplicate_count.load(Ordering::Relaxed), 1);

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

    let mut ordered_machine = build_append_machine();
    let mut append_ctx = new_context();

    // Process in order
    for event in &append_events {
        ordered_machine
            .handle(event.clone(), &mut append_ctx)
            .await
            .unwrap();
    }

    let ordered_ops = if let BeastState::Counting { operations, .. } = ordered_machine.state() {
        operations.clone()
    } else {
        vec![]
    };

    assert_eq!(
        ordered_ops,
        vec![
            "First".to_string(),
            "Second".to_string(),
            "Third".to_string()
        ]
    );

    // Process in reverse order
    let mut reversed_machine = build_append_machine();
    for event in append_events.iter().rev() {
        reversed_machine
            .handle(event.clone(), &mut append_ctx)
            .await
            .unwrap();
    }

    let reversed_ops = if let BeastState::Counting { operations, .. } = reversed_machine.state() {
        operations.clone()
    } else {
        vec![]
    };

    assert_eq!(
        reversed_ops,
        vec![
            "Third".to_string(),
            "Second".to_string(),
            "First".to_string()
        ]
    );
    assert_ne!(
        ordered_ops, reversed_ops,
        "Operations are commutative when they shouldn't be!"
    );

    // Trial 3: Regrouping
    //
    // Debit/subtract events are safe to batch when their payloads are combined by addition.
    // Sequential: (100 - 10) - 5 = 85.
    // Batched:    100 - (10 + 5) = 85.
    let mut sequential_debits = build_subtract_machine(100);
    sequential_debits
        .handle(
            BeastEvent::Subtract {
                id: "sequential_a".to_string(),
                value: 10,
            },
            &mut ctx,
        )
        .await
        .unwrap();
    sequential_debits
        .handle(
            BeastEvent::Subtract {
                id: "sequential_b".to_string(),
                value: 5,
            },
            &mut ctx,
        )
        .await
        .unwrap();

    let batched_delta = 10_i64.saturating_add(5);
    let mut batched_debit = build_subtract_machine(100);
    batched_debit
        .handle(
            BeastEvent::Subtract {
                id: "batched".to_string(),
                value: batched_delta,
            },
            &mut ctx,
        )
        .await
        .unwrap();

    let sequential_balance = balance_of(sequential_debits.state());
    let batched_balance = balance_of(batched_debit.state());

    assert_eq!(sequential_balance, 85);
    assert_eq!(batched_balance, 85);
    assert_eq!(
        sequential_balance, batched_balance,
        "Debit regrouping must combine deltas by addition"
    );

    // A genuinely non-associative combine: naive averaging of partial averages.
    // This loses the count carried by each partial, so regrouping changes the result.
    let mut left_associated_average = build_naive_average_machine(100);
    left_associated_average
        .handle(
            BeastEvent::NaiveAverage {
                id: "avg_a".to_string(),
                value: 10,
            },
            &mut ctx,
        )
        .await
        .unwrap();
    left_associated_average
        .handle(
            BeastEvent::NaiveAverage {
                id: "avg_b".to_string(),
                value: 5,
            },
            &mut ctx,
        )
        .await
        .unwrap();

    let grouped_average = naive_average(10, 5);
    let mut right_grouped_average = build_naive_average_machine(100);
    right_grouped_average
        .handle(
            BeastEvent::NaiveAverage {
                id: "avg_grouped".to_string(),
                value: grouped_average,
            },
            &mut ctx,
        )
        .await
        .unwrap();

    let left_average = balance_of(left_associated_average.state());
    let right_average = balance_of(right_grouped_average.state());

    assert_eq!(left_average, 30);
    assert_eq!(right_average, 53);
    assert_ne!(
        left_average, right_average,
        "Naive averaging was treated as associative; regrouped partials changed no state"
    );

    // Saturating arithmetic has its own boundary behaviour. The small-value debit example above
    // is batchable, but saturation can still make mixed updates order-dependent at the edge.
    let credit_then_debit = i64::MAX.saturating_add(10).saturating_sub(10);
    let debit_then_credit = i64::MAX.saturating_sub(10).saturating_add(10);
    assert_ne!(
        credit_then_debit, debit_then_credit,
        "Saturating arithmetic boundaries should remain visible in this harness"
    );

    // Trial 4: The Number of the Beast

    // Send debits until the mid-loop mark check reaches 666 from 1000.
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
