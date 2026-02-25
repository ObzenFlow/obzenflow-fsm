# Test suite

The integration tests are organised in two tiers.

## The circles of distributed systems hell

Six numbered tests that target real failure modes in async, event-sourced state machines. Each one is written as documentation: the module-level doc comment names the failure mode and explains why it matters.

They are loosely modelled after Dante's circles. In practice, they stress-test the properties that keep ObzenFlow's pipeline runtime correct under concurrency, delivery semantics, and resource pressure.

| Circle | File | What it proves |
|--------|------|----------------|
| 1 | `circle_1_race_condition.rs` | Shared-state correctness under contention. Ten concurrent FSMs, atomic counters, barrier-synchronised drain. |
| 2 | `circle_2_async_coordination.rs` | Coordinated startup and shutdown across a pipeline of nested FSMs. Barrier sync, broadcast drain, staggered initialisation. |
| 3 | `circle_3_journal_subscription.rs` | Journal writes, subscription filtering, and causal ordering under chaos. Vector clocks, bounded channels, deliberate out-of-order delivery. |
| 4 | `circle_4_mathematical_properties.rs` | The "unholy trinity" (idempotence, commutativity, associativity) vs at-least-once delivery. Deduplication by operation ID, non-commutativity proof, overflow semantics. |
| 5 | `circle_5_timeout_cancellation.rs` | Timeout correctness without silent data loss. The "Jonestown Protocol": the FSM self-destructs rather than dropping a message. Downstream failure propagation. |
| 6 | `circle_6_memory_corruption.rs` | Cyclic references, abandoned async tasks, mass spawn/kill. Proves `Arc`-based structures survive abuse without leaking history or corrupting state. |

### Running the tests

Run the full suite:

```bash
cargo test
```

Run a single circle with visible output:

```bash
cargo test circle_4 -- --nocapture
```

`cargo test` accepts a substring filter. `circle_4` matches any test whose name contains that string, so it picks up everything in `circle_4_mathematical_properties.rs` without needing the full path. The `--nocapture` flag (after the `--` separator) tells the test runner to print stdout instead of swallowing it on success. This is useful for the circle tests because they log what they're doing as they go.

## Feature and safety tests

These cover the API surface, builder validation, DSL mechanics, and compile-time guarantees.

| File | Focus |
|------|-------|
| `test_simple.rs` | Baseline transitions, entry/exit hooks, timeouts, unhandled events |
| `test_dsl_basic.rs` | `fsm!` macro smoke test |
| `test_dsl_features.rs` | DSL entry/exit hooks, timeout clauses, unhandled handling |
| `test_builder_enforcement.rs` | Duplicate handler detection, strict mode validation |
| `test_builder_only_construction.rs` | Proves `StateMachine` cannot be constructed outside the builder |
| `test_compile_safety.rs` | Type-level safety: wrong state/event/context/action types must not compile |
| `test_edge_cases.rs` | Deeply nested state, ZSTs, async handlers, float edge cases, ten-state machines |
| `test_comprehensive.rs` | Full connection lifecycle with metrics, timeouts, and graceful degradation |
