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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Pre-buffer target: 5 seconds of output audio at target sample rate.
const BUFFER_SIZE_SECONDS: usize = 5;
/// Minimum samples in ring buffer before `is_ready()` returns `true`.
const MIN_BUFFER_SAMPLES: usize = 12_000;

// ── Commands sent to the decoder background thread ────────────────────────────

#[derive(Debug)]
enum DsdCommand {
    Seek(Duration),
    Stop,
}

// ── Shared atomic state ───────────────────────────────────────────────────────

struct SharedState {
    samples_produced: AtomicUsize,
    is_eof: AtomicBool,
    seek_pending: AtomicBool,
    generation: AtomicUsize, // incremented on seek to invalidate stale ring-buffer data
}

impl SharedState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            samples_produced: AtomicUsize::new(0),
            is_eof: AtomicBool::new(false),
            seek_pending: AtomicBool::new(false),
            generation: AtomicUsize::new(0),
        })
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
        })
    }
}

impl AudioSource for DsdAudioSource {
    fn read_samples(&mut self, buffer: &mut [f32]) -> Result<usize> {
        // If a seek completed, flush stale samples from the ring buffer.
        let gen = self.shared.generation.load(Ordering::Acquire);
        if gen != self.last_generation {
            while self.buffer_consumer.read_chunk(1).is_ok() {}
            self.last_generation = gen;
        }

        let available = self.buffer_consumer.slots();
        let to_read = buffer.len().min(available);

        if to_read == 0 {
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
            buffer[filled..].fill(0.0);
        }

        Ok(filled)
    }

    fn seek(&mut self, position: Duration) -> Result<()> {
        self.shared.seek_pending.store(true, Ordering::Release);
        self.command_tx
            .send(DsdCommand::Seek(position))
            .map_err(|_| PlaybackError::AudioSource("decoder thread disconnected".into()))?;
        Ok(())
    }

    fn duration(&self) -> Duration {
        self.duration
    }

    fn position(&self) -> Duration {
        // Derive from samples produced minus ring-buffer backlog.
        let produced = self.shared.samples_produced.load(Ordering::Relaxed) as u64;
        let backlog = self.buffer_consumer.slots() as u64;
        let consumed = produced.saturating_sub(backlog);
        // Convert interleaved sample count to frames, then to time.
        let frames = consumed / self.channels as u64;
        Duration::from_secs_f64(frames as f64 / self.target_rate as f64)
    }

    fn is_finished(&self) -> bool {
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
    const CHUNK_FRAMES: usize = 4096;
    // Frames fed to the resampler per call (must stay constant).
    const RESAMPLER_CHUNK: usize = 1024;

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
            interpolation: SincInterpolationType::Linear,
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
                DsdCommand::Seek(pos) => {
                    let target_sample = (pos.as_secs_f64() * dsd_rate as f64) as u64;
                    if let Err(e) = container.seek(target_sample) {
                        tracing::warn!("[DSD] seek error: {e}");
                    }
                    for f in &mut filters {
                        f.reset();
                    }
                    if let Some(ref mut r) = resampler {
                        let _ = r.reset();
                    }
                    input_buf.clear();
                    chunk_filled = 0;
                    shared.seek_pending.store(false, Ordering::Release);
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
        while chunk_filled < CHUNK_FRAMES {
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
                        chunk_filled = 0;
                    }
                    resample_and_flush(
                        &mut input_buf,
                        &mut resampler,
                        channels,
                        RESAMPLER_CHUNK,
                        &mut producer,
                        &shared,
                        true,
                    );
                    shared.is_eof.store(true, Ordering::Release);
                    // Park until Stop or a seek-to-restart.
                    loop {
                        match cmd_rx.recv() {
                            Err(_) | Ok(DsdCommand::Stop) => return,
                            Ok(DsdCommand::Seek(pos)) => {
                                shared.is_eof.store(false, Ordering::Release);
                                let target = (pos.as_secs_f64() * dsd_rate as f64) as u64;
                                let _ = container.seek(target);
                                for f in &mut filters {
                                    f.reset();
                                }
                                if let Some(ref mut r) = resampler {
                                    let _ = r.reset();
                                }
                                input_buf.clear();
                                shared.generation.fetch_add(1, Ordering::Release);
                                continue 'outer;
                            }
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
        );
    }
}

/// Drain `input_buf` through the resampler (or directly) into `producer`.
/// When `flush=true`, processes any leftover samples that don't fill a full chunk.
fn resample_and_flush(
    input_buf: &mut VecDeque<f32>,
    resampler: &mut Option<SincFixedIn<f32>>,
    channels: usize,
    chunk_frames: usize,
    producer: &mut Producer<f32>,
    shared: &SharedState,
    flush: bool,
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
                    flush_to_ring(&interleaved, producer, shared);
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
                    flush_to_ring(&interleaved, producer, shared);
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
        let expected = (blocks * 4096) as f64 / 2_822_400.0;
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
        let expected = samples as f64 / 2_822_400.0;
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
}
