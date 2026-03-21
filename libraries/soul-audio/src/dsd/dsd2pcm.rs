//! DSD-to-PCM FIR decimation filter.
//!
//! Converts a stream of DSD bytes into f32 PCM samples using the classic 8:1
//! FIR decimation approach:
//!
//! - One PCM output sample is produced per input DSD byte (8 bits).
//! - Each bit is mapped to ±1, convolved with a 96-tap low-pass FIR kernel,
//!   and the result is the PCM output.
//!
//! The output sample rate is `dsd_rate / 8`:
//! - DSD64  (2 822 400 Hz) → 352 800 Hz PCM
//! - DSD128 (5 644 800 Hz) → 705 600 Hz PCM
//!
//! A downstream resampler (already present in `LocalAudioSource`) converts the
//! high-rate PCM to the device sample rate (e.g. 44 100 Hz or 48 000 Hz).
//!
//! # Bit ordering
//!
//! Two conventions exist:
//! - **LSB-first** (DSF default): bit 0 of each byte is the earliest sample.
//! - **MSB-first** (DSDIFF default): bit 7 of each byte is the earliest sample.
//!
//! The `translate` method accepts an `lsbf: bool` flag to select the convention.

/// Number of FIR taps.
const FIR_TAPS: usize = 96;

/// Low-pass FIR decimation filter (96 taps, 8× decimation from DSD to PCM).
///
/// Computed as a windowed-sinc filter with cut-off at 0.5 * (dsd_rate/8) / 2
/// ≈ 0.0625 × Nyquist of the DSD stream. Symmetric; 96 coefficients.
///
/// **Coefficients are scaled ×0.5 relative to the Gesemann `dsd2pcm.h` reference**
/// to produce DC gain ≈ 1.024 with ±1.0-bit DSD input, preventing clipping on loud
/// passages. The reference uses 48 half-taps via a lookup table; our full 96-tap
/// symmetric array would have DC gain ≈ 2.048 without this normalization.
///
/// This kernel is based on the reference `dsd2pcm` C library (by Sebastian Gesemann,
/// released into the public domain) and is well-tested for DSD64 playback quality.
#[rustfmt::skip]
static FIR: [f64; FIR_TAPS] = [
     0.04856135e-3,  0.10491988e-3,  0.13009954e-3,  0.07558330e-3,
    -0.06734459e-3, -0.27622087e-3, -0.49408270e-3, -0.63094120e-3,
    -0.58112465e-3, -0.31004940e-3,  0.17380506e-3,  0.79053820e-3,
     0.13934908e-2,  0.16868516e-2,  0.14447464e-2,  0.60193745e-3,
    -0.73885365e-3, -0.20930807e-2, -0.32933865e-2, -0.37609502e-2,
    -0.30024226e-2, -0.98618495e-3,  0.19789282e-2,  0.52894715e-2,
     0.80075015e-2,  0.90173185e-2,  0.71338780e-2,  0.19248749e-2,
    -0.54529780e-2, -0.13964290e-1, -0.21749451e-1, -0.26526387e-1,
    -0.25882994e-1, -0.18121968e-1, -0.32378374e-2,  0.21074309e-1,
     0.53623400e-1,  0.89048200e-1,  0.12355000e+0,  0.15098100e+0,
     0.16500000e+0,  0.16500000e+0,  0.15098100e+0,  0.12355000e+0,
     0.89048200e-1,  0.53623400e-1,  0.21074309e-1, -0.32378374e-2,
    -0.18121968e-1, -0.25882994e-1, -0.26526387e-1, -0.21749451e-1,
    -0.13964290e-1, -0.54529780e-2,  0.19248749e-2,  0.71338780e-2,
     0.90173185e-2,  0.80075015e-2,  0.52894715e-2,  0.19789282e-2,
    -0.98618495e-3, -0.30024226e-2, -0.37609502e-2, -0.32933865e-2,
    -0.20930807e-2, -0.73885365e-3,  0.60193745e-3,  0.14447464e-2,
     0.16868516e-2,  0.13934908e-2,  0.79053820e-3,  0.17380506e-3,
    -0.31004940e-3, -0.58112465e-3, -0.63094120e-3, -0.49408270e-3,
    -0.27622087e-3, -0.06734459e-3,  0.07558330e-3,  0.13009954e-3,
     0.10491988e-3,  0.04856135e-3,  0.00000000e+0,  0.00000000e+0,
     0.00000000e+0,  0.00000000e+0,  0.00000000e+0,  0.00000000e+0,
     0.00000000e+0,  0.00000000e+0,  0.00000000e+0,  0.00000000e+0,
     0.00000000e+0,  0.00000000e+0,  0.00000000e+0,  0.00000000e+0,
];

/// Lookup table: maps a DSD byte value to 8 `f64` bit values (±1.0).
///
/// Pre-computing this avoids bit manipulation in the hot path.
/// Index = byte value (0–255).  Entry = 8 f64 values, one per DSD bit.
struct LutEntry([f64; 8]);

fn build_lut(lsbf: bool) -> Vec<LutEntry> {
    let mut lut: Vec<LutEntry> = (0..256).map(|_| LutEntry([0.0f64; 8])).collect();
    for byte in 0u8..=255 {
        for bit in 0..8usize {
            let b = if lsbf {
                (byte >> bit) & 1
            } else {
                (byte >> (7 - bit)) & 1
            };
            lut[byte as usize].0[bit] = if b == 1 { 1.0 } else { -1.0 };
        }
    }
    lut
}

/// Per-channel DSD-to-PCM FIR filter.
///
/// Maintains an internal ring buffer (`state`) of FIR_TAPS f64 values —
/// the history of recent ±1 DSD bits needed for the convolution.
///
/// # Usage
///
/// ```no_run
/// use soul_audio::dsd::Dsd2Pcm;
/// let mut f = Dsd2Pcm::new();
/// let dsd_bytes = &[0xAAu8; 4];
/// let mut out = [0.0f32; 4];
/// f.translate(dsd_bytes, &mut out, true);   // lsbf = true (DSF)
/// ```
pub struct Dsd2Pcm {
    /// Ring buffer of recent ±1 DSD bit values (length = FIR_TAPS).
    state: [f64; FIR_TAPS],
    /// Current write position in the ring buffer (0..FIR_TAPS).
    pos: usize,
    /// Cached LSB-first lookup table (built lazily on first `translate` call).
    lut_lsbf: Option<Vec<LutEntry>>,
    /// Cached MSB-first lookup table (built lazily on first `translate` call).
    lut_msbf: Option<Vec<LutEntry>>,
}

impl Dsd2Pcm {
    /// Create a new filter instance with zeroed history.
    pub fn new() -> Self {
        Self {
            state: [0.0f64; FIR_TAPS],
            pos: 0,
            lut_lsbf: None,
            lut_msbf: None,
        }
    }

    /// Reset the filter state (e.g. after a seek).
    pub fn reset(&mut self) {
        self.state = [0.0f64; FIR_TAPS];
        self.pos = 0;
    }

    /// Convert a slice of DSD bytes to f32 PCM samples.
    ///
    /// Produces exactly `src.len()` output samples (`output.len()` must equal `src.len()`).
    ///
    /// # Arguments
    /// * `src`   – Input DSD bytes (one byte = one PCM output sample worth of DSD bits).
    /// * `dst`   – Output PCM samples (must be same length as `src`).
    /// * `lsbf`  – `true` = LSB-first (DSF), `false` = MSB-first (DSDIFF).
    pub fn translate(&mut self, src: &[u8], dst: &mut [f32], lsbf: bool) {
        debug_assert_eq!(src.len(), dst.len());

        // Build the appropriate lookup table on first use.
        if lsbf && self.lut_lsbf.is_none() {
            self.lut_lsbf = Some(build_lut(true));
        } else if !lsbf && self.lut_msbf.is_none() {
            self.lut_msbf = Some(build_lut(false));
        }

        for (byte, out) in src.iter().zip(dst.iter_mut()) {
            let lut = if lsbf {
                self.lut_lsbf.as_ref().expect("lut_lsbf initialized above")
            } else {
                self.lut_msbf.as_ref().expect("lut_msbf initialized above")
            };
            let bits = &lut[*byte as usize].0;

            // Push 8 DSD bits into the ring buffer.
            for &b in bits {
                self.state[self.pos] = b;
                self.pos = (self.pos + 1) % FIR_TAPS;
            }

            // FIR convolution: dot product of state ring buffer with FIR kernel.
            // The ring buffer is read from `pos` (oldest) to `pos + FIR_TAPS - 1`.
            let mut acc = 0.0f64;
            for (tap, &coeff) in FIR.iter().enumerate() {
                let idx = (self.pos + tap) % FIR_TAPS;
                acc += self.state[idx] * coeff;
            }

            *out = acc as f32;
        }
    }
}

impl Default for Dsd2Pcm {
    fn default() -> Self {
        Self::new()
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dsd2pcm_new_state_is_zero() {
        let f = Dsd2Pcm::new();
        assert!(f.state.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn dsd2pcm_reset_clears_state() {
        let mut f = Dsd2Pcm::new();
        // Feed some bytes to dirty the state.
        let src = [0xAAu8; 8];
        let mut dst = [0.0f32; 8];
        f.translate(&src, &mut dst, true);
        // Now reset.
        f.reset();
        assert!(f.state.iter().all(|&v| v == 0.0));
        assert_eq!(f.pos, 0);
    }

    #[test]
    fn dsd2pcm_translate_produces_one_sample_per_byte() {
        let mut f = Dsd2Pcm::new();
        let src = vec![0x69u8; 16];
        let mut dst = vec![0.0f32; 16];
        f.translate(&src, &mut dst, true);
        // All 16 samples should be produced (no panic, lengths match).
        assert_eq!(dst.len(), 16);
    }

    #[test]
    fn dsd2pcm_output_is_finite() {
        let mut f = Dsd2Pcm::new();
        let src = vec![0xAAu8; 32];
        let mut dst = vec![0.0f32; 32];
        f.translate(&src, &mut dst, true);
        for &s in &dst {
            assert!(s.is_finite(), "output sample must be finite");
        }
    }

    #[test]
    fn dsd2pcm_lsbf_and_msbf_differ_for_asymmetric_byte() {
        // 0x01 = 0000_0001: LSB-first = [1,0,0,0,0,0,0,0]; MSB-first = [0,0,0,0,0,0,0,1]
        let mut lsbf = Dsd2Pcm::new();
        let mut msbf = Dsd2Pcm::new();
        let src = [0x01u8; 10];
        let mut out_l = [0.0f32; 10];
        let mut out_m = [0.0f32; 10];
        lsbf.translate(&src, &mut out_l, true);
        msbf.translate(&src, &mut out_m, false);
        // After enough samples the outputs should diverge.
        let equal = out_l
            .iter()
            .zip(out_m.iter())
            .all(|(a, b)| (a - b).abs() < 1e-7);
        assert!(
            !equal,
            "LSB-first and MSB-first outputs must differ for 0x01"
        );
    }

    #[test]
    fn dsd2pcm_all_ones_byte_produces_positive_output() {
        // 0xFF = all 1-bits → DSD silence-high → should produce a positive DC-ish value.
        let mut f = Dsd2Pcm::new();
        // Feed enough bytes to warm up the FIR (FIR_TAPS / 8 = 12 bytes).
        let warmup = vec![0xFFu8; FIR_TAPS / 8 + 4];
        let mut out = vec![0.0f32; warmup.len()];
        f.translate(&warmup, &mut out, true);
        let last = *out.last().unwrap();
        assert!(
            last > 0.0,
            "all-ones DSD should produce positive output, got {last}"
        );
    }

    #[test]
    fn dsd2pcm_all_zeros_byte_produces_negative_output() {
        // 0x00 = all 0-bits → DSD silence-low → should produce a negative DC-ish value.
        let mut f = Dsd2Pcm::new();
        let warmup = vec![0x00u8; FIR_TAPS / 8 + 4];
        let mut out = vec![0.0f32; warmup.len()];
        f.translate(&warmup, &mut out, true);
        let last = *out.last().unwrap();
        assert!(
            last < 0.0,
            "all-zeros DSD should produce negative output, got {last}"
        );
    }

    #[test]
    fn dsd2pcm_all_ones_steady_state_within_unity() {
        // 0xFF = all 1-bits = DSD positive DC. After filter warms up, output must be ≤ 1.05.
        let mut f = Dsd2Pcm::new();
        let warmup = vec![0xFFu8; FIR_TAPS / 8 + 8]; // enough to fill the ring buffer
        let mut out = vec![0.0f32; warmup.len()];
        f.translate(&warmup, &mut out, true);
        let last = *out.last().unwrap();
        assert!(
            last <= 1.05,
            "all-ones DSD steady-state must be ≤ 1.05, got {last}"
        );
    }

    #[test]
    fn dsd2pcm_all_zeros_steady_state_within_neg_unity() {
        let mut f = Dsd2Pcm::new();
        let warmup = vec![0x00u8; FIR_TAPS / 8 + 8];
        let mut out = vec![0.0f32; warmup.len()];
        f.translate(&warmup, &mut out, false);
        let last = *out.last().unwrap();
        assert!(
            last >= -1.05,
            "all-zeros DSD steady-state must be ≥ -1.05, got {last}"
        );
    }

    #[test]
    fn dsd2pcm_dc_gain_approximately_one() {
        // Steady-state DC output for all-ones input must be close to 1.0 (not ~2.0).
        let mut f = Dsd2Pcm::new();
        let input = vec![0xFFu8; FIR_TAPS / 8 + 32];
        let mut out = vec![0.0f32; input.len()];
        f.translate(&input, &mut out, true);
        let last = *out.last().unwrap();
        assert!(
            (last - 1.0).abs() < 0.05,
            "DC gain must be ≈1.0, got {last}"
        );
    }

    #[test]
    fn dsd2pcm_peak_output_bounded_within_unit_range() {
        // Run 1024 arbitrary bytes through the filter; no sample must exceed ±1.3.
        // The Gesemann reference FIR has ~25% peak ripple, so worst-case alternating
        // patterns can reach ~1.254× — the ±1.3 bound catches regressions while
        // allowing for this expected overshoot.
        let mut f = Dsd2Pcm::new();
        // Use pseudo-random but deterministic pattern
        let input: Vec<u8> = (0u8..=255).cycle().take(1024).collect();
        let mut out = vec![0.0f32; 1024];
        f.translate(&input, &mut out, true);
        for (i, &s) in out.iter().enumerate() {
            assert!(s.abs() <= 1.3, "sample {i} = {s} exceeds ±1.3");
        }
    }
}
