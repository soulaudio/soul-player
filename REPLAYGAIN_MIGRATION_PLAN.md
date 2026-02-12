# ReplayGain Migration Plan

## Objective
Remove LUFS analysis (patent concerns with EBU R128) and simplify to ReplayGain tag reading only.

## What to Keep
1. **ReplayGain tag reading** (`libraries/soul-loudness/src/tags.rs`)
   - `read_replaygain_tags()` - reads RG tags from file metadata
   - `ReplayGainTags` struct

2. **Simple gain application** during playback
   - Read RG values from database (populated during import)
   - Apply gain as simple linear multiplier
   - No realtime analysis needed

3. **Database schema** (mostly unchanged)
   - Keep `replaygain_track_gain`, `replaygain_track_peak` columns
   - Keep `replaygain_album_gain`, `replaygain_album_peak` columns
   - Remove or ignore LUFS columns (deprecate, don't delete yet)

## What to Remove
1. **soul-loudness library** - entire crate
   - `analyzer.rs` - LUFS analysis using ebur128
   - `normalizer.rs` - LoudnessNormalizer
   - `limiter.rs` - TruePeakLimiter
   - `headroom.rs` - HeadroomManager
   - `replaygain.rs` - TrackGain/AlbumGain calculations (depends on LUFS)

2. **ebur128 dependency** from all Cargo.toml files

3. **Volume leveling UI** (desktop app)
   - Remove analysis queue UI
   - Remove LUFS mode selection
   - Keep simple ReplayGain On/Off toggle

4. **Tauri commands** for analysis
   - `analyze_track()`
   - `queue_track_analysis()`
   - `start_analysis_worker()`
   - etc.

## Implementation Steps

### Step 1: Create Simple ReplayGain Module in soul-playback
- Add `ReplayGainMode` enum (Off, Track, Album)
- Add `ReplayGain` struct with gain_db and peak fields
- Add simple gain calculation (dB to linear)

### Step 2: Update AudioPipeline
- Remove `#[cfg(feature = "volume-leveling")]` sections
- Remove `loudness_normalizer`, `headroom_manager`, `output_limiter` fields
- Add simple `replay_gain: ReplayGain` field
- Add `apply_replay_gain()` method (10 lines)

### Step 3: Update PlaybackManager
- Remove volume leveling mode setters
- Add `set_replay_gain_mode(ReplayGainMode)`
- Add `set_replay_gain_preamp(f32)` (-15 to +15 dB)
- Update track loading to read RG from database

### Step 4: Update Importer
- Read ReplayGain tags during import (use lofty crate directly)
- Store in database
- Remove LUFS analysis calls

### Step 5: Remove soul-loudness Dependency
- Remove from workspace Cargo.toml
- Remove from soul-playback/Cargo.toml
- Remove from desktop/Cargo.toml
- Remove feature flag `volume-leveling`

### Step 6: Update UI
- Remove loudness analysis UI components
- Add simple ReplayGain toggle (Off/Track/Album)
- Add preamp slider

### Step 7: Remove Tauri Commands
- Remove all loudness.rs commands
- Keep settings persistence for RG mode

## Benefits
- **No patents**: ReplayGain 2.0 is open standard
- **99% reduction**: ~5000 lines → ~200 lines
- **Instant**: No analysis needed, reads from tags
- **Compatible**: Works with all RG-tagged files
- **Simple**: Easy to understand and maintain

## Testing
- Verify RG tags are read during import
- Verify gain is applied correctly during playback
- Test Off/Track/Album modes
- Test preamp adjustment
- Verify no clipping with high gain

## Rollout
1. Implement in feature branch
2. Test with RG-tagged library
3. Document migration for users
4. Release with clear notes about removed features
