# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/obzenflow/obzenflow-fsm/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/obzenflow/obzenflow-fsm/releases/tag/v0.1.0
