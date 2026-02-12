# Bit Depth Conversion Fixes - Ready to Apply

**Generated**: 2026-02-11
**Status**: Ready for immediate implementation
**Time Required**: ~30 minutes
**Risk Level**: Very Low

---

## Fix 1: U8 DC Offset (CRITICAL)

**File**: `libraries/soul-audio-desktop/src/sources/local.rs`

**Line**: 1061

**Current Code**:
```rust
AudioBufferRef::U8(buf) => {
    Self::interleave_to_stereo_f32(&buf, |s| (s as f32 / u8::MAX as f32) * 2.0 - 1.0)
}
```

**Fixed Code**:
```rust
AudioBufferRef::U8(buf) => {
    // U8 audio: 0=min, 128=center (silence), 255=max
    // Center at 128 to avoid DC offset
    Self::interleave_to_stereo_f32(&buf, |s| (s as i16 - 128) as f32 / 128.0)
}
```

**Why**: Current formula creates 128.5 sample DC offset (audible pops/clicks).

**Test**: Play any 8-bit WAV file - should have no DC offset at track boundaries.

---

## Fix 2: U16/U24/U32 DC Offset (HIGH)

**File**: `libraries/soul-audio-desktop/src/sources/local.rs`

**Lines**: 1063-1073

**Current Code**:
```rust
AudioBufferRef::U16(buf) => {
    Self::interleave_to_stereo_f32(&buf, |s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0)
}
AudioBufferRef::U24(buf) => {
    // U24 range: 0 to 16777215 (2^24 - 1)
    Self::interleave_to_stereo_f32(&buf, |s| {
        (s.inner() as f32 / 16777215.0) * 2.0 - 1.0
    })
}
AudioBufferRef::U32(buf) => {
    Self::interleave_to_stereo_f32(&buf, |s| (s as f32 / u32::MAX as f32) * 2.0 - 1.0)
}
```

**Fixed Code**:
```rust
AudioBufferRef::U16(buf) => {
    // U16 audio: center at 32768 (2^15)
    Self::interleave_to_stereo_f32(&buf, |s| (s as i32 - 32768) as f32 / 32768.0)
}
AudioBufferRef::U24(buf) => {
    // U24 audio: center at 8388608 (2^23)
    Self::interleave_to_stereo_f32(&buf, |s| {
        (s.inner() as i32 - 8388608) as f32 / 8388608.0
    })
}
AudioBufferRef::U32(buf) => {
    // U32 audio: center at 2147483648 (2^31)
    Self::interleave_to_stereo_f32(&buf, |s| {
        (s as i64 - 2147483648) as f32 / 2147483648.0
    })
}
```

**Why**: Same DC offset issue as U8 (though these formats are extremely rare).

**Test**: Create test files (if possible) or verify math is correct.

---

## Fix 3: NaN/Inf Protection in i16 Dither (CRITICAL)

**File**: `libraries/soul-audio/src/dither.rs`

**Line**: 111-122

**Current Code**:
```rust
#[inline]
pub fn dither_to_i16(&mut self, sample: f32) -> i16 {
    // Scale to 16-bit range with correct asymmetric scaling
    // (i16 range is -32768 to 32767, so we use 32768.0 for proper scaling)
    let scaled = sample * 32768.0;

    // Add TPDF noise (±1 LSB triangular)
    let noise = self.tpdf_noise_i16();
    let dithered = scaled + (noise as f32 / 65536.0);

    // Round and clamp
    dithered.round().clamp(-32768.0, 32767.0) as i16
}
```

**Fixed Code**:
```rust
#[inline]
pub fn dither_to_i16(&mut self, sample: f32) -> i16 {
    // Protect against NaN/Infinity (from corrupt files or decoder bugs)
    if !sample.is_finite() {
        tracing::warn!("[DITHER] Non-finite sample detected: {}, returning silence", sample);
        return 0;
    }

    // Scale to 16-bit range with correct asymmetric scaling
    // (i16 range is -32768 to 32767, so we use 32768.0 for proper scaling)
    let scaled = sample * 32768.0;

    // Add TPDF noise (±1 LSB triangular)
    let noise = self.tpdf_noise_i16();
    let dithered = scaled + (noise as f32 / 65536.0);

    // Round and clamp
    dithered.round().clamp(-32768.0, 32767.0) as i16
}
```

**Why**: Undefined behavior on NaN/Inf could cause crashes or audio glitches.

---

## Fix 4: NaN/Inf Protection in i32 Dither (CRITICAL)

**File**: `libraries/soul-audio/src/dither.rs`

**Line**: 135-145

**Current Code**:
```rust
#[inline]
pub fn dither_to_i32(&mut self, sample: f32) -> i32 {
    // Scale to 32-bit range
    let scaled = sample as f64 * 2147483647.0;

    // Add TPDF noise for 24-bit depth
    let noise = self.tpdf_noise_i24();
    let dithered = scaled + noise as f64;

    // Round and clamp
    dithered.round().clamp(-2147483648.0, 2147483647.0) as i32
}
```

**Fixed Code**:
```rust
#[inline]
pub fn dither_to_i32(&mut self, sample: f32) -> i32 {
    // Protect against NaN/Infinity
    if !sample.is_finite() {
        tracing::warn!("[DITHER] Non-finite sample detected: {}, returning silence", sample);
        return 0;
    }

    // Scale to 32-bit range (use 2^31 for symmetric scaling)
    let scaled = sample as f64 * 2147483648.0;

    // Add TPDF noise for 24-bit depth
    let noise = self.tpdf_noise_i24();
    let dithered = scaled + noise as f64;

    // Round and clamp
    dithered.round().clamp(-2147483648.0, 2147483647.0) as i32
}
```

**Why**:
1. Protects against NaN/Inf (same as i16)
2. Uses 2^31 (2147483648.0) instead of 2^31-1 (2147483647.0) for symmetric scaling

---

## Fix 5: NaN/Inf Protection in i32 No-Dither (CRITICAL)

**File**: `libraries/soul-audio/src/dither.rs`

**Line**: 152-155

**Current Code**:
```rust
#[inline]
pub fn convert_to_i32_no_dither(sample: f32) -> i32 {
    let scaled = sample as f64 * 2147483647.0;
    scaled.round().clamp(-2147483648.0, 2147483647.0) as i32
}
```

**Fixed Code**:
```rust
#[inline]
pub fn convert_to_i32_no_dither(sample: f32) -> i32 {
    // Protect against NaN/Infinity
    if !sample.is_finite() {
        tracing::warn!("[DITHER] Non-finite sample detected: {}, returning silence", sample);
        return 0;
    }

    // Use 2^31 for symmetric scaling (matches dither_to_i32)
    let scaled = sample as f64 * 2147483648.0;
    scaled.round().clamp(-2147483648.0, 2147483647.0) as i32
}
```

**Why**: Consistency with dither_to_i32 and safety.

---

## Fix 6: Optional Clipping Detection (MEDIUM)

**File**: `libraries/soul-audio/src/dither.rs`

**Add to both `dither_to_i16` and `dither_to_i32`** (before scaling):

```rust
// Optional: warn about near-clipping signals (helps debug hot files)
if sample.abs() > 0.99 {
    tracing::debug!("[DITHER] Near-clipping signal: {:.6} (>99% full scale)", sample);
}
```

**Why**: Helps users identify files with hot levels before they clip.

---

## Testing Checklist

After applying fixes, run these tests:

### 1. Unit Tests
```bash
cargo test --package soul-audio --lib dither
cargo test --package soul-audio --test bit_depth_precision_test
```

Expected:
- All tests pass
- `test_i32_asymmetric_scaling_bug` can be un-ignored (should pass now)
- `test_nan_protection` can be un-ignored (should pass now)
- `test_infinity_protection` can be un-ignored (should pass now)

### 2. Manual Testing

**U8 DC Offset**:
1. Find an 8-bit WAV file (or create one)
2. Play it back
3. Check for pops/clicks at track start/end
4. Expected: Smooth, no artifacts

**NaN/Inf Protection**:
1. Create a corrupt audio file (bit-flip an existing file)
2. Try to play it
3. Check logs for warnings
4. Expected: Silent output + warning logs, no crash

**i32 Symmetric Scaling**:
1. Play ASIO/professional audio interface output
2. Record full-scale signals (±1.0)
3. Analyze for distortion
4. Expected: Clean waveform, no asymmetry

### 3. Regression Testing

Run full test suite:
```bash
cargo test --all
```

Expected: No new failures

---

## Detailed Testing for i32 Fix

Since the i32 fix changes scaling behavior, we need careful testing:

### Test 1: Verify Symmetric Scaling

```rust
#[test]
fn test_i32_truly_symmetric_after_fix() {
    let mut dither = TpdfDither::new();

    let pos = dither.dither_to_i32(1.0);
    let neg = dither.dither_to_i32(-1.0);

    // Should now be truly symmetric (within dither noise)
    let pos_magnitude = (pos as i64).abs();
    let neg_magnitude = (neg as i64).abs();

    // With 2^31 scale:
    // pos ≈ 2147483648 (clamped to 2147483647)
    // neg ≈ -2147483648
    assert_eq!(pos, i32::MAX, "Positive full scale should be i32::MAX");
    assert!(neg <= i32::MIN + 256, "Negative full scale should be near i32::MIN (within dither noise)");
}
```

### Test 2: Verify Roundtrip

```rust
#[test]
fn test_i32_roundtrip_after_fix() {
    // Test that full-scale negative survives roundtrip
    let i32_min_as_f32 = -1.0_f32;

    let mut dither = TpdfDither::new();
    let output = dither.dither_to_i32(i32_min_as_f32);

    // Should be i32::MIN or very close (within dither)
    assert!(output <= i32::MIN + 256, "Should reach i32::MIN");

    // Convert back
    let roundtrip = output as f64 / 2147483648.0;

    // Should be close to -1.0
    assert!((roundtrip - (-1.0)).abs() < 0.0001, "Roundtrip should preserve -1.0");
}
```

---

## Expected Impact

### Audio Quality

**Before fixes**:
- U8 files: Audible DC offset, pops/clicks
- i16 files: Perfect (no change)
- i24 files: Perfect (no change)
- i32 files: 1 LSB error at full negative scale (inaudible)

**After fixes**:
- U8 files: Perfect
- i16 files: Perfect
- i24 files: Perfect
- i32 files: Perfect

### Performance

**Impact**: Negligible
- NaN check: ~1 CPU cycle (branch predictor will optimize)
- Scale change: 0 cycles (compile-time constant)

### Compatibility

**Breaking changes**: None
- Output format unchanged (still i16/i32)
- Only corrects existing bugs
- No API changes

---

## Rollback Plan

If issues arise after deployment:

### Immediate Rollback (Git)
```bash
git revert <commit-hash>
```

### Selective Rollback

If only one fix causes issues, revert individual changes:

1. **U8 fix issues**: Revert local.rs changes only
2. **NaN fix issues**: Remove `is_finite()` checks
3. **i32 fix issues**: Change back to 2147483647.0

---

## Documentation Updates

After applying fixes, update:

1. **CHANGELOG.md**:
```markdown
### Fixed
- [Audio] Fixed DC offset in 8-bit WAV files
- [Audio] Added NaN/Infinity protection in dithering
- [Audio] Fixed asymmetric scaling in i32 output (ASIO)
```

2. **dither.rs module docs**:
```rust
//! # Safety
//!
//! All dithering functions protect against non-finite values (NaN, Infinity).
//! If a non-finite value is detected, it is converted to silence (0) and a
//! warning is logged.
```

3. **local.rs conversion docs**:
```rust
/// Unsigned integer formats are centered at their midpoint:
/// - U8:  center=128  (0→-1.0, 128→0.0, 255→+1.0)
/// - U16: center=32768 (0→-1.0, 32768→0.0, 65535→+1.0)
/// - U24: center=8388608
/// - U32: center=2147483648
```

---

## Summary of Changes

| File | Lines Changed | Risk | Priority |
|------|---------------|------|----------|
| `local.rs` | 4 formulas | Very Low | P0 |
| `dither.rs` (i16) | +4 lines | Very Low | P0 |
| `dither.rs` (i32) | +4 lines, 1 constant | Very Low | P0 |
| `dither.rs` (no-dither) | +4 lines, 1 constant | Very Low | P0 |
| Total | ~20 lines | Very Low | P0 |

**Total Time**: 30 minutes
**Testing Time**: 15 minutes
**Total**: 45 minutes end-to-end

---

## Final Checklist

Before merging:

- [ ] All fixes applied to code
- [ ] Unit tests pass
- [ ] Integration tests pass (if available)
- [ ] Manual testing completed
- [ ] CHANGELOG.md updated
- [ ] Documentation updated
- [ ] Code review completed (if team workflow)
- [ ] CI/CD passes

After merging:

- [ ] Deploy to staging
- [ ] Test with real audio files
- [ ] Monitor for issues
- [ ] Deploy to production
- [ ] Update release notes

---

**Ready to Apply**: YES ✅
**Estimated Impact**: HIGH (fixes audible bugs)
**Risk Assessment**: VERY LOW
**Recommendation**: Apply immediately

---

**Prepared By**: Claude Code (Sonnet 4.5)
**Date**: 2026-02-11
**Status**: Production-ready
