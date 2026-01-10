# Soul Player Desktop

Cross-platform desktop music player.

## Run

```bash
# From repository root
yarn dev:desktop

# Or from applications/desktop/
cd applications/desktop
yarn install
yarn tauri:dev
```

## Build

```bash
# From repository root
yarn build:desktop

# Or from applications/desktop/
yarn tauri:build
```

Output: `src-tauri/target/release/bundle/`

## Scripts

From applications/desktop/:
- `yarn dev` - Vite dev server only
- `yarn tauri:dev` - Run Tauri app with HMR
- `yarn tauri:build` - Build production binary
- `yarn type-check` - TypeScript check
- `yarn lint` - ESLint

## Requirements

- Rust 1.75+
- Node 20+

### Windows

LLVM is required for ASIO audio support (low-latency professional audio):

```powershell
# Install LLVM (one-time setup)
choco install llvm -y

# Set environment variable (add to your PowerShell profile for persistence)
$env:LIBCLANG_PATH = "C:\Program Files\LLVM\bin"

# Or set permanently:
[Environment]::SetEnvironmentVariable("LIBCLANG_PATH", "C:\Program Files\LLVM\bin", "User")
```

Alternatively, run the setup script:
```powershell
.\scripts\setup-windows-env.ps1
```

### Linux

```bash
sudo apt-get install -y libasound2-dev libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev pkg-config
```

### macOS

```bash
brew install pkg-config
```
