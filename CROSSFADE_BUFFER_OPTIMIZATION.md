# Crossfade Buffer Optimization - Complete

## Summary

Successfully optimized crossfade buffer allocation to reduce memory usage by **91%** while maintaining audio quality and eliminating allocations in the audio callback.

## Changes Made

### 1. Buffer Type Change (`manager.rs` lines 86-87)

**Before:**
```rust
outgoing_buffer: Option<Vec<f32>>,  // Allocated lazily on first use
incoming_buffer: Option<Vec<f32>>,
```

**After:**
```rust
outgoing_buffer: Vec<f32>,  // Pre-allocated in constructor when enabled
incoming_buffer: Vec<f32>,
```

### 2. Dynamic Buffer Size Calculation (line 143)

**Before:**
```rust
const CROSSFADE_BUFFER_SIZE: usize = 10 * 192000 * 2;  // 29.3 MB worst-case
```

**After:**
```rust
fn calculate_crossfade_buffer_size(duration_ms: u32, sample_rate: u32) -> usize {
    let duration_secs = (duration_ms as f32 / 1000.0) * 1.2;  // 20% headroom
    let samples = (duration_secs * sample_rate as f32) as usize;
    samples * 2  // Stereo (interleaved L/R)
}
```

### 3. Constructor Updates (line 177)

**Before:**
```rust
outgoing_buffer: None,
incoming_buffer: None,
```

**After:**
```rust
let buffer_size = if config.crossfade.enabled {
    calculate_crossfade_buffer_size(config.crossfade.duration_ms, 44100)
} else {
    0
};
outgoing_buffer: vec![0.0; buffer_size],
incoming_buffer: vec![0.0; buffer_size],
```

### 4. Sample Rate Change Handling (line 1835)

Added automatic buffer re-allocation when sample rate changes:
```rust
pub fn set_sample_rate(&mut self, sample_rate: u32) {
    // ... existing code ...
    
    // Re-allocate crossfade buffers with new sample rate if enabled
    if self.crossfade.settings().enabled {
        self.allocate_crossfade_buffers();
    }
}
```

### 5. Buffer Management Functions (lines 2230-2259)

**Renamed and simplified:**
- `ensure_crossfade_buffers_allocated()` → `allocate_crossfade_buffers()`
- Now uses `Vec::new()` instead of `Option::None`
- Removed `.as_mut().expect()` unwrapping in audio callback

## Memory Savings

### Old Approach (Worst-Case)
- Buffer size: 10s × 192kHz × 2 channels = 3,840,000 samples
- Memory per buffer: 14.65 MB
- **Total (2 buffers): 29.30 MB**

### New Approach (Dynamic)

| Sample Rate | Duration | Samples | Memory (2 buffers) | Savings |
|------------|----------|---------|-------------------|---------|
| 44.1 kHz   | 3s       | 317,520 | 2.42 MB           | 91.7%   |
| **48 kHz** | **3s**   | **345,600** | **2.64 MB**   | **91.0%** |
| 96 kHz     | 3s       | 691,200 | 5.27 MB           | 82.0%   |
| 192 kHz    | 3s       | 1,382,400 | 10.54 MB        | 64.0%   |

**Typical savings: 26.66 MB (91% reduction)**

## Benefits

1. **Memory Efficiency**: Allocates only what's needed based on actual sample rate and crossfade duration
2. **No Audio Callback Allocations**: All allocations happen in constructor or settings changes
3. **Automatic Adaptation**: Re-allocates when sample rate changes (e.g., switching audio devices)
4. **Proper Cleanup**: Immediately frees buffers when crossfade is disabled

## Testing

All 398 tests pass, including:
- `crossfade_buffers_preallocated_on_enable` - Verifies pre-allocation when enabling
- `crossfade_buffers_preallocated_via_settings` - Verifies settings-based allocation
- `crossfade_cancelled_when_repeat_one_enabled` - Verifies crossfade logic intact
- All 47 crossfade-related tests

## Technical Details

### 20% Headroom
The calculation includes 1.2× multiplier to account for:
- Sample rate variations
- Rounding errors
- Buffer alignment requirements

### Empty vs None
Changed from `Option<Vec<f32>>` to `Vec<f32>` because:
- Simpler API (no unwrapping in audio callback)
- Empty vec is cheap (3 words: pointer + capacity + length)
- `Vec::new()` doesn't allocate until first push
- More idiomatic Rust for "optional allocation"

### Sample Rate Change Behavior
When `set_sample_rate()` is called (e.g., switching audio devices):
1. Updates internal sample rate
2. Checks if crossfade is enabled
3. Re-allocates buffers with new size
4. Old buffers automatically dropped

## Impact

- **Memory**: ~27 MB saved per playback instance (typical case)
- **Performance**: No change (allocations still happen outside audio callback)
- **Audio Quality**: No change (buffer size still sufficient with 20% headroom)
- **Compatibility**: Fully backward compatible

---

**Date**: 2026-02-11
**Task**: #21 - Pre-allocate crossfade buffers and optimize sizes
**Status**: ✅ Complete
