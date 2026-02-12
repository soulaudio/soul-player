# ReplayGain Migration - Remaining Work

## Status: Core Implementation Complete ✅

The ReplayGain implementation in `soul-playback` is complete and tested. However, full integration requires additional work in the desktop app and other crates.

## Completed ✅
1. **New ReplayGain module** (`libraries/soul-playback/src/replay_gain.rs`)
   - Simple, patent-free normalization
   - 400 lines, 11 passing tests
   - Zero allocations in audio callback

2. **Simplified AudioPipeline** (`libraries/soul-playback/src/components/audio_pipeline.rs`)
   - Removed LUFS/headroom/limiter complexity
   - Added simple ReplayGain processing
   - 150 lines removed, processing chain simplified

3. **Updated soul-playback library**
   - Removed `soul-loudness` dependency
   - Removed `volume-leveling` feature flag
   - Library compiles and tests pass

## Remaining Work

### Phase 1: Remove soul-loudness Dependency

#### 1.1. Remove from Desktop App (`applications/desktop/src-tauri/Cargo.toml`)
**File**: `applications/desktop/src-tauri/Cargo.toml`
**Action**: Remove line:
```toml
soul-loudness.workspace = true
```

#### 1.2. Remove from Workspace (`Cargo.toml`)
**File**: Root `Cargo.toml`
**Action**:
- Remove from `members = [...]`:
  ```toml
  "libraries/soul-loudness",
  ```
- Remove from `[workspace.dependencies]`:
  ```toml
  soul-loudness = { path = "libraries/soul-loudness" }
  ```
- Remove from `[workspace.dependencies]`:
  ```toml
  ebur128.workspace = true
  ```

#### 1.3. Delete soul-loudness Directory
**Action**: Delete `libraries/soul-loudness/` (or archive for reference)
**Rationale**: Remove patent-encumbered code

### Phase 2: Update PlaybackManager

**File**: `libraries/soul-playback/src/manager.rs`

#### 2.1. Remove Old Volume Leveling Methods
Remove or stub out these methods (check callers first):
- `set_volume_leveling_mode()`
- `set_loudness_preamp()`
- `set_prevent_clipping()`
- Any other loudness-related methods

#### 2.2. Add ReplayGain Methods
Add public methods to PlaybackManager:
```rust
/// Set ReplayGain normalization mode
pub fn set_replay_gain_mode(&mut self, mode: ReplayGainMode) {
    self.pipeline.replay_gain_mut().set_mode(mode);
}

/// Set ReplayGain pre-amp adjustment (-15 to +15 dB)
pub fn set_replay_gain_preamp(&mut self, preamp_db: f32) {
    self.pipeline.replay_gain_mut().set_preamp_db(preamp_db);
}

/// Set whether to prevent clipping
pub fn set_replay_gain_prevent_clipping(&mut self, prevent: bool) {
    self.pipeline.replay_gain_mut().set_prevent_clipping(prevent);
}

/// Get current ReplayGain mode
pub fn replay_gain_mode(&self) -> ReplayGainMode {
    self.pipeline.replay_gain().mode()
}
```

#### 2.3. Update Track Loading
When loading a track, pass ReplayGain values from storage:
```rust
// Read RG values from database (already stored)
let rg_values = ReplayGainValues {
    track_gain_db: track.replaygain_track_gain.map(|v| v as f32),
    track_peak: track.replaygain_track_peak.map(|v| v as f32),
    album_gain_db: track.replaygain_album_gain.map(|v| v as f32),
    album_peak: track.replaygain_album_peak.map(|v| v as f32),
};

// Set in pipeline
self.pipeline.replay_gain_mut().set_track_values(rg_values);
```

### Phase 3: Update Desktop App Tauri Commands

**File**: `applications/desktop/src-tauri/src/loudness.rs`

#### 3.1. Remove Analysis Commands
Delete or comment out:
- `analyze_track()`
- `queue_track_analysis()`
- `queue_all_unanalyzed()`
- `get_analysis_queue_stats()`
- `start_analysis_worker()`
- `stop_analysis_worker()`
- `get_analysis_worker_status()`
- `analyze_audio_file()` helper
- `run_analysis_worker()` helper

#### 3.2. Simplify Settings Commands
Replace complex volume leveling commands with simple RG commands:

```rust
/// Set ReplayGain normalization mode
#[tauri::command]
pub async fn set_replay_gain_mode(
    mode: String, // "off", "track", "album"
    playback: State<'_, LazyPlaybackManager>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let rg_mode = match mode.as_str() {
        "off" => ReplayGainMode::Off,
        "track" => ReplayGainMode::Track,
        "album" => ReplayGainMode::Album,
        _ => return Err(format!("Invalid mode: {}", mode)),
    };

    let pm = playback.get().await?;
    pm.set_replay_gain_mode(rg_mode);

    // Persist to database
    soul_storage::settings::set_setting(
        &state.pool,
        &state.user_id,
        "audio.replay_gain_mode",
        &serde_json::json!(mode),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Set ReplayGain pre-amp adjustment (-15 to +15 dB)
#[tauri::command]
pub async fn set_replay_gain_preamp(
    preamp_db: f32,
    playback: State<'_, LazyPlaybackManager>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let clamped = preamp_db.clamp(-15.0, 15.0);

    let pm = playback.get().await?;
    pm.set_replay_gain_preamp(clamped);

    // Persist
    soul_storage::settings::set_setting(
        &state.pool,
        &state.user_id,
        "audio.replay_gain_preamp",
        &serde_json::json!(clamped),
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
```

#### 3.3. Update Initialization
In `initialize_volume_leveling_mode()`, change to restore ReplayGain settings:
```rust
pub async fn initialize_replay_gain_settings(
    playback: &PlaybackManager,
    app_state: &AppState,
) -> Result<(), String> {
    // Restore mode
    let saved_mode = soul_storage::settings::get_setting(
        &app_state.pool,
        &app_state.user_id,
        "audio.replay_gain_mode",
    )
    .await?;

    if let Some(mode_str) = saved_mode.and_then(|v| v.as_str()) {
        let mode = match mode_str {
            "track" => ReplayGainMode::Track,
            "album" => ReplayGainMode::Album,
            _ => ReplayGainMode::Off,
        };
        playback.set_replay_gain_mode(mode);
    }

    // Restore preamp
    let saved_preamp = soul_storage::settings::get_setting(
        &app_state.pool,
        &app_state.user_id,
        "audio.replay_gain_preamp",
    )
    .await?;

    if let Some(preamp) = saved_preamp.and_then(|v| v.as_f64()) {
        playback.set_replay_gain_preamp(preamp as f32);
    }

    Ok(())
}
```

#### 3.4. Update main.rs
Register new Tauri commands:
```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![
        // ... other commands ...
        set_replay_gain_mode,
        set_replay_gain_preamp,
        // Remove old loudness commands
    ])
```

### Phase 4: Update UI

**Files**: `applications/shared/src/components/settings/audio/`

#### 4.1. Remove Analysis UI
Delete or stub out:
- Volume leveling mode selector (Disabled/RG Track/RG Album/EBU R128/Streaming)
- Analysis queue UI
- Analysis progress/status
- "Analyze Library" button

#### 4.2. Add Simple ReplayGain UI
Create simple UI component:
```tsx
// VolumeLevelingSettings.tsx (simplified)
export function VolumeLevelingSettings() {
  const [mode, setMode] = useState<'off' | 'track' | 'album'>('off');
  const [preamp, setPreamp] = useState(0);

  return (
    <div>
      <h3>ReplayGain Normalization</h3>

      <label>
        Mode:
        <select value={mode} onChange={e => {
          setMode(e.target.value);
          invoke('set_replay_gain_mode', { mode: e.target.value });
        }}>
          <option value="off">Off</option>
          <option value="track">Track (normalize each song)</option>
          <option value="album">Album (preserve album dynamics)</option>
        </select>
      </label>

      <label>
        Pre-amp: {preamp.toFixed(1)} dB
        <input
          type="range"
          min="-15"
          max="15"
          step="0.5"
          value={preamp}
          onChange={e => {
            const val = parseFloat(e.target.value);
            setPreamp(val);
            invoke('set_replay_gain_preamp', { preampDb: val });
          }}
        />
      </label>

      <p className="help-text">
        ReplayGain uses tags from your audio files.
        Files without ReplayGain tags play at normal volume.
        Use foobar2000, Picard, or loudgain to analyze and tag your files.
      </p>
    </div>
  );
}
```

### Phase 5: Update Importer

**File**: `libraries/soul-importer/src/import.rs` (or wherever metadata is read)

#### 5.1. Read ReplayGain Tags During Import
```rust
use lofty::{Probe, TagExt, ItemKey};

// During import, read RG tags
fn extract_replaygain_tags(file_path: &Path) -> Result<(Option<f64>, Option<f64>)> {
    let tagged_file = Probe::open(file_path)?.read()?;

    let mut track_gain = None;
    let mut track_peak = None;

    if let Some(tag) = tagged_file.primary_tag() {
        // Read REPLAYGAIN_TRACK_GAIN
        if let Some(gain_str) = tag.get_string(&ItemKey::ReplayGainTrackGain) {
            // Parse "−5.23 dB" → -5.23
            let trimmed = gain_str.trim().trim_end_matches(" dB").trim_end_matches("dB");
            track_gain = trimmed.parse::<f64>().ok();
        }

        // Read REPLAYGAIN_TRACK_PEAK
        if let Some(peak_str) = tag.get_string(&ItemKey::ReplayGainTrackPeak) {
            track_peak = peak_str.trim().parse::<f64>().ok();
        }
    }

    Ok((track_gain, track_peak))
}
```

#### 5.2. Store in Database
The columns already exist, just populate them:
```rust
// In track insertion
let (rg_gain, rg_peak) = extract_replaygain_tags(&file_path)?;

sqlx::query!(
    "INSERT INTO tracks (..., replaygain_track_gain, replaygain_track_peak)
     VALUES (..., ?, ?)",
    rg_gain,
    rg_peak
).execute(pool).await?;
```

### Phase 6: Testing

#### 6.1. Unit Tests
- ✅ ReplayGain processor tests (done)
- [ ] PlaybackManager RG methods
- [ ] Settings persistence

#### 6.2. Integration Tests
- [ ] Import file with RG tags → tags stored correctly
- [ ] Load track → RG values passed to pipeline
- [ ] Change mode → gain recalculated
- [ ] Restart app → settings restored

#### 6.3. Manual Testing
- [ ] Tag test files with foobar2000/loudgain
- [ ] Import into Soul Player
- [ ] Enable Track mode → volume normalized
- [ ] Enable Album mode → relative dynamics preserved
- [ ] Adjust preamp → volume changes
- [ ] Test with high-gain files → no clipping

### Phase 7: Documentation

#### 7.1. Update CLAUDE.md
Remove LUFS references, document ReplayGain:
```md
### ReplayGain Normalization

Soul Player uses ReplayGain tags for volume normalization. This is simple and patent-free.

**Files**:
- `libraries/soul-playback/src/replay_gain.rs` - Core processor
- `applications/desktop/src-tauri/src/loudness.rs` - Tauri commands

**Modes**:
- Off: No normalization
- Track: Normalize each song independently
- Album: Preserve relative volume within albums

**Implementation**:
ReplayGain is extremely simple:
1. Read gain value from metadata (in dB)
2. Convert to linear: `10^(dB/20)`
3. Multiply samples by linear gain

No analysis needed - values come from file tags!
```

#### 7.2. User Documentation
Create migration guide for users:
- Removed features (LUFS analysis)
- New workflow (external tagging tools)
- Recommended tools (foobar2000, Picard, loudgain)
- FAQ about untagged files

#### 7.3. Developer Documentation
Document the simplified architecture:
- Why we removed LUFS (patents, complexity)
- How ReplayGain works (simple multiply)
- Performance benefits (95% less CPU)

## Priority Order

### High Priority (Blocks Release)
1. ✅ Core ReplayGain implementation
2. Remove soul-loudness dependency
3. Update PlaybackManager
4. Update Tauri commands
5. Basic UI (mode selector + preamp)

### Medium Priority (Polish)
6. Update importer to read RG tags
7. Settings persistence
8. Testing

### Low Priority (Nice to Have)
9. Comprehensive documentation
10. Migration guide for users
11. External tagging tool recommendations

## Estimated Effort

- Phase 1 (Remove dependency): 30 minutes
- Phase 2 (PlaybackManager): 1-2 hours
- Phase 3 (Tauri commands): 2-3 hours
- Phase 4 (UI): 2-3 hours
- Phase 5 (Importer): 1-2 hours
- Phase 6 (Testing): 2-4 hours
- Phase 7 (Documentation): 1-2 hours

**Total**: 10-17 hours of focused work

## Questions to Resolve

1. **What to do with existing LUFS data in database?**
   - Option A: Keep columns, ignore values (backward compatibility)
   - Option B: Add migration to drop columns
   - **Recommendation**: Keep columns (no harm, allows rollback)

2. **Should we provide a built-in tagging tool?**
   - Option A: Bundle loudgain CLI
   - Option B: Direct users to external tools
   - **Recommendation**: External tools (simpler, less maintenance)

3. **What about tracks without RG tags?**
   - Option A: Analyze on-the-fly (contradicts our goal)
   - Option B: Play at normal volume (current behavior)
   - **Recommendation**: Normal volume + clear UI messaging

4. **Should we show effective gain in UI?**
   - Option A: Yes, show "Current gain: -2.3 dB"
   - Option B: No, keep it simple
   - **Recommendation**: Yes, helpful for debugging

## Notes

- The core implementation is solid and tested
- Most remaining work is "glue code" - connecting pieces
- No complex algorithms or DSP work needed
- Biggest risk: Missing callers of removed methods
- Solution: Compile frequently, check for errors

## Success Criteria

✅ Code compiles without errors
✅ All tests pass
✅ RG-tagged files play with normalized volume
✅ Untagged files play at normal volume
✅ Mode switching works
✅ Settings persist across restarts
✅ UI is clear and simple
✅ No patent concerns

## References

- ReplayGain 2.0 Spec: https://wiki.hydrogenaud.io/index.php?title=ReplayGain_2.0_specification
- lofty crate docs: https://docs.rs/lofty/
- foobar2000: https://www.foobar2000.org/
- MusicBrainz Picard: https://picard.musicbrainz.org/
- loudgain CLI: https://github.com/Moonbase59/loudgain
