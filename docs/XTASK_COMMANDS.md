# Xtask Commands Reference

Soul Player uses `cargo xtask` for all development automation tasks. This provides a consistent, cross-platform interface for common development workflows.

## What is Xtask?

Xtask is a Rust convention for creating project-specific automation tools. It provides:
- **Cross-platform compatibility** - Works identically on Windows, macOS, and Linux
- **Type safety** - Written in Rust with compile-time guarantees
- **Consistency** - Same commands across all platforms
- **Integration** - Direct access to Cargo and project internals

## Shorthand

Use `cargo xt` as a shorthand for `cargo xtask`:
```bash
cargo xt check precommit
cargo xt dev desktop
cargo xt version bump 0.2.0
```

---

## Command Reference

### Quality Checks

#### `cargo xtask check precommit`
Runs the full pre-commit pipeline. This is the same set of checks that Husky runs automatically before each commit.

**What it does:**
- Rust formatting check
- Clippy lints
- Rust tests
- TypeScript type checking
- ESLint

**Usage:**
```bash
cargo xtask check precommit
```

**Use this before:**
- Creating a commit (if Husky is disabled)
- Pushing to a branch
- Opening a pull request

---

#### `cargo xtask check fmt [--fix]`
Checks Rust code formatting using `rustfmt`.

**Usage:**
```bash
cargo xtask check fmt          # Check only (CI mode)
cargo xtask check fmt --fix    # Format code
```

**What it checks:**
- Consistent indentation
- Line length limits
- Import ordering
- Code style conventions

---

#### `cargo xtask check clippy [--fix]`
Runs Clippy lints on all Rust code.

**Usage:**
```bash
cargo xtask check clippy       # Check only
cargo xtask check clippy --fix # Auto-fix issues
```

**What it checks:**
- Code quality issues
- Common mistakes
- Performance improvements
- Best practices

---

#### `cargo xtask check test`
Runs all Rust tests.

**Usage:**
```bash
cargo xtask check test
```

**What it runs:**
- Unit tests
- Integration tests
- Doc tests

---

#### `cargo xtask check typescript`
Checks TypeScript types across all workspaces.

**Usage:**
```bash
cargo xtask check typescript
```

**What it checks:**
- Type errors in desktop app
- Type errors in shared components
- Type errors in marketing site

---

#### `cargo xtask check lint [--fix]`
Runs ESLint on all TypeScript/React code.

**Usage:**
```bash
cargo xtask check lint         # Check only
cargo xtask check lint --fix   # Auto-fix issues
```

**What it checks:**
- Code style issues
- React best practices
- Unused imports
- Accessibility issues

---

### Setup Commands

#### `cargo xtask setup all`
Complete first-time setup for development.

**Usage:**
```bash
cargo xtask setup all
```

**What it does:**
1. Installs system dependencies
2. Sets up SQLx database
3. Creates environment files

**Equivalent to:**
```bash
cargo xtask setup deps
cargo xtask setup sqlx
cargo xtask setup env
```

---

#### `cargo xtask setup deps`
Installs system dependencies required for development.

**Usage:**
```bash
cargo xtask setup deps
```

**What it installs:**
- **macOS**: Xcode CLI tools, CMake, pkg-config, SQLite
- **Linux**: Build tools, audio libraries (ALSA), GTK, WebKit, SQLite
- **Windows**: Visual Studio Build Tools, CMake, LLVM/Clang

---

#### `cargo xtask setup sqlx`
Sets up SQLx database for compile-time query verification.

**Usage:**
```bash
cargo xtask setup sqlx
```

**What it does:**
1. Creates `.env` from `.env.example` if needed
2. Installs `sqlx-cli` if not present
3. Creates database at `libraries/soul-storage/.tmp/dev.db`
4. Runs all migrations
5. Prepares offline mode metadata
6. Verifies setup with `cargo check`

---

#### `cargo xtask setup env`
Creates environment files from templates.

**Usage:**
```bash
cargo xtask setup env
```

**What it does:**
- Creates `.env` from `.env.example`
- Creates `applications/desktop/.env` from template
- Preserves existing files (won't overwrite)

---

### Build Commands

#### `cargo xtask build desktop [--release]`
Builds the desktop application.

**Usage:**
```bash
cargo xtask build desktop           # Debug build
cargo xtask build desktop --release # Release build
```

**What it does:**
1. Installs frontend dependencies
2. Builds React frontend with Vite
3. Builds Rust backend
4. Creates platform-specific installer

**Output:**
- Windows: `.exe`, `.msi`
- macOS: `.dmg`, `.app`
- Linux: `.deb`, `.rpm`, `.AppImage`

---

#### `cargo xtask build wasm [--watch]`
Builds WASM modules for the marketing site.

**Usage:**
```bash
cargo xtask build wasm          # Build once
cargo xtask build wasm --watch  # Watch and rebuild
```

**What it does:**
1. Installs `wasm-pack` if needed
2. Compiles Rust to WebAssembly
3. Generates TypeScript bindings
4. Optimizes bundle size

**Output:** `libraries/soul-playback-web/pkg/`

---

#### `cargo xtask build all`
Builds everything (desktop + WASM).

**Usage:**
```bash
cargo xtask build all
```

**Equivalent to:**
```bash
cargo xtask build desktop
cargo xtask build wasm
```

---

### Development Servers

#### `cargo xtask dev desktop`
Starts desktop app development server with hot reload.

**Usage:**
```bash
cargo xtask dev desktop
```

**What it does:**
1. Starts Vite dev server on port 1420
2. Starts Tauri dev mode
3. Watches for file changes
4. Hot reloads on React changes
5. Rebuilds on Rust changes

**Equivalent to:** `yarn dev:desktop`

---

#### `cargo xtask dev marketing`
Starts marketing site development server.

**Usage:**
```bash
cargo xtask dev marketing
```

**What it does:**
1. Builds WASM if needed
2. Starts Vite dev server
3. Watches for changes

**Equivalent to:** `yarn dev:marketing`

---

### Testing Commands

#### `cargo xtask test audio e2e`
Runs audio end-to-end tests.

**Usage:**
```bash
cargo xtask test audio e2e
```

**What it tests:**
- Audio playback functionality
- Device handling
- Format support
- Crossfade and effects

---

#### `cargo xtask test import e2e`
Runs import end-to-end tests.

**Usage:**
```bash
cargo xtask test import e2e
```

**What it tests:**
- Library scanning
- Metadata extraction
- Import workflows
- Database population

---

#### `cargo xtask test cache e2e`
Runs cache invalidation end-to-end tests.

**Usage:**
```bash
cargo xtask test cache e2e
```

**What it tests:**
- React Query cache behavior
- Invalidation triggers
- Data consistency

---

### Cleanup Commands

#### `cargo xtask clean dev`
Removes development build artifacts (fast).

**Usage:**
```bash
cargo xtask clean dev
```

**What it removes:**
- `target/` (Rust build artifacts)
- `dist/` folders (frontend builds)
- Yarn cache
- Vite cache

**Preserves:**
- `node_modules/`
- `.sqlx/` (offline SQLx data)

**Use when:**
- Hot reload stops working
- Build artifacts become stale
- Switching between debug/release modes

---

#### `cargo xtask clean full`
Nuclear clean - removes everything including dependencies.

**Usage:**
```bash
cargo xtask clean full
```

**What it removes:**
- Everything from `clean dev`
- `node_modules/` (requires reinstall)
- All Cargo build artifacts
- All caches

**Use when:**
- Dependency issues persist
- Starting fresh after major changes
- Troubleshooting mysterious build errors

**Warning:** This requires reinstalling dependencies with `yarn install`.

---

### Version Management

#### `cargo xtask version current`
Shows the current version across all project files.

**Usage:**
```bash
cargo xtask version current
```

**Output:**
```
Current version: 0.1.4
```

---

#### `cargo xtask version bump <version> [--dry-run]`
Bumps version across all project files.

**Usage:**
```bash
cargo xtask version bump 0.2.0              # Bump to 0.2.0
cargo xtask version bump 0.2.0 --dry-run    # Preview changes
cargo xtask version bump 1.0.0-beta.1       # Pre-release
```

**What it updates:**
- `Cargo.toml` (workspace root)
- All library `Cargo.toml` files
- All application `Cargo.toml` files
- `package.json` (root and all workspaces)
- `applications/desktop/src-tauri/tauri.conf.json`
- `.github/release-config.json`

**Version format:** Must be valid semver (e.g., `0.1.0`, `1.0.0-alpha.1`)

**After bumping:**
1. Review changes with `git diff`
2. Commit: `git commit -m "chore: bump version to v0.2.0"`
3. Push: `git push origin main`
4. Automation creates git tag and triggers release

---

## Common Workflows

### Starting Development (First Time)
```bash
# 1. Clone and install dependencies
git clone https://github.com/soulaudio/soul-player.git
cd soul-player
corepack enable
yarn install

# 2. Setup development environment
cargo xtask setup all

# 3. Start dev server
cargo xtask dev desktop
```

### Daily Development
```bash
# Start your day
cargo xtask clean dev           # Optional: fresh start
cargo xtask dev desktop         # Start development

# Before committing
cargo xtask check precommit     # Run all checks
git add .
git commit -m "feat: ..."
git push
```

### Creating a Release
```bash
# 1. Bump version
cargo xtask version bump 0.2.0

# 2. Review and commit
git diff
git add -A
git commit -m "chore: bump version to v0.2.0"
git push origin main

# 3. Monitor automation
# https://github.com/soulaudio/soul-player/actions
```

### Troubleshooting Build Issues
```bash
# Level 1: Clean dev artifacts
cargo xtask clean dev
cargo xtask dev desktop

# Level 2: Nuclear clean
cargo xtask clean full
yarn install
cargo xtask setup sqlx
cargo xtask dev desktop

# Level 3: Check system dependencies
cargo xtask setup deps
```

---

## Migration from Old Scripts

If you're used to the old shell scripts, here's the mapping:

| Old Script | New Command | Notes |
|------------|-------------|-------|
| `./scripts/setup-sqlx.sh` | `cargo xtask setup sqlx` | Cross-platform |
| `./scripts/install-deps.sh` | `cargo xtask setup deps` | Auto-detects OS |
| `./scripts/pre-commit-check.sh` | `cargo xtask check precommit` | Single command |
| `./scripts/clean-dev.sh` | `cargo xtask clean dev` | Faster, cross-platform |
| `./scripts/bump-version.sh` | `cargo xtask version bump` | No bash required |
| `./scripts/build-wasm.mjs` | `cargo xtask build wasm` | Pure Rust |
| `yarn dev:desktop` | `cargo xtask dev desktop` | Still works via yarn |

**Old scripts are deprecated but still functional.** We recommend migrating to xtask commands for consistency.

---

## Environment Variables

### `XTASK_LOG_LEVEL`
Controls xtask output verbosity.

```bash
# Unix/Linux/macOS
XTASK_LOG_LEVEL=debug cargo xtask setup sqlx

# Windows PowerShell
$env:XTASK_LOG_LEVEL="debug"; cargo xtask setup sqlx
```

**Levels:** `error`, `warn`, `info` (default), `debug`, `trace`

---

## FAQ

**Q: Why xtask instead of shell scripts?**
A: Cross-platform compatibility, type safety, and consistency. One command works everywhere.

**Q: Can I still use yarn commands?**
A: Yes! `yarn dev:desktop` still works. Xtask wraps common operations for convenience.

**Q: Do I need to install xtask?**
A: No! It's part of the Soul Player workspace. Just run `cargo xtask`.

**Q: What if a command fails?**
A: Check the error output. Most commands provide helpful error messages and suggestions.

**Q: Can I see what a command does before running it?**
A: Use `--dry-run` where available, or check `xtask/src/` for implementation details.

---

## Support

- **Documentation**: See [docs/](../docs/) for detailed guides
- **Issues**: https://github.com/soulaudio/soul-player/issues
- **Discord**: https://discord.gg/pCkTFbY9hC

---

**Last Updated:** 2026-02-11
