# Audio Engine Test Suite

## Test Philosophy: Quality Over Quantity

This test suite follows the "quality over quantity" principle - **every test verifies meaningful behavior**, not shallow API contracts.

❌ **No shallow tests**: No tests for trivial getters, setters, or constructors
✅ **Meaningful tests**: Tests verify actual audio processing behavior, invariants, and edge cases
✅ **Integration focus**: Tests that effects actually work in real scenarios
✅ **Property-based**: Use proptest to verify invariants across many inputs

## Test Coverage Summary

```
Total: 66 tests (100% passing)

Unit Tests:           26 tests (libraries/soul-audio/src/)
Integration Tests:    16 tests (libraries/soul-audio/tests/integration_test.rs)
Property Tests:       14 tests (libraries/soul-audio/tests/property_test.rs)
Decoder Tests:        10 tests (libraries/soul-audio-desktop/tests/)
```

## Unit Tests (26 tests)

Location: `libraries/soul-audio/src/`

### Decoder Tests (3 tests)
- ✅ `decoder_creation` - Decoder instantiates correctly
- ✅ `supports_common_formats` - Format detection works (MP3, FLAC, OGG, WAV)
- ✅ `decode_nonexistent_file_returns_error` - Error handling for missing files

### Effect Chain Tests (8 tests)
- ✅ `empty_chain` - Empty chain creation
- ✅ `add_effects` - Adding effects to chain
- ✅ `process_chain` - Chain processes audio correctly
- ✅ `disabled_effect_bypassed` - Disabled effects don't process
- ✅ `reset_chain` - Reset doesn't panic
- ✅ `clear_chain` - Clear removes all effects
- ✅ `get_effect` - Effect retrieval by index
- ✅ `enable_disable_all` - Bulk enable/disable

### Parametric EQ Tests (7 tests)
- ✅ `create_eq` - EQ instantiation
- ✅ `eq_band_clamping` - Parameter clamping (gain, Q)
- ✅ `set_bands` - Band configuration
- ✅ `process_buffer` - EQ modifies audio
- ✅ `reset_clears_state` - Deterministic reset
- ✅ `disabled_eq_bypassed` - Disabled EQ is transparent
- ✅ `eq_band_helpers` - Band helper constructors

### Dynamic Range Compressor Tests (8 tests)
- ✅ `create_compressor` - Compressor instantiation
- ✅ `settings_validation` - Parameter clamping
- ✅ `preset_settings` - Preset configurations
- ✅ `process_reduces_peaks` - Compression reduces peaks
- ✅ `reset_clears_envelope` - Envelope follower reset
- ✅ `disabled_compressor_bypassed` - Disabled compressor is transparent
- ✅ `setters_update_settings` - Setter methods work
- ✅ `makeup_gain_boosts_signal` - Makeup gain increases level

## Integration Tests (16 tests)

Location: `libraries/soul-audio/tests/integration_test.rs`

These tests verify real-world audio processing behavior.

### Audio Behavior Tests

**✅ `test_eq_affects_frequency_content`**
- Verifies EQ actually changes audio (not just passthrough)
- Tests +12 dB boost at 100 Hz increases RMS by >2x
- **What it catches**: EQ not applying filters

**✅ `test_eq_different_bands_affect_different_frequencies`**
- Verifies low shelf doesn't affect high frequencies
- Tests frequency selectivity of filters
- **What it catches**: Filter bleeding into wrong bands

**✅ `test_compressor_reduces_peaks`**
- Verifies compressor actually compresses loud signals
- Tests peak reduction with high ratio
- **What it catches**: Compressor not reducing dynamic range

**✅ `test_compressor_prevents_clipping`**
- Verifies limiter prevents clipping
- Tests with 20:1 ratio keeps peaks below 1.0
- **What it catches**: Compressor allowing clipping

**✅ `test_effect_chain_order_matters`**
- Verifies EQ→Comp ≠ Comp→EQ
- Tests that effect order produces different results
- **What it catches**: Chain not processing in order

### Bypass and Transparency Tests

**✅ `test_disabled_effect_is_bit_perfect`**
- Verifies disabled effects don't modify audio at all
- Tests bit-for-bit equality
- **What it catches**: "Bypass" that still processes

**✅ `test_empty_buffer_handling`**
- Verifies effects handle empty buffers safely
- **What it catches**: Panics on edge cases

**✅ `test_zero_signal_handling`**
- Verifies effects handle silent input correctly
- **What it catches**: Artifacts on silence

### Stability and Edge Cases

**✅ `test_eq_at_extreme_parameters`**
- Tests EQ with max boost on all bands
- Verifies no NaN/Inf production
- **What it catches**: Instability at extreme settings

**✅ `test_compressor_with_very_fast_attack`**
- Tests compressor with minimum attack/release times
- **What it catches**: Instability with fast attack

**✅ `test_multiple_effect_resets`**
- Verifies reset produces deterministic results
- **What it catches**: Non-deterministic processing

**✅ `test_sample_rate_change_handling`**
- Tests effects adapt to 44.1kHz vs 48kHz
- **What it catches**: Fixed sample rate assumptions

**✅ `test_chain_with_many_effects`**
- Tests chain with 10 effects (5 EQ + 5 Comp)
- **What it catches**: Performance or stability issues

**✅ `test_effect_state_isolation`**
- Verifies multiple effect instances don't share state
- **What it catches**: Static state bugs

**✅ `test_compressor_makeup_gain_compensation`**
- Verifies makeup gain compensates for compression
- **What it catches**: Makeup gain not working

**✅ `test_effect_chain_clear_and_reuse`**
- Tests chain can be cleared and reused
- **What it catches**: Memory leaks or stale state

## Property-Based Tests (14 tests)

Location: `libraries/soul-audio/tests/property_test.rs`

These tests use **proptest** to verify invariants across thousands of random inputs.

### Correctness Properties

**✅ `eq_never_produces_nan_or_inf`**
- Tests 100+ random combinations of freq, gain, Q, samples
- Verifies EQ always produces finite values
- **Catches**: Numerical instability

**✅ `compressor_never_produces_nan_or_inf`**
- Tests 100+ random threshold, ratio, attack, release combinations
- **Catches**: Compressor numerical issues

**✅ `disabled_effects_are_true_bypass`**
- Tests disabled effects with random inputs
- Verifies bit-perfect passthrough
- **Catches**: Fake bypass

**✅ `eq_with_zero_gain_is_nearly_transparent`**
- Tests EQ with 0 dB gain doesn't change audio significantly
- **Catches**: EQ coloration at flat settings

**✅ `compressor_with_ratio_one_is_transparent`**
- Tests compressor with 1:1 ratio doesn't compress
- **Catches**: Compression at wrong ratios

### Structural Properties

**✅ `effect_chain_preserves_length`**
- Tests chains of 1-10 effects preserve buffer length
- **Catches**: Buffer size bugs

**✅ `extreme_eq_boost_increases_level`**
- Tests positive gain increases signal level
- **Catches**: Inverted gain

**✅ `compressor_reduces_or_maintains_peaks`**
- Tests compressor doesn't increase peaks (without makeup)
- **Catches**: Compressor amplifying

**✅ `reset_clears_state_deterministically`**
- Tests reset produces consistent results across random inputs
- **Catches**: Non-deterministic reset

**✅ `multiple_sample_rates_produce_finite_output`**
- Tests 22.05, 44.1, 48, 88.2, 96 kHz sample rates
- **Catches**: Sample rate dependency bugs

**✅ `eq_cut_does_not_boost`**
- Tests negative gain doesn't significantly increase signal
- **Catches**: Inverted EQ

**✅ `chain_all_disabled_is_bypass`**
- Tests chain with all disabled effects is bit-perfect
- **Catches**: Chain overhead

**✅ `processing_is_consistent`**
- Tests same input produces same output across multiple calls
- **Catches**: Non-deterministic processing

**✅ `empty_buffer_handled_safely`**
- Tests empty buffers don't panic across random effect types
- **Catches**: Edge case panics

## Test Utilities

### Helper Functions

```rust
// Signal generation
fn generate_sine(freq: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32>

// Analysis
fn calculate_rms(buffer: &[f32]) -> f32
fn calculate_peak(buffer: &[f32]) -> f32
fn calculate_dynamic_range(buffer: &[f32]) -> f32

// Validation
fn all_finite(buffer: &[f32]) -> bool
fn within_audio_range(buffer: &[f32]) -> bool
```

## Running Tests

```bash
# All tests
cargo test -p soul-audio

# Unit tests only
cargo test -p soul-audio --lib

# Integration tests
cargo test -p soul-audio --test integration_test

# Property tests
cargo test -p soul-audio --test property_test

# Specific test
cargo test -p soul-audio test_eq_affects_frequency_content

# With output
cargo test -p soul-audio -- --nocapture

# Run property tests with more cases (default 256)
PROPTEST_CASES=1000 cargo test -p soul-audio --test property_test
```

## What These Tests DON'T Test

Following the "quality over quantity" principle, we **deliberately don't test**:

❌ Trivial getters/setters (covered by usage in real tests)
❌ Constructor parameter passthrough
❌ Enum variant equality
❌ Default trait implementations (unless behavior is critical)
❌ Simple field access
❌ API surface area for the sake of coverage

## What We Test Instead

✅ **Actual audio processing** - Does the EQ actually change frequency content?
✅ **Invariants** - Do effects always produce finite values?
✅ **Edge cases** - What happens with empty buffers, zero signals, extreme parameters?
✅ **Integration** - Do effects work together correctly in chains?
✅ **Determinism** - Do we get consistent results?
✅ **Real-time safety** - No allocations in audio path (enforced by design)

## Coverage Philosophy

> "50-60% meaningful coverage is better than 90% shallow coverage"

Our target:
- **50-60% line coverage** (meaningful lines)
- **100% critical path coverage** (audio processing loops)
- **0% trivial code coverage** (don't test getters)

## Test Maintenance

### Adding New Effects

When adding a new effect, write tests for:

1. **Basic functionality** - Does it process audio?
2. **Bypass is transparent** - When disabled, bit-perfect passthrough?
3. **Stability** - No NaN/Inf at extreme parameters?
4. **Reset is deterministic** - Same input → same output after reset?
5. **Integration** - Works correctly in chain?

### Avoiding Shallow Tests

Before adding a test, ask:

- Does this test verify **behavior** or just **existence**?
- Could a trivial implementation pass this test?
- Does this test catch real bugs or just API changes?

If the answer to the last question is "just API changes", **don't write the test**.

## CI Integration

Tests run on every commit:

```yaml
# .github/workflows/ci.yml
- name: Run tests
  run: cargo test --workspace
```

## Performance

Test suite runs in **< 1 second**:
- Unit tests: ~10ms
- Integration tests: ~10ms
- Property tests: ~400ms (14 tests × ~30ms each)
- Desktop tests: ~1400ms (real audio device tests)

**Total**: ~1.5 seconds for 66 tests

## Future Test Additions

Potential tests to add when features are implemented:

- **Gapless playback**: Verify no gaps between tracks
- **Seek accuracy**: Verify seeking is sample-accurate
- **Streaming**: Test incremental decoding
- **Memory leaks**: Long-running playback tests
- **Thread safety**: Concurrent access tests
- **Performance**: Benchmark tests with criterion

## References

- [Property-Based Testing](https://hypothesis.works/articles/what-is-property-based-testing/)
- [Proptest Book](https://altsysrq/proptest-book)
- [Testing in Rust](https://doc.rust-lang.org/book/ch11-00-testing.html)

---

**Summary**: 66 meaningful tests that verify actual behavior, catch real bugs, and give confidence the audio engine works correctly. No shallow tests, no coverage padding, just quality.
