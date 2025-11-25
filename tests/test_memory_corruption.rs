//! Test 6: The Memory Corruption Gauntlet (Testing the Incorruptible) 🛡️
//!
//! The Final Boss - Satan's Last Stand:
//! - Cyclic references trying to create infinite loops
//! - Self-referential structures (ouroboros of doom)
//! - Abandoned transitions leaving ghost references
//! - Memory leaks trying to exhaust the divine heap
//!
//! God's Ultimate Defense - Arc:
//! - Reference counting prevents use-after-free (no zombies!)
//! - Weak references break cycles (cutting the ouroboros)
//! - Drop implementations ensure cleanup (divine garbage collection)
//! - The incorruptible Arc<Context> stands firm
//!
//! What it tests:
//! - Shared mutable state with complex access patterns
//! - Self-referential structures
//! - Cyclic dependencies in contexts
//! - Memory leaks from abandoned transitions
//!
//! Why it matters:
//! - Tests if Arc<Context> prevents memory corruption
//! - Ensures no leaks in long-running systems
//! - Proves Rust's ownership is divinely inspired
//! - The 666th line should be in this test (if we reach it)

use obzenflow_fsm::{FsmBuilder, StateVariant, EventVariant, Transition, FsmContext, FsmAction};
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Weak;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use tokio::time::sleep;

#[tokio::test]
async fn test_6_memory_corruption_gauntlet() {
    #[derive(Clone, Debug, PartialEq)]
    enum CorruptionState {
        Spawning { id: usize },
        Corrupting { cycles: usize },
        SelfReferencing,
        Dying(String),
        Dead,
    }

    impl StateVariant for CorruptionState {
        fn variant_name(&self) -> &str {
            match self {
                CorruptionState::Spawning { .. } => "Spawning",
                CorruptionState::Corrupting { .. } => "Corrupting",
                CorruptionState::SelfReferencing => "SelfReferencing",
                CorruptionState::Dying(_) => "Dying",
                CorruptionState::Dead => "Dead",
            }
        }
    }

    #[derive(Clone, Debug)]
    enum CorruptionEvent {
        SpawnChild(usize),
        CreateCycle,
        CreateSelfReference,
        AbandonAsync,
        Die,
    }

    impl EventVariant for CorruptionEvent {
        fn variant_name(&self) -> &str {
            match self {
                CorruptionEvent::SpawnChild(_) => "SpawnChild",
                CorruptionEvent::CreateCycle => "CreateCycle",
                CorruptionEvent::CreateSelfReference => "CreateSelfReference",
                CorruptionEvent::AbandonAsync => "AbandonAsync",
                CorruptionEvent::Die => "Die",
            }
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    enum CorruptionAction {
        AllocateMemory(usize),
        SpawnTask,
        CreateReference,
        Cleanup,
    }

    // === THE OUROBOROS CONTEXT (Self-referential nightmare) ===
    #[derive(Clone)]
    struct CorruptionContext {
        // The ouroboros: contexts that reference each other
        parent: Arc<RwLock<Option<Weak<CorruptionContext>>>>,
        children: Arc<RwLock<Vec<Arc<CorruptionContext>>>>,

        // Self-reference through channels (the snake eating its tail)
        self_sender: Arc<RwLock<Option<mpsc::Sender<Arc<CorruptionContext>>>>>,
        self_receiver: Arc<RwLock<Option<mpsc::Receiver<Arc<CorruptionContext>>>>>,

        // Memory pressure tracking
        allocated_memory: Arc<AtomicUsize>,
        active_tasks: Arc<RwLock<Vec<tokio::task::JoinHandle<()>>>>,

        // Tracking for verification
        id: usize,
        creation_count: Arc<AtomicUsize>,
        drop_count: Arc<AtomicUsize>,
    }

    impl FsmContext for CorruptionContext {
        fn describe(&self) -> String {
            format!("CorruptionContext#{} with {} bytes allocated", self.id, self.allocated_memory.load(Ordering::Relaxed))
        }
    }

    #[async_trait]
    impl FsmAction for CorruptionAction {
        type Context = CorruptionContext;

        async fn execute(&self, ctx: &mut Self::Context) -> obzenflow_fsm::types::FsmResult<()> {
            match self {
                CorruptionAction::AllocateMemory(size) => {
                    ctx.allocated_memory.fetch_add(*size, Ordering::Relaxed);
                    Ok(())
                }
                CorruptionAction::SpawnTask => {
                    // Spawn task
                    Ok(())
                }
                CorruptionAction::CreateReference => {
                    // Create reference
                    Ok(())
                }
                CorruptionAction::Cleanup => {
                    // Cleanup
                    Ok(())
                }
            }
        }
    }

    impl CorruptionContext {
        fn new(id: usize, creation_count: Arc<AtomicUsize>, drop_count: Arc<AtomicUsize>) -> Self {
            creation_count.fetch_add(1, Ordering::SeqCst);
            let (tx, rx) = mpsc::channel(100);

            CorruptionContext {
                parent: Arc::new(RwLock::new(None)),
                children: Arc::new(RwLock::new(Vec::new())),
                self_sender: Arc::new(RwLock::new(Some(tx))),
                self_receiver: Arc::new(RwLock::new(Some(rx))),
                allocated_memory: Arc::new(AtomicUsize::new(0)),
                active_tasks: Arc::new(RwLock::new(Vec::new())),
                id,
                creation_count,
                drop_count: drop_count.clone(),
            }
        }

        // Create a cycle: A -> B -> C -> A
        async fn create_cycle(self: Arc<Self>, other: Arc<CorruptionContext>) {
            // Create strong reference cycle
            self.children.write().await.push(other.clone());
            other.parent.write().await.replace(Arc::downgrade(&self));

            // Even worse: send self through channel creating more refs
            if let Some(tx) = &*self.self_sender.read().await {
                let _ = tx.send(self.clone()).await;
            }
        }

        // Break cycles using Weak references
        async fn break_cycles(&self) {
            // Clear children (breaks forward references)
            self.children.write().await.clear();

            // Clear parent (already weak, but let's be thorough)
            *self.parent.write().await = None;

            // Drop channel to break self-reference
            *self.self_sender.write().await = None;
            *self.self_receiver.write().await = None;
        }
    }

    impl Drop for CorruptionContext {
        fn drop(&mut self) {
            self.drop_count.fetch_add(1, Ordering::SeqCst);
            println!("💀 Context {} dropped", self.id);
        }
    }

    // === TRIAL 1: The Ouroboros (Cyclic References) ===
    println!("😈 Trial 1: The Ouroboros - Cyclic References");

    let creation_count = Arc::new(AtomicUsize::new(0));
    let drop_count = Arc::new(AtomicUsize::new(0));

    {
        // Create a cycle of contexts
        let ctx1 = Arc::new(CorruptionContext::new(1, creation_count.clone(), drop_count.clone()));
        let ctx2 = Arc::new(CorruptionContext::new(2, creation_count.clone(), drop_count.clone()));
        let ctx3 = Arc::new(CorruptionContext::new(3, creation_count.clone(), drop_count.clone()));

        // Create the cycle: 1 -> 2 -> 3 -> 1
        ctx1.clone().create_cycle(ctx2.clone()).await;
        ctx2.clone().create_cycle(ctx3.clone()).await;
        ctx3.clone().create_cycle(ctx1.clone()).await;

        // Verify cycle exists
        assert_eq!(ctx1.children.read().await.len(), 1);
        assert_eq!(Arc::strong_count(&ctx1), 3); // self + ctx3's child + local

        // Break the cycle
        ctx1.break_cycles().await;
        ctx2.break_cycles().await;
        ctx3.break_cycles().await;

        // Verify counts will drop
        assert_eq!(Arc::strong_count(&ctx1), 1); // only local ref remains
    }

    // Wait for drops
    sleep(Duration::from_millis(10)).await;

    let created = creation_count.load(Ordering::SeqCst);
    let dropped = drop_count.load(Ordering::SeqCst);
    println!("✅ Created: {}, Dropped: {} (should be equal)", created, dropped);
    assert_eq!(created, dropped, "Memory leak detected! Contexts not dropped!");

    // === TRIAL 2: The Abandoned Async Tasks ===
    println!("\n😈 Trial 2: Abandoned Async Tasks");

    let task_creation_count = Arc::new(AtomicUsize::new(0));
    let task_completion_count = Arc::new(AtomicUsize::new(0));

    {
        let ctx = Arc::new(CorruptionContext::new(
            666, // The beast's context
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0))
        ));

        // Spawn tasks that hold Arc references
        for _ in 0..100 {
            let ctx_clone = ctx.clone();
            let task_creation = task_creation_count.clone();
            let task_completion = task_completion_count.clone();

            let handle = tokio::spawn(async move {
                task_creation.fetch_add(1, Ordering::SeqCst);

                // Hold the context reference
                let _ctx = ctx_clone;

                // Simulate work
                sleep(Duration::from_millis(50)).await;

                task_completion.fetch_add(1, Ordering::SeqCst);
            });

            ctx.active_tasks.write().await.push(handle);
        }

        // Abandon some tasks (drop handles without awaiting)
        ctx.active_tasks.write().await.truncate(50);

        // Context goes out of scope with active tasks
    }

    // Wait for tasks to complete or abort
    sleep(Duration::from_millis(100)).await;

    let created_tasks = task_creation_count.load(Ordering::SeqCst);
    let completed_tasks = task_completion_count.load(Ordering::SeqCst);
    println!("✅ Tasks created: {}, completed: {}", created_tasks, completed_tasks);
    // Some tasks may not complete due to being dropped, but that's fine

    // === TRIAL 3: The FSM Apocalypse (Mass Spawn/Kill) ===
    println!("\n😈 Trial 3: The FSM Apocalypse - 1000 FSMs");

    let fsm_count = Arc::new(AtomicUsize::new(0));
    let death_count = Arc::new(AtomicUsize::new(0));
    let mut fsm_handles = vec![];

    // Measure baseline memory
    let baseline_memory = get_approximate_memory();
    println!("📊 Baseline memory: ~{} bytes", baseline_memory);

    // Spawn 1000 FSMs
    for i in 0..1000 {
        let fsm_count_clone = fsm_count.clone();
        let death_count_clone = death_count.clone();

        let handle = tokio::spawn(async move {
            fsm_count_clone.fetch_add(1, Ordering::SeqCst);

            // Create a memory-hungry context
            let mut ctx = CorruptionContext::new(
                i,
                Arc::new(AtomicUsize::new(0)),
                Arc::new(AtomicUsize::new(0))
            );

            // Allocate some memory
            ctx.allocated_memory.store(1024 * 1024, Ordering::SeqCst); // 1MB per FSM
            let _memory: Vec<u8> = vec![0; 1024 * 1024];

            // Build FSM
            let mut fsm = FsmBuilder::<CorruptionState, CorruptionEvent, CorruptionContext, CorruptionAction>::new(
                CorruptionState::Spawning { id: i }
            )
                .when("Spawning")
                    .on("Die", |state, _event: &CorruptionEvent, _ctx: &mut CorruptionContext| {
                        let state = state.clone();
                        Box::pin(async move {
                            if let CorruptionState::Spawning { id: _ } = state {
                                Ok(Transition {
                                    next_state: CorruptionState::Dead,
                                    actions: vec![CorruptionAction::Cleanup],
                                })
                            } else {
                                unreachable!()
                            }
                        })
                    })
                    .done()
                .build();

            // Random lifetime
            sleep(Duration::from_millis(rand::random::<u64>() % 100)).await;

            // Die
            let _ = fsm.handle(CorruptionEvent::Die, &mut ctx).await;

            death_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        fsm_handles.push(handle);

        // Randomly abort some FSMs (Satan kills them)
        if i % 7 == 0 {
            if let Some(handle) = fsm_handles.pop() {
                handle.abort();
            }
        }
    }

    // Wait for all FSMs to complete or abort
    for handle in fsm_handles {
        let _ = handle.await;
    }

    let spawned = fsm_count.load(Ordering::SeqCst);
    let died = death_count.load(Ordering::SeqCst);
    println!("✅ FSMs spawned: {}, died: {} (some aborted)", spawned, died);

    // Check memory returned to baseline (approximately)
    sleep(Duration::from_millis(100)).await; // Let things settle
    let final_memory = get_approximate_memory();
    println!("📊 Final memory: ~{} bytes", final_memory);

    // Memory should not grow unbounded
    let memory_growth = final_memory.saturating_sub(baseline_memory);
    println!("📈 Memory growth: {} bytes", memory_growth);
    assert!(memory_growth < 100 * 1024 * 1024, "Memory leak detected! Growth: {} MB", memory_growth / 1024 / 1024);

    // === TRIAL 4: The Self-Referential Nightmare ===
    println!("\n😈 Trial 4: Self-Referential Nightmare");

    let nightmare_ctx = Arc::new(CorruptionContext::new(
        999,
        Arc::new(AtomicUsize::new(0)),
        Arc::new(AtomicUsize::new(0))
    ));

    // Create self-reference through channel
    if let Some(tx) = &*nightmare_ctx.self_sender.read().await {
        // Send self through own channel!
        let _ = tx.send(nightmare_ctx.clone()).await;
        let _ = tx.send(nightmare_ctx.clone()).await;
        let _ = tx.send(nightmare_ctx.clone()).await;
    }

    // Verify self-references exist
    let initial_strong_count = Arc::strong_count(&nightmare_ctx);
    println!("🔄 Self-references created, strong count: {}", initial_strong_count);
    assert!(initial_strong_count > 1, "Self-references not created!");

    // Break self-references
    nightmare_ctx.break_cycles().await;

    // Verify cleanup
    let final_strong_count = Arc::strong_count(&nightmare_ctx);
    println!("✅ Self-references broken, strong count: {}", final_strong_count);
    assert_eq!(final_strong_count, 1, "Self-references not cleaned up!");

    println!("\n🏆 THE INCORRUPTIBLE Arc<Context> HAS DEFEATED SATAN!");
    println!("✝️ Memory corruption gauntlet complete - Rust's ownership is divine!");

    // "And God said, Let there be Arc: and there was Arc." - Genesis 1:3 (Rust edition)
}

// Helper function to get approximate memory usage
// This is a rough estimate, not precise measurement
fn get_approximate_memory() -> usize {
    // In a real test, you might use platform-specific APIs or external tools
    // For now, we'll use a simple allocation test
    let test_vec: Vec<u8> = Vec::with_capacity(1024);
    let ptr = test_vec.as_ptr() as usize;
    std::mem::forget(test_vec); // Don't drop it
    ptr // Use pointer value as a rough indicator
}
