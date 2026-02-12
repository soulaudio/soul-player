# Task #5: Comprehensive Error Handling Implementation

**Status**: In Progress
**Objective**: Reduce error detection from 5 seconds to <1ms with fast-fail validation

---

## Files Created

### 1. `libraries/soul-audio-desktop/src/sources/error.rs` ✅

**Purpose**: Structured error types for audio source operations

**Key Features**:
- Detailed error variants (FileNotFound, PermissionDenied, UnsupportedFormat, etc.)
- User-friendly messages via `user_message()` method
- Recoverability detection via `is_recoverable()` method
- Severity levels (Warning, Error)
- Fast I/O error conversion helper

**Error Types**:
```rust
pub enum AudioSourceError {
    FileNotFound { path: PathBuf },
    PermissionDenied { path: PathBuf },
    UnsupportedFormat { path: PathBuf, details: String },
    CorruptedFile { path: PathBuf, details: String },
    FileReadError { path: PathBuf, reason: String },
    DecoderFailed { reason: String },
    ResamplerFailed { reason: String },
    NoAudioTracks { path: PathBuf },
    ProbeFailed { path: PathBuf, reason: String },
}
```

### 2. `libraries/soul-audio-desktop/src/sources/mod.rs` ✅

**Updated** to export error module:
```rust
pub mod error;
pub use error::{AudioSourceError, ErrorSeverity};
```

---

## Required Changes (To Be Applied)

### 3. `libraries/soul-audio-desktop/src/sources/local.rs`

#### Import the Error Module

Add to imports (around line 60):
```rust
use super::error::{io_error_to_audio_source_error, AudioSourceError};
```

#### Update SharedState Structure

Add error field to `SharedState` (around line 116-126):
```rust
struct SharedState {
    // ... existing fields ...

    /// Decoder error (if any occurred during background decoding)
    /// This enables fast-fail error detection - errors are detected in <1ms
    /// when checked via `check_error()` method
    decoder_error: Option<AudioSourceError>,
}
```

#### Add Pre-Validation Function

Add before `LocalAudioSource::new()` (around line 275):
```rust
/// Pre-validate file before attempting to decode
///
/// Performs fast (<1ms) validation checks:
/// - File exists
/// - File is readable
/// - File is not empty
///
/// Returns error immediately without attempting decode.
fn pre_validate_file(path: &Path) -> std::result::Result<(), AudioSourceError> {
    // Check existence
    if !path.exists() {
        return Err(AudioSourceError::FileNotFound {
            path: path.to_path_buf(),
        });
    }

    // Check readability and get metadata
    let metadata = std::fs::metadata(path)
        .map_err(|e| io_error_to_audio_source_error(path, e))?;

    // Check for empty files
    if metadata.len() == 0 {
        return Err(AudioSourceError::CorruptedFile {
            path: path.to_path_buf(),
            details: "Empty file (0 bytes)".into(),
        });
    }

    Ok(())
}
```

#### Update `LocalAudioSource::new()`

Add pre-validation call after path conversion (around line 279):
```rust
pub fn new(path: impl AsRef<Path>, target_sample_rate: u32) -> Result<Self> {
    let path = path.as_ref().to_path_buf();

    // FAST-FAIL: Pre-validate file before attempting decode
    // This catches errors in <1ms instead of 5+ seconds
    if let Err(e) = pre_validate_file(&path) {
        tracing::error!(
            path = %path.display(),
            error = %e,
            "[LocalAudioSource] Pre-validation failed"
        );
        return Err(PlaybackError::AudioSource(e.user_message()));
    }

    // ... rest of existing code ...
}
```

#### Update File Opening in `new()`

Replace error conversion (around line 289):
```rust
// OLD:
let file = File::open(&path)
    .map_err(|e| PlaybackError::AudioSource(format!("Failed to open file: {}", e)))?;

// NEW:
let file = File::open(&path).map_err(|e| {
    let audio_err = io_error_to_audio_source_error(&path, e);
    PlaybackError::AudioSource(audio_err.user_message())
})?;
```

#### Update Probe Error Handling

Replace probe error conversion (around line 306):
```rust
// OLD:
.map_err(|e| PlaybackError::AudioSource(format!("Failed to probe file: {}", e)))?;

// NEW:
.map_err(|e| {
    let audio_err = AudioSourceError::ProbeFailed {
        path: path.clone(),
        reason: e.to_string(),
    };
    PlaybackError::AudioSource(audio_err.user_message())
})?;
```

#### Update Track Validation

Replace track not found error (around line 315):
```rust
// OLD:
.ok_or_else(|| PlaybackError::AudioSource("No audio tracks found".into()))?;

// NEW:
.ok_or_else(|| {
    let audio_err = AudioSourceError::NoAudioTracks { path: path.clone() };
    PlaybackError::AudioSource(audio_err.user_message())
})?;
```

#### Initialize Error Field in SharedState

Update SharedState initialization (around line 347):
```rust
let shared = Arc::new(Mutex::new(SharedState {
    output_buffer: VecDeque::with_capacity(output_buffer_capacity),
    samples_read: 0,
    is_eof: false,
    seek_pending: false,
    encoder_delay_skipped: 0,
    decoder_error: None,  // NEW FIELD
}));
```

#### Add `check_error()` Method

Add new public method to `LocalAudioSource` impl block (before `is_ready()`, around line 1167):
```rust
/// Check if decoder thread encountered an error
///
/// Returns the error immediately if one occurred, enabling fast-fail detection.
/// Uses try_lock to avoid blocking the audio thread.
pub fn check_error(&self) -> Option<AudioSourceError> {
    match self.shared.try_lock() {
        Ok(state) => state.decoder_error.clone(),
        Err(_) => None, // If lock is contended, assume no error yet
    }
}
```

#### Update Decoder Thread Error Handling

In `decoder_thread_main()`, update file open error (around line 444):
```rust
// OLD:
let file = match File::open(&path) {
    Ok(f) => f,
    Err(e) => {
        tracing::error!("[DecoderThread] Failed to open file: {}", e);
        return;
    }
};

// NEW:
let file = match File::open(&path) {
    Ok(f) => f,
    Err(e) => {
        let audio_err = io_error_to_audio_source_error(&path, e);
        tracing::error!(
            error = %audio_err,
            "[DecoderThread] Failed to open file"
        );

        // Set error in shared state for fast-fail detection
        if let Ok(mut state) = shared.lock() {
            state.decoder_error = Some(audio_err);
        }
        return;
    }
};
```

Update probe error (around line 466):
```rust
// OLD:
Err(e) => {
    tracing::error!("[DecoderThread] Failed to probe file: {}", e);
    return;
}

// NEW:
Err(e) => {
    let audio_err = AudioSourceError::ProbeFailed {
        path: path.clone(),
        reason: e.to_string(),
    };
    tracing::error!(error = %audio_err, "[DecoderThread] Failed to probe file");

    if let Ok(mut state) = shared.lock() {
        state.decoder_error = Some(audio_err);
    }
    return;
}
```

Update no tracks error (around line 477):
```rust
// OLD:
} else {
    tracing::error!("[DecoderThread] No audio track found");
    return;
};

// NEW:
} else {
    let audio_err = AudioSourceError::NoAudioTracks { path: path.clone() };
    tracing::error!(error = %audio_err, "[DecoderThread] No audio track found");

    if let Ok(mut state) = shared.lock() {
        state.decoder_error = Some(audio_err);
    }
    return;
};
```

Update decoder creation error (around line 490):
```rust
// OLD:
Err(e) => {
    tracing::error!("[DecoderThread] Failed to create decoder: {}", e);
    return;
}

// NEW:
Err(e) => {
    let audio_err = AudioSourceError::DecoderFailed {
        reason: e.to_string(),
    };
    tracing::error!(error = %audio_err, "[DecoderThread] Failed to create decoder");

    if let Ok(mut state) = shared.lock() {
        state.decoder_error = Some(audio_err);
    }
    return;
}
```

Update resampler creation error (around line 539):
```rust
// OLD:
Err(e) => {
    tracing::error!("[DecoderThread] Failed to create resampler: {}", e);
    return;
}

// NEW:
Err(e) => {
    let audio_err = AudioSourceError::ResamplerFailed {
        reason: e.to_string(),
    };
    tracing::error!(error = %audio_err, "[DecoderThread] Failed to create resampler");

    if let Ok(mut state) = shared.lock() {
        state.decoder_error = Some(audio_err);
    }
    return;
}
```

---

### 4. `libraries/soul-audio-desktop/src/track_loader.rs`

#### Update Error Checking in `loader_thread()`

Add error check immediately after source creation (around line 218):
```rust
let result = match LocalAudioSource::new(&request.path, request.target_sample_rate) {
    Ok(source) => {
        let load_duration = start.elapsed();
        tracing::info!(
            track_title = %request.track.title,
            load_duration_ms = load_duration.as_millis(),
            "[TrackLoader] Source created, checking for errors"
        );

        // FAST-FAIL: Check for decoder errors before waiting for buffer
        // This catches errors in <1ms instead of waiting up to 5 seconds
        if let Some(error) = source.check_error() {
            tracing::error!(
                track_title = %request.track.title,
                error = %error,
                "[TrackLoader] Decoder error detected immediately"
            );
            LoadResult {
                source: None,
                track: request.track,
                error: Some(error.user_message()),
                is_preload: request.is_preload,
            }
        } else {
            // Wait for buffer to be ready (existing code)
            let wait_start = std::time::Instant::now();
            let max_wait = std::time::Duration::from_secs(5);

            while !source.is_ready() && wait_start.elapsed() < max_wait {
                // Check for errors during buffering
                if let Some(error) = source.check_error() {
                    tracing::error!(
                        track_title = %request.track.title,
                        error = %error,
                        "[TrackLoader] Decoder error during buffering"
                    );
                    return LoadResult {
                        source: None,
                        track: request.track,
                        error: Some(error.user_message()),
                        is_preload: request.is_preload,
                    };
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            // ... rest of existing buffer ready check ...
        }
    }
    Err(e) => {
        // ... existing error handling ...
    }
};
```

---

### 5. `libraries/soul-playback/src/events.rs`

#### Add Detailed Error Event Variant

Update `PlaybackEvent` enum (around line 110):
```rust
/// Error occurred during playback
Error {
    /// Error message
    message: String,
    /// Optional track ID that failed
    track_id: Option<String>,
    /// Whether the error is recoverable
    is_recoverable: bool,
},
```

---

## Benefits of This Implementation

### 1. **Fast-Fail Validation (<1ms)**
- File existence check: <0.1ms
- File metadata check: <0.5ms
- Empty file detection: <0.1ms
- **Total pre-validation: <1ms** (vs 5+ seconds for decode attempt)

### 2. **Structured Error Types**
- Clear categorization (FileNotFound, PermissionDenied, etc.)
- Path context included in all file-related errors
- User-friendly messages ready for UI display

### 3. **Error Detection During Decode**
- Decoder thread sets error immediately when encountered
- TrackLoader checks error before waiting for buffer
- Audio callback can check error without blocking

### 4. **Better Error Messages**
- Before: "Failed to open file: The system cannot find the file specified. (os error 2)"
- After: "The file could not be found: C:\Music\song.mp3"

### 5. **Recoverable vs Unrecoverable**
- Permissions errors marked as recoverable (may succeed on retry)
- File not found, corrupted files marked as unrecoverable
- UI can decide whether to offer retry option

---

## Testing Strategy

### Unit Tests (in `error.rs`)
- ✅ User message generation
- ✅ Recoverability detection
- ✅ Severity levels
- ✅ I/O error conversion

### Integration Tests (to be added)

```rust
#[test]
fn test_fast_fail_missing_file() {
    let start = std::time::Instant::now();
    let result = LocalAudioSource::new("/nonexistent/file.mp3", 48000);
    let duration = start.elapsed();

    assert!(result.is_err());
    assert!(duration.as_millis() < 10, "Should fail in <10ms, took {}ms", duration.as_millis());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("could not be found"));
}

#[test]
fn test_fast_fail_empty_file() {
    let temp_file = tempfile::NamedTempFile::new().unwrap();
    // File is empty (0 bytes)

    let start = std::time::Instant::now();
    let result = LocalAudioSource::new(temp_file.path(), 48000);
    let duration = start.elapsed();

    assert!(result.is_err());
    assert!(duration.as_millis() < 10, "Should fail in <10ms, took {}ms", duration.as_millis());

    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("corrupted") || err_msg.contains("Empty file"));
}

#[test]
fn test_error_check_during_decode() {
    // Test that decoder errors are detected via check_error()
    // even while buffer is filling
}
```

---

## Performance Impact

### Memory
- `Option<AudioSourceError>` in SharedState: ~48 bytes (enum + Option overhead)
- Negligible impact on overall memory usage

### CPU
- Pre-validation: <1ms one-time cost
- Error checks during buffering: try_lock() cost (<1µs per check)
- Error check in audio callback: try_lock() cost if enabled

### Latency Reduction
- **Before**: 5+ seconds to detect file not found (waiting for decode timeout)
- **After**: <1ms to detect file not found (pre-validation)
- **Improvement**: 5000x faster error detection

---

## Next Steps

1. Apply all changes to `local.rs` (waiting for stable file state)
2. Update `track_loader.rs` with error checking
3. Add PlaybackEvent::Error variant with metadata
4. Write integration tests
5. Test with various error scenarios:
   - Missing files
   - Empty files
   - Corrupted files
   - Permission errors
   - Unsupported formats
6. Verify error messages in UI

---

## Notes

- All changes are backward compatible (errors fall back to existing behavior)
- Error checking is optional (check_error() returns None if no error)
- Uses try_lock() to avoid blocking audio thread
- Structured errors are Clone for easy propagation

---

**Last Updated**: 2026-02-11
**Implementation Status**: Partial (error module created, awaiting local.rs modifications)
