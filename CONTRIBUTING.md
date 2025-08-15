# Contributing to ObzenFlow FSM

Thank you for your interest in contributing to ObzenFlow FSM! We welcome contributions from the community.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/obzenflow-fsm.git`
3. Create a new branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test`
6. Commit your changes: `git commit -am 'Add some feature'`
7. Push to the branch: `git push origin feature/your-feature-name`
8. Submit a pull request

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo

### Building

```bash
cargo build
```

### Testing

Run all tests:
```bash
cargo test
```

Run specific test:
```bash
cargo test test_race_condition
```

Run tests with output:
```bash
cargo test -- --nocapture
```

## Code Style

- Follow standard Rust conventions
- Use `cargo fmt` to format your code
- Use `cargo clippy` to catch common mistakes
- Add documentation comments for public APIs
- Include examples in doc comments where appropriate

## Testing Guidelines

### Test Organization

- Unit tests go in the same file as the code they test (in `mod tests`)
- Integration tests go in the `tests/` directory
- Each test file should focus on a specific aspect of functionality
- Use descriptive test names that explain what is being tested

### Writing Tests

- Test both success and failure cases
- Test edge cases and boundary conditions
- Use property-based testing for complex invariants
- Ensure tests are deterministic (avoid relying on timing unless necessary)

## Documentation

- All public APIs must have documentation comments
- Include examples in documentation where helpful
- Update the README if you add new features
- Update the CHANGELOG.md for notable changes

## Pull Request Process

1. **Before submitting:**
   - Ensure all tests pass
   - Run `cargo fmt` and `cargo clippy`
   - Update documentation as needed
   - Add entries to CHANGELOG.md under "Unreleased"

2. **PR Description:**
   - Clearly describe what the PR does
   - Reference any related issues
   - Include examples of usage if adding new features
   - List any breaking changes

3. **Review Process:**
   - PRs require at least one review before merging
   - Address reviewer feedback promptly
   - Keep PRs focused - one feature/fix per PR

## Types of Contributions

### Bug Reports

- Use the issue tracker to report bugs
- Include a minimal reproducible example
- Describe expected vs actual behavior
- Include environment details (OS, Rust version)

### Feature Requests

- Open an issue to discuss new features first
- Explain the use case and motivation
- Consider implementation complexity

### Code Contributions

We especially welcome:
- Performance improvements
- Additional test coverage
- Documentation improvements
- Bug fixes
- New examples

## Design Principles

When contributing, please keep these design principles in mind:

1. **Async-First**: The FSM is designed for async Rust
2. **Type Safety**: Leverage Rust's type system for compile-time guarantees
3. **Zero-Copy Where Possible**: Minimize allocations and copies
4. **Composability**: Features should compose well with existing functionality
5. **Simplicity**: Keep the API surface small and intuitive

## Questions?

Feel free to open an issue for any questions about contributing.

## License

By contributing, you agree that your contributions will be licensed under the same dual MIT/Apache-2.0 license as the project.