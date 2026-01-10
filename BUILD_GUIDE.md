# Build Guide - Multi-Platform Development

## Quick Start

**Use your existing workflows - they work perfectly!**

```powershell
# Windows PowerShell
yarn dev:desktop    # Run desktop app
yarn dev:marketing  # Run marketing site
```

```bash
# WSL/Linux (for development/testing)
yarn dev:desktop
yarn dev:marketing
```

## Best Practice Setup (RECOMMENDED)

To avoid cross-platform build conflicts when using both WSL and Windows, set platform-specific target directories:

### Windows (PowerShell)

Add to your PowerShell profile (`$PROFILE`):
```powershell
# Soul Player: Use Windows-specific target directory
$env:CARGO_TARGET_DIR = "target-windows"
```

Apply immediately:
```powershell
notepad $PROFILE  # Add the line above
. $PROFILE        # Reload profile
```

### WSL/Linux

Add to your shell profile (`~/.bashrc` or `~/.zshrc`):
```bash
# Soul Player: Use WSL-specific target directory
export CARGO_TARGET_DIR="target-wsl"
```

Apply immediately:
```bash
echo 'export CARGO_TARGET_DIR="target-wsl"' >> ~/.bashrc
source ~/.bashrc
```

### Why This Works

- ✅ **No more conflicts** - WSL and Windows use separate build directories
- ✅ **No `cargo clean` needed** - Build artifacts never clash
- ✅ **Works with `yarn dev:*`** - Your scripts automatically use correct target dir
- ✅ **Official Cargo pattern** - Environment variables override config ([Cargo Book](https://doc.rust-lang.org/cargo/reference/config.html))

## Alternative: Clean Before Switching (If Not Using Env Vars)

If you haven't set `CARGO_TARGET_DIR` and get this error:
```
error[E0461]: couldn't find crate with expected target triple
```

Just run:
```powershell
cargo clean
```

Then continue with your normal workflow.

## Quick Commands

### Desktop App (Windows)
```powershell
cargo clean  # If switching from WSL
cargo build -p soul-audio-desktop
cargo run -p soul-desktop
```

### Marketing Site (WASM)
```bash
cd applications/marketing
yarn install
yarn build
yarn dev
```

### Run Tests
```powershell
# Clean first if switching platforms
cargo clean
cargo test --all
```

## Best Practice

**Golden Rule**: Run `cargo clean` whenever you switch between:
- WSL ↔ Windows PowerShell
- Native ↔ WASM builds
- Different operating systems

This adds ~2 minutes to build time but prevents all cross-compilation errors.
