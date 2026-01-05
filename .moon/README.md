# Moon Task Orchestration

Soul Player uses [Moon](https://moonrepo.dev) for task orchestration and build management.

## Quick Start

### Install Moon

```bash
# Install Moon globally
curl -fsSL https://moonrepo.dev/install/moon.sh | bash

# Or via npm
npm install -g @moonrepo/cli

# Or via cargo
cargo install moon --locked
```

### Common Commands

```bash
# Run all tests
moon run :test

# Run integration tests
moon run :test-integration

# Lint code
moon run :lint

# Format code
moon run :format

# Security audit
moon run :audit

# Build workspace
moon run :build

# Build release
moon run :build-release
```

## Task Structure

All tasks are defined in `/moon.yml` at the workspace root.

### Build Tasks
- **`:build`** - Build all workspace crates (debug)
- **`:build-release`** - Build all workspace crates (release)
- **`:build-desktop`** - Build desktop application
- **`:build-server`** - Build server application
- **`:build-firmware`** - Build ESP32-S3 DAP firmware

### Test Tasks
- **`:test`** - Run unit and library tests
- **`:test-integration`** - Run integration tests
- **`:test-all`** - Run all tests (unit + integration + doc tests)
- **`:test-coverage`** - Generate code coverage report

### Quality Tasks
- **`:lint`** - Run Clippy linter
- **`:lint-fix`** - Auto-fix Clippy warnings
- **`:format`** - Check code formatting
- **`:format-fix`** - Auto-format code

### Security Tasks
- **`:audit`** - Security vulnerability audit
- **`:audit-fix`** - Auto-fix security issues
- **`:outdated`** - Check for outdated dependencies

### Documentation Tasks
- **`:doc`** - Build documentation
- **`:doc-open`** - Build and open documentation

### Development Tasks
- **`:dev-desktop`** - Run desktop application
- **`:dev-server`** - Run server application
- **`:watch`** - Watch and auto-test on changes

### CI/CD Tasks
- **`:ci-check`** - Run all CI checks (format, lint, test, audit)
- **`:ci-full`** - Full CI pipeline (including release build)

## Workspace Structure

```
soul-player/
├── libraries/           # Rust libraries
│   ├── soul-core/
│   ├── soul-storage/
│   ├── soul-audio/
│   ├── soul-audio-desktop/
│   ├── soul-audio-mobile/
│   ├── soul-audio-embedded/
│   ├── soul-metadata/
│   ├── soul-discovery/
│   └── soul-sync/
├── applications/        # Applications
│   ├── desktop/        # Tauri desktop app
│   ├── server/         # Axum server
│   ├── mobile/         # Tauri mobile app
│   ├── firmware/       # ESP32-S3 firmware
│   └── shared/         # Shared application code
└── .moon/
    ├── workspace.yml   # Moon workspace config
    └── README.md       # This file
```

## Smart Caching

Moon automatically caches task outputs based on file hashes. This means:

- Tasks only run when their inputs change
- Rebuilds are significantly faster
- CI/CD pipelines are optimized

Cache is stored in `.moon/cache/` (gitignored).

### Clear Cache

```bash
moon run :clean-cache
```

## CI Integration

Moon tasks are integrated into GitHub Actions workflows:

### Workflows
- **CI** (`.github/workflows/ci.yml`) - Main CI pipeline
  - Format check → Lint → Test → Security audit → Build
  - Multi-platform (Linux, macOS, Windows)
  - MSRV check (Rust 1.75)
  - ESP32-S3 firmware check

- **Security** (`.github/workflows/security.yml`) - Automated security
  - Daily vulnerability scans
  - Dependency review on PRs
  - Supply chain security checks
  - Outdated dependency reports

- **Docs** (`.github/workflows/docs.yml`) - Documentation
  - Build and deploy to GitHub Pages
  - Check for broken links
  - Validate doc comments

## Task Dependencies

Moon automatically handles task dependencies:

```yaml
test:
  deps: ['~:build']  # Tests depend on successful build
```

When you run `:test`, Moon will:
1. Check if `:build` has run with current inputs
2. Run `:build` if needed (or use cached result)
3. Run `:test`

## Parallel Execution

Moon runs independent tasks in parallel by default:

```bash
# These run in parallel
moon run :lint :format :audit
```

## Project-Specific Tasks

You can also define tasks per-project in `<project>/moon.yml`:

```yaml
# libraries/soul-audio/moon.yml
tasks:
  bench-decoder:
    command: cargo bench --bench decoder
```

Then run with:

```bash
moon run soul-audio:bench-decoder
```

## Environment Variables

Tasks can define environment variables:

```yaml
test-integration:
  env:
    RUST_TEST_THREADS: '4'
```

## Tips

1. **Use Moon for all CI tasks** - Ensures local and CI behave identically
2. **Check task graph** - `moon query tasks --graph`
3. **See what changed** - `moon query touched-files`
4. **Profile tasks** - Add `--profile` to any moon command
5. **Watch mode** - Use `:watch` task for TDD workflow

## Further Reading

- [Moon Documentation](https://moonrepo.dev/docs)
- [Task Configuration](https://moonrepo.dev/docs/config/project)
- [Workspace Configuration](https://moonrepo.dev/docs/config/workspace)
