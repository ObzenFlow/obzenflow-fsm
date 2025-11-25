# FSM Timeouts, Self-Transitions, and Error Surfacing

## Overview
- Clarify runtime semantics for initial-state timeouts and self-transitions in the dynamic-dispatch FSM.
- Adopt structured errors (`FsmError`) across the public API.
- Add builder-time validation to catch configuration mistakes early.
- Target: production-grade ergonomics while retaining runtime flexibility.

## Context / Background
- “Initial state” is simply the starting state provided to `FsmBuilder::new(...)`. Any state (including the initial one) can have a timeout if configured via `timeout(...)` in the builder.
- Current behavior leaves `state_timeout = None` on construction, so an initial-state timeout never starts counting down until after some transition occurs.
- Proposed behavior: at construction, if the initial state has a configured timeout, set `state_timeout = Instant::now() + duration` so `check_timeout` can trigger it immediately when due.

## Current Pain Points
- **Initial state timeout ignored**: `state_timeout` is never scheduled in `new`, so a configured initial-state timeout never fires until after a transition.
- **Self-transitions are no-ops**: when `next_state == current_state`, entry/exit hooks and timeout refresh are skipped, leading to stale timers and surprising behavior.
- **Silent overwrites**: builder uses string keys and overwrites duplicate `(state, event)` handlers with no warning.
- **Unstructured errors**: APIs return `Result<_, String>` even though `FsmError` exists, losing context (which handler, which state/event).

## Benefits
- Predictable timeout semantics from the moment the machine starts.
- Explicit, documented behavior for self-transitions (hooks + timeout refresh).
- Early detection of misconfiguration (duplicate handlers, potentially unknown states/events).
- Structured errors improve observability, logging, and caller handling.

## Challenges / Risks
- Backward compatibility: changed timeout and self-transition semantics can alter existing user behavior/tests.
- Validation strictness vs. flexibility: string-based keys must stay dynamic-friendly while surfacing mistakes.
- Error type rollout: converting from `String` to `FsmError` without excessive boilerplate.

## Decisions (to confirm)
1) **Initial-state timeout scheduling**
   - **Option A (preferred):** schedule timeout in `StateMachine::new` when the initial state has a timeout handler.
   - Option B: keep current “no initial timeout” behavior but document it.
2) **Self-transition semantics**
   - **Option A (preferred):** treat self-transitions as full transitions (run exit→entry hooks, refresh timeout).
   - Option B: skip hooks but refresh timeout.
   - Option C: keep pure no-op (current) and document.
3) **Timeout refreshing**
   - Refresh `state_timeout` on every transition (including self) based on the resulting state; clear when none exists.
4) **Builder validation**
   - Error on duplicate `(state, event)` registrations (`FsmError::DuplicateHandler { state, event }`).
   - Consider optional “strict mode” to also validate state/event names against allowlists (only if provided by user).
5) **Error surfacing**
   - Public APIs (`handle`, `check_timeout`, `execute_actions`, `build`) return `Result<_, FsmError>`.
   - Map handler/timeout failures to `FsmError::HandlerError`; add variants for `UnhandledEvent`, `Timeout`, `DuplicateHandler`, etc.
   - Unhandled events: prefer failing with `FsmError::UnhandledEvent { state, event }` unless the user installs an explicit `when_unhandled` handler that chooses to return `Ok(())`. Keep precedence: specific handler > wildcard > unhandled hook.
   - Wildcard routes: retain support for cross-cutting events, but document best practices and precedence clearly so they do not mask missing state-specific handlers.

## Technical Plan
- **Error plumbing**
  - Update signatures and internal calls to return `FsmError`.
  - Ensure `when_unhandled` failures propagate via `HandlerError`.
- **Initial timeout**
  - In `StateMachine::new`, set `state_timeout` when the initial state has a timeout handler; use `Instant::now() + duration`.
- **Self-transition handling**
  - Implement chosen option (recommended: full hooks + timeout refresh) in `apply_transition`.
  - Ensure stay/goto helpers trigger the same semantics.
- **Builder validation**
  - Track and reject duplicate `(state, event)` inserts at build time with structured errors.
  - Add optional strict mode in the builder (if desired) for validating known state/event names.
- **Docs & tests**
  - Add tests for: initial-state timeout triggering, self-transition hooks + timeout refresh, duplicate handler rejection, unhandled events with `FsmError`.
  - Update README/CHANGELOG to document semantics, migration notes, and error type changes.

## Compatibility & Migration Notes
- Changing timeout/self-transition semantics may require test updates for existing users; document clearly.
- Switching to `FsmError` changes public signatures; callers need to handle the enum instead of `String`.
- Validation errors at build time will fail fast for previously-silent duplicates; highlight in release notes.

## Open Questions (Resolved with preferences)
- **State/event validation strictness:** Default to strict validation (fail fast) since callers already provide `variant_name()`. Allow opt-out via a `strict_validation` flag if needed for highly dynamic cases.
- **Self-transition hooks:** Treat self-transitions as full transitions (run exit→entry hooks and refresh timeout). If a no-hook path is desired, provide an explicit “stay_without_hooks” helper instead of silent special-casing.
- **Telemetry/tracing:** Emit structured tracing events for transitions, timeouts firing, handler failures, and unhandled events to aid production debugging without changing semantics.
- **Feature flags:** None planned; changes are acceptable as breaking changes.

## Test Plan
- **Initial-state timeout scheduling:** Add a test where the initial state has a timeout; verify `check_timeout` fires without any prior transition and advances state/actions. Cover zero/very short durations.
- **Self-transitions:** Tests for (a) self-transition runs exit/entry hooks, (b) timeout is refreshed on self-transition, (c) optional `stay_without_hooks` (if added) skips hooks but still refreshes timeout.
- **Unhandled events & precedence:** Tests that ordering is specific > wildcard > unhandled hook; unhandled without hook returns `FsmError::UnhandledEvent`; unhandled hook errors surface via `HandlerError`.
- **Builder validation:** Duplicate `(state, event)` registration fails with `FsmError::DuplicateHandler`; strict validation catches mismatched `variant_name()` strings when enabled; lax mode (if provided) allows dynamic names.
- **Error plumbing:** Representative path ensures `handle`, `check_timeout`, and `execute_actions` return `FsmError` variants (UnhandledEvent, Timeout, HandlerError mappings).
- **Wildcard routes:** Sanity test that a wildcard handler is invoked when no specific handler exists, and specific handlers override wildcard.
- **Regression sweep:** Rerun existing suites (simple, comprehensive, edge cases, async/race/memory/timeout cancellation, journal subscription) to ensure new semantics don’t regress existing behaviors.

## Effort Estimate (rough)
- Error plumbing to `FsmError`: ~2–3 hours (signatures, mappings, tests/examples).
- Initial-state timeout scheduling: ~30–45 minutes (code + test + doc note).
- Self-transition semantics (hooks + timeout refresh, optional no-hook helper): ~1–1.5 hours (logic + tests).
- Builder validation (duplicates, strict/opt-out flag): ~1–1.5 hours (logic + tests + error variant).
- Tracing additions: ~45–60 minutes (events + minimal assertions where practical).
- Docs/CHANGELOG/README updates: ~30–45 minutes.

## Impact on Consumers
- API change: `Result<_, String>` → `Result<_, FsmError>` requires caller updates to error handling/imports.
- Behavior change: initial-state timeouts now fire; self-transitions run exit/entry hooks and refresh timeouts—tests or logic assuming no-ops/never-timeout will need updates.
- Validation change: duplicate `(state, event)` registrations error at build time; with strict validation on by default, mismatched `variant_name()` strings fail fast.
- Tracing: additive, no breaking change.
- Overall: medium impact—signature updates plus timeout/self-transition/validation behavior shifts may require caller and test adjustments.
- Performance expectations: no material change in complexity; dynamic-dispatch overhead remains as-is. Validation adds minor upfront cost at build time, not per-transition.
