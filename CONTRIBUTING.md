# Contributing to Soul Player

## Workflow

1. Pick an issue (look for `good first issue` labels)
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make changes following [CONVENTIONS.md](docs/CONVENTIONS.md)
4. Write meaningful tests (see [TESTING.md](docs/TESTING.md) - quality over quantity)
5. Ensure CI passes: `cargo fmt`, `cargo clippy`, `cargo test --all`
6. Commit using Conventional Commits: `feat(audio): add support for X`
7. Create a Pull Request

## Testing Requirements

Write tests for business logic, edge cases, and integration points. Do not write shallow tests for getters/setters or trivial code. Use testcontainers for database integration tests with real SQLite. Target 50-60% coverage with meaningful tests.

## Architecture Guidelines

See [ARCHITECTURE.md](docs/ARCHITECTURE.md), , and [TESTING.md](docs/TESTING.md) for detailed guidelines.

## PR Checklist

- All GitHub CI gates must pass (formatting, linting, tests, security audit)
- Documentation updated if API changed

## Security

Do not open public issues for security vulnerabilities. Email security concerns to sebastian.stupak@pm.me.

## License

By contributing, you agree that your contributions will be licensed under AGPL-3.0.
