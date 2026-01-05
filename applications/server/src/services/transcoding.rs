/// Transcoding service - FFmpeg wrapper for format/quality conversion
use crate::{
    config::{AudioFormat, Quality},
    error::{Result, ServerError},
};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct TranscodingService {
    ffmpeg_path: PathBuf,
}

impl TranscodingService {
    pub fn new(ffmpeg_path: PathBuf) -> Self {
        Self { ffmpeg_path }
    }

    /// Transcode an audio file to a specific quality and format
    pub async fn transcode(
        &self,
        input: &Path,
        output: &Path,
        quality: Quality,
        format: AudioFormat,
    ) -> Result<()> {
        // Build FFmpeg command based on quality and format
        let mut cmd = Command::new(&self.ffmpeg_path);
        cmd.arg("-i")
            .arg(input)
            .arg("-y") // Overwrite output file
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Add format-specific arguments
        match format {
            AudioFormat::Mp3 => {
                let bitrate = match quality {
                    Quality::Original => "320k", // Highest MP3 quality
                    Quality::High => "320k",
                    Quality::Medium => "192k",
                    Quality::Low => "128k",
                };
                cmd.arg("-b:a").arg(bitrate).arg("-f").arg("mp3");
            }
            AudioFormat::Flac => {
                let compression = match quality {
                    Quality::Original => "0", // No compression (fastest)
                    Quality::High => "5",     // Moderate compression
                    Quality::Medium => "8",   // High compression
                    Quality::Low => "8",
                };
                cmd.arg("-compression_level")
                    .arg(compression)
                    .arg("-f")
                    .arg("flac");
            }
            AudioFormat::Ogg => {
                let quality_level = match quality {
                    Quality::Original => "10", // Highest quality
                    Quality::High => "8",
                    Quality::Medium => "5",
                    Quality::Low => "3",
                };
                cmd.arg("-q:a")
                    .arg(quality_level)
                    .arg("-f")
                    .arg("ogg")
                    .arg("-c:a")
                    .arg("libvorbis");
            }
            AudioFormat::Wav => {
                // WAV is typically uncompressed, sample rate determines "quality"
                let sample_rate = match quality {
                    Quality::Original => "48000",
                    Quality::High => "48000",
                    Quality::Medium => "44100",
                    Quality::Low => "44100",
                };
                cmd.arg("-ar")
                    .arg(sample_rate)
                    .arg("-f")
                    .arg("wav")
                    .arg("-c:a")
                    .arg("pcm_s16le");
            }
            AudioFormat::Opus => {
                let bitrate = match quality {
                    Quality::Original => "256k",
                    Quality::High => "192k",
                    Quality::Medium => "128k",
                    Quality::Low => "96k",
                };
                cmd.arg("-b:a")
                    .arg(bitrate)
                    .arg("-f")
                    .arg("opus")
                    .arg("-c:a")
                    .arg("libopus");
            }
        }

        cmd.arg(output);

        // Execute FFmpeg
        let output_result = cmd.output().await?;

        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            return Err(ServerError::Transcoding(format!(
                "FFmpeg failed: {}",
                stderr
            )));
        }

        Ok(())
    }

    /// Probe an audio file to get metadata
    pub async fn probe(&self, input: &Path) -> Result<AudioMetadata> {
        let ffmpeg_dir = self
            .ffmpeg_path
            .parent()
            .ok_or_else(|| ServerError::Config("Invalid FFmpeg path".to_string()))?;
        let ffprobe_path = ffmpeg_dir.join("ffprobe");

        let output = Command::new(ffprobe_path)
            .arg("-v")
            .arg("quiet")
            .arg("-print_format")
            .arg("json")
            .arg("-show_format")
            .arg("-show_streams")
            .arg(input)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ServerError::Transcoding(format!("FFprobe failed: {}", stderr)));
        }

        let json_output = String::from_utf8_lossy(&output.stdout);
        let probe_data: serde_json::Value = serde_json::from_str(&json_output)
            .map_err(|e| ServerError::Transcoding(format!("Failed to parse FFprobe output: {}", e)))?;

        // Extract relevant metadata
        let format = probe_data
            .get("format")
            .and_then(|f| f.get("format_name"))
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();

        let duration_secs = probe_data
            .get("format")
            .and_then(|f| f.get("duration"))
            .and_then(|d| d.as_str())
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let bitrate = probe_data
            .get("format")
            .and_then(|f| f.get("bit_rate"))
            .and_then(|b| b.as_str())
            .and_then(|s| s.parse::<u64>().ok());

        let sample_rate = probe_data
            .get("streams")
            .and_then(|s| s.as_array())
            .and_then(|arr| arr.first())
            .and_then(|stream| stream.get("sample_rate"))
            .and_then(|sr| sr.as_str())
            .and_then(|s| s.parse::<u32>().ok());

        Ok(AudioMetadata {
            format,
            duration_ms: (duration_secs * 1000.0) as u64,
            bitrate,
            sample_rate,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AudioMetadata {
    pub format: String,
    pub duration_ms: u64,
    pub bitrate: Option<u64>,
    pub sample_rate: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcoding_service_creation() {
        let service = TranscodingService::new(PathBuf::from("/usr/bin/ffmpeg"));
        assert_eq!(service.ffmpeg_path, PathBuf::from("/usr/bin/ffmpeg"));
    }
}
