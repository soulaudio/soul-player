//! DSD container abstraction over DSF and DSDIFF file formats.
//!
//! Both formats expose the same interface to `DsdAudioSource`:
//! - Audio format parameters (rate, channels, lsbf, duration)
//! - Seeking by sample position
//! - Streaming DSD bytes as interleaved channel frames
//!   (one byte per channel per call to `read_frame`)

use soul_playback::PlaybackError;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::Duration;

// ── Shared types ─────────────────────────────────────────────────────────────

/// DSD container error.
#[derive(Debug, thiserror::Error)]
pub enum ContainerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("DSF parse error: {0}")]
    DsfParse(String),
    #[error("DSDIFF parse error: {0}")]
    DsdiffParse(String),
    #[error("Unsupported: {0}")]
    Unsupported(String),
}

impl From<ContainerError> for PlaybackError {
    fn from(e: ContainerError) -> Self {
        PlaybackError::AudioSource(e.to_string())
    }
}

/// Extracted metadata from a DSD container.
#[derive(Debug, Clone)]
pub struct DsdMeta {
    /// DSD bitstream sample rate (e.g. 2,822,400 for DSD64).
    pub dsd_rate: u32,
    /// PCM output rate after 8:1 FIR decimation (= dsd_rate / 8).
    pub pcm_rate: u32,
    /// Number of channels.
    pub channels: u16,
    /// `true` = LSB-first (DSF default), `false` = MSB-first (DSDIFF default).
    pub lsbf: bool,
    /// Total playback duration.
    pub duration: Duration,
    /// Human-readable DSD format string, e.g. "DSD64".
    pub dsd_format: String,
}

impl DsdMeta {
    /// Derive PCM output rate and format name from DSD bitstream rate.
    pub fn from_dsd_rate(dsd_rate: u32, channels: u16, lsbf: bool, sample_count: u64) -> Self {
        let pcm_rate = dsd_rate / 8;
        let duration_secs = sample_count as f64 / dsd_rate as f64;
        let duration = Duration::from_secs_f64(duration_secs);
        let dsd_format = match dsd_rate {
            2_822_400 => "DSD64".to_string(),
            5_644_800 => "DSD128".to_string(),
            11_289_600 => "DSD256".to_string(),
            22_579_200 => "DSD512".to_string(),
            r => format!("DSD@{}Hz", r),
        };
        Self {
            dsd_rate,
            pcm_rate,
            channels,
            lsbf,
            duration,
            dsd_format,
        }
    }
}

// ── DSF container ─────────────────────────────────────────────────────────────

/// DSF block size — fixed by the DSF specification.
const DSF_BLOCK_SIZE: usize = 4096;

/// DSF container reader.
///
/// DSF audio data is **block-interleaved**: all bytes for channel 0 come first
/// in a 4096-byte block, then channel 1, etc., then the pattern repeats.
/// This reader deinterleaves on the fly, yielding one byte per channel per call
/// to `read_frame`, matching the DSDIFF byte-interleaved convention.
pub struct DsfContainer {
    pub meta: DsdMeta,
    reader: BufReader<File>,
    /// Deinterleave buffer: [ch0_block][ch1_block] = 4096 * channels bytes.
    block_buf: Vec<u8>,
    /// Next sample index within the current block (0..4096).
    block_pos: usize,
    /// Number of blocks consumed so far (used for seeking).
    blocks_done: u64,
    /// Total DSD samples per channel.
    total_samples: u64,
    /// Samples decoded so far (per channel).
    samples_done: u64,
    channels: usize,
}

impl DsfContainer {
    pub fn open(path: &Path) -> Result<Self, ContainerError> {
        let dsf =
            dsf_meta::DsfFile::open(path).map_err(|e| ContainerError::DsfParse(e.to_string()))?;

        let fmt = dsf.fmt_chunk();
        let dsd_rate = fmt.sampling_frequency();
        let channels = fmt.channel_num() as u16;
        let lsbf = fmt.bits_per_sample() == 1; // 1 = LSB-first, 8 = MSB-first
        let total_samples = fmt.sample_count();
        let meta = DsdMeta::from_dsd_rate(dsd_rate, channels, lsbf, total_samples);

        // Open a fresh file handle for streaming (dsf.file() is &File, not seekable via &mut).
        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(dsf_meta::DSF_SAMPLE_DATA_OFFSET))?;
        let reader = BufReader::with_capacity(DSF_BLOCK_SIZE * channels as usize * 4, file);

        let block_buf = vec![0u8; DSF_BLOCK_SIZE * channels as usize];

        Ok(Self {
            meta,
            reader,
            block_buf,
            block_pos: DSF_BLOCK_SIZE, // force initial block load
            blocks_done: 0,
            total_samples,
            samples_done: 0,
            channels: channels as usize,
        })
    }

    /// Read one DSD byte per channel into `frame` (len == channels).
    /// Returns `false` when end-of-stream is reached.
    pub fn read_frame(&mut self, frame: &mut [u8]) -> Result<bool, ContainerError> {
        debug_assert_eq!(frame.len(), self.channels);

        if self.samples_done >= self.total_samples {
            return Ok(false);
        }

        // Load next block when the current one is exhausted.
        if self.block_pos >= DSF_BLOCK_SIZE {
            let n = self.reader.read(&mut self.block_buf)?;
            if n < self.block_buf.len() {
                // Pad with silence on short read (trailing zero-padding is normal in DSF).
                self.block_buf[n..].fill(0x69);
            }
            self.block_pos = 0;
            self.blocks_done += 1;
        }

        // Deinterleave: block layout is [ch0_block][ch1_block]…
        for ch in 0..self.channels {
            frame[ch] = self.block_buf[ch * DSF_BLOCK_SIZE + self.block_pos];
        }

        self.block_pos += 1;
        self.samples_done += 1;
        Ok(true)
    }

    /// Seek to the nearest block boundary at or before `target_sample`.
    pub fn seek(&mut self, target_sample: u64) -> Result<(), ContainerError> {
        let block_idx = target_sample / DSF_BLOCK_SIZE as u64;
        let byte_offset = dsf_meta::DSF_SAMPLE_DATA_OFFSET
            + block_idx * (DSF_BLOCK_SIZE as u64 * self.channels as u64);
        self.reader.seek(SeekFrom::Start(byte_offset))?;
        self.block_pos = DSF_BLOCK_SIZE; // force reload on next read
        self.blocks_done = block_idx;
        self.samples_done = block_idx * DSF_BLOCK_SIZE as u64;
        Ok(())
    }

    pub fn position(&self) -> Duration {
        Duration::from_secs_f64(self.samples_done as f64 / self.meta.dsd_rate as f64)
    }

    pub fn is_finished(&self) -> bool {
        self.samples_done >= self.total_samples
    }
}

// ── DSDIFF container ─────────────────────────────────────────────────────────

/// DSDIFF container reader.
///
/// DSDIFF audio data is **byte-interleaved**: bytes for all channels alternate,
/// `[ch0][ch1][ch0][ch1]…`. One byte per channel per frame — no deinterleaving needed.
/// Bit ordering is MSB-first (lsbf = false).
pub struct DsdiffContainer {
    pub meta: DsdMeta,
    reader: BufReader<File>,
    audio_offset: u64,
    audio_length: u64,
    bytes_consumed: u64,
    channels: usize,
}

impl DsdiffContainer {
    pub fn open(path: &Path) -> Result<Self, ContainerError> {
        let dff = match dff_meta::DffFile::open(path) {
            Ok(f) => f,
            Err(dff_meta::model::Error::Id3Error(_, partial)) => partial, // recoverable
            Err(e) => return Err(ContainerError::DsdiffParse(e.to_string())),
        };

        let dsd_rate = dff
            .get_sample_rate()
            .map_err(|e| ContainerError::DsdiffParse(e.to_string()))?;
        let channels = dff
            .get_num_channels()
            .map_err(|e| ContainerError::DsdiffParse(e.to_string()))? as u16;
        let audio_offset = dff.get_dsd_data_offset();
        let audio_length = dff.get_audio_length();

        // DSDIFF is always MSB-first; lsbf = false.
        // Sample count = total bytes / channels (1 byte per channel per sample).
        let total_samples = audio_length / channels as u64;
        let meta = DsdMeta::from_dsd_rate(dsd_rate, channels, false, total_samples);

        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(audio_offset))?;
        let reader = BufReader::with_capacity(64 * 1024, file);

        Ok(Self {
            meta,
            reader,
            audio_offset,
            audio_length,
            bytes_consumed: 0,
            channels: channels as usize,
        })
    }

    /// Read one DSD byte per channel into `frame`. Returns `false` at end-of-stream.
    pub fn read_frame(&mut self, frame: &mut [u8]) -> Result<bool, ContainerError> {
        debug_assert_eq!(frame.len(), self.channels);
        if self.bytes_consumed >= self.audio_length {
            return Ok(false);
        }
        self.reader.read_exact(frame)?;
        self.bytes_consumed += self.channels as u64;
        Ok(true)
    }

    /// Seek to the nearest frame boundary at or before `target_sample`.
    pub fn seek(&mut self, target_sample: u64) -> Result<(), ContainerError> {
        // Clamp to valid range and align to channel frame boundary.
        let max_samples = self.audio_length / self.channels as u64;
        let sample = target_sample.min(max_samples);
        let byte_offset = self.audio_offset + sample * self.channels as u64;
        self.reader.seek(SeekFrom::Start(byte_offset))?;
        self.bytes_consumed = sample * self.channels as u64;
        Ok(())
    }

    pub fn position(&self) -> Duration {
        let samples = self.bytes_consumed / self.channels as u64;
        Duration::from_secs_f64(samples as f64 / self.meta.dsd_rate as f64)
    }

    pub fn is_finished(&self) -> bool {
        self.bytes_consumed >= self.audio_length
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
pub mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use tempfile::NamedTempFile;

    // ── DSF synthetic file builder ────────────────────────────────────────────

    /// Build a minimal valid DSF file in memory for testing.
    /// Produces `num_blocks` blocks of stereo DSD64 audio.
    pub fn build_dsf(num_blocks: usize, pattern: u8) -> Vec<u8> {
        let channels: u32 = 2;
        let dsd_rate: u32 = 2_822_400;
        let sample_count: u64 = (num_blocks * DSF_BLOCK_SIZE) as u64;
        let audio_data_len = num_blocks * DSF_BLOCK_SIZE * channels as usize;
        let total_size: u64 = 92 + audio_data_len as u64; // 28 + 52 + 12 = 92 header

        let mut buf = Vec::new();

        // DSD chunk (28 bytes)
        buf.extend_from_slice(b"DSD ");
        buf.extend_from_slice(&28u64.to_le_bytes());
        buf.extend_from_slice(&total_size.to_le_bytes());
        buf.extend_from_slice(&0u64.to_le_bytes()); // no ID3

        // fmt chunk (52 bytes)
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&52u64.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes()); // format version
        buf.extend_from_slice(&0u32.to_le_bytes()); // format ID (DSD raw)
        buf.extend_from_slice(&2u32.to_le_bytes()); // channel type: stereo
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&dsd_rate.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes()); // bits_per_sample = 1 (LSB-first)
        buf.extend_from_slice(&sample_count.to_le_bytes());
        buf.extend_from_slice(&(DSF_BLOCK_SIZE as u32).to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes()); // reserved

        // data chunk header (12 bytes)
        buf.extend_from_slice(b"data");
        let data_chunk_size: u64 = 12 + audio_data_len as u64;
        buf.extend_from_slice(&data_chunk_size.to_le_bytes());

        // Audio data: block-interleaved [ch0 block][ch1 block] repeated num_blocks times.
        for _ in 0..num_blocks {
            buf.extend(std::iter::repeat(pattern).take(DSF_BLOCK_SIZE)); // ch0
            buf.extend(std::iter::repeat(pattern).take(DSF_BLOCK_SIZE)); // ch1
        }

        buf
    }

    fn write_temp(data: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(data).unwrap();
        f.flush().unwrap();
        f
    }

    // ── DSF unit tests ────────────────────────────────────────────────────────

    #[test]
    fn dsf_meta_parses_sample_rate() {
        let data = build_dsf(1, 0xFF);
        let f = write_temp(&data);
        let c = DsfContainer::open(f.path()).unwrap();
        assert_eq!(c.meta.dsd_rate, 2_822_400);
        assert_eq!(c.meta.pcm_rate, 352_800);
    }

    #[test]
    fn dsf_meta_parses_channels() {
        let data = build_dsf(1, 0xFF);
        let f = write_temp(&data);
        let c = DsfContainer::open(f.path()).unwrap();
        assert_eq!(c.meta.channels, 2);
    }

    #[test]
    fn dsf_meta_lsbf_from_bits_per_sample_field() {
        let data = build_dsf(1, 0xFF); // builder sets bits_per_sample=1 → lsbf=true
        let f = write_temp(&data);
        let c = DsfContainer::open(f.path()).unwrap();
        assert!(
            c.meta.lsbf,
            "DSF with bits_per_sample=1 should be LSB-first"
        );
    }

    #[test]
    fn dsf_duration_matches_sample_count() {
        let blocks = 3;
        let data = build_dsf(blocks, 0x69);
        let f = write_temp(&data);
        let c = DsfContainer::open(f.path()).unwrap();
        let expected_secs = (blocks * DSF_BLOCK_SIZE) as f64 / 2_822_400.0;
        let got_secs = c.meta.duration.as_secs_f64();
        assert!(
            (got_secs - expected_secs).abs() < 1e-6,
            "duration mismatch: expected {expected_secs:.6}s got {got_secs:.6}s"
        );
    }

    #[test]
    fn dsf_dsd_format_string_for_dsd64() {
        let data = build_dsf(1, 0xFF);
        let f = write_temp(&data);
        let c = DsfContainer::open(f.path()).unwrap();
        assert_eq!(c.meta.dsd_format, "DSD64");
    }

    #[test]
    fn dsf_read_frame_returns_correct_bytes() {
        let data = build_dsf(2, 0xAB);
        let f = write_temp(&data);
        let mut c = DsfContainer::open(f.path()).unwrap();
        let mut frame = [0u8; 2];
        let ok = c.read_frame(&mut frame).unwrap();
        assert!(ok, "first read_frame should return true");
        // Both channels should be the fill pattern.
        assert_eq!(frame[0], 0xAB);
        assert_eq!(frame[1], 0xAB);
    }

    #[test]
    fn dsf_read_frame_returns_false_at_end() {
        let data = build_dsf(1, 0xFF); // 4096 samples per channel
        let f = write_temp(&data);
        let mut c = DsfContainer::open(f.path()).unwrap();
        let mut frame = [0u8; 2];
        // Drain all samples.
        for _ in 0..DSF_BLOCK_SIZE {
            assert!(c.read_frame(&mut frame).unwrap());
        }
        assert!(
            !c.read_frame(&mut frame).unwrap(),
            "should return false after last sample"
        );
    }

    #[test]
    fn dsf_seek_jumps_to_block_boundary() {
        let data = build_dsf(3, 0xFF);
        let f = write_temp(&data);
        let mut c = DsfContainer::open(f.path()).unwrap();
        // Seek to sample 4096 (start of second block).
        c.seek(DSF_BLOCK_SIZE as u64).unwrap();
        assert_eq!(c.samples_done, DSF_BLOCK_SIZE as u64);
        assert!(!c.is_finished());
    }

    #[test]
    fn dsf_is_finished_after_all_samples_read() {
        let data = build_dsf(1, 0x69);
        let f = write_temp(&data);
        let mut c = DsfContainer::open(f.path()).unwrap();
        let mut frame = [0u8; 2];
        while c.read_frame(&mut frame).unwrap() {}
        assert!(c.is_finished());
    }

    #[test]
    fn dsf_block_deinterleaving_yields_independent_channel_bytes() {
        // Build DSF where ch0 = 0xAA and ch1 = 0x55 (different patterns per channel).
        let channels: u32 = 2;
        let dsd_rate: u32 = 2_822_400;
        let sample_count: u64 = DSF_BLOCK_SIZE as u64;
        let audio_len = DSF_BLOCK_SIZE * 2;
        let total_size: u64 = 92 + audio_len as u64;

        let mut buf = Vec::new();
        buf.extend_from_slice(b"DSD ");
        buf.extend_from_slice(&28u64.to_le_bytes());
        buf.extend_from_slice(&total_size.to_le_bytes());
        buf.extend_from_slice(&0u64.to_le_bytes());
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&52u64.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&dsd_rate.to_le_bytes());
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&sample_count.to_le_bytes());
        buf.extend_from_slice(&(DSF_BLOCK_SIZE as u32).to_le_bytes());
        buf.extend_from_slice(&0u32.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&(12u64 + audio_len as u64).to_le_bytes());
        // ch0 block = 0xAA, ch1 block = 0x55
        buf.extend(std::iter::repeat(0xAAu8).take(DSF_BLOCK_SIZE));
        buf.extend(std::iter::repeat(0x55u8).take(DSF_BLOCK_SIZE));

        let f = write_temp(&buf);
        let mut c = DsfContainer::open(f.path()).unwrap();
        let mut frame = [0u8; 2];
        c.read_frame(&mut frame).unwrap();
        assert_eq!(frame[0], 0xAA, "ch0 should be 0xAA");
        assert_eq!(frame[1], 0x55, "ch1 should be 0x55");
    }

    // ── DSDIFF unit tests ─────────────────────────────────────────────────────

    /// Build a minimal valid DSDIFF file for testing (big-endian, uncompressed).
    pub fn build_dsdiff(num_samples_per_channel: u64, pattern: u8) -> Vec<u8> {
        let channels: u16 = 2;
        let dsd_rate: u32 = 2_822_400u32;
        let audio_data = vec![pattern; (num_samples_per_channel * channels as u64) as usize];

        // Inner PROP chunks (sample rate + channels + compression + loudspeaker).
        let mut prop_inner = Vec::<u8>::new();
        // PROP type: "SND "
        prop_inner.extend_from_slice(b"SND ");
        // FS chunk: "FS  " header (12) + sample_rate (4) = 16 bytes
        prop_inner.extend_from_slice(b"FS  ");
        prop_inner.extend_from_slice(&4u64.to_be_bytes());
        prop_inner.extend_from_slice(&dsd_rate.to_be_bytes());
        // CHNL chunk: "CHNL" (12) + num_channels (2) + 2 * 4-byte IDs = 22 bytes
        prop_inner.extend_from_slice(b"CHNL");
        prop_inner.extend_from_slice(&10u64.to_be_bytes()); // data size: 2 + 4 + 4 = 10
        prop_inner.extend_from_slice(&channels.to_be_bytes());
        prop_inner.extend_from_slice(b"MLFT"); // left
        prop_inner.extend_from_slice(b"MRGT"); // right
                                               // CMPR chunk: "CMPR" (12) + compression type (4) + count (1) + name + pad.
                                               // The pad byte is included in ck_data_size (matching real DFF files) so that
                                               // dff-meta's sub-chunk loop terminates cleanly at an even chunk boundary.
                                               // "not compressed" = 14 bytes → payload = 4+1+14+1pad = 20 (even).
        let cmpr_name = b"not compressed";
        let cmpr_data_size: u64 = 4 + 1 + cmpr_name.len() as u64 + 1; // = 20 (includes pad)
        prop_inner.extend_from_slice(b"CMPR");
        prop_inner.extend_from_slice(&cmpr_data_size.to_be_bytes());
        prop_inner.extend_from_slice(b"DSD ");
        prop_inner.push(cmpr_name.len() as u8);
        prop_inner.extend_from_slice(cmpr_name);
        prop_inner.push(0); // pad byte counted in ck_data_size

        // PROP chunk: "PROP" (12) + inner data.
        // PROP ck_data_size = "SND " (4) + sub-chunks (70) = prop_inner.len() = 74.
        let mut prop_chunk = Vec::<u8>::new();
        prop_chunk.extend_from_slice(b"PROP");
        prop_chunk.extend_from_slice(&(prop_inner.len() as u64).to_be_bytes());
        prop_chunk.extend_from_slice(&prop_inner);

        // FVER chunk: "FVER" + 4 + version (0x01050000)
        let mut fver = Vec::<u8>::new();
        fver.extend_from_slice(b"FVER");
        fver.extend_from_slice(&4u64.to_be_bytes());
        fver.extend_from_slice(&0x01050000u32.to_be_bytes());

        // DSD sound data chunk: "DSD " + size + audio
        let mut dsd_chunk = Vec::<u8>::new();
        dsd_chunk.extend_from_slice(b"DSD ");
        dsd_chunk.extend_from_slice(&(audio_data.len() as u64).to_be_bytes());
        dsd_chunk.extend_from_slice(&audio_data);

        // FRM8 outer container
        let inner: Vec<u8> = {
            let mut v = Vec::new();
            v.extend_from_slice(b"DSD "); // form type
            v.extend_from_slice(&fver);
            v.extend_from_slice(&prop_chunk);
            v.extend_from_slice(&dsd_chunk);
            v
        };

        let mut out = Vec::new();
        out.extend_from_slice(b"FRM8");
        out.extend_from_slice(&(inner.len() as u64).to_be_bytes());
        out.extend_from_slice(&inner);
        out
    }

    #[test]
    fn dsdiff_meta_parses_sample_rate() {
        let data = build_dsdiff(64, 0xFF);
        let f = write_temp(&data);
        let c = DsdiffContainer::open(f.path()).unwrap();
        assert_eq!(c.meta.dsd_rate, 2_822_400);
        assert_eq!(c.meta.pcm_rate, 352_800);
    }

    #[test]
    fn dsdiff_meta_parses_channels() {
        let data = build_dsdiff(64, 0xFF);
        let f = write_temp(&data);
        let c = DsdiffContainer::open(f.path()).unwrap();
        assert_eq!(c.meta.channels, 2);
    }

    #[test]
    fn dsdiff_is_msbf() {
        let data = build_dsdiff(64, 0xFF);
        let f = write_temp(&data);
        let c = DsdiffContainer::open(f.path()).unwrap();
        assert!(!c.meta.lsbf, "DSDIFF should be MSB-first (lsbf=false)");
    }

    #[test]
    fn dsdiff_read_frame_returns_pattern_bytes() {
        let data = build_dsdiff(64, 0xAB);
        let f = write_temp(&data);
        let mut c = DsdiffContainer::open(f.path()).unwrap();
        let mut frame = [0u8; 2];
        let ok = c.read_frame(&mut frame).unwrap();
        assert!(ok);
        assert_eq!(frame[0], 0xAB);
        assert_eq!(frame[1], 0xAB);
    }

    #[test]
    fn dsdiff_read_frame_returns_false_at_end() {
        let data = build_dsdiff(4, 0xFF);
        let f = write_temp(&data);
        let mut c = DsdiffContainer::open(f.path()).unwrap();
        let mut frame = [0u8; 2];
        for _ in 0..4 {
            assert!(c.read_frame(&mut frame).unwrap());
        }
        assert!(!c.read_frame(&mut frame).unwrap());
    }

    #[test]
    fn dsdiff_seek_positions_correctly() {
        let data = build_dsdiff(100, 0x96);
        let f = write_temp(&data);
        let mut c = DsdiffContainer::open(f.path()).unwrap();
        c.seek(50).unwrap();
        // bytes_consumed = 50 * 2 channels = 100
        assert_eq!(c.bytes_consumed, 100);
    }

    #[test]
    fn dsdiff_dsd_format_string_for_dsd64() {
        let data = build_dsdiff(64, 0xFF);
        let f = write_temp(&data);
        let c = DsdiffContainer::open(f.path()).unwrap();
        assert_eq!(c.meta.dsd_format, "DSD64");
    }
}
