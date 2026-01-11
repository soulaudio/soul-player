//! Convolution reverb engine for IR-based room correction and reverb effects
//!
//! Implements partitioned convolution for low-latency processing of impulse responses.
//! Uses FFT-based overlap-save method for efficient processing of impulse responses
//! longer than 64 samples, with direct time-domain convolution for shorter IRs.
//!
//! # Example
//!
//! ```rust,no_run
//! use soul_audio::effects::{ConvolutionEngine, AudioEffect};
//!
//! let mut engine = ConvolutionEngine::new();
//! engine.load_impulse_response(&[0.5, 0.3, 0.1], 44100, 2).unwrap();
//! engine.set_dry_wet_mix(0.5); // 50% wet signal
//!
//! let mut buffer = vec![0.5; 1024];
//! engine.process(&mut buffer, 44100);
//! ```

use super::AudioEffect;
#[allow(unused_imports)]
use rustfft::FftPlanner;
use rustfft::{num_complex::Complex, Fft};
use std::path::Path;
use std::sync::Arc;

/// Threshold below which we use direct time-domain convolution
const TIME_DOMAIN_THRESHOLD: usize = 64;

/// Minimum FFT size for efficient processing
const MIN_FFT_SIZE: usize = 256;

/// Convolution engine for applying impulse response-based effects
///
/// Uses a hybrid approach for efficient real-time processing:
/// - Direct time-domain convolution for very short IRs (< 64 samples)
/// - FFT-based overlap-save convolution for longer IRs
///
/// Supports stereo impulse responses and configurable dry/wet mix.
pub struct ConvolutionEngine {
    /// Impulse response samples (interleaved stereo)
    ir_samples: Vec<f32>,
    /// Sample rate of the loaded IR
    ir_sample_rate: u32,
    /// Number of channels (1 or 2)
    ir_channels: usize,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet)
    dry_wet_mix: f32,
    /// Whether the engine is enabled
    enabled: bool,
    /// FFT-based convolution state (None if using time-domain)
    fft_state: Option<FftConvolutionState>,
    /// Pre-allocated output buffer (avoids allocation in process())
    output_scratch: Vec<f32>,
    /// Partition size for overlap-add convolution
    partition_size: usize,
    /// IR partitions for efficient convolution
    ir_partitions: Vec<Vec<f32>>,
    /// Input partition history for overlap-add
    input_partitions: Vec<Vec<f32>>,
    /// Current partition index
    partition_index: usize,
    /// Input buffer for accumulating samples
    input_buffer: Vec<f32>,
    /// Output buffer for processed samples
    output_buffer: Vec<f32>,
    /// Current position in the input buffer
    buffer_pos: usize,
}

/// State for FFT-based convolution using overlap-save method
struct FftConvolutionState {
    /// FFT size (power of 2, >= 2 * max(buffer_size, ir_length))
    fft_size: usize,
    /// Pre-computed FFT of IR for left channel (complex, in frequency domain)
    ir_fft_left: Vec<Complex<f32>>,
    /// Pre-computed FFT of IR for right channel (complex, in frequency domain)
    ir_fft_right: Vec<Complex<f32>>,
    /// Forward FFT planner
    fft_forward: Arc<dyn Fft<f32>>,
    /// Inverse FFT planner
    fft_inverse: Arc<dyn Fft<f32>>,
    /// Input history for overlap-save (left channel)
    input_history_left: Vec<f32>,
    /// Input history for overlap-save (right channel)
    input_history_right: Vec<f32>,
    /// Output overlap buffer (left channel)
    output_overlap_left: Vec<f32>,
    /// Output overlap buffer (right channel)
    output_overlap_right: Vec<f32>,
    /// Scratch buffer for FFT operations
    fft_scratch: Vec<Complex<f32>>,
    /// Scratch buffer for input FFT
    input_fft_scratch: Vec<Complex<f32>>,
    /// Scratch buffer for multiplication result
    mult_scratch: Vec<Complex<f32>>,
}

impl Default for ConvolutionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ConvolutionEngine {
    /// Create a new convolution engine
    pub fn new() -> Self {
        Self {
            ir_samples: Vec::new(),
            ir_sample_rate: 44100,
            ir_channels: 2,
            dry_wet_mix: 1.0, // Fully wet by default
            enabled: false,
            fft_state: None,
            output_scratch: Vec::new(),
            partition_size: 512,
            ir_partitions: Vec::new(),
            input_partitions: Vec::new(),
            partition_index: 0,
            input_buffer: Vec::new(),
            output_buffer: Vec::new(),
            buffer_pos: 0,
        }
    }

    /// Load an impulse response from raw samples
    ///
    /// # Arguments
    /// * `samples` - Interleaved audio samples (mono or stereo)
    /// * `sample_rate` - Sample rate of the IR
    /// * `channels` - Number of channels (1 or 2)
    pub fn load_impulse_response(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        channels: usize,
    ) -> Result<(), ConvolutionError> {
        if samples.is_empty() {
            return Err(ConvolutionError::EmptyImpulseResponse);
        }

        if channels != 1 && channels != 2 {
            return Err(ConvolutionError::InvalidChannelCount(channels));
        }

        // Store IR data
        self.ir_samples = samples.to_vec();
        self.ir_sample_rate = sample_rate;
        self.ir_channels = channels;

        // Prepare FFT-based convolution for longer IRs
        let ir_frames = samples.len() / channels;
        if ir_frames > TIME_DOMAIN_THRESHOLD {
            self.prepare_fft_convolution();
        } else {
            self.fft_state = None;
        }

        self.enabled = true;
        Ok(())
    }

    /// Prepare FFT-based convolution state
    fn prepare_fft_convolution(&mut self) {
        let ir_frames = self.ir_samples.len() / self.ir_channels;

        // Choose FFT size: needs to be at least 2x the IR length for overlap-save
        // Use a reasonable default block size for processing
        let block_size = 512;
        let fft_size = (ir_frames + block_size)
            .next_power_of_two()
            .max(MIN_FFT_SIZE);

        // Create FFT planners
        let mut planner = FftPlanner::new();
        let fft_forward = planner.plan_fft_forward(fft_size);
        let fft_inverse = planner.plan_fft_inverse(fft_size);

        // Extract left and right IR channels
        let mut ir_left = vec![0.0f32; fft_size];
        let mut ir_right = vec![0.0f32; fft_size];

        for i in 0..ir_frames {
            let idx = i * self.ir_channels;
            ir_left[i] = self.ir_samples[idx];
            ir_right[i] = if self.ir_channels == 2 {
                self.ir_samples[idx + 1]
            } else {
                self.ir_samples[idx]
            };
        }

        // Compute FFT of IR
        let mut ir_fft_left: Vec<Complex<f32>> =
            ir_left.iter().map(|&x| Complex::new(x, 0.0)).collect();
        let mut ir_fft_right: Vec<Complex<f32>> =
            ir_right.iter().map(|&x| Complex::new(x, 0.0)).collect();

        fft_forward.process(&mut ir_fft_left);
        fft_forward.process(&mut ir_fft_right);

        // Pre-allocate all buffers
        let fft_state = FftConvolutionState {
            fft_size,
            ir_fft_left,
            ir_fft_right,
            fft_forward,
            fft_inverse,
            input_history_left: vec![0.0; fft_size],
            input_history_right: vec![0.0; fft_size],
            output_overlap_left: vec![0.0; fft_size],
            output_overlap_right: vec![0.0; fft_size],
            fft_scratch: vec![Complex::new(0.0, 0.0); fft_size],
            input_fft_scratch: vec![Complex::new(0.0, 0.0); fft_size],
            mult_scratch: vec![Complex::new(0.0, 0.0); fft_size],
        };

        self.fft_state = Some(fft_state);
    }

    /// Load an impulse response from a WAV file
    pub fn load_from_wav<P: AsRef<Path>>(&mut self, path: P) -> Result<(), ConvolutionError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(ConvolutionError::FileNotFound(path.display().to_string()));
        }

        // Read WAV file using hound
        let reader = hound::WavReader::open(path)
            .map_err(|e| ConvolutionError::WavReadError(e.to_string()))?;

        let spec = reader.spec();
        let channels = spec.channels as usize;
        let sample_rate = spec.sample_rate;

        // Read samples based on format
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => reader
                .into_samples::<f32>()
                .filter_map(Result::ok)
                .collect(),
            hound::SampleFormat::Int => {
                let bits = spec.bits_per_sample;
                let max_val = (1i32 << (bits - 1)) as f32;
                reader
                    .into_samples::<i32>()
                    .filter_map(Result::ok)
                    .map(|s| s as f32 / max_val)
                    .collect()
            }
        };

        if samples.is_empty() {
            return Err(ConvolutionError::EmptyImpulseResponse);
        }

        self.load_impulse_response(&samples, sample_rate, channels)
    }

    /// Set the dry/wet mix (0.0 = fully dry, 1.0 = fully wet)
    pub fn set_dry_wet_mix(&mut self, mix: f32) {
        self.dry_wet_mix = mix.clamp(0.0, 1.0);
    }

    /// Get the current dry/wet mix
    pub fn dry_wet_mix(&self) -> f32 {
        self.dry_wet_mix
    }

    /// Enable or disable the convolution engine
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if the engine is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the length of the loaded impulse response in samples
    pub fn ir_length(&self) -> usize {
        self.ir_samples.len() / self.ir_channels
    }

    /// Get the length of the loaded impulse response in seconds
    pub fn ir_duration_seconds(&self) -> f32 {
        if self.ir_sample_rate == 0 {
            return 0.0;
        }
        self.ir_length() as f32 / self.ir_sample_rate as f32
    }

    /// Process using FFT-based overlap-add convolution
    ///
    /// This is efficient for IRs longer than TIME_DOMAIN_THRESHOLD samples.
    /// Uses the overlap-add method: input blocks are zero-padded, convolved via FFT,
    /// and results are added together with proper overlap.
    fn convolve_fft(&mut self, input: &[f32], output: &mut [f32]) {
        let state = match &mut self.fft_state {
            Some(s) => s,
            None => {
                output.copy_from_slice(input);
                return;
            }
        };

        let frames = input.len() / 2;
        let fft_size = state.fft_size;
        let ir_frames = self.ir_samples.len() / self.ir_channels;

        // Block size for overlap-add: we process input in blocks
        // FFT size = block_size + ir_frames - 1, so block_size = fft_size - ir_frames + 1
        let block_size = fft_size - ir_frames + 1;

        // Process input in blocks
        let mut input_pos = 0;
        let mut output_pos = 0;

        while input_pos < frames {
            let chunk_size = (frames - input_pos).min(block_size);

            // Zero-pad input block for FFT (left channel)
            for i in 0..fft_size {
                if i < chunk_size {
                    let idx = (input_pos + i) * 2;
                    state.input_fft_scratch[i] = Complex::new(input[idx], 0.0);
                } else {
                    state.input_fft_scratch[i] = Complex::new(0.0, 0.0);
                }
            }

            // FFT of input (left)
            state.fft_forward.process(&mut state.input_fft_scratch);

            // Multiply in frequency domain (left)
            for i in 0..fft_size {
                state.mult_scratch[i] = state.input_fft_scratch[i] * state.ir_fft_left[i];
            }

            // Inverse FFT (left)
            state.fft_inverse.process(&mut state.mult_scratch);
            let scale = 1.0 / fft_size as f32;

            // Add to overlap buffer (left) - full convolution result is added with overlap
            let conv_len = chunk_size + ir_frames - 1;
            for i in 0..conv_len.min(fft_size) {
                state.output_overlap_left[i] += state.mult_scratch[i].re * scale;
            }

            // Zero-pad input block for FFT (right channel)
            for i in 0..fft_size {
                if i < chunk_size {
                    let idx = (input_pos + i) * 2 + 1;
                    state.input_fft_scratch[i] = Complex::new(input[idx], 0.0);
                } else {
                    state.input_fft_scratch[i] = Complex::new(0.0, 0.0);
                }
            }

            // FFT of input (right)
            state.fft_forward.process(&mut state.input_fft_scratch);

            // Multiply in frequency domain (right)
            for i in 0..fft_size {
                state.mult_scratch[i] = state.input_fft_scratch[i] * state.ir_fft_right[i];
            }

            // Inverse FFT (right)
            state.fft_inverse.process(&mut state.mult_scratch);

            // Add to overlap buffer (right)
            for i in 0..conv_len.min(fft_size) {
                state.output_overlap_right[i] += state.mult_scratch[i].re * scale;
            }

            // Extract output from overlap buffer and apply dry/wet mix
            for i in 0..chunk_size {
                let out_idx = (output_pos + i) * 2;
                let in_idx = (input_pos + i) * 2;
                output[out_idx] = input[in_idx] * (1.0 - self.dry_wet_mix)
                    + state.output_overlap_left[i] * self.dry_wet_mix;
                output[out_idx + 1] = input[in_idx + 1] * (1.0 - self.dry_wet_mix)
                    + state.output_overlap_right[i] * self.dry_wet_mix;
            }

            // Shift overlap buffers left by chunk_size, zero the tail
            state.output_overlap_left.copy_within(chunk_size.., 0);
            state.output_overlap_right.copy_within(chunk_size.., 0);
            for i in (fft_size - chunk_size)..fft_size {
                state.output_overlap_left[i] = 0.0;
                state.output_overlap_right[i] = 0.0;
            }

            input_pos += chunk_size;
            output_pos += chunk_size;
        }
    }

    /// Process using time-domain convolution for short IRs
    ///
    /// This is efficient for IRs shorter than TIME_DOMAIN_THRESHOLD samples.
    fn convolve_time_domain(&self, input: &[f32], output: &mut [f32]) {
        let frames = input.len() / 2;
        let ir_frames = self.ir_samples.len() / self.ir_channels.max(1);

        // Direct convolution (efficient for short IRs)
        for i in 0..frames {
            let mut left_sum = 0.0f32;
            let mut right_sum = 0.0f32;

            // Convolve with IR
            let max_j = ir_frames.min(i + 1);
            for j in 0..max_j {
                let input_idx = (i - j) * 2;
                let ir_idx = j * self.ir_channels;

                let ir_left = self.ir_samples[ir_idx];
                let ir_right = if self.ir_channels == 2 {
                    self.ir_samples[ir_idx + 1]
                } else {
                    ir_left
                };

                left_sum += input[input_idx] * ir_left;
                right_sum += input[input_idx + 1] * ir_right;
            }

            // Apply dry/wet mix
            let out_idx = i * 2;
            output[out_idx] =
                input[out_idx] * (1.0 - self.dry_wet_mix) + left_sum * self.dry_wet_mix;
            output[out_idx + 1] =
                input[out_idx + 1] * (1.0 - self.dry_wet_mix) + right_sum * self.dry_wet_mix;
        }
    }
}

impl AudioEffect for ConvolutionEngine {
    fn process(&mut self, buffer: &mut [f32], _sample_rate: u32) {
        if !self.enabled || self.ir_samples.is_empty() || buffer.is_empty() {
            return;
        }

        let buffer_len = buffer.len();

        // Ensure output scratch buffer is large enough
        if self.output_scratch.len() < buffer_len {
            self.output_scratch.resize(buffer_len, 0.0);
        }

        // Choose convolution method based on IR length
        let use_fft = self.fft_state.is_some();
        if use_fft {
            // Use FFT-based convolution for longer IRs
            // Split borrow by taking output_scratch out temporarily
            let mut output = std::mem::take(&mut self.output_scratch);
            self.convolve_fft(buffer, &mut output[..buffer_len]);
            self.output_scratch = output;
        } else {
            // Use time-domain convolution for short IRs
            let mut output = std::mem::take(&mut self.output_scratch);
            for i in 0..buffer_len {
                output[i] = 0.0;
            }
            self.convolve_time_domain(buffer, &mut output[..buffer_len]);
            self.output_scratch = output;
        }

        buffer.copy_from_slice(&self.output_scratch[..buffer_len]);
    }

    fn reset(&mut self) {
        self.buffer_pos = 0;
        self.partition_index = 0;
        for partition in &mut self.input_partitions {
            partition.fill(0.0);
        }
        self.input_buffer.fill(0.0);
        self.output_buffer.fill(0.0);

        // Reset FFT state buffers
        if let Some(state) = &mut self.fft_state {
            state.input_history_left.fill(0.0);
            state.input_history_right.fill(0.0);
            state.output_overlap_left.fill(0.0);
            state.output_overlap_right.fill(0.0);
        }
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        "Convolution"
    }
}

/// Errors that can occur during convolution operations
#[derive(Debug, Clone)]
pub enum ConvolutionError {
    /// The impulse response is empty
    EmptyImpulseResponse,
    /// Invalid channel count (must be 1 or 2)
    InvalidChannelCount(usize),
    /// File not found
    FileNotFound(String),
    /// Error reading WAV file
    WavReadError(String),
}

impl std::fmt::Display for ConvolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConvolutionError::EmptyImpulseResponse => write!(f, "Impulse response is empty"),
            ConvolutionError::InvalidChannelCount(c) => {
                write!(f, "Invalid channel count: {} (must be 1 or 2)", c)
            }
            ConvolutionError::FileNotFound(path) => write!(f, "File not found: {}", path),
            ConvolutionError::WavReadError(e) => write!(f, "Failed to read WAV file: {}", e),
        }
    }
}

impl std::error::Error for ConvolutionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convolution_engine_creation() {
        let engine = ConvolutionEngine::new();
        assert!(!engine.is_enabled());
        assert_eq!(engine.ir_length(), 0);
    }

    #[test]
    fn test_load_impulse_response() {
        let mut engine = ConvolutionEngine::new();

        // Create a simple stereo impulse response
        let ir = vec![1.0, 1.0, 0.5, 0.5, 0.25, 0.25, 0.125, 0.125];

        engine.load_impulse_response(&ir, 44100, 2).unwrap();
        assert!(engine.is_enabled());
        assert_eq!(engine.ir_length(), 4);
    }

    #[test]
    fn test_convolution_with_dirac() {
        let mut engine = ConvolutionEngine::new();

        // Dirac impulse (unit impulse) should pass signal through unchanged
        let ir = vec![1.0, 1.0]; // Single stereo sample at full amplitude
        engine.load_impulse_response(&ir, 44100, 2).unwrap();
        engine.set_dry_wet_mix(1.0);

        // Create test signal
        let mut buffer = vec![0.5, 0.5, 0.3, 0.3, 0.0, 0.0, 0.0, 0.0];
        let original = buffer.clone();

        engine.process(&mut buffer, 44100);

        // Output should be close to input for a Dirac impulse
        for (out, inp) in buffer.iter().zip(original.iter()) {
            assert!((out - inp).abs() < 0.01, "Expected ~{}, got {}", inp, out);
        }
    }

    #[test]
    fn test_dry_wet_mix() {
        let mut engine = ConvolutionEngine::new();
        let ir = vec![1.0, 1.0];
        engine.load_impulse_response(&ir, 44100, 2).unwrap();

        // Test dry/wet = 0 (fully dry)
        engine.set_dry_wet_mix(0.0);
        let mut buffer = vec![0.5, 0.5];
        engine.process(&mut buffer, 44100);
        assert!((buffer[0] - 0.5).abs() < 0.001); // Should be unchanged

        // Test dry/wet = 0.5 (50/50 mix)
        engine.set_dry_wet_mix(0.5);
        let mut buffer = vec![1.0, 1.0];
        engine.process(&mut buffer, 44100);
        // Mixed signal should be original
        assert!(buffer[0] >= 0.5 && buffer[0] <= 1.5);
    }

    #[test]
    fn test_reset() {
        let mut engine = ConvolutionEngine::new();
        let ir = vec![1.0, 1.0, 0.5, 0.5];
        engine.load_impulse_response(&ir, 44100, 2).unwrap();

        // Process some audio
        let mut buffer = vec![1.0; 128];
        engine.process(&mut buffer, 44100);

        // Reset
        engine.reset();

        // Internal state should be cleared
        assert_eq!(engine.buffer_pos, 0);
    }

    #[test]
    fn test_empty_ir_error() {
        let mut engine = ConvolutionEngine::new();
        let result = engine.load_impulse_response(&[], 44100, 2);
        assert!(matches!(
            result,
            Err(ConvolutionError::EmptyImpulseResponse)
        ));
    }

    #[test]
    fn test_invalid_channels_error() {
        let mut engine = ConvolutionEngine::new();
        let ir = vec![1.0, 1.0, 1.0];
        let result = engine.load_impulse_response(&ir, 44100, 3);
        assert!(matches!(
            result,
            Err(ConvolutionError::InvalidChannelCount(3))
        ));
    }
}
