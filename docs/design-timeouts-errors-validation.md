# FSM Timeouts, Errors, and Validation (Merged)

This design has been merged into the broader mutable-context refactor documented in:

- `docs/flowip-fsm-001-mutable-context-timeouts-errors.md`

Please refer to that document—specifically the **“Timeouts, Self-Transitions, Errors, and Validation”** section—for the current, integrated plan covering:

- Initial-state timeout behavior
- Self-transition semantics and timeout refresh
- Structured error plumbing via `FsmError`
- Builder-time validation of duplicate handlers and (optionally) state/event names
- Unhandled event behavior and precedence
- Tracing and observability for transitions, timeouts, and handler failures

This file remains only as a pointer to avoid breaking existing references.
