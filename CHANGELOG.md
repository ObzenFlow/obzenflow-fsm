# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-11-02

`fsm!` now catches a misspelled state or event name when you compile, where before the event went unhandled at runtime. This release also removes the old builder API, so `fsm!` is the only way to build a state machine.

### Breaking changes
- Enums used as the state or event type in `fsm!` must derive `StateVariant` or `EventVariant`. A hand-written impl no longer compiles.
- The `builder` module is no longer public. Use `fsm!` instead.
- The `goto` and `stay` helpers, and their `_with_actions` forms, moved to `obzenflow_fsm::transitions`.

### Added
- The derives now work on generic enums.

### Fixed
- A misspelled state or event name in an `fsm!` table is now a compile error on that line.
- Circle 4's tests now check the properties they describe, and `cargo test circle_4` runs them.

### Changed
- `obzenflow-fsm-macros` now shares this crate's version, so it jumps from 0.1.2 to 0.4.0. You don't need to add it to your dependencies.

### Upgrading from 0.3

Replace hand-written impls with the derive:

```diff
-#[derive(Clone, Debug, PartialEq)]
+#[derive(Clone, Debug, PartialEq, StateVariant)]
 enum DoorState {
     Closed,
     Open,
 }
-
-impl StateVariant for DoorState {
-    fn variant_name(&self) -> &str {
-        match self {
-            DoorState::Closed => "Closed",
-            DoorState::Open => "Open",
-        }
-    }
-}
```

Do the same for your event enum with `EventVariant`. Then update the helper imports:

```diff
-use obzenflow_fsm::builder::transitions::{goto, stay};
+use obzenflow_fsm::transitions::{goto, stay};
```

If you used the builder directly, move to `fsm!`. The README's quick start shows a full example.

## [0.3.2] - 2026-03-01

This release changes no code. It updates the README, the contributing guide and the copyright years, and contributors now sign off each commit under the Developer Certificate of Origin.

## [0.3.1] - 2026-02-25

### Fixed
- The derives now handle unit and tuple variants correctly.

### Changed
- The circle tests are renamed `circle_1_*` to `circle_6_*`, and `tests/README.md` describes each one.

## [0.3.0] - 2026-01-21

The first release on crates.io. It adds `fsm!`, which writes a state machine as a table of states and the events each one handles, and makes it the main way to build one.

### Breaking changes
- `FsmBuilder` is no longer exported from the crate root. Use `fsm!` instead.
- Building a machine now fails if two handlers cover the same state and event, or if the initial state has no transition or timeout.

### Added
- `fsm!`, with `on` rows, timeouts, and entry and exit hooks for each state, plus an optional handler for unhandled events.
- The `StateVariant` and `EventVariant` derives, so you don't write those impls by hand.

### Deprecated
- The builder's string-keyed methods (`when`, `from_any`, `on_entry`, `on_exit` and `on`). Use `fsm!` instead.

### Upgrading from 0.2

Rewrite builder code with `fsm!`. The README's quick start shows a full example.

## 0.2.0 - 2025-11-25

Handlers and actions now get the context as `&mut Context` instead of a shared `Arc<Context>`, and errors are `FsmError` values instead of strings.

### Breaking changes
- Handlers, `StateMachine::handle` and `check_timeout` take `&mut Context` in place of `Arc<Context>`, and `FsmAction::execute` takes `&mut Self::Context`.
- Errors are `FsmError` values instead of strings.
- An event with no handler and no `when_unhandled` hook now returns an error, where before it was logged and ignored.

### Changed
- If the initial state has a timeout, it now starts when the machine is built.
- A transition back to the same state now runs its exit and entry hooks and restarts its timeout.

### Upgrading from 0.1

```diff
-async fn execute(&self, ctx: &Self::Context) -> Result<(), String> {
+async fn execute(&self, ctx: &mut Self::Context) -> FsmResult<()> {
```

```diff
-let actions = machine.handle(event, ctx.clone()).await?;
+let actions = machine.handle(event, &mut ctx).await?;
```

## 0.1.0 - 2025-08-15

The first version, extracted from ObzenFlow.

### Added
- An async state machine. Each event runs a handler that returns the next state and a list of actions, and the caller runs the actions after the transition.
- `FsmBuilder`, for declaring each state's handlers, timeouts, and entry and exit hooks.
- A context shared with every handler through `Arc<Context>`.
- An example and a test suite, including stress tests.

[Unreleased]: https://github.com/obzenflow/obzenflow-fsm/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/obzenflow/obzenflow-fsm/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.3.2
[0.3.1]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.3.1
[0.3.0]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.3.0
