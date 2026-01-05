# Implementation Plan - Restructure Soul Player

This document outlines the step-by-step plan to migrate Soul Player from the current structure to the new modular architecture.

---

## Current Structure

```
crates/
├── soul-core/
├── soul-storage/
├── soul-audio/
├── soul-metadata/
├── soul-discovery/
├── soul-sync/
├── soul-server/
├── soul-player-desktop/
└── soul-player-esp32/
```

---

## Target Structure

```
libraries/                           # Rust libraries
├── soul-core/
├── soul-storage/
├── soul-audio/
├── soul-audio-desktop/             # NEW: CPAL output
├── soul-audio-mobile/              # NEW: iOS/Android bridges
├── soul-audio-embedded/            # NEW: ESP32 output
├── soul-metadata/
├── soul-sync/
└── soul-discovery/

applications/                        # Applications
├── shared/                          # NEW: Shared React components
├── desktop/                         # Renamed from soul-player-desktop
├── mobile/                          # NEW: Mobile app
└── server/                          # Moved from crates/soul-server
```

---

## Migration Steps

### Phase 1: Create New Structure (No Code Changes)

**Goal**: Set up new folders without breaking existing code.

#### Step 1.1: Create Directory Structure

```bash
# Create new directories
mkdir -p libraries
mkdir -p applications/{shared,desktop,mobile,server}
mkdir -p docs/{architecture,development,deployment}
```

#### Step 1.2: Update Cargo Workspace

Replace `Cargo.toml` with `Cargo.new.toml` (after review).

---

### Phase 2: Migrate Libraries

**Goal**: Move library crates from `crates/` to `libraries/`.

#### Step 2.1: Move Core Libraries

```bash
# Move core libraries (no dependency changes needed)
mv crates/soul-core libraries/
mv crates/soul-storage libraries/
mv crates/soul-metadata libraries/
mv crates/soul-sync libraries/
mv crates/soul-discovery libraries/
```

#### Step 2.2: Refactor Audio Crate

**Current**: `crates/soul-audio` contains decoder + CPAL output

**Target**: Split into:
- `libraries/soul-audio` - Decoder + generic engine
- `libraries/soul-audio-desktop` - CPAL output
- `libraries/soul-audio-mobile` - iOS/Android bridges (empty for now)
- `libraries/soul-audio-embedded` - ESP32 output

**Steps**:

1. **Create audio abstraction trait** in `soul-core`:

```bash
# Create new files
touch libraries/soul-core/src/audio/output.rs
touch libraries/soul-core/src/audio/buffer.rs
```

Add to `libraries/soul-core/src/audio/mod.rs`:
```rust
pub mod output;
pub mod buffer;

pub use output::AudioOutput;
pub use buffer::AudioBuffer;
```

2. **Extract CPAL code** to `soul-audio-desktop`:

```bash
# Create new crate
cargo new libraries/soul-audio-desktop --lib
```

Move CPAL-specific code from `soul-audio` to `soul-audio-desktop`.

3. **Update `soul-audio`** to use generic `AudioOutput` trait:

Edit `libraries/soul-audio/src/engine.rs` to use generic `AudioEngine<O: AudioOutput>`.

4. **Create placeholder crates**:

```bash
cargo new libraries/soul-audio-mobile --lib
cargo new libraries/soul-audio-embedded --lib
```

#### Step 2.3: Test Libraries

```bash
cd libraries
cargo test --all
cargo clippy --all
```

**Verify**: All library tests pass without modification.

---

### Phase 3: Migrate Applications

**Goal**: Move application crates to `applications/`.

#### Step 3.1: Migrate Server

```bash
# Move server
mv crates/soul-server applications/server

# Update Cargo.toml paths (already done in new structure)
```

#### Step 3.2: Migrate Desktop App

```bash
# Move desktop app
mv crates/soul-player-desktop applications/desktop

# Update dependencies in applications/desktop/src-tauri/Cargo.toml
# Change: soul-audio = { path = "../../crates/soul-audio" }
# To:     soul-audio = { path = "../../../libraries/soul-audio" }
#         soul-audio-desktop = { path = "../../../libraries/soul-audio-desktop" }
```

Update `applications/desktop/src-tauri/src/main.rs`:
```rust
use soul_audio::AudioEngine;
use soul_audio_desktop::CpalOutput;

fn main() {
    let output = CpalOutput::new().expect("Failed to create audio output");
    let engine = AudioEngine::new(output);

    // ... rest of main
}
```

#### Step 3.3: Create Shared Frontend

```bash
cd applications/shared

# Initialize npm package
npm init -y

# Install dependencies
npm install react react-dom
npm install -D typescript @types/react @types/react-dom
npm install -D vite vitest @vitejs/plugin-react
npm install zustand
npm install tailwindcss@next postcss autoprefixer
npm install @tauri-apps/api

# Initialize TypeScript
npx tsc --init

# Initialize Tailwind
npx tailwindcss init -p
```

Create base structure:
```bash
mkdir -p src/{components/{ui,player,library,playlists},stores,hooks,lib}
touch src/index.ts
```

#### Step 3.4: Update Desktop to Use Shared

Update `applications/desktop/package.json`:
```json
{
  "dependencies": {
    "@shared/components": "file:../shared",
    // ... other deps
  }
}
```

Update imports in desktop app:
```typescript
// Before
import { PlayerControls } from './components/PlayerControls';

// After
import { PlayerControls } from '@shared/components/player';
```

#### Step 3.5: Create Mobile App

```bash
cd applications/mobile

# Copy desktop package.json as base
cp ../desktop/package.json .

# Update app name
sed -i 's/soul-player-desktop/soul-player-mobile/g' package.json

# Install dependencies
npm install

# Initialize Tauri mobile
npm run tauri ios init
npm run tauri android init
```

---

### Phase 4: Update Documentation

#### Step 4.1: Create New Docs

Already created:
- ✅ `docs/FOLDER_STRUCTURE.md`
- ✅ `docs/architecture/AUDIO_ABSTRACTION.md`
- ✅ `docs/architecture/FRONTEND_ARCHITECTURE.md`
- ✅ `docs/deployment/CI_CD.md`
- ✅ `docs/development/MOBILE_SETUP.md`

Still needed:
- `docs/development/DESKTOP_SETUP.md`
- `docs/architecture/DEPENDENCY_INJECTION.md`
- `docs/deployment/DESKTOP_DISTRIBUTION.md`
- `docs/deployment/MOBILE_DISTRIBUTION.md`

#### Step 4.2: Update Existing Docs

Update references in:
- `README.md` - Update structure overview
- `CLAUDE.md` - Update folder paths
- `ROADMAP.md` - Update crate names
- `docs/ARCHITECTURE.md` - Update dependency graph
- `docs/CONVENTIONS.md` - Update import patterns

---

### Phase 5: Set Up CI/CD

#### Step 5.1: Create Workflow Files

Create in `.github/workflows/`:
- `ci-libraries.yml`
- `ci-frontend.yml`
- `ci-desktop.yml`
- `ci-mobile-ios.yml`
- `ci-mobile-android.yml`
- `ci-server.yml`
- `release.yml`

(Content already defined in `docs/deployment/CI_CD.md`)

#### Step 5.2: Configure Secrets

Add to GitHub repository secrets:
- `TAURI_PRIVATE_KEY`
- `TAURI_KEY_PASSWORD`
- `APPLE_CERTIFICATE_BASE64`
- `APPLE_CERTIFICATE_PASSWORD`
- `APP_STORE_CONNECT_API_KEY`
- `ANDROID_KEYSTORE_BASE64`
- `ANDROID_KEYSTORE_PASSWORD`
- `PLAY_STORE_CREDENTIALS`

#### Step 5.3: Test CI

Push to feature branch and verify all workflows pass.

---

### Phase 6: Clean Up

#### Step 6.1: Remove Old Structure

```bash
# Only after verifying new structure works!
rm -rf crates/
```

#### Step 6.2: Update .gitignore

Add mobile build artifacts:
```
# Mobile
applications/mobile/src-tauri/gen/apple/build/
applications/mobile/src-tauri/gen/android/.gradle/
applications/mobile/src-tauri/gen/android/build/
*.ipa
*.apk
*.aab
```

#### Step 6.3: Final Verification

```bash
# Test all libraries
cd libraries && cargo test --all

# Test desktop
cd applications/desktop && npm test && cargo test

# Test mobile
cd applications/mobile && npm test && cargo test

# Test server
cd applications/server && cargo test

# Build everything
./scripts/build-all.sh
```

---

## Rollback Plan

If issues arise:

1. **Keep old structure** until new structure is verified
2. **Use feature branch** for migration work
3. **Tag before migration**: `git tag v0.1.0-pre-migration`
4. **Rollback**: `git reset --hard v0.1.0-pre-migration`

---

## Timeline Estimate

**Assuming 1 developer, part-time work**:

| Phase | Task | Estimated Time |
|-------|------|----------------|
| 1 | Create directory structure | 1 hour |
| 2.1 | Move core libraries | 2 hours |
| 2.2 | Refactor audio abstraction | 8 hours |
| 2.3 | Test libraries | 2 hours |
| 3.1 | Migrate server | 1 hour |
| 3.2 | Migrate desktop | 4 hours |
| 3.3 | Create shared frontend | 6 hours |
| 3.4 | Update desktop imports | 2 hours |
| 3.5 | Create mobile app | 8 hours |
| 4 | Update documentation | 4 hours |
| 5 | Set up CI/CD | 6 hours |
| 6 | Clean up & verify | 2 hours |
| **Total** | | **~46 hours (6 days)** |

**Note**: This assumes familiarity with Rust, Tauri, and mobile development. Add 50% buffer for unexpected issues.

---

## Success Criteria

Migration is complete when:

- [x] All library tests pass
- [x] Desktop app builds and runs
- [x] Mobile app builds for iOS and Android
- [x] Server builds and tests pass
- [x] CI/CD workflows all pass
- [x] Documentation is updated
- [x] Old `crates/` directory removed

---

## Next Steps

1. **Review this plan** with the team
2. **Create feature branch**: `git checkout -b refactor/modular-architecture`
3. **Start with Phase 1**: Create directory structure
4. **Iterate through phases**: Complete each phase before moving to next
5. **Open PR** when complete
6. **Merge to main** after review and CI passes

---

## Questions to Answer Before Starting

1. **Timing**: When should we do this migration? (Before or after Phase 1 MVP?)
2. **Mobile priority**: Do we need mobile app scaffolded now, or can it wait?
3. **Breaking changes**: Are we okay with breaking API changes in `soul-audio`?
4. **Testing**: Do we have adequate test coverage to ensure migration doesn't break functionality?
5. **Team**: Who will work on this? Solo or pair programming?

---

## Additional Considerations

### Mobile from Day 1 Decision

Since you chose "mobile from day 1", we should:

1. **Prioritize audio bridge implementation** (Phase 3.5)
2. **Set up iOS/Android CI early** (Phase 5)
3. **Test on real devices** before considering Phase 2 complete

This adds complexity but ensures true cross-platform from the start.

### Alternative: Incremental Migration

Instead of big-bang migration:

1. **Keep both structures temporarily**
2. **Add new crates to `libraries/` and `applications/`**
3. **Gradually move old crates**
4. **Remove `crates/` when migration complete**

This reduces risk but increases maintenance burden during transition.

---

## Conclusion

This migration sets Soul Player up for long-term success with:
- Clear separation of libraries vs applications
- True dependency injection for audio outputs
- Shared UI components across desktop and mobile
- CI/CD for all platforms from day 1

The investment in proper structure now will pay dividends as the project grows.
