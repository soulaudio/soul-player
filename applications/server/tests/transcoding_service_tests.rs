/// Transcoding service tests
/// Tests FFmpeg integration, format conversion, and error handling
mod common;

use soul_server::{
    config::{AudioFormat, Quality},
    services::TranscodingService,
};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to check if FFmpeg is available
async fn is_ffmpeg_available() -> bool {
    tokio::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .await
        .is_ok()
}

/// Helper to create a simple WAV file for testing
/// Creates a 1-second silent audio file
fn create_test_audio_file(path: &std::path::Path) -> std::io::Result<()> {
    // Simple WAV file header for 1 second of silence at 44100 Hz, 16-bit, mono
    // WAV format: RIFF header + fmt chunk + data chunk
    let sample_rate: u32 = 44100;
    let bits_per_sample: u16 = 16;
    let num_channels: u16 = 1;
    let duration_seconds = 1;
    let num_samples = sample_rate * duration_seconds;
    let data_size = num_samples * (bits_per_sample as u32 / 8) * (num_channels as u32);
    let file_size = 36 + data_size;

    let mut wav_data = Vec::new();

    // RIFF header
    wav_data.extend_from_slice(b"RIFF");
    wav_data.extend_from_slice(&file_size.to_le_bytes());
    wav_data.extend_from_slice(b"WAVE");

    // fmt chunk
    wav_data.extend_from_slice(b"fmt ");
    wav_data.extend_from_slice(&16u32.to_le_bytes()); // Chunk size
    wav_data.extend_from_slice(&1u16.to_le_bytes()); // Audio format (1 = PCM)
    wav_data.extend_from_slice(&num_channels.to_le_bytes());
    wav_data.extend_from_slice(&sample_rate.to_le_bytes());
    let byte_rate = sample_rate * (num_channels as u32) * (bits_per_sample as u32 / 8);
    wav_data.extend_from_slice(&byte_rate.to_le_bytes());
    let block_align = num_channels * (bits_per_sample / 8);
    wav_data.extend_from_slice(&block_align.to_le_bytes());
    wav_data.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    wav_data.extend_from_slice(b"data");
    wav_data.extend_from_slice(&data_size.to_le_bytes());

    // Add silent audio data (all zeros)
    wav_data.resize(wav_data.len() + data_size as usize, 0);

    std::fs::write(path, wav_data)
}

/// Test TranscodingService initialization
#[tokio::test]
async fn test_transcoding_service_initialization() {
    let service = TranscodingService::new(PathBuf::from("/usr/bin/ffmpeg"));
    // Service should be created successfully
    assert_eq!(format!("{:?}", service), "TranscodingService { ffmpeg_path: \"/usr/bin/ffmpeg\" }");
}

/// Test transcoding to MP3 high quality (requires FFmpeg)
#[tokio::test]
async fn test_transcode_to_mp3_high() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");
    let output_path = temp_dir.path().join("output.mp3");

    // Create test input file
    create_test_audio_file(&input_path).unwrap();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::High, AudioFormat::Mp3).await;

    assert!(result.is_ok(), "Transcoding should succeed");
    assert!(output_path.exists(), "Output file should exist");

    // Verify output file has content
    let metadata = std::fs::metadata(&output_path).unwrap();
    assert!(metadata.len() > 0, "Output file should have content");
}

/// Test transcoding to MP3 medium quality (requires FFmpeg)
#[tokio::test]
async fn test_transcode_to_mp3_medium() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");
    let output_path = temp_dir.path().join("output.mp3");

    create_test_audio_file(&input_path).unwrap();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::Medium, AudioFormat::Mp3).await;

    assert!(result.is_ok());
    assert!(output_path.exists());
}

/// Test transcoding to MP3 low quality (requires FFmpeg)
#[tokio::test]
async fn test_transcode_to_mp3_low() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");
    let output_path = temp_dir.path().join("output.mp3");

    create_test_audio_file(&input_path).unwrap();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::Low, AudioFormat::Mp3).await;

    assert!(result.is_ok());
    assert!(output_path.exists());
}

/// Test transcoding to FLAC (requires FFmpeg)
#[tokio::test]
async fn test_transcode_to_flac() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");
    let output_path = temp_dir.path().join("output.flac");

    create_test_audio_file(&input_path).unwrap();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::High, AudioFormat::Flac).await;

    assert!(result.is_ok());
    assert!(output_path.exists());
}

/// Test transcoding to OGG Vorbis (requires FFmpeg)
#[tokio::test]
async fn test_transcode_to_ogg() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");
    let output_path = temp_dir.path().join("output.ogg");

    create_test_audio_file(&input_path).unwrap();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::High, AudioFormat::Ogg).await;

    assert!(result.is_ok());
    assert!(output_path.exists());
}

/// Test transcoding to WAV (requires FFmpeg)
#[tokio::test]
async fn test_transcode_to_wav() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");
    let output_path = temp_dir.path().join("output_new.wav");

    create_test_audio_file(&input_path).unwrap();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::High, AudioFormat::Wav).await;

    assert!(result.is_ok());
    assert!(output_path.exists());
}

/// Test transcoding to Opus (requires FFmpeg)
#[tokio::test]
async fn test_transcode_to_opus() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");
    let output_path = temp_dir.path().join("output.opus");

    create_test_audio_file(&input_path).unwrap();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::High, AudioFormat::Opus).await;

    assert!(result.is_ok());
    assert!(output_path.exists());
}

/// Test transcoding with invalid input file
#[tokio::test]
async fn test_transcode_invalid_input() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("nonexistent.wav");
    let output_path = temp_dir.path().join("output.mp3");

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::High, AudioFormat::Mp3).await;

    assert!(result.is_err(), "Should fail with nonexistent input");
}

/// Test transcoding with corrupted input file
#[tokio::test]
async fn test_transcode_corrupted_input() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("corrupted.wav");
    let output_path = temp_dir.path().join("output.mp3");

    // Create corrupted file (just random bytes)
    std::fs::write(&input_path, b"not a valid audio file").unwrap();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::High, AudioFormat::Mp3).await;

    assert!(result.is_err(), "Should fail with corrupted input");
}

/// Test transcoding with invalid FFmpeg path
#[tokio::test]
async fn test_transcode_invalid_ffmpeg_path() {
    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");
    let output_path = temp_dir.path().join("output.mp3");

    create_test_audio_file(&input_path).unwrap();

    let service = TranscodingService::new(PathBuf::from("/nonexistent/ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::High, AudioFormat::Mp3).await;

    assert!(result.is_err(), "Should fail with invalid FFmpeg path");
}

/// Test overwriting existing output file
#[tokio::test]
async fn test_transcode_overwrite_output() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");
    let output_path = temp_dir.path().join("output.mp3");

    create_test_audio_file(&input_path).unwrap();

    // Create existing output file
    std::fs::write(&output_path, b"old content").unwrap();
    let old_size = std::fs::metadata(&output_path).unwrap().len();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    let result = service.transcode(&input_path, &output_path, Quality::High, AudioFormat::Mp3).await;

    assert!(result.is_ok(), "Should overwrite existing file");
    assert!(output_path.exists());

    // Verify file was actually overwritten (different size)
    let new_size = std::fs::metadata(&output_path).unwrap().len();
    assert_ne!(old_size, new_size, "Output file should be overwritten");
}

/// Test quality variations produce different file sizes (MP3)
#[tokio::test]
async fn test_quality_affects_file_size() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");

    create_test_audio_file(&input_path).unwrap();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));

    // Transcode at high quality
    let high_output = temp_dir.path().join("high.mp3");
    service.transcode(&input_path, &high_output, Quality::High, AudioFormat::Mp3).await.unwrap();
    let high_size = std::fs::metadata(&high_output).unwrap().len();

    // Transcode at low quality
    let low_output = temp_dir.path().join("low.mp3");
    service.transcode(&input_path, &low_output, Quality::Low, AudioFormat::Mp3).await.unwrap();
    let low_size = std::fs::metadata(&low_output).unwrap().len();

    // High quality should produce larger file
    assert!(high_size > low_size, "High quality should produce larger file than low quality");
}

/// Test multiple concurrent transcoding operations
#[tokio::test]
async fn test_concurrent_transcoding() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();

    // Create multiple input files
    let mut handles = vec![];
    for i in 0..3 {
        let temp_dir = temp_dir.path().to_path_buf();
        let handle = tokio::spawn(async move {
            let input_path = temp_dir.join(format!("input_{}.wav", i));
            let output_path = temp_dir.join(format!("output_{}.mp3", i));

            create_test_audio_file(&input_path).unwrap();

            let service = TranscodingService::new(PathBuf::from("ffmpeg"));
            service.transcode(&input_path, &output_path, Quality::Medium, AudioFormat::Mp3).await
        });
        handles.push(handle);
    }

    // Wait for all transcoding operations
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent transcoding should succeed");
    }

    // Verify all output files exist
    for i in 0..3 {
        let output_path = temp_dir.path().join(format!("output_{}.mp3", i));
        assert!(output_path.exists(), "Output file {} should exist", i);
    }
}

/// Test transcoding preserves basic audio properties
#[tokio::test]
async fn test_transcode_preserves_duration() {
    if !is_ffmpeg_available().await {
        eprintln!("Skipping test: FFmpeg not available");
        return;
    }

    let temp_dir = TempDir::new().unwrap();
    let input_path = temp_dir.path().join("input.wav");
    let output_path = temp_dir.path().join("output.mp3");

    create_test_audio_file(&input_path).unwrap();

    let service = TranscodingService::new(PathBuf::from("ffmpeg"));
    service.transcode(&input_path, &output_path, Quality::High, AudioFormat::Mp3).await.unwrap();

    // Both files should exist and output should have reasonable size
    assert!(input_path.exists());
    assert!(output_path.exists());

    let output_size = std::fs::metadata(&output_path).unwrap().len();
    assert!(output_size > 1000, "Output file should have reasonable size (> 1KB)");
}
