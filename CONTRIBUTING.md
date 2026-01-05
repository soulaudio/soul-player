# Contributing to Soul Player

## Prerequisites

- Rust 1.75+
- Node.js 16+ (for Tauri desktop)
- Docker (for integration tests)

## Setup

```bash
git clone https://github.com/yourusername/soul-player.git
cd soul-player
cargo build --all
cargo test --all
```

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

All storage operations must include user context for multi-user support. Use trait abstractions for platform-specific code (desktop/server/ESP32). See [ARCHITECTURE.md](docs/ARCHITECTURE.md) for details.

## PR Checklist

- Code compiles without warnings
- Tests pass (`cargo test --all`)
- Formatted (`cargo fmt --all`)
- Linted (`cargo clippy --all-targets --all-features -- -D warnings`)
- Security audit passes (`cargo audit`)
- Documentation updated if API changed

## Security

Do not open public issues for security vulnerabilities. Email security concerns to security@soul-player.dev.

## License

By contributing, you agree that your contributions will be licensed under MIT OR Apache-2.0.
