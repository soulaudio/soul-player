# Husky Git Hooks

This directory contains Git hooks managed by [Husky](https://typicode.github.io/husky/).

## What is Husky?

Husky automatically runs scripts (like linting, formatting, and tests) before Git operations like commits and pushes. This ensures code quality and prevents broken code from being committed.

## Configured Hooks

### `pre-commit`

Runs before each commit. Executes the following checks:

1. **Rust Formatting**: `cargo fmt --all --check`
2. **Rust Linting**: `cargo clippy --workspace --lib --bins --release -- -D warnings`
3. **Rust Tests**: `cargo test --all --quiet`
4. **TypeScript Checks**:
   - Desktop: `yarn workspace soul-player-desktop run tsc --noEmit`
   - Shared: `yarn workspace @soul-player/shared run tsc --noEmit`
   - Marketing: `yarn workspace @soul-player/marketing run tsc --noEmit`
5. **ESLint**:
   - Desktop: `yarn workspace soul-player-desktop run lint`
   - Shared: `yarn workspace @soul-player/shared run lint`

If any check fails, the commit is blocked until you fix the issues.

**Windows Note**: If Rust tests fail due to file locks (running dev server, VS Code rust-analyzer, etc.), the hook will warn but allow the commit. Tests will still run in CI. To avoid this:
- Stop the dev server before committing: `Ctrl+C` in the terminal
- Close VS Code or disable rust-analyzer temporarily
- Or use `git commit --no-verify` for WIP commits

## Setup

Husky is automatically initialized when you run:

```bash
yarn install
```

The `prepare` script in `package.json` ensures Husky is set up correctly.

## Bypassing Hooks

**Not recommended**, but you can bypass pre-commit hooks for WIP commits:

```bash
git commit --no-verify -m "WIP: work in progress"
```

## Manual Testing

You can manually run the same checks without committing:

```bash
# Unix/Linux/macOS
./scripts/pre-commit-check.sh

# Windows (PowerShell)
.\scripts\pre-commit-check.ps1
```

## Troubleshooting

### Hooks not running

1. Ensure Git hooks path is configured:
   ```bash
   git config core.hooksPath
   # Should output: .husky
   ```

2. If not set, run:
   ```bash
   git config core.hooksPath .husky
   ```

3. Verify the hook file is executable:
   ```bash
   # Unix/Linux/macOS
   chmod +x .husky/pre-commit

   # Windows: File should already be executable
   ```

### Updating hooks

Simply edit the hook files in this directory. They are regular shell scripts.

## CI Integration

The same checks run in CI (GitHub Actions). Husky ensures your local commits match CI requirements before pushing.
