# Audio Abstraction & Dependency Injection

This document describes the audio abstraction layer that enables Soul Player to work across desktop, mobile, server, and embedded platforms.

---

## Problem Statement

Soul Player needs to output audio on multiple platforms with different audio APIs:

| Platform | Audio API | Implementation |
|----------|-----------|----------------|
| Desktop (Windows/macOS/Linux) | CPAL | `soul-audio-desktop` |
| Mobile (iOS) | AVAudioEngine | `soul-audio-mobile` (Swift bridge) |
| Mobile (Android) | AudioTrack | `soul-audio-mobile` (Kotlin bridge) |
| Server | Streaming | `soul-audio-server` (network streaming) |
| ESP32-S3 | I2S | `soul-audio-embedded` (awedio_esp32) |

**Challenge**: The audio engine (`soul-audio`) should not be coupled to any specific platform implementation.

**Solution**: Trait-based dependency injection with generic audio engine.

---

## Architecture Overview

```
┌─────────────────────────────────────────────┐
│         soul-audio (Audio Engine)           │
│  ┌────────────────────────────────────────┐ │
│  │  AudioEngine<O: AudioOutput>           │ │
│  │  - decoder: SymphoniaDecoder           │ │
│  │  - output: O (generic)                 │ │
│  │  - effect_chain: EffectChain           │ │
│  └────────────────────────────────────────┘ │
└─────────────────────────────────────────────┘
                     │
                     │ depends on
                     ▼
┌─────────────────────────────────────────────┐
│      soul-core (AudioOutput trait)          │
│  pub trait AudioOutput: Send {              │
│      fn play(...);                          │
│      fn pause(...);                         │
│      fn set_volume(...);                    │
│  }                                          │
└─────────────────────────────────────────────┘
                     △
                     │ implements
         ┌───────────┴───────────┬─────────────┬──────────────┐
         │                       │             │              │
┌────────────────┐  ┌──────────────────┐  ┌────────┐  ┌──────────┐
│ soul-audio-    │  │ soul-audio-      │  │ soul-  │  │ soul-    │
│ desktop        │  │ mobile           │  │ audio- │  │ audio-   │
│ (CPAL)         │  │ (iOS/Android)    │  │ server │  │ embedded │
└────────────────┘  └──────────────────┘  └────────┘  └──────────┘
```

---

## Core Trait Definition

**Location**: `libraries/soul-core/src/audio/output.rs`

```rust
use std::time::Duration;
use crate::types::AudioBuffer;
use crate::error::Result;

/// Platform-agnostic audio output abstraction.
///
/// Implementations of this trait handle platform-specific audio output
/// (CPAL, AVAudioEngine, AudioTrack, I2S, etc.).
pub trait AudioOutput: Send {
    /// Initialize the audio output device.
    ///
    /// # Arguments
    /// * `sample_rate` - Desired sample rate (e.g., 44100, 48000)
    /// * `channels` - Number of audio channels (1 = mono, 2 = stereo)
    ///
    /// # Errors
    /// Returns error if device initialization fails.
    fn initialize(&mut self, sample_rate: u32, channels: u16) -> Result<()>;

    /// Send audio buffer to output device for playback.
    ///
    /// # Arguments
    /// * `buffer` - Audio samples to play (interleaved if multi-channel)
    ///
    /// # Errors
    /// Returns error if playback fails (device disconnected, buffer overflow, etc.)
    fn play(&mut self, buffer: &AudioBuffer) -> Result<()>;

    /// Pause audio playback without clearing the buffer.
    fn pause(&mut self) -> Result<()>;

    /// Resume audio playback from paused state.
    fn resume(&mut self) -> Result<()>;

    /// Stop playback and clear all buffers.
    fn stop(&mut self) -> Result<()>;

    /// Set output volume (0.0 = silent, 1.0 = full volume).
    ///
    /// # Arguments
    /// * `volume` - Volume level between 0.0 and 1.0
    ///
    /// # Panics
    /// May panic if volume is outside [0.0, 1.0] range (implementation-dependent).
    fn set_volume(&mut self, volume: f32) -> Result<()>;

    /// Get current playback latency (time between buffer submission and audio output).
    ///
    /// Used for A/V sync and playback progress calculation.
    fn get_latency(&self) -> Duration;

    /// Check if output device is currently playing audio.
    fn is_playing(&self) -> bool;

    /// Get the actual sample rate being used by the output device.
    ///
    /// May differ from requested rate if device doesn't support it.
    fn sample_rate(&self) -> u32;

    /// Get the number of channels being used.
    fn channels(&self) -> u16;
}

/// Audio buffer representation.
///
/// Contains raw PCM samples and metadata.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Raw PCM samples (f32, interleaved if multi-channel)
    pub samples: Vec<f32>,

    /// Sample rate of this buffer
    pub sample_rate: u32,

    /// Number of channels (1 = mono, 2 = stereo, etc.)
    pub channels: u16,

    /// Duration of this buffer
    pub duration: Duration,
}

impl AudioBuffer {
    /// Create a new audio buffer.
    pub fn new(samples: Vec<f32>, sample_rate: u32, channels: u16) -> Self {
        let duration = Duration::from_secs_f64(
            samples.len() as f64 / (sample_rate as f64 * channels as f64)
        );

        Self {
            samples,
            sample_rate,
            channels,
            duration,
        }
    }

    /// Get the number of frames (samples per channel).
    pub fn frame_count(&self) -> usize {
        self.samples.len() / self.channels as usize
    }
}
```

---

## Generic Audio Engine

**Location**: `libraries/soul-audio/src/engine.rs`

```rust
use soul_core::audio::{AudioOutput, AudioBuffer};
use soul_core::types::Track;
use soul_core::error::Result;
use crate::decoder::SymphoniaDecoder;
use crate::effects::EffectChain;
use std::sync::{Arc, Mutex};
use std::thread;

/// Generic audio engine that works with any AudioOutput implementation.
///
/// This is the core playback engine used across all platforms.
pub struct AudioEngine<O: AudioOutput> {
    /// Platform-specific audio output (injected)
    output: Arc<Mutex<O>>,

    /// Audio decoder (Symphonia)
    decoder: SymphoniaDecoder,

    /// Audio effects chain (EQ, compressor, etc.)
    effect_chain: EffectChain,

    /// Current playback state
    state: PlaybackState,

    /// Playback thread handle
    playback_thread: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum PlaybackState {
    Stopped,
    Playing,
    Paused,
}

impl<O: AudioOutput + 'static> AudioEngine<O> {
    /// Create a new audio engine with injected output.
    ///
    /// # Arguments
    /// * `output` - Platform-specific audio output implementation
    ///
    /// # Example
    /// ```rust
    /// use soul_audio::AudioEngine;
    /// use soul_audio_desktop::CpalOutput;
    ///
    /// let output = CpalOutput::new()?;
    /// let engine = AudioEngine::new(output);
    /// ```
    pub fn new(output: O) -> Self {
        Self {
            output: Arc::new(Mutex::new(output)),
            decoder: SymphoniaDecoder::new(),
            effect_chain: EffectChain::new(),
            state: PlaybackState::Stopped,
            playback_thread: None,
        }
    }

    /// Load and play a track.
    ///
    /// # Arguments
    /// * `track` - Track to play
    pub fn play_track(&mut self, track: &Track) -> Result<()> {
        // Decode track
        let decoded = self.decoder.decode(&track.file_path)?;

        // Initialize output with track's sample rate and channels
        {
            let mut output = self.output.lock().unwrap();
            output.initialize(decoded.sample_rate, decoded.channels)?;
        }

        // Spawn playback thread
        let output = Arc::clone(&self.output);
        let mut effect_chain = self.effect_chain.clone();

        self.playback_thread = Some(thread::spawn(move || {
            for mut buffer in decoded.buffers {
                // Apply effects
                effect_chain.process(&mut buffer);

                // Send to output
                let mut output = output.lock().unwrap();
                if let Err(e) = output.play(&buffer) {
                    eprintln!("Playback error: {}", e);
                    break;
                }
            }
        }));

        self.state = PlaybackState::Playing;
        Ok(())
    }

    /// Pause playback.
    pub fn pause(&mut self) -> Result<()> {
        let mut output = self.output.lock().unwrap();
        output.pause()?;
        self.state = PlaybackState::Paused;
        Ok(())
    }

    /// Resume playback.
    pub fn resume(&mut self) -> Result<()> {
        let mut output = self.output.lock().unwrap();
        output.resume()?;
        self.state = PlaybackState::Playing;
        Ok(())
    }

    /// Stop playback.
    pub fn stop(&mut self) -> Result<()> {
        let mut output = self.output.lock().unwrap();
        output.stop()?;
        self.state = PlaybackState::Stopped;

        // Wait for playback thread to finish
        if let Some(handle) = self.playback_thread.take() {
            let _ = handle.join();
        }

        Ok(())
    }

    /// Set volume (0.0 to 1.0).
    pub fn set_volume(&mut self, volume: f32) -> Result<()> {
        let mut output = self.output.lock().unwrap();
        output.set_volume(volume)
    }

    /// Add an effect to the chain.
    pub fn add_effect(&mut self, effect: Box<dyn AudioEffect>) {
        self.effect_chain.add(effect);
    }

    /// Get current playback state.
    pub fn state(&self) -> PlaybackState {
        self.state
    }
}
```

---

## Platform Implementations

### Desktop (CPAL)

**Location**: `libraries/soul-audio-desktop/src/cpal_output.rs`

```rust
use soul_core::audio::{AudioOutput, AudioBuffer};
use soul_core::error::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct CpalOutput {
    device: Device,
    stream: Option<Stream>,
    config: StreamConfig,
    is_playing: bool,
}

impl CpalOutput {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| "No output device available")?;

        let config = device.default_output_config()?.into();

        Ok(Self {
            device,
            stream: None,
            config,
            is_playing: false,
        })
    }
}

impl AudioOutput for CpalOutput {
    fn initialize(&mut self, sample_rate: u32, channels: u16) -> Result<()> {
        self.config.sample_rate = cpal::SampleRate(sample_rate);
        self.config.channels = channels;
        Ok(())
    }

    fn play(&mut self, buffer: &AudioBuffer) -> Result<()> {
        // CPAL implementation details...
        // Build stream, write samples, etc.
        self.is_playing = true;
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            stream.pause()?;
        }
        self.is_playing = false;
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        if let Some(stream) = &self.stream {
            stream.play()?;
        }
        self.is_playing = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.stream = None;
        self.is_playing = false;
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> Result<()> {
        // CPAL doesn't have built-in volume control
        // Apply in software during buffer write
        Ok(())
    }

    fn get_latency(&self) -> Duration {
        // Query CPAL stream latency
        Duration::from_millis(50) // Example
    }

    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    fn channels(&self) -> u16 {
        self.config.channels
    }
}
```

### Mobile (iOS)

**Location**: `libraries/soul-audio-mobile/src/ios.rs`

```rust
use soul_core::audio::{AudioOutput, AudioBuffer};
use soul_core::error::Result;
use std::time::Duration;

/// iOS audio output that bridges to Swift AVAudioEngine.
pub struct IosAudioOutput {
    // This struct holds a reference to the Swift bridge
    // Actual audio output happens in Swift code
    is_playing: bool,
    sample_rate: u32,
    channels: u16,
}

impl IosAudioOutput {
    pub fn new() -> Result<Self> {
        // Call Swift bridge to initialize AVAudioEngine
        // See applications/mobile/src-tauri/gen/apple/Sources/AudioBridge.swift
        Ok(Self {
            is_playing: false,
            sample_rate: 44100,
            channels: 2,
        })
    }
}

impl AudioOutput for IosAudioOutput {
    fn initialize(&mut self, sample_rate: u32, channels: u16) -> Result<()> {
        self.sample_rate = sample_rate;
        self.channels = channels;
        // Call Swift: setupAudioEngine(sampleRate, channels)
        Ok(())
    }

    fn play(&mut self, buffer: &AudioBuffer) -> Result<()> {
        // Call Swift: playBuffer(buffer.samples)
        self.is_playing = true;
        Ok(())
    }

    fn pause(&mut self) -> Result<()> {
        // Call Swift: pausePlayback()
        self.is_playing = false;
        Ok(())
    }

    fn resume(&mut self) -> Result<()> {
        // Call Swift: resumePlayback()
        self.is_playing = true;
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        // Call Swift: stopPlayback()
        self.is_playing = false;
        Ok(())
    }

    fn set_volume(&mut self, volume: f32) -> Result<()> {
        // Call Swift: setVolume(volume)
        Ok(())
    }

    fn get_latency(&self) -> Duration {
        Duration::from_millis(30)
    }

    fn is_playing(&self) -> bool {
        self.is_playing
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn channels(&self) -> u16 {
        self.channels
    }
}
```

**Swift Bridge**: `applications/mobile/src-tauri/gen/apple/Sources/AudioBridge.swift`

```swift
import AVFoundation
import Tauri

class AudioBridge: Plugin {
    private var audioEngine: AVAudioEngine?
    private var playerNode: AVAudioPlayerNode?

    @objc public func setupAudioEngine(_ invoke: Invoke) {
        let args = invoke.args as! [String: Any]
        let sampleRate = args["sampleRate"] as! Double
        let channels = args["channels"] as! UInt32

        audioEngine = AVAudioEngine()
        playerNode = AVAudioPlayerNode()

        guard let engine = audioEngine, let player = playerNode else {
            invoke.reject("Failed to initialize audio engine")
            return
        }

        engine.attach(player)

        let format = AVAudioFormat(
            standardFormatWithSampleRate: sampleRate,
            channels: channels
        )!

        engine.connect(player, to: engine.mainMixerNode, format: format)

        try? engine.start()
        invoke.resolve()
    }

    @objc public func playBuffer(_ invoke: Invoke) {
        let args = invoke.args as! [String: Any]
        let samples = args["samples"] as! [Float]

        // Convert samples to AVAudioPCMBuffer
        // Schedule buffer for playback
        playerNode?.play()

        invoke.resolve()
    }

    @objc public func pausePlayback(_ invoke: Invoke) {
        playerNode?.pause()
        invoke.resolve()
    }

    @objc public func resumePlayback(_ invoke: Invoke) {
        playerNode?.play()
        invoke.resolve()
    }

    @objc public func stopPlayback(_ invoke: Invoke) {
        playerNode?.stop()
        invoke.resolve()
    }

    @objc public func setVolume(_ invoke: Invoke) {
        let args = invoke.args as! [String: Any]
        let volume = args["volume"] as! Float

        playerNode?.volume = volume
        invoke.resolve()
    }
}
```

---

## Dependency Injection in Applications

### Desktop Application

**Location**: `applications/desktop/src-tauri/src/main.rs`

```rust
use soul_audio::AudioEngine;
use soul_audio_desktop::CpalOutput;

fn main() {
    // Create platform-specific output
    let output = CpalOutput::new().expect("Failed to initialize audio output");

    // Inject into generic engine
    let engine = AudioEngine::new(output);

    // Use engine in Tauri app state
    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(engine)))
        .invoke_handler(tauri::generate_handler![
            commands::play_track,
            commands::pause,
            commands::resume,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Mobile Application

**Location**: `applications/mobile/src-tauri/src/lib.rs`

```rust
use soul_audio::AudioEngine;
use soul_audio_mobile::IosAudioOutput;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create platform-specific output (iOS in this case)
    let output = IosAudioOutput::new().expect("Failed to initialize audio output");

    // Inject into generic engine
    let engine = AudioEngine::new(output);

    tauri::Builder::default()
        .manage(Arc::new(Mutex::new(engine)))
        .invoke_handler(tauri::generate_handler![
            commands::play_track,
            commands::pause,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

---

## Benefits of This Approach

### 1. **Platform Independence**
- `soul-audio` has ZERO platform-specific code
- Easy to add new platforms (just implement `AudioOutput`)

### 2. **Testability**
```rust
// Mock audio output for testing
struct MockAudioOutput {
    played_buffers: Vec<AudioBuffer>,
}

impl AudioOutput for MockAudioOutput {
    fn play(&mut self, buffer: &AudioBuffer) -> Result<()> {
        self.played_buffers.push(buffer.clone());
        Ok(())
    }
    // ... other methods
}

#[test]
fn test_audio_engine() {
    let mock_output = MockAudioOutput::default();
    let mut engine = AudioEngine::new(mock_output);

    engine.play_track(&track).unwrap();

    // Verify buffers were sent to output
    assert_eq!(engine.output.played_buffers.len(), 42);
}
```

### 3. **Code Reuse**
- Decoder logic: 100% shared across platforms
- Effect chain: 100% shared
- Engine logic: 100% shared
- Only output layer differs

### 4. **Compile-Time Safety**
- Trait bounds ensure correct implementation
- No runtime errors from missing methods
- Type system enforces contracts

---

## Testing Strategy

### Unit Tests (soul-audio)
- Test with `MockAudioOutput`
- Verify decoder integration
- Test effect chain processing
- No actual audio output needed

### Integration Tests (Platform Crates)
- Test actual audio output on each platform
- Verify latency calculations
- Test volume control
- May require hardware or CI environment setup

---

## Future Extensions

### Server Streaming Output
```rust
pub struct StreamingAudioOutput {
    clients: Vec<StreamingClient>,
}

impl AudioOutput for StreamingAudioOutput {
    fn play(&mut self, buffer: &AudioBuffer) -> Result<()> {
        // Encode buffer to Opus/AAC
        // Send to all connected clients via WebRTC or HLS
        Ok(())
    }
}
```

### ESP32 Embedded Output
```rust
pub struct Esp32AudioOutput {
    i2s_driver: I2sDriver,
}

impl AudioOutput for Esp32AudioOutput {
    fn play(&mut self, buffer: &AudioBuffer) -> Result<()> {
        // Write to I2S peripheral
        self.i2s_driver.write(&buffer.samples)?;
        Ok(())
    }
}
```

---

## Summary

The audio abstraction layer uses **trait-based dependency injection** to enable Soul Player's audio engine to work across all platforms without modification.

**Key Principles**:
1. Define platform-agnostic trait (`AudioOutput`)
2. Implement trait for each platform
3. Generic engine accepts any implementation
4. Applications inject platform-specific output at runtime

This pattern is used throughout Soul Player for storage, sync, and other cross-platform concerns.
