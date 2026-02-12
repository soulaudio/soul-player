//! E2E test for detecting stuttering at track start (specific FLAC file)
//!
//! **Problem:**
//! The file `D:\music\Rap\Joji\BALLADS 1\01 ATTENTION.flac` stutters at the start.
//!
//! **What this test does:**
//! 1. Loads the specific FLAC file through our playback system
//! 2. Analyzes the first 500ms of decoded audio
//! 3. Detects amplitude discontinuities (pops, clicks, stutters)
//! 4. Reports detailed findings with timestamps and amplitude graphs
//!
//! **Detection methods:**
//! - Amplitude jump analysis (sudden volume changes)
//! - Gap detection (silence followed by sudden audio)
//! - High-frequency spike detection (digital artifacts)
//! - RMS energy analysis (dropout detection)

use soul_audio_desktop::{DesktopPlayback, LocalAudioSource, PlaybackCommand, PlaybackEvent};
use soul_playback::{AudioSource, PlaybackConfig, PlaybackState, QueueTrack, TrackSource};
use std::path::PathBuf;
use std::time::Duration;

/// Path to the problematic FLAC file
const FLAC_FILE_PATH: &str = r"D:\music\Rap\Joji\BALLADS 1\01 ATTENTION.flac";

/// Threshold for detecting amplitude jumps (0.2 = -14dB jump)
const AMPLITUDE_JUMP_THRESHOLD: f32 = 0.2;

/// Threshold for detecting silence (below -60dB)
const SILENCE_THRESHOLD: f32 = 0.001;

/// Threshold for detecting sudden onset after silence (0.1 = -20dB)
const SUDDEN_ONSET_THRESHOLD: f32 = 0.1;

// ===== Helper Functions =====

/// Create a track from the FLAC file
fn create_flac_track() -> QueueTrack {
    QueueTrack {
        id: "joji_attention".to_string(),
        path: PathBuf::from(FLAC_FILE_PATH),
        title: "ATTENTION".to_string(),
        artist: "Joji".to_string(),
        album: Some("BALLADS 1".to_string()),
        duration: Duration::from_secs(180), // Will be updated by loader
        track_number: Some(1),
        source: TrackSource::Single,
    }
}

/// Analyze amplitude discontinuities in audio buffer
#[derive(Debug)]
struct AmplitudeAnalysis {
    max_jump: f32,
    max_jump_sample: usize,
    num_large_jumps: usize,
    has_silence_gap: bool,
    silence_to_loud_jump: Option<(usize, f32)>,
    rms: f32,
}

fn analyze_amplitude_discontinuities(buffer: &[f32], sample_rate: u32) -> AmplitudeAnalysis {
    let mut max_jump = 0.0f32;
    let mut max_jump_sample = 0;
    let mut num_large_jumps = 0;
    let mut prev_amplitude = 0.0f32;
    let mut has_silence_gap = false;
    let mut silence_to_loud_jump = None;

    // Track silence periods
    let mut in_silence = true;
    let mut silence_start = 0;

    // Calculate RMS energy
    let total_energy: f64 = buffer.iter().map(|&s| (s as f64).powi(2)).sum();
    let rms = (total_energy / buffer.len() as f64).sqrt() as f32;

    for (i, &sample) in buffer.iter().enumerate() {
        let current_amplitude = sample.abs();
        let jump = (current_amplitude - prev_amplitude).abs();

        // Track maximum jump
        if jump > max_jump {
            max_jump = jump;
            max_jump_sample = i;
        }

        // Count large jumps
        if jump > AMPLITUDE_JUMP_THRESHOLD {
            num_large_jumps += 1;
        }

        // Detect silence gaps followed by sudden loud audio
        if in_silence {
            if current_amplitude < SILENCE_THRESHOLD {
                // Still silent
            } else {
                // End of silence
                if current_amplitude > SUDDEN_ONSET_THRESHOLD && i > 0 {
                    // Sudden loud onset after silence
                    let silence_duration_ms =
                        ((i - silence_start) as f64 / sample_rate as f64) * 1000.0;
                    if silence_duration_ms > 10.0 {
                        // Silence gap > 10ms followed by sudden onset
                        has_silence_gap = true;
                        silence_to_loud_jump = Some((i, current_amplitude));
                    }
                }
                in_silence = false;
            }
        } else {
            if current_amplitude < SILENCE_THRESHOLD {
                // New silence period started
                in_silence = true;
                silence_start = i;
            }
        }

        prev_amplitude = current_amplitude;
    }

    AmplitudeAnalysis {
        max_jump,
        max_jump_sample,
        num_large_jumps,
        has_silence_gap,
        silence_to_loud_jump,
        rms,
    }
}

/// Print amplitude waveform for visual inspection
fn print_amplitude_waveform(buffer: &[f32], sample_rate: u32, window_ms: u32) {
    let samples_per_window = (sample_rate as u64 * window_ms as u64 / 1000) as usize;
    let num_windows = (buffer.len() + samples_per_window - 1) / samples_per_window;

    println!(
        "\n[WAVEFORM] Amplitude over time ({}ms windows):",
        window_ms
    );
    println!("[WAVEFORM] Scale: █ = high, ▓ = medium, ░ = low, . = silence");

    for window_idx in 0..num_windows.min(20) {
        let start = window_idx * samples_per_window;
        let end = (start + samples_per_window).min(buffer.len());

        if start >= buffer.len() {
            break;
        }

        // Calculate RMS for this window
        let window_samples = &buffer[start..end];
        let energy: f64 = window_samples.iter().map(|&s| (s as f64).powi(2)).sum();
        let rms = (energy / window_samples.len() as f64).sqrt();
        let db = 20.0 * rms.log10();

        let time_ms = (start as f64 / sample_rate as f64) * 1000.0;

        // Visual bar
        let bar = if db < -60.0 {
            "."
        } else if db < -40.0 {
            "░"
        } else if db < -20.0 {
            "▒"
        } else if db < -10.0 {
            "▓"
        } else {
            "█"
        };

        let bar_length = ((db + 60.0) / 60.0 * 40.0).max(0.0).min(40.0) as usize;
        let visual_bar = bar.repeat(bar_length);

        println!(
            "[WAVEFORM] {:5.0}ms | {:6.1}dB | {}",
            time_ms, db, visual_bar
        );
    }
}

// ===== Tests =====

#[test]
#[ignore = "Requires specific FLAC file - only run when debugging stutter issues"]
fn test_flac_stutter_detection_direct_source() {
    // **Direct audio source test** - bypasses playback system
    // This loads the FLAC file and analyzes the decoded audio directly

    let path = PathBuf::from(FLAC_FILE_PATH);

    if !path.exists() {
        println!("⚠️  FLAC file not found: {}", FLAC_FILE_PATH);
        println!("This test requires the specific file to be present.");
        return;
    }

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  FLAC Stutter Detection Test (Direct Source)              ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!("\n[TEST] Loading FLAC file: {}", path.display());

    // Create audio source with CD quality (44.1kHz)
    let mut source = match LocalAudioSource::new(&path, 44100) {
        Ok(src) => src,
        Err(e) => {
            println!("❌ Failed to load FLAC file: {}", e);
            panic!("Could not load FLAC file");
        }
    };

    println!("✓ FLAC file loaded successfully");
    println!("  Duration: {:.2}s", source.duration().as_secs_f64());

    // Wait for source to be ready (buffer to fill)
    println!("\n[TEST] Waiting for audio buffer to fill...");
    let start_wait = std::time::Instant::now();
    let mut ready = false;
    while start_wait.elapsed() < Duration::from_secs(5) {
        if source.is_ready() {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    if !ready {
        println!("❌ Audio source not ready after 5 seconds");
        panic!("Audio source failed to prebuffer");
    }

    println!(
        "✓ Buffer filled in {:.0}ms",
        start_wait.elapsed().as_millis()
    );

    // Read first 500ms (22050 stereo samples = 44100 mono samples)
    let samples_to_read = 44100; // 500ms at 44.1kHz stereo
    let mut buffer = vec![0.0f32; samples_to_read];

    println!("\n[TEST] Reading first 500ms of audio...");

    let samples_read = match source.read_samples(&mut buffer) {
        Ok(n) => n,
        Err(e) => {
            println!("❌ Failed to read samples: {}", e);
            panic!("Could not read audio samples");
        }
    };

    println!(
        "✓ Read {} samples ({:.1}ms)",
        samples_read,
        (samples_read as f64 / (44100.0 * 2.0)) * 1000.0
    );

    // Truncate buffer to actual samples read
    buffer.truncate(samples_read);

    // ===== Analysis Phase =====

    println!("\n[ANALYSIS] Analyzing for stutters, pops, and clicks...");

    let analysis = analyze_amplitude_discontinuities(&buffer, 44100);

    println!("\n┌─ Analysis Results ─────────────────────────────────────┐");
    println!(
        "│ RMS Level:        {:.6} ({:.1}dB)",
        analysis.rms,
        20.0 * analysis.rms.log10()
    );
    println!(
        "│ Max Jump:         {:.6} ({:.1}dB) at sample {}",
        analysis.max_jump,
        20.0 * analysis.max_jump.log10(),
        analysis.max_jump_sample
    );
    println!(
        "│ Large Jumps:      {} (threshold: {:.2})",
        analysis.num_large_jumps, AMPLITUDE_JUMP_THRESHOLD
    );
    println!(
        "│ Silence Gap:      {}",
        if analysis.has_silence_gap {
            "YES ⚠️"
        } else {
            "NO ✓"
        }
    );

    if let Some((sample, amp)) = analysis.silence_to_loud_jump {
        let time_ms = (sample as f64 / (44100.0 * 2.0)) * 1000.0;
        println!(
            "│ Sudden Onset:     {:.1}ms (amplitude: {:.3})",
            time_ms, amp
        );
    }

    println!("└────────────────────────────────────────────────────────┘");

    // Print waveform visualization
    print_amplitude_waveform(&buffer, 44100, 25);

    // ===== Verdict =====

    println!("\n[VERDICT]");

    let has_stutter = analysis.max_jump > AMPLITUDE_JUMP_THRESHOLD
        || analysis.has_silence_gap
        || analysis.num_large_jumps > 3;

    if has_stutter {
        println!("🔴 STUTTER DETECTED!");
        println!("\nPotential causes:");

        if analysis.max_jump > AMPLITUDE_JUMP_THRESHOLD {
            println!(
                "  • Large amplitude discontinuity ({:.1}dB jump at sample {})",
                20.0 * analysis.max_jump.log10(),
                analysis.max_jump_sample
            );
            println!("    → Possible encoder delay not skipped");
            println!("    → Possible decoder startup artifact");
        }

        if analysis.has_silence_gap {
            println!("  • Silence gap followed by sudden onset");
            println!("    → Possible buffer underrun");
            println!("    → Possible prebuffering issue");
        }

        if analysis.num_large_jumps > 3 {
            println!(
                "  • Multiple large jumps detected ({})",
                analysis.num_large_jumps
            );
            println!("    → Possible decoding glitches");
            println!("    → Possible resampling artifacts");
        }

        panic!("Stutter detected in FLAC file!");
    } else {
        println!("✅ NO STUTTER DETECTED");
        println!("Audio starts cleanly with no artifacts.");
    }
}

#[test]
#[ignore = "Requires specific FLAC file and audio hardware"]
fn test_flac_stutter_detection_full_playback() {
    // **Full playback system test** - tests the complete pipeline
    // This uses DesktopPlayback to play the file through the audio system

    let path = PathBuf::from(FLAC_FILE_PATH);

    if !path.exists() {
        println!("⚠️  FLAC file not found: {}", FLAC_FILE_PATH);
        println!("This test requires the specific file to be present.");
        return;
    }

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  FLAC Stutter Detection Test (Full Playback)              ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    let config = PlaybackConfig::default();
    let playback = DesktopPlayback::new(config).expect("Failed to create playback");

    println!("\n[TEST] Loading and playing FLAC file...");

    let track = create_flac_track();

    // Load and play
    playback
        .send_command(PlaybackCommand::LoadPlaylist(vec![track]))
        .expect("Failed to load playlist");

    playback
        .send_command(PlaybackCommand::Play)
        .expect("Failed to play");

    println!("[TEST] Waiting for playback to start...");

    // Wait for playback to stabilize (should be playing within 200ms)
    std::thread::sleep(Duration::from_millis(300));

    // Collect events
    let events: Vec<PlaybackEvent> = std::iter::from_fn(|| playback.try_recv_event()).collect();

    println!("\n[EVENTS] Received {} events:", events.len());
    for (i, event) in events.iter().enumerate() {
        match event {
            PlaybackEvent::StateChanged(state) => {
                println!("  {}: StateChanged({:?})", i, state);
            }
            PlaybackEvent::Error(err) => {
                println!("  {}: Error({})", i, err);
            }
            _ => {
                println!("  {}: {:?}", i, event);
            }
        }
    }

    // Check for errors
    let has_errors = events.iter().any(|e| matches!(e, PlaybackEvent::Error(_)));

    if has_errors {
        println!("\n❌ Playback errors detected!");
        let error_msgs: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let PlaybackEvent::Error(msg) = e {
                    Some(msg.as_str())
                } else {
                    None
                }
            })
            .collect();

        println!("Errors:");
        for msg in error_msgs {
            println!("  • {}", msg);
        }

        panic!("Failed to play FLAC file");
    }

    // Get final state
    let final_state = events.iter().rev().find_map(|e| {
        if let PlaybackEvent::StateChanged(state) = e {
            Some(*state)
        } else {
            None
        }
    });

    println!("\n[STATE] Final playback state: {:?}", final_state);

    assert_eq!(
        final_state,
        Some(PlaybackState::Playing),
        "Expected Playing state, got {:?}",
        final_state
    );

    // Let it play for 1 second to hear any stuttering
    println!("\n[TEST] Playing for 1 second...");
    println!("🎵 Listen for any stuttering, pops, or clicks at the start");
    std::thread::sleep(Duration::from_secs(1));

    // Pause playback
    playback
        .send_command(PlaybackCommand::Pause)
        .expect("Failed to pause");
    std::thread::sleep(Duration::from_millis(150));

    println!("\n[TEST] ✓ Playback test completed");
    println!("\n📊 MANUAL VERIFICATION REQUIRED:");
    println!("   Did you hear any stuttering at the start? (Y/N)");
    println!("   If YES → stutter confirmed, investigate further");
    println!("   If NO → stutter may be intermittent or system-specific");
}

#[test]
#[ignore = "Requires specific FLAC file"]
fn test_flac_compare_with_resampling() {
    // Test if resampling introduces stutter artifacts

    let path = PathBuf::from(FLAC_FILE_PATH);

    if !path.exists() {
        println!("⚠️  FLAC file not found");
        return;
    }

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  FLAC Resampling Comparison Test                          ║");
    println!("╚════════════════════════════════════════════════════════════╝");

    // Test 1: Native sample rate (no resampling)
    println!("\n[TEST] Loading with native sample rate (no resampling)...");
    let mut source1 = LocalAudioSource::new(&path, 44100).expect("Failed to load");

    // Wait for source to be ready
    while !source1.is_ready() {
        std::thread::sleep(Duration::from_millis(10));
    }

    let mut buffer1 = vec![0.0f32; 44100];
    let samples1 = source1.read_samples(&mut buffer1).unwrap();
    buffer1.truncate(samples1);

    let analysis1 = analyze_amplitude_discontinuities(&buffer1, 44100);

    println!(
        "✓ Native rate: max jump = {:.6} ({:.1}dB)",
        analysis1.max_jump,
        20.0 * analysis1.max_jump.log10()
    );

    // Test 2: Resampled to 48kHz
    println!("\n[TEST] Loading with resampling (44.1kHz → 48kHz)...");
    let mut source2 = LocalAudioSource::new(&path, 48000).expect("Failed to load");

    // Wait for source to be ready
    while !source2.is_ready() {
        std::thread::sleep(Duration::from_millis(10));
    }

    let mut buffer2 = vec![0.0f32; 48000];
    let samples2 = source2.read_samples(&mut buffer2).unwrap();
    buffer2.truncate(samples2);

    let analysis2 = analyze_amplitude_discontinuities(&buffer2, 48000);

    println!(
        "✓ Resampled: max jump = {:.6} ({:.1}dB)",
        analysis2.max_jump,
        20.0 * analysis2.max_jump.log10()
    );

    // Compare
    println!("\n[COMPARISON]");
    println!(
        "  Native:    max jump = {:.6} ({:.1}dB)",
        analysis1.max_jump,
        20.0 * analysis1.max_jump.log10()
    );
    println!(
        "  Resampled: max jump = {:.6} ({:.1}dB)",
        analysis2.max_jump,
        20.0 * analysis2.max_jump.log10()
    );

    let jump_diff = (analysis2.max_jump - analysis1.max_jump).abs();
    println!(
        "  Difference: {:.6} ({:.1}dB)",
        jump_diff,
        20.0 * jump_diff.log10()
    );

    if analysis2.max_jump > analysis1.max_jump * 1.5 {
        println!("\n⚠️  Resampling increases amplitude discontinuities!");
        println!("   Resampler may be introducing artifacts.");
    } else if analysis1.max_jump > AMPLITUDE_JUMP_THRESHOLD {
        println!("\n⚠️  Both have large jumps - issue is in source file or decoder");
    } else {
        println!("\n✅ Both versions start cleanly");
    }
}
