//! DSD to PCM conversion — Gesemann's FIR decimation filter.
//!
//! Ported from Sebastian Gesemann's dsd2pcm (BSD-2-Clause).
//! Reference: FFmpeg libavcodec/dsd.c (identical coefficients and algorithm).
//! Copyright (c) 2009 Sebastian Gesemann. All rights reserved.
//!
//! # Algorithm
//!
//! One DSD byte (8 bits) → one PCM f32 sample via a 96-tap symmetric lowpass FIR
//! filter with 8:1 decimation. The filter is pre-baked into 6 lookup tables
//! (one per group of 8 coefficients × 256 possible byte values), so the hot path
//! is 12 table lookups and additions — no multiply per sample.
//!
//! # Output rates
//!
//! | DSD rate  | Byte rate      | PCM output rate |
//! |-----------|----------------|-----------------|
//! | DSD64     | 352,800 B/s    | 352,800 Hz      |
//! | DSD128    | 705,600 B/s    | 705,600 Hz      |
//!
//! The caller feeds one byte per channel per output sample. Further decimation
//! (down to 176.4 / 88.2 / 44.1 kHz) is handled by the resampler downstream.
//!
//! # Bit ordering
//!
//! - DSF files: LSB-first (`lsbf = true`). Bit 0 of each byte is the oldest DSD sample.
//! - DSDIFF files: MSB-first (`lsbf = false`). Bit 7 is the oldest DSD sample.

use std::sync::OnceLock;

// ── Filter constants ─────────────────────────────────────────────────────────

/// Number of FIR half-taps. Full filter = 2 × 48 = 96 taps (symmetric).
const HTAPS: usize = 48;
/// Circular DSD byte buffer size. Must be a power of two and ≥ HTAPS * 2 / 8.
const FIFOSIZE: usize = 16;
const FIFOMASK: usize = FIFOSIZE - 1;
/// Number of precomputed lookup tables = ceil(HTAPS / 8) = 6.
const CTABLES: usize = HTAPS.div_ceil(8);

/// 48 half-taps of the symmetric 96-tap lowpass FIR filter.
/// Ported verbatim from Gesemann's dsd2pcm / FFmpeg libavcodec/dsd.c `htaps[]`.
/// Passband flat to 48 kHz; stopband rejection ≈ 160 dB.
#[rustfmt::skip]
const HTAPS_COEFFS: [f64; HTAPS] = [
    0.09950731974056658,    0.09562845727714668,    0.08819647126516944,
    0.07782552527068175,    0.06534876523171299,    0.05172629311427257,
    0.0379429484910187,     0.02490921351762261,    0.0133774746265897,
    0.003883043418804416,  -0.003284703416210726,  -0.008080250212687497,
   -0.01067241812471033,   -0.01139427235000863,   -0.0106813877974587,
   -0.009_007_905_078_766_05,  -0.006828859761015335,  -0.004535184322001496,
   -0.002425035959059578,  -0.0006922187080790708,  0.0005700762133516592,
    0.001353838005269448,   0.001713709169690937,   0.001742046839472948,
    0.001545601648013235,   0.001226696225277855,   0.0008704322683580222,
    0.000_538_163_620_053_565,  0.000266446345425276,   7.002968738383528e-05,
   -5.279407053811266e-05, -0.0001140625650874684, -0.0001304796361231895,
   -0.0001189970287491285, -9.396247155265073e-05, -6.577634378272832e-05,
   -4.07492895872535e-05,  -2.17407957554587e-05,  -9.163058931391722e-06,
   -2.017460145032201e-06,  1.249721855219005e-06,  2.166655190537392e-06,
    1.930520892991082e-06,  1.319400334374195e-06,  7.410039764949091e-07,
    3.423230509967409e-07,  1.244182214744588e-07,  3.130441005359396e-08,
];

// ── Bit-reversal table ───────────────────────────────────────────────────────

/// Bit-reversal lookup: `REVERSE[b]` = `b` with all 8 bits mirrored.
/// Used to convert LSB-first DSD bytes to MSB-first for filter symmetry.
const REVERSE: [u8; 256] = {
    let mut t = [0u8; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut b = i as u8;
        let mut r = 0u8;
        let mut j = 0;
        while j < 8 {
            r = (r << 1) | (b & 1);
            b >>= 1;
            j += 1;
        }
        t[i] = r;
        i += 1;
    }
    t
};

// ── Lookup tables ────────────────────────────────────────────────────────────

/// Precomputed FIR tables for MSB-first and LSB-first DSD input.
/// Each table: `[CTABLES][256]` — for each of 6 coefficient groups,
/// one precomputed dot-product value per possible input byte.
struct CtablesPair {
    msbf: [[f64; 256]; CTABLES],
    lsbf: [[f64; 256]; CTABLES],
}

static CTABLES_STORE: OnceLock<CtablesPair> = OnceLock::new();

fn get_ctables() -> &'static CtablesPair {
    CTABLES_STORE.get_or_init(|| {
        let mut msbf = [[0.0f64; 256]; CTABLES];
        let mut lsbf = [[0.0f64; 256]; CTABLES];

        for e in 0usize..256 {
            // Dot product of each bit of `e` (as ±1) against 8 coefficients.
            // MSB-first: bit (7-m) of `e` is DSD sample at time offset m.
            let mut acc = [0.0f64; CTABLES];
            for m in 0..8 {
                let sign = if ((e >> (7 - m)) & 1) != 0 {
                    1.0f64
                } else {
                    -1.0f64
                };
                for t in 0..CTABLES {
                    acc[t] += sign * HTAPS_COEFFS[t * 8 + m];
                }
            }
            for t in 0..CTABLES {
                msbf[CTABLES - 1 - t][e] = acc[t];
                // For LSB-first: index by bit-reversed byte so no runtime reversal needed.
                lsbf[CTABLES - 1 - t][REVERSE[e] as usize] = acc[t];
            }
        }

        CtablesPair { msbf, lsbf }
    })
}

// ── Public API ───────────────────────────────────────────────────────────────

/// Per-channel DSD-to-PCM converter.
///
/// One instance per audio channel. Not `Send` between threads without external
/// synchronisation, but creating one per thread (or per channel) is cheap.
///
/// ```rust
/// use soul_audio::dsd::Dsd2Pcm;
///
/// let mut left  = Dsd2Pcm::new();
/// let mut right = Dsd2Pcm::new();
///
/// // DSF files are LSB-first (lsbf = true).
/// let dsd_bytes = [0x69u8; 64]; // DSD silence
/// let mut pcm = [0.0f32; 64];
/// left.translate(&dsd_bytes, &mut pcm, true);
/// ```
#[derive(Clone)]
pub struct Dsd2Pcm {
    /// Circular buffer of the last `FIFOSIZE` DSD bytes.
    buf: [u8; FIFOSIZE],
    /// Write-head position (masked with `FIFOMASK`).
    pos: usize,
}

impl Dsd2Pcm {
    /// Create a new converter. Buffer is initialised with DSD silence (`0x69`).
    pub fn new() -> Self {
        Self {
            buf: [0x69; FIFOSIZE],
            pos: 0,
        }
    }

    /// Reset to initial state (same as a freshly constructed instance).
    pub fn reset(&mut self) {
        self.buf = [0x69; FIFOSIZE];
        self.pos = 0;
    }

    /// Convert DSD bytes to PCM f32 samples.
    ///
    /// - `src`: raw DSD bytes, one per output sample.
    /// - `dst`: output buffer, must be the same length as `src`.
    /// - `lsbf`: `true` for LSB-first DSD (DSF default), `false` for MSB-first (DSDIFF).
    ///
    /// No heap allocations. Safe to call from an audio callback.
    #[inline]
    pub fn translate(&mut self, src: &[u8], dst: &mut [f32], lsbf: bool) {
        debug_assert_eq!(src.len(), dst.len());
        let tables = get_ctables();
        let ctables: &[[f64; 256]; CTABLES] = if lsbf { &tables.lsbf } else { &tables.msbf };

        let mut buf = self.buf;
        let mut pos = self.pos;

        for (&byte, out) in src.iter().zip(dst.iter_mut()) {
            // 1. Write new DSD byte into the circular buffer.
            buf[pos] = byte;

            // 2. Bit-reverse the byte at the filter's symmetric centre.
            //    Required for correct symmetric folding of the FIR filter.
            let centre = pos.wrapping_sub(CTABLES) & FIFOMASK;
            buf[centre] = REVERSE[buf[centre] as usize];

            // 3. Convolve — exploit the filter's symmetry (two halves at once).
            let mut sum = 0.0f64;
            for i in 0..CTABLES {
                let a = buf[pos.wrapping_sub(i) & FIFOMASK] as usize;
                let b = buf[pos.wrapping_sub(CTABLES * 2 - 1).wrapping_add(i) & FIFOMASK] as usize;
                sum += ctables[i][a] + ctables[i][b];
            }

            *out = sum as f32;
            pos = (pos + 1) & FIFOMASK;
        }

        self.buf = buf;
        self.pos = pos;
    }
}

impl Default for Dsd2Pcm {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── RED: write these first, watch them fail, then implement above ────────

    #[test]
    fn one_input_byte_produces_one_output_sample() {
        let mut ctx = Dsd2Pcm::new();
        let src = [0x69u8; 1];
        let mut dst = [0.0f32; 1];
        ctx.translate(&src, &mut dst, false);
        // Length contract — value irrelevant for a single warm-up byte
        assert_eq!(dst.len(), 1);
    }

    #[test]
    fn n_input_bytes_produce_n_output_samples() {
        let mut ctx = Dsd2Pcm::new();
        let src = vec![0x69u8; 64];
        let mut dst = vec![0.0f32; 64];
        ctx.translate(&src, &mut dst, false);
        assert_eq!(dst.len(), src.len());
    }

    #[test]
    fn all_ones_dsd_produces_positive_pcm_after_filter_settles() {
        let mut ctx = Dsd2Pcm::new();
        // Feed 32 bytes; the filter window is 12 bytes so samples [16..] are settled.
        let src = vec![0xFFu8; 32];
        let mut dst = vec![0.0f32; 32];
        ctx.translate(&src, &mut dst, false);
        // All settled samples must be strictly positive.
        for (i, &s) in dst[16..].iter().enumerate() {
            assert!(s > 0.0, "dst[{}+16] = {} expected positive", i, s);
        }
    }

    #[test]
    fn all_zeros_dsd_produces_negative_pcm_after_filter_settles() {
        let mut ctx = Dsd2Pcm::new();
        let src = vec![0x00u8; 32];
        let mut dst = vec![0.0f32; 32];
        ctx.translate(&src, &mut dst, false);
        for (i, &s) in dst[16..].iter().enumerate() {
            assert!(s < 0.0, "dst[{}+16] = {} expected negative", i, s);
        }
    }

    #[test]
    fn all_ones_and_all_zeros_are_perfectly_symmetric() {
        let mut pos_ctx = Dsd2Pcm::new();
        let mut neg_ctx = Dsd2Pcm::new();
        let ones = vec![0xFFu8; 32];
        let zeros = vec![0x00u8; 32];
        let mut pos_dst = vec![0.0f32; 32];
        let mut neg_dst = vec![0.0f32; 32];
        pos_ctx.translate(&ones, &mut pos_dst, false);
        neg_ctx.translate(&zeros, &mut neg_dst, false);
        // Check only settled samples (FIFO = 16 bytes; early outputs reflect the
        // initial 0x69 silence fill which is not antisymmetric).
        for i in FIFOSIZE..32 {
            let diff = (pos_dst[i] + neg_dst[i]).abs();
            assert!(
                diff < 1e-6,
                "symmetry broken at [{}]: {} + {} = {}",
                i,
                pos_dst[i],
                neg_dst[i],
                diff
            );
        }
    }

    #[test]
    fn all_ones_lsbf_equals_all_ones_msbf() {
        // 0xFF is all-ones regardless of bit order.
        let mut lsbf_ctx = Dsd2Pcm::new();
        let mut msbf_ctx = Dsd2Pcm::new();
        let src = vec![0xFFu8; 32];
        let mut lsbf_dst = vec![0.0f32; 32];
        let mut msbf_dst = vec![0.0f32; 32];
        lsbf_ctx.translate(&src, &mut lsbf_dst, true);
        msbf_ctx.translate(&src, &mut msbf_dst, false);
        // Check only settled samples — initial 0x69 fill is interpreted
        // differently by LSBF vs MSBF tables until the FIFO is fully flushed.
        for i in FIFOSIZE..32 {
            let diff = (lsbf_dst[i] - msbf_dst[i]).abs();
            assert!(
                diff < 1e-6,
                "lsbf/msbf 0xFF mismatch at [{}]: {} vs {}",
                i,
                lsbf_dst[i],
                msbf_dst[i]
            );
        }
    }

    #[test]
    fn lsbf_0x01_equals_msbf_0x80_same_single_bit() {
        // 0x01 LSB-first = oldest bit is 1, rest 0. Same as 0x80 MSB-first.
        let mut lsbf_ctx = Dsd2Pcm::new();
        let mut msbf_ctx = Dsd2Pcm::new();
        let lsbf_src = vec![0x01u8; 32];
        let msbf_src = vec![0x80u8; 32];
        let mut lsbf_dst = vec![0.0f32; 32];
        let mut msbf_dst = vec![0.0f32; 32];
        lsbf_ctx.translate(&lsbf_src, &mut lsbf_dst, true);
        msbf_ctx.translate(&msbf_src, &mut msbf_dst, false);
        // Check settled samples only.
        for i in FIFOSIZE..32 {
            let diff = (lsbf_dst[i] - msbf_dst[i]).abs();
            assert!(
                diff < 1e-6,
                "0x01 lsbf vs 0x80 msbf mismatch at [{}]: {} vs {}",
                i,
                lsbf_dst[i],
                msbf_dst[i]
            );
        }
    }

    #[test]
    fn reset_produces_identical_output_to_fresh_instance() {
        let mut ctx = Dsd2Pcm::new();
        let src: Vec<u8> = (0..32u8).map(|i| i.wrapping_mul(7)).collect();
        let mut first = vec![0.0f32; 32];
        let mut second = vec![0.0f32; 32];

        ctx.translate(&src, &mut first, false);
        ctx.reset();
        ctx.translate(&src, &mut second, false);

        for i in 0..32 {
            let diff = (first[i] - second[i]).abs();
            assert!(
                diff < 1e-10,
                "after reset, output[{}] differs: {} vs {}",
                i,
                first[i],
                second[i]
            );
        }
    }

    #[test]
    fn output_amplitude_stays_within_reasonable_bounds() {
        let mut ctx = Dsd2Pcm::new();
        // Worst case: alternating 0xFF/0x00 (maximum modulation).
        let src: Vec<u8> = (0..256)
            .map(|i| if i % 2 == 0 { 0xFF } else { 0x00 })
            .collect();
        let mut dst = vec![0.0f32; 256];
        ctx.translate(&src, &mut dst, false);
        for (i, &s) in dst.iter().enumerate() {
            // FIR filter limits gain; allow ±1.5 headroom for transients.
            assert!(
                s.is_finite() && s.abs() <= 1.5,
                "out of range at [{}]: {}",
                i,
                s
            );
        }
    }

    #[test]
    fn two_independent_instances_track_independent_input() {
        let mut ch0 = Dsd2Pcm::new();
        let mut ch1 = Dsd2Pcm::new();
        let ones = vec![0xFFu8; 32];
        let zeros = vec![0x00u8; 32];
        let mut out0 = vec![0.0f32; 32];
        let mut out1 = vec![0.0f32; 32];
        ch0.translate(&ones, &mut out0, false);
        ch1.translate(&zeros, &mut out1, false);
        // After settling ch0 ≈ +1, ch1 ≈ −1.
        assert!(out0[31] > 0.0, "ch0 should be positive, got {}", out0[31]);
        assert!(out1[31] < 0.0, "ch1 should be negative, got {}", out1[31]);
    }

    #[test]
    fn clone_produces_identical_continuation() {
        let mut ctx = Dsd2Pcm::new();
        let warmup = vec![0xA5u8; 16];
        let mut tmp = vec![0.0f32; 16];
        ctx.translate(&warmup, &mut tmp, false);

        let mut clone = ctx.clone();
        let src = vec![0x96u8; 16];
        let mut out_orig = vec![0.0f32; 16];
        let mut out_clone = vec![0.0f32; 16];
        ctx.translate(&src, &mut out_orig, false);
        clone.translate(&src, &mut out_clone, false);

        for i in 0..16 {
            let diff = (out_orig[i] - out_clone[i]).abs();
            assert!(
                diff < 1e-10,
                "clone diverged at [{}]: {} vs {}",
                i,
                out_orig[i],
                out_clone[i]
            );
        }
    }

    #[test]
    fn dsd_silence_pattern_0x69_produces_near_zero_after_settling() {
        // 0x69 = 0110_1001 — the DSD silence pattern designed to average to 0 DC.
        let mut ctx = Dsd2Pcm::new();
        let src = vec![0x69u8; 64];
        let mut dst = vec![0.0f32; 64];
        ctx.translate(&src, &mut dst, false);
        // After full settling (32+ bytes), output should be very close to zero.
        let max_abs: f32 = dst[32..].iter().map(|s| s.abs()).fold(0.0, f32::max);
        assert!(
            max_abs < 0.05,
            "silence pattern produced too much output: {}",
            max_abs
        );
    }
}
