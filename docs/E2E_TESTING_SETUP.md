# E2E Testing Setup Guide

## Quick Setup

### 1. Install Prerequisites

```bash
# Install tauri-driver
cargo install tauri-driver

# Verify installation
tauri-driver --version
```

### 2. Build Desktop App for Testing

```bash
# From repository root
yarn build:desktop:e2e
```

### 3. Install E2E Test Dependencies

```bash
cd applications/desktop/e2e-tests
yarn install
```

### 4. Run Tests

```bash
# Audio settings tests
yarn test:audio:e2e

# All E2E tests
yarn test:audio:e2e:all

# DSP effects tests
yarn test:dsp:e2e

# Lazy queue tests (requires test data)
yarn seed-test-data
yarn test:lazy-queue:e2e
```

## Configuration Files

All E2E test configuration is now properly set up:

- Root `Cargo.toml` - Test dependencies added
- `xtask/` - Build automation crate created
- Desktop `Cargo.toml` - WebDriver support added
- `.cargo/config.toml` - Test profile configured
- `tauri.conf.json` - Test mode CLI args added
- `package.json` - Convenient test scripts added
- `.env.example` - Test configuration template

## Available Test Scripts

From repository root:

- `yarn test:audio:e2e` - Audio settings tests only
- `yarn test:audio:e2e:all` - All E2E tests
- `yarn test:audio:e2e:ci` - CI mode (headless)
- `yarn test:dsp:e2e` - DSP effects tests
- `yarn test:lazy-queue:e2e` - Lazy queue tests (auto-seeds data)
- `yarn build:desktop:e2e` - Build app for testing
- `yarn seed-test-data` - Seed test database
- `yarn cleanup-test-data` - Clean test database

## Using xtask for Automation

The `xtask` crate provides build automation:

```bash
# Build app for E2E testing
cargo xtask build-e2e

# Seed test data
cargo xtask seed-test-data

# Clean test data
cargo xtask clean-test-data

# Run E2E tests
cargo xtask run-e2e
```

## Environment Configuration

Copy `.env.example` to `.env` in `applications/desktop/e2e-tests/`:

```bash
cd applications/desktop/e2e-tests
cp .env.example .env
```

Adjust settings as needed for your environment.

## Platform-Specific Notes

### Windows
- Microsoft Edge required (included with Windows 10/11)
- msedgedriver auto-detected

### Linux
- Install webkit2gtk-4.0: `sudo apt install webkit2gtk-4.0`
- May need virtual display for CI: `Xvfb :99 -screen 0 1920x1080x24 &`

### macOS
- WebKit included with Safari
- May need to enable developer mode

## Next Steps

See `applications/desktop/e2e-tests/README.md` for:
- Writing tests
- Test structure
- Helper functions
- Best practices
- Debugging tips

---

**Setup Complete!** All dependencies and configuration are in place.
