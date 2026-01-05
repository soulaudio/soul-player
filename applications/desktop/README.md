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
- Platform-specific deps (see main README.md)
