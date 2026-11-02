# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-11-02

### Added
- `#[derive(StateVariant)]` and `#[derive(EventVariant)]` now accept generic enums.
- A Publish workflow releases each crate from CI, behind a pre-flight report and the `release` environment's approval, with a crates.io token minted for the run.
- An `advisories` check runs `cargo deny check advisories` on every CI run and daily.

### Changed
- Both crates now share one version, starting at 0.4.0 (`obzenflow-fsm-macros` moves from 0.1.2), and `obzenflow-fsm` requires `obzenflow-fsm-macros` at exactly that version.
- `fsm!` requires its state and event types to derive `StateVariant` and `EventVariant`. A hand-written impl fails to compile at the table's `state:` or `event:` line.
- `obzenflow_fsm::builder` is no longer public. The builder is reachable only through the hidden `obzenflow_fsm::internal` path, with no stability promise, and its transition helpers moved to `obzenflow_fsm::transitions`.
- CI uses Rust 1.99.0. Both manifests deny the Clippy lints that catch `#[allow]` and `let _` discards, and CI refuses `#[expect]`.

### Removed
- The builder's deprecation, and the `#[allow(deprecated)]` that every `fsm!` expansion carried.

### Fixed
- `fsm!` no longer compiles a `state` block or `on` row that names a variant its enum does not have. It fails with `E0599` at that line, where before the event went unhandled at runtime.
- Corrected Circle 4's mathematical-property harness: idempotence now depends on in-state operation IDs, duplicate credits suppress downstream actions, append ordering is compared from equivalent initial states, debit batching uses additive deltas, and the non-associative regrouping example now uses naive partial averages.

### Security
- The lockfile moves tokio to 1.54.1, past RUSTSEC-2026-0386. The published requirement is unchanged.

### Migration
- Import `goto`, `goto_with_actions`, `stay` and `stay_with_actions` from `obzenflow_fsm::transitions`.
- Replace a hand-written `StateVariant` or `EventVariant` impl on an `fsm!` type with the derive.
- Move code that used `obzenflow_fsm::builder` to `fsm!`.

## [0.3.3] - 2026-05-02

### Fixed
- Added the missing Circle 4 associativity/regrouping test coverage.
- Renamed the Circle 4 test so `cargo test circle_4 -- --nocapture` runs the documented target.
- Updated the Circle 4 test README description to match the implemented mathematical-property coverage.

## [0.3.2] - 2026-03-01

### Added
- Added DCO governance docs.
- Added a PR-only DCO check in CI.

### Changed
- Tightened README (removed redundancy, clarified design goals).
- Updated `CONTRIBUTING.md` and trademark policy docs.
- Updated MIT license year ranges.

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

[Unreleased]: https://github.com/obzenflow/obzenflow-fsm/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/obzenflow/obzenflow-fsm/compare/v0.3.3...v0.4.0
[0.3.3]: https://github.com/obzenflow/obzenflow-fsm/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.3.2
[0.3.1]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.3.1
[0.3.0]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.3.0
[0.2.1]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.2.1
[0.2.0]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.2.0
[0.1.0]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.1.0
