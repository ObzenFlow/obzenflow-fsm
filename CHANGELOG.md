# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1] - 2026-02-25

### Fixed
- `#[derive(StateVariant)]` and `#[derive(EventVariant)]` now correctly match unit and tuple enum variants.

### Changed
- Prepared `obzenflow-fsm-macros` for publishing (crate metadata + policy files) and fixed `cargo publish --dry-run`.
- Added SPDX headers across Rust sources and aligned CI with ObzenFlow (pinned toolchain, `--locked`, SPDX checks).
- Added `Cargo.lock`, renamed “circle” tests to `circle_N_*`, and added `tests/README.md`.
- Documentation and README improvements.

## [0.3.0] - 2025-11-27

### Changed
- Promoted the `fsm!` macro and derives (`StateVariant`, `EventVariant`) to the primary public front-end for building state machines.
- Removed `pub use builder::FsmBuilder` from the crate root; the builder now lives under `obzenflow_fsm::internal::FsmBuilder` and is intended only for macro expansion and focused tests.
- Tightened builder-time validation so strict mode is always enabled (duplicate `(state, event)` handlers are rejected and the initial state must have at least one transition or timeout).
- Aligned `StateMachine` error behaviour with the design docs by returning structured `FsmError` for unhandled events and builder failures instead of stringly errors or silent success.

### Migration
- Updated crate examples and internal tests to prefer the `fsm!` DSL where it improves clarity, retaining legacy builder coverage only in dedicated tests that exercise `internal::FsmBuilder`.
- Coordinated with ObzenFlow so runtime supervisors now construct FSMs exclusively via `fsm!` and a `StateMachine`-returning API, with no public dependency on `FsmBuilder`.

## [0.2.1] - 2025-11-26

### Added
- `fsm!` macro front-end with support for:
  - Top-level `state` / `event` / `context` / `action` / `initial` declarations.
  - Per-state `on` clauses, `timeout` clauses, and `on_entry` / `on_exit` hooks.
  - Optional top-level `unhandled => handler;` mapping to `FsmBuilder::when_unhandled`.
- New DSL-focused tests exercising the macro (`test_dsl_basic`, `test_dsl_features`).

### Changed
- Marked the string-based builder API as deprecated in preparation for `0.3.0`:
  - `FsmBuilder::when(&str)`, `from_any()`, `on_entry(&str, …)`, `on_exit(&str, …)`.
  - `WhenBuilder::on(&str, …)` and `TimeoutBuilder::on(&str, …)`.
- Documented that these methods will become crate-private or be removed in `0.3.0`, leaving the typed DSL as the primary public entry point.

## [0.2.0] - 2025-11-25

### Changed
- Switched from `Arc<Context>` to a single mutable-context FSM API:
  - `StateMachine::handle(&mut self, event, &mut Context)`
  - `FsmAction::execute(&self, &mut Context)`
- Updated `FsmBuilder` handler signatures to receive `&mut Context` instead of `Arc<Context>`.
- Clarified Mealy-machine / event-sourcing semantics and added guardrails for keeping FSM usage deterministic and “functional in spirit”.

### Improved
- Timeout semantics:
  - Initial-state timeouts are now scheduled when the machine is constructed if configured.
  - Self-transitions run exit/entry hooks and refresh timeouts.
- Error handling:
  - Introduced structured `FsmError` across public APIs (timeouts, unhandled events, duplicate handlers, handler failures).
  - Builder-time validation rejects duplicate `(state, event)` handlers.
- Unhandled event behavior:
  - Unhandled events without a `when_unhandled` hook now produce a structured error instead of silently succeeding.

## [0.1.0] - 2025-08-15

### Added
- Initial release of obzenflow-fsm
- Async-first Finite State Machine implementation
- Builder pattern for compile-time safety
- Support for async closures via `Arc<Context>`
- Mealy machine implementation
- Comprehensive test suite including stress tests
- Examples demonstrating common usage patterns

### Core Features
- **State Management**: Type-safe state transitions with compile-time guarantees
- **Event Handling**: Flexible event routing with pattern matching
- **Async Support**: Full async/await support with proper lifetime management
- **Context Sharing**: Thread-safe context sharing via `Arc`
- **Action System**: Deferred action execution after state transitions
- **Error Handling**: Comprehensive error types and recovery patterns

### Documentation
- Detailed README with design rationale
- API documentation for all public types
- Example FSM implementations
- Test suite demonstrating advanced patterns

[Unreleased]: https://github.com/obzenflow/obzenflow-fsm/compare/v0.3.1...HEAD
[0.3.1]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.3.1
[0.3.0]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.3.0
[0.2.1]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.2.1
[0.2.0]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.2.0
[0.1.0]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.1.0
