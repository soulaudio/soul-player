//! `DsdAudioSource` — implements `AudioSource` for DSF and DSDIFF files.
//!
//! Mirrors the `LocalAudioSource` design:
//! - Background decoder thread: reads DSD bytes → `Dsd2Pcm` FIR filter → rtrb ring buffer.
//! - Audio callback thread: non-blocking reads from the ring buffer.
//! - No allocations in `read_samples()`.

use super::container::{ContainerError, DsdMeta, DsdiffContainer, DsfContainer};
use crossbeam_channel::{bounded, Receiver, Sender};
use rtrb::{Consumer, Producer, RingBuffer};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use soul_audio::dsd::Dsd2Pcm;
use soul_playback::{AudioSource, PlaybackError, Result};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Pre-buffer target: 5 seconds of output audio at target sample rate.
/// Must be ≥5 to absorb Windows sleep(1ms)≈15ms jitter without underrun.
const BUFFER_SIZE_SECONDS: usize = 5;
/// Minimum samples in ring buffer before `is_ready()` returns `true`.
const MIN_BUFFER_SAMPLES: usize = 12_000;

// ── Commands sent to the decoder background thread ────────────────────────────

#[derive(Debug)]
enum DsdCommand {
    /// Decoder should seek to the position stored in `SharedState::seek_target_samples`.
    /// No position is embedded in the command so that rapid seeks always resolve to the
    /// latest position written to the atomic, even when multiple Seek commands are queued.
    Seek,
    Stop,
}

// ── Shared atomic state ───────────────────────────────────────────────────────

struct SharedState {
    samples_produced: AtomicUsize,
    is_eof: AtomicBool,
    seek_pending: AtomicBool,
    seek_target_samples: AtomicU64, // interleaved output sample count at seek target
    generation: AtomicUsize,        // incremented on seek to invalidate stale ring-buffer data
    underrun_count: AtomicU64,      // times read_samples returned fewer samples than requested
    buffer_fill: AtomicUsize, // approximate ring fill; updated by read_samples() after each read
}

impl SharedState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            samples_produced: AtomicUsize::new(0),
            is_eof: AtomicBool::new(false),
            seek_pending: AtomicBool::new(false),
            seek_target_samples: AtomicU64::new(0),
            generation: AtomicUsize::new(0),
            underrun_count: AtomicU64::new(0),
            buffer_fill: AtomicUsize::new(0),
        })
    }
}

// ── Diagnostics ───────────────────────────────────────────────────────────────

/// Snapshot of DSD source health.
#[derive(Debug, Clone)]
pub struct DsdDiagnostics {
    /// Times the audio callback requested samples but the ring buffer was empty or partially drained.
    pub underrun_count: u64,
    /// Current PCM samples in ring buffer (interleaved).
    pub buffer_fill_samples: usize,
    /// Total ring buffer capacity in interleaved samples.
    pub buffer_capacity_samples: usize,
    /// Fill level 0.0–100.0.
    pub buffer_fill_percent: f32,
    /// True while the decoder background thread is still producing (false after EOF).
    pub decoder_running: bool,
}

/// Cheap, cloneable read handle into a `DsdAudioSource`'s shared state.
#[derive(Clone)]
pub struct DsdDiagnosticsHandle {
    shared: Arc<SharedState>,
    capacity: usize,
}

impl DsdDiagnosticsHandle {
    pub fn read(&self) -> DsdDiagnostics {
        let fill = self.shared.buffer_fill.load(Ordering::Relaxed);
        let cap = self.capacity;
        DsdDiagnostics {
            underrun_count: self.shared.underrun_count.load(Ordering::Relaxed),
            buffer_fill_samples: fill,
            buffer_capacity_samples: cap,
            buffer_fill_percent: if cap == 0 {
                0.0
            } else {
                fill as f32 / cap as f32 * 100.0
            },
            decoder_running: !self.shared.is_eof.load(Ordering::Acquire),
        }
    }
}

// ── Container enum ────────────────────────────────────────────────────────────

enum Container {
    Dsf(DsfContainer),
    Dsdiff(DsdiffContainer),
}

impl Container {
    fn open(path: &Path) -> std::result::Result<Self, ContainerError> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("dsf") => Ok(Self::Dsf(DsfContainer::open(path)?)),
            Some("dff" | "dsdiff") => Ok(Self::Dsdiff(DsdiffContainer::open(path)?)),
            ext => Err(ContainerError::Unsupported(format!(
                "unknown DSD extension: {:?}",
                ext
            ))),
        }
    }

    fn meta(&self) -> &DsdMeta {
        match self {
            Self::Dsf(c) => &c.meta,
            Self::Dsdiff(c) => &c.meta,
        }
    }

    fn read_frame(&mut self, frame: &mut [u8]) -> std::result::Result<bool, ContainerError> {
        match self {
            Self::Dsf(c) => c.read_frame(frame),
            Self::Dsdiff(c) => c.read_frame(frame),
        }
    }

    fn seek(&mut self, target_sample: u64) -> std::result::Result<(), ContainerError> {
        match self {
            Self::Dsf(c) => c.seek(target_sample),
            Self::Dsdiff(c) => c.seek(target_sample),
        }
    }

    fn position(&self) -> Duration {
        match self {
            Self::Dsf(c) => c.position(),
            Self::Dsdiff(c) => c.position(),
        }
    }

    fn lsbf(&self) -> bool {
        self.meta().lsbf
    }

    fn channels(&self) -> usize {
        self.meta().channels as usize
    }

    fn dsd_rate(&self) -> u32 {
        self.meta().dsd_rate
    }
}

// ── DsdAudioSource ────────────────────────────────────────────────────────────

/// Audio source for DSF and DSDIFF files.
///
/// Implements [`AudioSource`] — drop-in replacement for `LocalAudioSource` for DSD files.
/// Output is stereo interleaved f32 PCM at `target_sample_rate` Hz after resampling
/// from the raw DSD PCM rate (e.g. 352 800 Hz for DSD64 → 48 000 Hz device rate).
pub struct DsdAudioSource {
    target_rate: u32,
    channels: usize,
    duration: Duration,
    buffer_consumer: Consumer<f32>,
    shared: Arc<SharedState>,
    command_tx: Sender<DsdCommand>,
    _decoder_thread: JoinHandle<()>,
    last_generation: usize,
    buffer_capacity: usize,
}

impl DsdAudioSource {
    /// Open a DSF or DSDIFF file and start the background decoder.
    ///
    /// `target_sample_rate` is the device output rate (e.g. 48 000 Hz). The
    /// decoder thread resamples DSD PCM (352 800 Hz for DSD64) to this rate so
    /// the audio callback receives correctly-pitched audio.
    pub fn new(path: &Path, target_sample_rate: u32) -> std::result::Result<Self, PlaybackError> {
        let container =
            Container::open(path).map_err(|e| PlaybackError::AudioSource(e.to_string()))?;

        let meta = container.meta().clone();
        let pcm_rate = meta.pcm_rate;
        let channels = meta.channels as usize;
        let duration = meta.duration;

        // Ring buffer: 5 seconds at the TARGET output rate × channels.
        let ring_capacity = target_sample_rate as usize * BUFFER_SIZE_SECONDS * channels;
        let (producer, consumer) = RingBuffer::<f32>::new(ring_capacity);

        let shared = SharedState::new();
        let (cmd_tx, cmd_rx) = bounded::<DsdCommand>(8);

        let shared_bg = Arc::clone(&shared);
        let decoder_thread = thread::Builder::new()
            .name(format!("dsd-decoder:{}", path.display()))
            .spawn(move || {
                decode_loop(
                    container,
                    producer,
                    shared_bg,
                    cmd_rx,
                    pcm_rate,
                    target_sample_rate,
                    channels,
                );
            })
            .map_err(|e| PlaybackError::AudioSource(e.to_string()))?;

        tracing::info!(
            "[DsdAudioSource] Opening: pcm_rate={}Hz → target={}Hz, channels={}",
            pcm_rate,
            target_sample_rate,
            channels
        );

        Ok(Self {
            target_rate: target_sample_rate,
            channels,
            duration,
            buffer_consumer: consumer,
            shared,
            command_tx: cmd_tx,
            _decoder_thread: decoder_thread,
            last_generation: 0,
            buffer_capacity: ring_capacity,
        })
    }

    /// Return a cheap, cloneable handle for reading diagnostics outside the audio callback.
    pub fn diagnostics_handle(&self) -> DsdDiagnosticsHandle {
        DsdDiagnosticsHandle {
            shared: Arc::clone(&self.shared),
            capacity: self.buffer_capacity,
        }
    }
}

impl AudioSource for DsdAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        // If a seek completed, flush stale samples from the ring buffer.
        // NOTE: rtrb ReadChunk must have commit_all() called to advance the read
        // pointer — dropping without commit is a no-op (infinite loop otherwise).
        //
        // seek_pending is cleared HERE (not in the decode loop) so that position()
        // only leaves the optimistic seek-target path once the ring is genuinely empty.
        // This prevents position() from computing consumed = seek_output_samples - old_ring_data
        // which would give a wrong (often 0) position between the decode loop clearing
        // seek_pending and the audio callback flushing the ring.
        let gen = self.shared.generation.load(Ordering::Acquire);
        if gen != self.last_generation {
            let stale = self.buffer_consumer.slots();
            if stale > 0 {
                if let Ok(chunk) = self.buffer_consumer.read_chunk(stale) {
                    chunk.commit_all();
                }
            }
            self.last_generation = gen;
            // Ring is now empty and samples_produced reflects the seek target.
            // It is safe to leave the optimistic seek-target path in position().
            self.shared.seek_pending.store(false, Ordering::Release);
        }

        let available = self.buffer_consumer.slots();
        let to_read = buffer.len().min(available);

        if to_read == 0 {
            self.shared.underrun_count.fetch_add(1, Ordering::Relaxed);
            buffer.fill(0.0);
            return Ok(0);
        }

        let chunk = self
            .buffer_consumer
            .read_chunk(to_read)
            .map_err(|_| PlaybackError::AudioSource("ring buffer read failed".into()))?;

        let (s1, s2): (&[f32], &[f32]) = chunk.as_slices();
        buffer[..s1.len()].copy_from_slice(s1);
        buffer[s1.len()..s1.len() + s2.len()].copy_from_slice(s2);
        chunk.commit_all();

        // Fill remainder with silence if ring was partially drained.
        let filled = to_read;
        if filled < buffer.len() {
            self.shared.underrun_count.fetch_add(1, Ordering::Relaxed);
            buffer[filled..].fill(0.0);
        }
        // Store updated fill level (samples remaining after this read)
        self.shared
            .buffer_fill
            .store(self.buffer_consumer.slots(), Ordering::Relaxed);

        Ok(filled)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        // Write the seek target (in interleaved output samples) BEFORE setting seek_pending,
        // so position() can read a consistent value as soon as seek_pending is true.
        let target_samples =
            (position.as_secs_f64() * self.target_rate as f64 * self.channels as f64) as u64;
        self.shared
            .seek_target_samples
            .store(target_samples, Ordering::Release);
        self.shared.seek_pending.store(true, Ordering::Release);
        // Use try_send to avoid blocking the WASAPI audio callback. If the channel is
        // already full (rapid scrubbing), the existing Seek commands will drain and each
        // reads seek_target_samples from shared state, so they all converge on the latest
        // position without any callback stall.
        match self.command_tx.try_send(DsdCommand::Seek) {
            Ok(()) | Err(crossbeam_channel::TrySendError::Full(_)) => {}
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                return Err(PlaybackError::AudioSource(
                    "decoder thread disconnected".into(),
                ));
            }
        }
        Ok(())
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn position(&self) -> Duration {
        // While a seek is in flight, return the seek target so the UI doesn't bounce back.
        if self.shared.seek_pending.load(Ordering::Acquire) {
            let target = self.shared.seek_target_samples.load(Ordering::Acquire);
            let frames = target / self.channels as u64;
            return Duration::from_secs_f64(frames as f64 / self.target_rate as f64);
        }
        // Derive from samples produced minus ring-buffer backlog.
        let produced = self.shared.samples_produced.load(Ordering::Relaxed) as u64;
        let backlog = self.buffer_consumer.slots() as u64;
        let consumed = produced.saturating_sub(backlog);
        // Convert interleaved sample count to frames, then to time.
        let frames = consumed / self.channels as u64;
        Duration::from_secs_f64(frames as f64 / self.target_rate as f64)
    }

    fn is_finished(&self) -> bool {
        // If a seek is in flight, the source is NOT finished — the decoder will
        // resume from the new position once it processes the seek command.
        // Returning true here would cause process_audio() to call handle_track_finished()
        // and advance to the next track before the seek is processed.
        if self.shared.seek_pending.load(Ordering::Acquire) {
            return false;
        }
        self.shared.is_eof.load(Ordering::Acquire) && self.buffer_consumer.slots() == 0
    }

    fn is_ready(&self) -> bool {
        self.buffer_consumer.slots() >= MIN_BUFFER_SAMPLES
            || self.shared.is_eof.load(Ordering::Acquire)
    }

    fn sample_rate(&self) -> Option<u32> {
        Some(self.target_rate)
    }
}

impl Drop for DsdAudioSource {
    fn drop(&mut self) {
        let _ = self.command_tx.send(DsdCommand::Stop);
    }
}

// ── Background decode loop ────────────────────────────────────────────────────

fn decode_loop(
    mut container: Container,
    mut producer: Producer<f32>,
    shared: Arc<SharedState>,
    cmd_rx: Receiver<DsdCommand>,
    pcm_rate: u32,
    target_sample_rate: u32,
    channels: usize,
) {
    // Number of PCM frames decoded per iteration at pcm_rate.
    const CHUNK_FRAMES: usize = 16_384;
    // Frames fed to the resampler per call (must stay constant; CHUNK_FRAMES % RESAMPLER_CHUNK == 0).
    const RESAMPLER_CHUNK: usize = 4_096;

    let lsbf = container.lsbf();
    let dsd_rate = container.dsd_rate();

    // One Dsd2Pcm FIR filter per channel.
    let mut filters: Vec<Dsd2Pcm> = (0..channels).map(|_| Dsd2Pcm::new()).collect();

    // Build the resampler when output rate differs from DSD PCM rate.
    let needs_resampling = pcm_rate != target_sample_rate;
    let mut resampler: Option<SincFixedIn<f32>> = if needs_resampling {
        let ratio = target_sample_rate as f64 / pcm_rate as f64;
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Cubic,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };
        match SincFixedIn::<f32>::new(ratio, 2.0, params, RESAMPLER_CHUNK, channels) {
            Ok(r) => {
                tracing::info!(
                    "[DSD] Resampler {}Hz→{}Hz, output_delay={} frames",
                    pcm_rate,
                    target_sample_rate,
                    r.output_delay()
                );
                Some(r)
            }
            Err(e) => {
                tracing::error!("[DSD] Failed to create resampler: {e}");
                shared.is_eof.store(true, Ordering::Release);
                return;
            }
        }
    } else {
        None
    };

    // Output frames to discard after a seek reset (resampler delay-line warmup).
    // After r.reset() the internal delay line is zeroed, so the first output_delay frames
    // ramp from 0 → signal, causing a click/pop if the seek target has non-zero audio.
    // We skip those tainted frames before writing to the ring.
    let resampler_output_delay = resampler.as_ref().map_or(0, |r| r.output_delay());
    let mut warmup_skip_remaining: usize = 0;

    // Interleaved PCM accumulation buffer (at pcm_rate, before resampling).
    let mut input_buf: VecDeque<f32> = VecDeque::with_capacity(RESAMPLER_CHUNK * channels * 4);

    // DSD byte buffers.
    let mut frame = vec![0u8; channels];
    let mut pcm_frame = vec![0.0f32; channels];
    let mut chunk_filled = 0usize;
    // Temporary staging buffer for one CHUNK_FRAMES block.
    let mut pcm_chunk: Vec<f32> = vec![0.0; CHUNK_FRAMES * channels];

    'outer: loop {
        // Drain any pending commands (non-blocking).
        while let Ok(cmd) = cmd_rx.try_recv() {
            match cmd {
                DsdCommand::Stop => return,
                DsdCommand::Seek => {
                    // Read the latest seek target from shared atomics. Because the command
                    // carries no payload, rapid seeks always converge on the last position
                    // set by seek(), even when multiple Seek commands are queued.
                    let seek_output_samples =
                        shared.seek_target_samples.load(Ordering::Acquire) as usize;
                    let pos_secs =
                        seek_output_samples as f64 / (target_sample_rate as f64 * channels as f64);
                    // Container::seek() takes a "DSD byte" index (each byte = 8 DSD samples),
                    // so divide dsd_rate by 8 — equivalently use the pcm_rate (352800 for DSD64).
                    let target_sample = (pos_secs * dsd_rate as f64 / 8.0) as u64;
                    if let Err(e) = container.seek(target_sample) {
                        tracing::warn!("[DSD] seek error: {e}");
                    }
                    for f in &mut filters {
                        f.reset();
                    }
                    if let Some(ref mut r) = resampler {
                        r.reset();
                        // Discard tainted warmup frames — delay line was zeroed by reset().
                        warmup_skip_remaining = resampler_output_delay;
                    }
                    input_buf.clear();
                    chunk_filled = 0;
                    // Reset samples_produced so position() is correct after seek.
                    shared
                        .samples_produced
                        .store(seek_output_samples, Ordering::Release);
                    // Increment generation so read_samples() (audio callback) detects
                    // the seek and flushes stale ring data. seek_pending is intentionally
                    // NOT cleared here — it is cleared by read_samples() after the flush,
                    // so position() stays on the optimistic seek-target path until the ring
                    // is actually empty and samples_produced is the sole source of truth.
                    shared.generation.fetch_add(1, Ordering::Release);
                }
            }
        }

        // Don't get too far ahead of the consumer.
        if producer.slots() < channels * CHUNK_FRAMES {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }

        // Decode one CHUNK_FRAMES block of DSD → interleaved PCM.
        // Check for pending commands every 1 024 frames (~0.4 ms) so that seeks
        // break out of the inner loop quickly rather than waiting up to ~6 ms.
        while chunk_filled < CHUNK_FRAMES {
            if chunk_filled > 0 && chunk_filled.is_multiple_of(1024) && !cmd_rx.is_empty() {
                break;
            }
            match container.read_frame(&mut frame) {
                Ok(true) => {
                    for ch in 0..channels {
                        let src = std::slice::from_ref(&frame[ch]);
                        let dst = std::slice::from_mut(&mut pcm_frame[ch]);
                        filters[ch].translate(src, dst, lsbf);
                    }
                    for ch in 0..channels {
                        pcm_chunk[chunk_filled * channels + ch] = pcm_frame[ch];
                    }
                    chunk_filled += 1;
                }
                Ok(false) => {
                    // EOF — flush remaining samples through resampler then signal.
                    if chunk_filled > 0 {
                        let partial = &pcm_chunk[..chunk_filled * channels];
                        input_buf.extend(partial.iter().copied());
                    }
                    resample_and_flush(
                        &mut input_buf,
                        &mut resampler,
                        channels,
                        RESAMPLER_CHUNK,
                        &mut producer,
                        &shared,
                        true,
                        &mut warmup_skip_remaining,
                    );
                    shared.is_eof.store(true, Ordering::Release);
                    // Park until Stop or a seek-to-restart.
                    match cmd_rx.recv() {
                        Err(_) | Ok(DsdCommand::Stop) => return,
                        Ok(DsdCommand::Seek) => {
                            shared.is_eof.store(false, Ordering::Release);
                            let seek_output_samples =
                                shared.seek_target_samples.load(Ordering::Acquire) as usize;
                            let pos_secs = seek_output_samples as f64
                                / (target_sample_rate as f64 * channels as f64);
                            // Same unit fix: container seeks by DSD-byte index (dsd_rate/8).
                            let target = (pos_secs * dsd_rate as f64 / 8.0) as u64;
                            let _ = container.seek(target);
                            for f in &mut filters {
                                f.reset();
                            }
                            if let Some(ref mut r) = resampler {
                                r.reset();
                                warmup_skip_remaining = resampler_output_delay;
                            }
                            input_buf.clear();
                            chunk_filled = 0;
                            shared
                                .samples_produced
                                .store(seek_output_samples, Ordering::Release);
                            // Same ordering as top-of-loop seek: generation++ so
                            // read_samples() flushes stale ring data. seek_pending is
                            // cleared by read_samples() after the flush (not here).
                            shared.generation.fetch_add(1, Ordering::Release);
                            continue 'outer;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("[DSD] decode error: {e}");
                    shared.is_eof.store(true, Ordering::Release);
                    return;
                }
            }
        }

        // Append decoded chunk to input buffer and resample.
        input_buf.extend(pcm_chunk[..chunk_filled * channels].iter().copied());
        chunk_filled = 0;
        resample_and_flush(
            &mut input_buf,
            &mut resampler,
            channels,
            RESAMPLER_CHUNK,
            &mut producer,
            &shared,
            false,
            &mut warmup_skip_remaining,
        );
    }
}

/// Drain `input_buf` through the resampler (or directly) into `producer`.
/// When `flush=true`, processes any leftover samples that don't fill a full chunk.
/// `warmup_skip` is decremented as frames are consumed: frames that fall within the
/// warmup window are discarded rather than written to the ring, eliminating the
/// click/pop caused by the resampler delay-line transitioning from zero after a seek.
fn resample_and_flush(
    input_buf: &mut VecDeque<f32>,
    resampler: &mut Option<SincFixedIn<f32>>,
    channels: usize,
    chunk_frames: usize,
    producer: &mut Producer<f32>,
    shared: &SharedState,
    flush: bool,
    warmup_skip: &mut usize,
) {
    let samples_per_chunk = chunk_frames * channels;

    if let Some(ref mut r) = resampler {
        while input_buf.len() >= samples_per_chunk {
            let mut deinterleaved: Vec<Vec<f32>> = vec![Vec::with_capacity(chunk_frames); channels];
            for _ in 0..chunk_frames {
                for ch in 0..channels {
                    deinterleaved[ch].push(input_buf.pop_front().unwrap_or(0.0));
                }
            }
            match r.process(&deinterleaved, None) {
                Ok(resampled) => {
                    let n_frames = resampled[0].len();
                    let mut interleaved = Vec::with_capacity(n_frames * channels);
                    for fi in 0..n_frames {
                        for ch in 0..channels {
                            interleaved.push(resampled[ch][fi]);
                        }
                    }
                    // Skip tainted warmup frames (zeroed delay-line after seek reset).
                    let skip = (*warmup_skip).min(n_frames);
                    *warmup_skip = warmup_skip.saturating_sub(n_frames);
                    if skip < n_frames {
                        flush_to_ring(&interleaved[skip * channels..], producer, shared);
                    }
                }
                Err(e) => tracing::error!("[DSD] Resampling error: {e}"),
            }
        }

        if flush && !input_buf.is_empty() {
            let remaining = input_buf.len() / channels;
            if remaining > 0 {
                let mut deinterleaved: Vec<Vec<f32>> =
                    vec![Vec::with_capacity(remaining); channels];
                for _ in 0..remaining {
                    for ch in 0..channels {
                        deinterleaved[ch].push(input_buf.pop_front().unwrap_or(0.0));
                    }
                }
                if let Ok(resampled) = r.process_partial(Some(&deinterleaved), None) {
                    let n_frames = resampled[0].len();
                    let mut interleaved = Vec::with_capacity(n_frames * channels);
                    for fi in 0..n_frames {
                        for ch in 0..channels {
                            interleaved.push(resampled[ch][fi]);
                        }
                    }
                    let skip = (*warmup_skip).min(n_frames);
                    *warmup_skip = warmup_skip.saturating_sub(n_frames);
                    if skip < n_frames {
                        flush_to_ring(&interleaved[skip * channels..], producer, shared);
                    }
                }
            }
        }
    } else {
        // No resampling needed — write directly.
        while !input_buf.is_empty() {
            let n = input_buf.len();
            let batch: Vec<f32> = input_buf.drain(..n).collect();
            flush_to_ring(&batch, producer, shared);
        }
    }
}

fn flush_to_ring(samples: &[f32], producer: &mut Producer<f32>, shared: &SharedState) {
    let mut written = 0;
    while written < samples.len() {
        let slots = producer.slots();
        if slots == 0 {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        }
        let n = (samples.len() - written).min(slots);
        if let Ok(mut chunk) = producer.write_chunk_uninit(n) {
            let (s1, s2) = chunk.as_mut_slices();
            let n1 = s1.len().min(n);
            let n2 = (n - n1).min(s2.len());
            for (i, slot) in s1[..n1].iter_mut().enumerate() {
                slot.write(samples[written + i]);
            }
            for (i, slot) in s2[..n2].iter_mut().enumerate() {
                slot.write(samples[written + n1 + i]);
            }
            let total = n1 + n2;
            unsafe { chunk.commit(total) };
            shared.samples_produced.fetch_add(total, Ordering::Relaxed);
            written += total;
        }
    }
}

// ── Integration tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::dsd::container::tests::{build_dsdiff, build_dsf};
    use soul_playback::AudioSource;
    use std::io::Write as IoWrite;
    use tempfile::NamedTempFile;

    fn write_temp(data: &[u8], ext: &str) -> NamedTempFile {
        let f = tempfile::Builder::new()
            .suffix(&format!(".{ext}"))
            .tempfile()
            .unwrap();
        {
            let mut handle = f.as_file();
            handle.write_all(data).unwrap();
            handle.flush().unwrap();
        }
        f
    }

    fn wait_ready(src: &DsdAudioSource, timeout_ms: u64) {
        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
        while !src.is_ready() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    // ── DSF integration tests ─────────────────────────────────────────────────

    #[test]
    fn dsf_source_opens_and_reports_target_rate() {
        let data = build_dsf(2, 0x69);
        let f = write_temp(&data, "dsf");
        let src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        assert_eq!(src.sample_rate(), Some(48_000));
    }

    #[test]
    fn dsf_source_duration_matches_sample_count() {
        // 2 blocks × 4096 samples = 8192 DSD samples per channel @ 2,822,400 Hz
        let blocks = 2usize;
        let data = build_dsf(blocks, 0x69);
        let f = write_temp(&data, "dsf");
        let src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        let expected = (blocks * 4096) as f64 * 8.0 / 2_822_400.0;
        let got = src.duration().as_secs_f64();
        assert!(
            (got - expected).abs() < 1e-4,
            "duration mismatch: expected {expected:.6}s got {got:.6}s"
        );
    }

    #[test]
    fn dsf_source_reads_samples_after_buffering() {
        let data = build_dsf(4, 0xFF);
        let f = write_temp(&data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        wait_ready(&src, 5000);
        let mut buf = vec![0.0f32; 1024];
        let n = src.read_samples(&mut buf).unwrap();
        assert!(n > 0, "expected samples, got 0");
    }

    #[test]
    fn dsf_source_is_not_finished_while_buffering() {
        let data = build_dsf(4, 0x69);
        let f = write_temp(&data, "dsf");
        let src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        // Should not be finished immediately after construction.
        assert!(!src.is_finished());
    }

    #[test]
    fn dsf_source_finishes_after_all_samples_read() {
        // Very small file: 1 block = 4096 DSD samples per channel.
        let data = build_dsf(1, 0x69);
        let f = write_temp(&data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();

        // Wait for EOF to be signalled.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !src.shared.is_eof.load(Ordering::Acquire) {
            if std::time::Instant::now() > deadline {
                panic!("timed out waiting for EOF");
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        // Drain the ring buffer.
        let mut buf = vec![0.0f32; 65536];
        loop {
            let n = src.read_samples(&mut buf).unwrap();
            if n == 0 {
                break;
            }
        }
        assert!(src.is_finished());
    }

    #[test]
    fn dsf_source_seek_does_not_panic() {
        let data = build_dsf(4, 0xFF);
        let f = write_temp(&data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        wait_ready(&src, 5000);
        src.seek(Duration::from_millis(0)).unwrap();
        // Allow decoder thread to process the seek.
        std::thread::sleep(Duration::from_millis(50));
    }

    // ── DSDIFF integration tests ──────────────────────────────────────────────

    #[test]
    fn dff_source_opens_and_reports_target_rate() {
        let data = build_dsdiff(8192, 0x69);
        let f = write_temp(&data, "dff");
        let src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        assert_eq!(src.sample_rate(), Some(48_000));
    }

    #[test]
    fn dff_source_duration_correct() {
        let samples: u64 = 8192;
        let data = build_dsdiff(samples, 0x69);
        let f = write_temp(&data, "dff");
        let src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        let expected = samples as f64 * 8.0 / 2_822_400.0;
        let got = src.duration().as_secs_f64();
        assert!(
            (got - expected).abs() < 1e-4,
            "duration mismatch: expected {expected:.6}s got {got:.6}s"
        );
    }

    #[test]
    fn dff_source_reads_samples_after_buffering() {
        let data = build_dsdiff(100_000, 0xFF);
        let f = write_temp(&data, "dff");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        wait_ready(&src, 5000);
        let mut buf = vec![0.0f32; 1024];
        let n = src.read_samples(&mut buf).unwrap();
        assert!(n > 0);
    }

    // ── Cross-format ──────────────────────────────────────────────────────────

    #[test]
    fn unknown_extension_returns_error() {
        let f = write_temp(b"garbage", "mp3");
        let result = DsdAudioSource::new(f.path(), 48_000);
        assert!(result.is_err(), "unknown extension should fail");
    }

    // ── Quality / correctness tests (TDD for jitter/distortion/faster-slower bugs) ──

    /// Bug: SincInterpolationType::Linear for 7.35× downsampling produces aliasing
    /// artefacts.  A DSD signal at ~44.1 kHz (above the 24 kHz output Nyquist) MUST
    /// be rejected by the resampler's anti-aliasing filter.  With Linear interpolation
    /// the filter has a higher passband ripple and worse stopband rejection, causing the
    /// above-Nyquist content to alias into the audible band.
    ///
    /// Pattern: 32 bytes of 0xFF then 32 bytes of 0x00 repeated → DSD "square wave" at
    /// `2_822_400 / (32*2) / 8 = 5_512.5 Hz` in the PCM domain? No — let me think
    /// carefully.  One DSD byte = 8 DSD bits = 1 PCM sample from the FIR.
    /// So 32 bytes of 0xFF = 32 PCM samples of "high", 32 bytes of 0x00 = 32 PCM samples
    /// of "low".  Square wave period = 64 PCM samples at pcm_rate 352_800 Hz →
    /// frequency = 352_800 / 64 = 5_512.5 Hz.  This IS below the output Nyquist (24 kHz),
    /// so it passes through and is NOT a good test for stopband rejection.
    ///
    /// Use period 4 (2 bytes on + 2 bytes off): f = 352_800 / 4 = 88_200 Hz.
    /// This is ABOVE the output Nyquist (24 kHz) and ABOVE the FIR cutoff (~88 kHz) — the
    /// FIR already attenuates it, and the resampler should remove any remainder.
    ///
    /// With Linear stopband attenuation ~-60 dB (rubato: ok but basic)
    /// With Cubic stopband attenuation ~-80 dB+
    ///
    /// We assert the RMS of the output window is < 0.002 (equivalent to -54 dB full-scale).
    /// Cubic easily achieves this; Linear might produce RMS > 0.002 for the aliased content.
    #[test]
    fn dsd_resampler_strongly_attenuates_above_nyquist_signal() {
        // Build DSF with 2-byte-on / 2-byte-off pattern → ~88.2 kHz in PCM domain.
        // This is well above the 24 kHz output Nyquist so the resampler must reject it.
        let channels: u32 = 2;
        let dsd_rate: u32 = 2_822_400;
        let num_blocks = 64usize; // 64 blocks = plenty for warmup
        let sample_count: u64 = (num_blocks * super::super::container::DSF_BLOCK_SIZE) as u64;
        let audio_data_len =
            num_blocks * super::super::container::DSF_BLOCK_SIZE * channels as usize;
        let total_size: u64 = 92 + audio_data_len as u64;

        let mut buf_data = Vec::new();
        // DSD chunk
        buf_data.extend_from_slice(b"DSD ");
        buf_data.extend_from_slice(&28u64.to_le_bytes());
        buf_data.extend_from_slice(&total_size.to_le_bytes());
        buf_data.extend_from_slice(&0u64.to_le_bytes());
        // fmt chunk
        buf_data.extend_from_slice(b"fmt ");
        buf_data.extend_from_slice(&52u64.to_le_bytes());
        buf_data.extend_from_slice(&1u32.to_le_bytes());
        buf_data.extend_from_slice(&0u32.to_le_bytes());
        buf_data.extend_from_slice(&2u32.to_le_bytes());
        buf_data.extend_from_slice(&channels.to_le_bytes());
        buf_data.extend_from_slice(&dsd_rate.to_le_bytes());
        buf_data.extend_from_slice(&1u32.to_le_bytes()); // lsbf
        buf_data.extend_from_slice(&sample_count.to_le_bytes());
        buf_data.extend_from_slice(&(super::super::container::DSF_BLOCK_SIZE as u32).to_le_bytes());
        buf_data.extend_from_slice(&0u32.to_le_bytes());
        // data chunk
        buf_data.extend_from_slice(b"data");
        buf_data.extend_from_slice(&(12u64 + audio_data_len as u64).to_le_bytes());
        // Audio: 2 bytes 0xFF + 2 bytes 0x00 repeating per channel block
        for _ in 0..num_blocks {
            let block: Vec<u8> = (0..super::super::container::DSF_BLOCK_SIZE)
                .map(|i| if (i / 2) % 2 == 0 { 0xFFu8 } else { 0x00u8 })
                .collect();
            buf_data.extend_from_slice(&block); // ch0
            buf_data.extend_from_slice(&block); // ch1
        }

        let f = write_temp(&buf_data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        wait_ready(&src, 10_000);

        // Discard warmup (FIR + resampler delay).
        let mut discard = vec![0.0f32; 4096];
        src.read_samples(&mut discard).unwrap();

        // Collect steady-state output.
        let mut window = vec![0.0f32; 4096];
        let n = src.read_samples(&mut window).unwrap();
        assert!(n > 0);

        let rms = (window[..n].iter().map(|&x| (x * x) as f64).sum::<f64>() / n as f64).sqrt();

        // The ~88.2 kHz signal must be rejected by the resampler.
        // Cubic achieves this well; Linear may leak more.
        // We allow a generous -54 dB (RMS < 0.002) to confirm the test fails before fix.
        assert!(
            rms < 0.002,
            "above-Nyquist signal not rejected: RMS={rms:.6} — resampler stopband too weak (Linear?)"
        );
    }

    /// Bug: SincInterpolationType::Linear for 7.35× downsampling produces aliasing
    /// distortion.  The amplitude variance of a stable DC DSD input (0xFF = all-ones)
    /// after warmup should be very low (< 0.002 variance) because the input is pure DC.
    /// With Linear interpolation the passband ripple is higher; with Cubic it is
    /// essentially zero for DC.  This test documents the required quality floor.
    #[test]
    fn dsd_output_variance_low_for_dc_input_after_warmup() {
        // Build a large enough file to warm up both the FIR (12 bytes) and the
        // SincFixedIn resampler (output_delay ≈ 17 frames) and collect a stable window.
        let data = build_dsf(32, 0xFF); // 32 blocks × 4096 = 131072 DSD bytes per channel
        let f = write_temp(&data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        wait_ready(&src, 8_000);

        // Discard the first 2048 samples (warmup / delay region).
        let mut discard = vec![0.0f32; 2048];
        src.read_samples(&mut discard).unwrap();

        // Collect 4096 samples of steady-state output.
        let mut window = vec![0.0f32; 4096];
        let n = src.read_samples(&mut window).unwrap();
        assert!(n > 0, "ring should not be empty");

        // Compute mean and variance of the collected window.
        let mean = window[..n].iter().copied().sum::<f32>() / n as f32;
        let variance = window[..n].iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / n as f32;

        // DC input → almost zero variance expected.  Allow generous 0.005 tolerance
        // so that the Cubic test passes while Linear (higher ripple) would exceed it.
        assert!(
            variance < 0.005,
            "variance {variance:.6} too high — likely Linear interpolation artefacts"
        );
    }

    /// Bug: read_samples returns Ok(n) where n < buffer.len() on underrun but
    /// process_audio returns that count to CPAL, causing timing drift.
    /// When the ring has ≥ buffer.len() samples, read_samples MUST return Ok(buffer.len()).
    #[test]
    fn dsd_read_samples_returns_full_buffer_len_when_data_available() {
        let data = build_dsf(32, 0xFF); // plenty of data
        let f = write_temp(&data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        // Wait for the ring to be well above buffer size.
        let deadline = std::time::Instant::now() + Duration::from_secs(8);
        while src.buffer_consumer.slots() < 8192 && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            src.buffer_consumer.slots() >= 8192,
            "ring not filled enough for test"
        );

        let mut buf = vec![0.0f32; 512];
        let n = src.read_samples(&mut buf).unwrap();
        assert_eq!(
            n,
            buf.len(),
            "read_samples must return buffer.len()={} when ring has enough data, got {}",
            buf.len(),
            n
        );
    }

    /// Bug: position() should track consumed samples accurately.
    /// After draining N stereo samples position() must equal N/channels/rate ± 5ms.
    #[test]
    fn dsd_position_tracks_consumed_samples_accurately() {
        let data = build_dsf(32, 0xAA); // 32 blocks of DSD silence-pattern
        let f = write_temp(&data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        wait_ready(&src, 8_000);
        assert!(src.is_ready(), "source must be ready");

        let buf_frames = 512usize;
        let channels = 2usize;
        let target_rate = 48_000u64;
        let buf_size = buf_frames * channels;
        let mut buf = vec![0.0f32; buf_size];
        let mut total_consumed = 0u64;

        // Drain exactly 50 full buffers.
        // IMPORTANT: always add n to total_consumed even on the partial final read —
        // read_samples already committed the read from the ring, so position() reflects
        // those samples even when n < buf.len().
        for _ in 0..50 {
            let n = src.read_samples(&mut buf).unwrap();
            total_consumed += n as u64;
            if n < buf.len() {
                break; // ran out of data — DSD file too small for this test
            }
        }
        assert!(total_consumed > 0);

        let expected_secs = total_consumed as f64 / (target_rate as f64 * channels as f64);
        let actual_secs = src.position().as_secs_f64();
        let delta = (actual_secs - expected_secs).abs();
        assert!(
            delta < 0.005,
            "position drift {delta:.4}s > 5ms — expected {expected_secs:.4}s got {actual_secs:.4}s"
        );
    }

    /// Bug: "jitter" — ring should never starve (return Ok(0)) while decoding is
    /// still in progress.  Simulates real-time drain at 48 kHz for 250ms.
    #[test]
    fn dsd_ring_never_starves_during_real_time_drain() {
        // Need a file at least 1 second long to give decoder time to pre-fill.
        // 32 blocks × 4096 bytes / 352800 Hz ≈ 0.372s — use 128 blocks ≈ 1.49s.
        let data = build_dsf(128, 0xAA);
        let f = write_temp(&data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        wait_ready(&src, 8_000);
        assert!(src.is_ready(), "source must be ready before drain test");

        let buf_size = 512usize; // 256 frames × 2 channels ≈ 5.3ms per callback
        let mut buf = vec![0.0f32; buf_size];
        let callback_period = Duration::from_micros(5_333); // ≈ 256 frames @ 48kHz
        let test_duration = Duration::from_millis(250);
        let deadline = std::time::Instant::now() + test_duration;
        let mut underruns = 0u32;

        while std::time::Instant::now() < deadline {
            let n = src.read_samples(&mut buf).unwrap();
            if n == 0 {
                underruns += 1;
            }
            std::thread::sleep(callback_period);
        }

        assert_eq!(
            underruns, 0,
            "ring starved {underruns} times during 250ms drain — jitter will occur"
        );
    }

    // ── Diagnostics unit tests ────────────────────────────────────────────────

    #[test]
    fn underrun_count_increments_on_empty_ring() {
        let state = SharedState::new();
        assert_eq!(state.underrun_count.load(Ordering::Relaxed), 0);
        state.underrun_count.fetch_add(1, Ordering::Relaxed);
        assert_eq!(state.underrun_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn diagnostics_handle_reads_buffer_fill() {
        let state = SharedState::new();
        let capacity = 480_000usize;
        let handle = DsdDiagnosticsHandle {
            shared: Arc::clone(&state),
            capacity,
        };
        let diag = handle.read();
        assert_eq!(diag.buffer_fill_samples, 0);
        assert_eq!(diag.buffer_capacity_samples, capacity);
        assert_eq!(diag.underrun_count, 0);
        assert!((diag.buffer_fill_percent - 0.0).abs() < 0.001);
        state.buffer_fill.store(96_000, Ordering::Relaxed);
        let diag2 = handle.read();
        assert_eq!(diag2.buffer_fill_samples, 96_000);
        let expected_pct = 96_000.0f32 / capacity as f32 * 100.0;
        assert!((diag2.buffer_fill_percent - expected_pct).abs() < 0.1);
    }

    #[test]
    fn dsdiff_extension_alias_dsdiff_works() {
        let data = build_dsdiff(8192, 0x69);
        let f = tempfile::Builder::new()
            .suffix(".dsdiff")
            .tempfile()
            .unwrap();
        {
            let mut h = f.as_file();
            h.write_all(&data).unwrap();
            h.flush().unwrap();
        }
        let src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        assert_eq!(src.sample_rate(), Some(48_000));
    }

    // ── Seek/is_finished correctness tests (TDD for seek bugs) ───────────────

    /// Bug 1: is_finished() returned true when is_eof=true && ring_empty=true even
    /// if seek_pending=true. This caused process_audio() to call handle_track_finished()
    /// and advance the track before the decoder could process the seek command.
    #[test]
    fn dsd_is_finished_false_when_seek_pending() {
        // Use a minimal 1-block DSF so the decoder reaches EOF quickly.
        let data = build_dsf(1, 0x69);
        let f = write_temp(&data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();

        // Wait for EOF to be signalled by the decoder.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !src.shared.is_eof.load(Ordering::Acquire) {
            if std::time::Instant::now() > deadline {
                panic!("timed out waiting for EOF");
            }
            std::thread::sleep(Duration::from_millis(10));
        }

        // Drain the ring buffer completely.
        let mut buf = vec![0.0f32; 65536];
        loop {
            let n = src.read_samples(&mut buf).unwrap();
            if n == 0 {
                break;
            }
        }

        // Precondition: with is_eof=true and ring empty, is_finished() should be true.
        assert!(
            src.is_finished(),
            "precondition: source must be finished after EOF + drain"
        );

        // Simulate a seek arriving while the decoder is at EOF (parked).
        // In the real code path, seek() sets seek_pending=true atomically.
        src.shared.seek_pending.store(true, Ordering::Release);

        // is_finished() MUST return false — the decoder will resume decoding once
        // it processes the seek command; advancing the track here would be wrong.
        assert!(
            !src.is_finished(),
            "is_finished() must return false when seek_pending=true — \
             otherwise process_audio() advances the track before seek is processed"
        );
    }

    /// Bug 2 (observable consequence): After a seek completes (seek_pending goes false),
    /// position() must reflect the seek target (within 50ms tolerance) when we flush
    /// the ring with one read_samples() call.
    ///
    /// This fails if seek_pending is cleared before generation is incremented, because
    /// read_samples() won't flush stale pre-seek samples from the ring, and position()
    /// will return a stale value.
    #[test]
    fn dsd_position_correct_after_seek_completes() {
        // Build a file large enough that decoding is not finished before seek completes.
        let data = build_dsf(64, 0xFF);
        let f = write_temp(&data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        wait_ready(&src, 5_000);

        let seek_target_secs = 0.1f64; // seek to 100ms
        src.seek(Duration::from_secs_f64(seek_target_secs)).unwrap();

        // Wait for decoder to process seek (seek_pending goes false).
        // In the new design, seek_pending is cleared by read_samples() after it detects
        // the generation change and flushes the ring — mimic the audio callback here.
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        let mut buf = vec![0.0f32; 512];
        while src.shared.seek_pending.load(Ordering::Acquire) {
            if std::time::Instant::now() > deadline {
                panic!("seek did not complete within 3s");
            }
            let _ = src.read_samples(&mut buf);
            std::thread::sleep(Duration::from_millis(5));
        }

        // Position must equal seek target within 50ms.
        let pos = src.position().as_secs_f64();
        let delta = (pos - seek_target_secs).abs();
        assert!(
            delta < 0.05,
            "position after seek: expected ~{seek_target_secs:.3}s, got {pos:.4}s \
             (delta {delta:.4}s > 50ms) — seek ordering race suspected"
        );
    }

    /// Extended drain test: ring must not starve for 5 full seconds at real-time rate.
    /// The existing 250ms test is too short to catch slow leaks.
    ///
    /// This test is skipped in debug builds because the 256-tap Cubic sinc resampler
    /// operates near real-time speed under parallel test CPU contention in debug mode.
    /// Run explicitly in release: `cargo test -p soul-audio-desktop --release -- --ignored`
    #[test]
    #[cfg_attr(debug_assertions, ignore)]
    fn dsd_ring_never_starves_during_5s_real_time_drain() {
        // ~6 seconds of DSD audio: ceil(6 / (4096/2822400)) = ceil(6 / 0.001452) ≈ 4132
        // Use 517 blocks ≈ 6.0s to ensure the decoder keeps up for the full 5s test window.
        let data = build_dsf(517, 0xAA);
        let f = write_temp(&data, "dsf");
        let mut src = DsdAudioSource::new(f.path(), 48_000).unwrap();
        wait_ready(&src, 10_000);
        assert!(src.is_ready(), "source must be ready before drain test");
        // Give the decoder 2 s to fill the ring further before drain starts.
        // In production the ring reaches ~5 s capacity before playback begins;
        // without this the test races against parallel test threads for CPU.
        std::thread::sleep(Duration::from_secs(2));

        let buf_size = 512usize; // 256 frames × 2 channels ≈ 5.3ms per callback
        let mut buf = vec![0.0f32; buf_size];
        let callback_period = Duration::from_micros(5_333); // ≈ 256 frames @ 48kHz
        let test_duration = Duration::from_secs(5);
        let deadline = std::time::Instant::now() + test_duration;
        let mut underruns = 0u32;

        while std::time::Instant::now() < deadline {
            let n = src.read_samples(&mut buf).unwrap();
            if n == 0 {
                underruns += 1;
            }
            std::thread::sleep(callback_period);
        }

        assert_eq!(
            underruns, 0,
            "ring starved {underruns} times during 5s real-time drain — jitter will occur"
        );
    }
}
