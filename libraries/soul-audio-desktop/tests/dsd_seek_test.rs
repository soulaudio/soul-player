//! Integration tests for DsdAudioSource seek behaviour.

use soul_audio_desktop::sources::dsd::container::DSF_BLOCK_SIZE;
use soul_audio_desktop::sources::dsd::source::DsdAudioSource;
use soul_playback::AudioSource;
use std::io::Write as IoWrite;
use std::time::Duration;
use tempfile::Builder;

/// Build a minimal valid DSF file in memory.
/// Produces `num_blocks` blocks of stereo DSD64 audio.
fn build_dsf(num_blocks: usize, pattern: u8) -> Vec<u8> {
    let channels: u32 = 2;
    let dsd_rate: u32 = 2_822_400;
    let sample_count: u64 = (num_blocks * DSF_BLOCK_SIZE) as u64;
    let audio_data_len = num_blocks * DSF_BLOCK_SIZE * channels as usize;
    let total_size: u64 = 92 + audio_data_len as u64;

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

fn make_dsf_file(num_blocks: usize) -> tempfile::NamedTempFile {
    let data = build_dsf(num_blocks, 0xAA);
    let mut f = Builder::new().suffix(".dsf").tempfile().unwrap();
    f.write_all(&data).unwrap();
    f.flush().unwrap();
    f
}

const _BLOCK_SECS: f64 = 4096.0 / 2_822_400.0; // ≈1.45 ms

#[test]
fn dsd_source_position_returns_seek_target_immediately() {
    let file = make_dsf_file(200); // ~290ms of audio
    let mut src = DsdAudioSource::new(file.path(), 48_000).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    let seek_target = Duration::from_millis(100);
    src.seek(seek_target).unwrap();
    let pos = src.position();
    assert!(
        (pos.as_secs_f64() - seek_target.as_secs_f64()).abs() < 0.05,
        "position() must return seek target immediately, got {:?}",
        pos
    );
}

#[test]
fn dsd_source_seek_pending_cleared_after_background_processes() {
    let file = make_dsf_file(400);
    let mut src = DsdAudioSource::new(file.path(), 48_000).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    src.seek(Duration::from_millis(50)).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut buf = vec![0.0f32; 256];
    let mut got_audio = false;
    while std::time::Instant::now() < deadline {
        let n = src.read_samples(&mut buf).unwrap();
        if n > 0 {
            got_audio = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        got_audio,
        "DSD source must produce audio after seek completes"
    );
}

#[test]
fn dsd_source_position_accurate_after_seek_completes() {
    let file = make_dsf_file(800); // ~1.2s of audio
    let mut src = DsdAudioSource::new(file.path(), 48_000).unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let target = Duration::from_millis(200);
    src.seek(target).unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut buf = vec![0.0f32; 1024];
    while std::time::Instant::now() < deadline {
        let n = src.read_samples(&mut buf).unwrap();
        if n > 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let pos = src.position();
    assert!(
        (pos.as_secs_f64() - target.as_secs_f64()).abs() < 0.1,
        "position after seek must be ≈{:?}, got {:?}",
        target,
        pos
    );
}

#[test]
fn dsd_source_eof_path_seek_clears_seek_pending_and_position_not_stuck() {
    let file = make_dsf_file(100); // ~145ms of audio
    let mut src = DsdAudioSource::new(file.path(), 48_000).unwrap();
    let near_end = Duration::from_millis(130);
    src.seek(near_end).unwrap();
    let mut buf = vec![0.0f32; 512];
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !src.is_finished() && std::time::Instant::now() < deadline {
        let _ = src.read_samples(&mut buf);
        std::thread::sleep(Duration::from_millis(5));
    }
    let new_target = Duration::from_millis(10);
    src.seek(new_target).unwrap();
    let pos = src.position();
    assert!(
        (pos.as_secs_f64() - new_target.as_secs_f64()).abs() < 0.05,
        "position after EOF+seek must be ≈{:?}, got {:?}",
        new_target,
        pos
    );
}

#[test]
fn dsd_source_buffer_fills_faster_with_larger_chunk_frames() {
    let file = make_dsf_file(500);
    let mut src = DsdAudioSource::new(file.path(), 48_000).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    src.seek(Duration::from_millis(10)).unwrap();
    let mut buf = vec![0.0f32; 4096];
    let _ = src.read_samples(&mut buf);
    let start = std::time::Instant::now();
    let deadline = start + Duration::from_millis(500);
    while !src.is_ready() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    let elapsed = start.elapsed();
    assert!(
        src.is_ready(),
        "DSD source must be ready within 500ms after seek"
    );
    assert!(
        elapsed.as_millis() < 200,
        "buffer should fill in <200ms, took {:?}",
        elapsed
    );
}
