# Contributing to Soul Player

## Workflow

1. Pick an issue (look for `good first issue` labels)
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make changes following [CONVENTIONS.md](docs/CONVENTIONS.md)
4. Write meaningful tests (see [TESTING.md](docs/TESTING.md) - quality over quantity)
5. **Run pre-commit checks** (see below - MUST pass before committing)
6. Commit using Conventional Commits: `feat(audio): add support for X`
7. Create a Pull Request

## Pre-Commit Checklist (REQUIRED)

**IMPORTANT**: All checks below MUST pass before committing. CI will fail if any check fails.

### Rust Checks
```bash
# Format check
cargo fmt --all --check

# Lint check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Tests
cargo test --all
```

### TypeScript/Frontend Checks
```bash
# TypeScript type checking
yarn workspace @soul-player/desktop run tsc --noEmit
yarn workspace @soul-player/shared run tsc --noEmit
yarn workspace @soul-player/marketing run tsc --noEmit

# ESLint
yarn workspace @soul-player/desktop run lint
yarn workspace @soul-player/shared run lint
```

### Quick Pre-Commit Script
Run all checks at once using the provided scripts:

**Unix/Linux/macOS:**
```bash
./scripts/pre-commit-check.sh
```

**Windows (PowerShell):**
```powershell
.\scripts\pre-commit-check.ps1
```

**Manual (all platforms):**
```bash
# Rust
cargo fmt --all --check && \
cargo clippy --workspace --lib --bins --release -- -D warnings && \
cargo test --all

# TypeScript + ESLint
yarn workspace @soul-player/desktop run tsc --noEmit && \
yarn workspace @soul-player/shared run tsc --noEmit && \
yarn workspace @soul-player/marketing run tsc --noEmit && \
yarn workspace @soul-player/desktop run lint && \
yarn workspace @soul-player/shared run lint
```

**For AI Agents/Claude Code**: Always run these checks after making code changes and fix any errors before committing. Use the pre-commit scripts for convenience.

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
