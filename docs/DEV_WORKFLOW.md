# Development Workflow Guide

Complete guide for developing Soul Player with optimal hot reload and dev experience.

---

## Quick Start

```bash
# First time setup
yarn install
cargo xtask setup all    # Complete setup (deps + sqlx + env)

# Start development
cargo xtask dev desktop  # Or: yarn dev:desktop
```

---

## Common Issues & Solutions

### Hot Reload Not Working

**Symptoms:**
- Changes to React/TypeScript files don't update automatically
- App shows old version after code changes
- Vite dev server starts but changes aren't reflected

**Solution:**
```bash
# Quick cleanup (recommended)
cargo xtask clean dev

# Or use yarn
yarn clean

# Restart dev server
cargo xtask dev desktop  # Or: yarn dev:desktop
```

**What this does:**
1. Stops any running Soul Player or Vite processes
2. Removes stale `dist/` folders
3. Clears Yarn cache
4. Forces fresh builds

### Slow Startup or Stale Builds

**Symptoms:**
- Dev server takes very long to start
- Old code keeps running despite changes
- Build errors that don't make sense

**Solution:**
```bash
# Full cleanup (nuclear option)
cargo xtask clean full

# Or use yarn
yarn clean:full

# This will:
# 1. Clean all build artifacts
# 2. Remove all node_modules
# 3. Reinstall dependencies
```

### Windows-Specific Issues

**File watching not working:**
- Vite now uses polling on Windows for more reliable HMR
- Port 1420 must be available for dev server
- Port 1421 must be available for HMR websocket

**Antivirus interference:**
- Add `target/` and `node_modules/` to exclusions
- Exclude Soul Player executable from real-time scanning

---

## Development Commands

### Desktop App
```bash
cargo xtask dev desktop       # Start desktop app with hot reload
# Or use yarn directly:
yarn dev:desktop              # Start desktop app in dev mode
yarn dev:desktop:logs         # Start with verbose logging

cargo xtask build desktop     # Build production desktop app
# Or: yarn build:desktop
```

### Cleanup
```bash
cargo xtask clean dev         # Remove build artifacts (fast)
cargo xtask clean full        # Remove artifacts + node_modules (slow)
# Or use yarn:
yarn clean
yarn clean:full
```

### Testing
```bash
cargo xtask test audio e2e    # Audio E2E tests
cargo xtask test import e2e   # Import tests
cargo xtask test cache e2e    # Cache tests
cargo test --all              # Run all Rust tests
yarn test                     # Run all frontend tests
```

### Code Quality
```bash
cargo xtask check precommit   # Full pre-commit pipeline (recommended)
cargo xtask check fmt         # Rust formatting
cargo xtask check clippy      # Clippy lints
cargo xtask check test        # Rust tests
cargo xtask check typescript  # TypeScript type checks
cargo xtask check lint        # ESLint
```

---

## How Hot Reload Works

### Frontend (React/TypeScript)

**What gets hot reloaded:**
- React components (instant)
- TypeScript files (instant)
- CSS/styles (instant)
- Assets (instant)

**Powered by:**
- Vite dev server (port 1420)
- HMR websocket (port 1421)
- React Fast Refresh

**Config:** `applications/desktop/vite.config.ts`

### Backend (Rust/Tauri)

**What triggers rebuilds:**
- Changes to `*.rs` files
- Changes to `Cargo.toml`
- Changes to Tauri configuration

**What happens:**
1. `tauri dev` detects file change
2. Cargo rebuilds changed crates
3. App window reloads automatically

**Important:** Rust rebuilds are slower than frontend HMR (10-60 seconds depending on changes)

---

## Dev Server Architecture

```
User runs: yarn dev:desktop
    │
    ├──> Vite Dev Server (port 1420)
    │    ├─ Serves React app
    │    ├─ Hot Module Replacement
    │    └─ Fast Refresh
    │
    └──> Tauri CLI
         ├─ Runs beforeDevCommand (starts Vite)
         ├─ Watches Rust files
         ├─ Builds & runs native app
         └─ Points webview to http://localhost:1420
```

**Critical:** If the `dist/` folder exists, Tauri might serve from it instead of the dev server. This is why `yarn clean` is essential when hot reload breaks.

---

## Optimization Tips

### Faster Rust Builds

1. **Use workspace dependencies** (already configured)
   - Shares compiled dependencies across crates
   - Reduces incremental build time

2. **Limit changed scope**
   - Changes to `libraries/` rebuild multiple crates
   - Changes to `applications/desktop/src-tauri/src/` only rebuild desktop app

3. **Use release profiles for testing only**
   ```bash
   # Dev builds (fast compile, slower runtime)
   cargo build

   # Release builds (slow compile, fast runtime)
   cargo build --release
   ```

### Faster Frontend Builds

1. **Vite warmup** (already configured)
   - Pre-builds commonly used files
   - Reduces initial load time

2. **File watching optimization** (already configured)
   - Ignores `target/`, `.git/`, `src-tauri/`
   - Uses polling on Windows for reliability

3. **Split large components**
   - Smaller files = faster HMR
   - Better for incremental compilation

---

## Troubleshooting Checklist

When dev experience breaks, try these in order:

- [ ] **Level 1:** Restart dev server (`Ctrl+C` then `cargo xtask dev desktop`)
- [ ] **Level 2:** Clean artifacts (`cargo xtask clean dev` then restart)
- [ ] **Level 3:** Check ports (kill processes on 1420/1421)
- [ ] **Level 4:** Full reset (`cargo xtask clean full`)
- [ ] **Level 5:** Check git status for uncommitted changes causing issues
- [ ] **Level 6:** Restart IDE (sometimes TypeScript server gets stuck)

---

## Platform-Specific Notes

### Windows
- Uses PowerShell cleanup script
- File watching uses polling (more reliable but slightly higher CPU)
- Requires WebView2 runtime
- First build after cleanup: ~2-5 minutes

### macOS
- Uses Bash cleanup script
- Native file watching (fsevents)
- First launch requires right-click → Open (Gatekeeper)
- First build after cleanup: ~1-3 minutes

### Linux
- Uses Bash cleanup script
- Native file watching (inotify)
- May need to increase inotify limits for large projects
- First build after cleanup: ~1-3 minutes

---

## Performance Benchmarks

**Initial build (after yarn clean):**
- Frontend: 5-10 seconds
- Rust (dev): 60-120 seconds
- Total: ~2 minutes

**Incremental builds:**
- Frontend HMR: <1 second
- Rust (single file): 5-15 seconds
- Rust (library change): 15-45 seconds

**Hot reload:**
- Component edit → visible: <1 second
- Style edit → visible: <500ms

---

## Best Practices

1. **Use xtask for automation** - `cargo xtask dev desktop` instead of manual commands
2. **Clean before major work** - Start your day with `cargo xtask clean dev`
3. **Watch the terminal** - Vite and Cargo print helpful error messages
4. **Port conflicts?** - Check if another app is using 1420/1421
5. **Commit often** - Makes it easier to track when issues started
6. **Update dependencies regularly** - `yarn upgrade-interactive`
7. **Run pre-commit checks** - `cargo xtask check precommit` before committing

---

## Related Documentation

- [Architecture Overview](./ARCHITECTURE.md)
- [Testing Guide](./TESTING.md)
- [SQLx Setup](./SQLX_SETUP.md)
- [Tauri v2 Dev Docs](https://v2.tauri.app/develop/)
- [Vite HMR API](https://vitejs.dev/guide/api-hmr.html)

---

**Last Updated:** 2026-02-10
