# WASM Build Integration

## Overview

The marketing demo uses **automatic WASM building** to ensure the latest Rust changes are always reflected. WASM modules are built automatically before development and production builds.

## How It Works

### Automatic Builds

WASM builds are triggered automatically via npm lifecycle hooks:

```json
{
  "scripts": {
    "predev": "node ../../scripts/build-wasm.mjs",    // ← Before yarn dev
    "dev": "next dev -p 3001",
    "prebuild": "node ../../scripts/build-wasm.mjs",  // ← Before yarn build
    "build": "next build"
  }
}
```

**What happens:**
1. You run `yarn dev` or `yarn build`
2. npm automatically runs the `predev` or `prebuild` script first
3. `build-wasm.mjs` compiles the Rust code to WASM
4. Output is placed in `src/wasm/soul-playback/`
5. Next.js starts with the fresh WASM module

### Build Scripts

#### 1. `scripts/build-wasm.mjs` - Cross-Platform Build

**Features:**
- ✅ Works on Windows, macOS, Linux (pure Node.js, no bash)
- ✅ Checks for `wasm-pack` installation
- ✅ Colored terminal output
- ✅ Proper error handling

**Source:** `libraries/soul-playback/src/`
**Output:** `applications/marketing/src/wasm/soul-playback/`
**Target:** web (ES modules)
**Mode:** release (optimized)

#### 2. `scripts/watch-wasm.mjs` - Development Watcher (Optional)

**Features:**
- 👀 Watches Rust source files (`*.rs`)
- 🔄 Auto-rebuilds WASM on changes
- ⚡ Debounced builds (prevents spam)
- 🎨 Live feedback during development

**Usage:**
```bash
# In one terminal
cd applications/marketing
yarn dev:wasm-watch

# In another terminal
yarn dev
```

## Usage

### Standard Development (Recommended)

```bash
cd applications/marketing
yarn dev
```

**What happens:**
1. WASM builds automatically (takes ~10-30 seconds first time)
2. Next.js dev server starts
3. Open http://localhost:3001

**Note:** WASM only rebuilds when you run `yarn dev` again. For live Rust changes during development, use watch mode below.

### With WASM Auto-Rebuild (Advanced)

If you're actively developing Rust code:

```bash
# Terminal 1: Watch and rebuild WASM on Rust changes
yarn dev:wasm-watch

# Terminal 2: Run Next.js dev server
yarn dev
```

**Benefits:**
- Edit Rust files in `libraries/soul-playback/src/`
- WASM auto-rebuilds on save
- Refresh browser to see changes

### Production Build

```bash
cd applications/marketing
yarn build
```

WASM builds automatically before the Next.js build.

### Manual WASM Build

```bash
cd applications/marketing
yarn build:wasm
```

Useful for:
- Verifying WASM builds without starting dev server
- CI/CD pipelines
- Troubleshooting

## Prerequisites

### Install wasm-pack

```bash
# Using cargo
cargo install wasm-pack

# Or using the installer
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
```

**Verify installation:**
```bash
wasm-pack --version
# Should print: wasm-pack 0.x.x
```

## File Structure

```
soul-player/
├── scripts/
│   ├── build-wasm.mjs        # Cross-platform build script
│   ├── watch-wasm.mjs        # Development watcher (optional)
│   └── build-wasm.sh         # Legacy bash script (deprecated)
├── libraries/
│   └── soul-playback/
│       ├── src/              # Rust source code
│       │   ├── wasm/         # WASM-specific code
│       │   │   └── manager.rs # ← Fixed play() event bug here
│       │   └── ...
│       └── Cargo.toml        # Rust dependencies
└── applications/
    └── marketing/
        ├── src/
        │   └── wasm/
        │       └── soul-playback/  # ← WASM output (auto-generated)
        │           ├── soul_playback.js
        │           ├── soul_playback_bg.wasm
        │           └── ...
        └── package.json        # predev/prebuild hooks
```

## Troubleshooting

### Error: "wasm-pack is not installed"

**Solution:**
```bash
cargo install wasm-pack
```

### Error: "Source directory not found"

**Solution:**
Ensure you're running from the correct directory:
```bash
cd applications/marketing
yarn dev
```

### WASM build succeeds but changes don't appear

**Solution:**
1. Stop the dev server (`Ctrl+C`)
2. Clear Next.js cache: `rm -rf .next`
3. Restart: `yarn dev`

### Watch mode not detecting changes

**Solution:**
- Ensure you're editing files in `libraries/soul-playback/src/`
- Check file extensions are `.rs`
- Try manual build: `yarn build:wasm`

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Build Marketing Site

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install wasm-pack
        run: cargo install wasm-pack

      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'

      - name: Install dependencies
        run: yarn install

      - name: Build marketing site
        working-directory: applications/marketing
        run: yarn build  # WASM builds automatically via prebuild hook
```

## Performance Notes

### Build Times

- **First WASM build:** ~10-30 seconds (depends on machine)
- **Incremental builds:** ~5-15 seconds
- **Next.js build:** ~30-60 seconds

**Total `yarn dev` startup:** ~40-90 seconds first time, ~5-15 seconds subsequent runs (WASM cached)

### Optimization

The build uses `--release` mode by default for optimal WASM performance:
- Smaller bundle size
- Faster runtime performance
- Longer build time (acceptable for automatic builds)

For faster dev builds during active Rust development, you could modify the script to use `--dev` mode, but this is not recommended for the marketing demo.

## Comparison: Old vs New

### Before (Manual)

```bash
# Every time you changed Rust code:
cd libraries/soul-playback
wasm-pack build --target web --out-dir ../../applications/marketing/src/wasm/soul-playback
cd ../../applications/marketing
yarn dev
```

❌ Easy to forget
❌ Different commands on Windows/Unix
❌ No verification checks
❌ Manual process prone to errors

### After (Automatic)

```bash
cd applications/marketing
yarn dev  # That's it!
```

✅ Always up-to-date
✅ Cross-platform
✅ Automatic verification
✅ Seamless DevX
✅ Works in CI/CD

## Additional Scripts

All available scripts in `package.json`:

```bash
yarn dev              # Start dev server (auto-builds WASM)
yarn dev:wasm-watch   # Watch Rust files and rebuild WASM
yarn build            # Production build (auto-builds WASM)
yarn build:wasm       # Manually build WASM only
yarn start            # Start production server
yarn lint             # Lint TypeScript
yarn type-check       # Check types
```

## Questions?

- WASM build issues: Check `scripts/build-wasm.mjs`
- Rust compilation errors: Check `libraries/soul-playback/Cargo.toml` dependencies
- Output issues: Verify `applications/marketing/src/wasm/soul-playback/` exists
- Event system issues: See `WASM_PLAYBACK_FIX.md`
