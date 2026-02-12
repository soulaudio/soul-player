# Audio Resampling and Bit Depth Handling Research

**Research Date**: 2026-02-11
**Purpose**: Analyze open-source music players for best practices in resampling and bit depth conversion

---

## Executive Summary

This research analyzes four major open-source music players (Audacity, VLC, MPD, Clementine) and multiple resampling libraries to identify industry best practices for high-quality audio resampling and bit depth conversion.

**Key Findings**:
1. **libsamplerate** and **SoXR** are industry standards, with SoXR being 10x faster with comparable/better quality
2. **TPDF (Triangular) dither** is the professional standard for bit depth reduction
3. **256-512 sample buffers** provide the best balance for real-time playback
4. **Rust's rubato crate** offers production-ready real-time resampling with zero allocations
5. Quality settings should expose SNR (97-145 dB) and bandwidth (80-97%) tradeoffs

---

## 1. Resampling Libraries Comparison

### 1.1 libsamplerate (Secret Rabbit Code)

**Source**: [libsamplerate homepage](https://libsndfile.github.io/libsamplerate/)

**Converter Types**:
- `SRC_SINC_BEST_QUALITY` (0): SNR 97.43 dB, Bandwidth 96.96%, slowest
- `SRC_SINC_MEDIUM_QUALITY` (1): SNR 98.99 dB, Bandwidth 90.68%, medium speed
- `SRC_SINC_FASTEST` (2): SNR 97 dB, Bandwidth 80%, fastest sinc
- `SRC_ZERO_ORDER_HOLD` (3): Poor quality, very fast
- `SRC_LINEAR` (4): Poor quality, fast

**Features**:
- Arbitrary and time-varying sample rate conversion
- Downsampling/upsampling by factor of 256
- Irrational conversion ratios supported
- Signal-to-noise ratio up to 145 dB for best converter
- Industry standard, widely deployed

**Limitations**:
- High CPU usage at best quality settings
- Being replaced by SoXR in some applications

### 1.2 SoXR (SoX Resampler)

**Source**: [SoXR GitHub](https://github.com/chirlu/soxr)

**Quality Levels**:
- Very High Quality
- High Quality (default)
- Medium Quality
- Low Quality (fast)

**Key Features**:
- **10x faster than libsamplerate** with comparable/better quality
- FFT-based oversampling combined with bandlimited interpolation
- Configurable parameters:
  - `precision`: 16, 24, 32 bits
  - `phase_response`: Linear phase vs minimum phase
  - `passband_end`: 0-100% of Nyquist
  - `stopband_begin`: 0-100% of Nyquist
  - `attenuation`: Stop-band attenuation in dB
  - `flags`: Various processing options

**Integer Ratio Optimization**:
When target sample rate is an integer multiple of source rate, SoXR uses **50% less CPU** (e.g., 44.1 kHz → 88.2 kHz is faster than 44.1 kHz → 96 kHz)

**Latency Consideration**:
For real-time resampling, SoXR may have higher latency than non-FFT resamplers. For 44.1 kHz → 48 kHz at High Quality, latency is ~1000 output samples (~21ms at 48 kHz).

**MPD Configuration Example**:
```conf
audio_output_format "192000:24:2"
samplerate_converter "soxr very high"

# Custom SoXR settings
resampler {
    plugin "soxr"
    quality "custom"
    precision "32"
    phase_response "50"
    passband_end "95.45"
    stopband_begin "100"
    attenuation "30"
    flags "0"
}
```

### 1.3 r8brain-free-src

**Source**: [r8brain GitHub](https://github.com/avaneev/r8brain-free-src)

**Algorithm**:
- 2X oversampling relative to source/destination rate
- Polynomial-interpolated sinc function fractional delay filters
- 8-30 taps depending on precision
- Among the fastest high-precision converters

**Quality Settings**:
- **Transition Band**: 0.5% to 45% of spectral bandwidth
- **Stop-band Attenuation**: 49 to 218 dB
- Default 2% transition band gives linear response below 0.965×Nyquist

**Performance** (Ryzen 3700X, 24-bit, 44.1→96 kHz, 2% transition):
- **860×n_cores to 1270×n_cores** concurrent real-time streams at full CPU
- Automatic optimization for power-of-2 resampling (2X, 4X, 8X)

**Key Advantage**:
Fully linear-phase response with high precision, C++ header-only library (no dependencies).

### 1.4 Rubato (Rust)

**Source**: [rubato GitHub](https://github.com/HEnquist/rubato)

**Resampler Types**:

1. **Synchronous (FFT-based)**:
   - Fixed ratio conversions (e.g., 44.1 kHz → 48 kHz)
   - Faster than asynchronous
   - No ratio changes during runtime

2. **Asynchronous (Sinc interpolation)**:
   - Variable ratio support
   - Handles clock drift between devices
   - Cubic sinc interpolation (high quality)
   - Configurable sinc length and interpolation method

**Real-Time Safety**:
- Uses `process_into_buffer()` method for pre-allocated buffers
- **Zero allocations during processing**
- Disable logging for real-time applications

**Quality Configuration**:
- `sinc_len`: Length of sinc function (longer = higher quality, more CPU)
- `f_cutoff`: Anti-aliasing filter cutoff frequency
- `interpolation`: Cubic (best), Linear, or Nearest
- `oversampling_factor`: Higher = better quality at steeper cutoff

**Chunk Size Modes**:
- Fixed input, variable output
- Fixed output, variable input
- Both fixed (synchronous only)

**Rust Integration**:
Best choice for Rust projects, requires rustc 1.74+.

### 1.5 Speex Resampler

**Source**: [VLC configuration examples](https://gist.github.com/ageis/c79ada44c8208f688298bb8437c1d69e)

**Features**:
- Fast real-time audio resampler
- High perceptual sound quality
- Used in VLC, PulseAudio, ALSA

**Quality Settings**:
- Quality levels: 1 (lowest) to 10 (highest)
- Default: 4
- Recommended for high quality: 10

**VLC Configuration**:
```
audio-resampler=speex_resampler
speex-resampler-quality=10
```

---

## 2. Bit Depth Conversion & Dithering

### 2.1 Dither Types

**Sources**:
- [TPDF Dither Technical](https://robin-prillwitz.de/misc/tpdf/tpdf.html)
- [Audacity Dither Manual](https://manual.audacityteam.org/man/dither.html)
- [Prism Sound Dither Guide](https://www.prismsound.com/music_recording/products_subs/orpheus/online_manual/tech_dither.htm)

#### 2.1.1 No Dither
- Direct truncation/rounding
- Causes harmonic distortion on low-level signals
- **Only use when**:
  - Output bit depth ≥ source bit depth
  - Intermediate processing (dither once at final output)

#### 2.1.2 RPDF (Rectangular Probability Density Function)
- White noise with uniform distribution
- **Noise floor**: +4.8 dB above quantization noise
- Simple to implement: add random(-0.5 to +0.5) × LSB
- Low-level white noise character
- **Not recommended** for professional mastering

#### 2.1.3 TPDF (Triangular Probability Density Function)
- **Industry standard for professional audio**
- Two independent RPDF sources summed
- **Noise floor**: +10.8 dB above quantization noise
- Completely eliminates harmonic distortion
- Creates "white noise" floor perceptually more pleasant than RPDF
- **Use for**: Final mastering, CD preparation (24→16 bit)

**Implementation** (pseudo-code):
```rust
// TPDF dither: sum of two random values in [-0.5, 0.5]
let dither = (random(-0.5, 0.5) + random(-0.5, 0.5)) * lsb_value;
let dithered_sample = sample + dither;
let quantized = round(dithered_sample);
```

#### 2.1.4 Shaped Dither (Noise Shaping)
- Pushes quantization noise to higher frequencies (less audible)
- **Most complex** but best perceived quality
- Default in Audacity for "High-quality conversion"
- **Critical limitation**: Filter coefficients are sample-rate dependent
  - Audacity's shaped dither coefficients are designed for 44.1 kHz
  - At other sample rates, filter characteristics change proportionally
- Requires psychoacoustic modeling
- **Use for**: Final mastering to 16-bit when ultimate quality needed

**Audacity Implementation Details**:
- Source: [Dither.cpp](https://github.com/spinlockirqsave/audio/blob/master/audacity-read-only/src/Dither.cpp)
- Derived from Ardour project (Steve Harris)
- Dithering only applied when necessary (not to equal/higher bit depths)
- Samples always checked for clipping

### 2.2 When to Apply Dither

**Critical Rules**:

1. **Dither only once** at the final output stage
2. **Never dither**:
   - 24-bit or 32-bit output (noise floor below dither level)
   - Intermediate processing stages
   - Equal or higher bit depth conversions
   - Absolute silence (consider "rectangle" mode in Audacity)

3. **Always dither**:
   - 24-bit/32-bit → 16-bit for CD/streaming
   - Final mastering/export
   - Any bit depth reduction in final output

**Audacity Settings Architecture**:
- **Real-time conversion**: Used during playback (lower quality acceptable)
- **High-quality conversion**: Used for export/rendering (TPDF shaped default)

### 2.3 Bit Depth Conversion Process

**Standard Workflow**:
1. Internal processing at 32-bit float
2. Apply effects/processing
3. Check for clipping (mandatory)
4. Apply dither (if reducing bit depth)
5. Quantize to target bit depth

**Common Bit Depths**:
- **32-bit float**: Internal processing, no quantization noise
- **24-bit**: DVD-Audio, Blu-ray, professional recording (144 dB dynamic range)
- **16-bit**: CD, streaming (96 dB dynamic range, requires dither from 24/32-bit)

---

## 3. Buffer Size Recommendations

**Sources**:
- [Sweetwater Buffer Size Guide](https://www.sweetwater.com/sweetcare/articles/which-buffer-size-setting-should-i-use-in-my-daw/)
- [Rubato Documentation](https://docs.rs/rubato)
- [Gig Performer Latency Guide](https://gigperformer.com/audio-latency-buffer-size-and-sample-rate-explained)

### 3.1 Real-Time Playback

**Recommended Sizes**:
- **256 samples**: Best balance for responsive playback with stability
  - Latency at 44.1 kHz: ~5.8ms
  - Latency at 48 kHz: ~5.3ms
  - Good for live monitoring, real-time effects

- **512 samples**: Standard for music playback (acceptable latency)
  - Latency at 44.1 kHz: ~11.6ms
  - Latency at 48 kHz: ~10.7ms
  - Most common for consumer music players

- **1024 samples**: Mixing/mastering with heavy processing
  - Latency at 44.1 kHz: ~23.2ms
  - Latency at 48 kHz: ~21.3ms
  - Suitable when low latency not critical

- **2048+ samples**: Non-real-time processing, high plugin loads
  - Latency at 44.1 kHz: ~46.4ms+
  - Use for batch processing, complex mixes

### 3.2 Resampling Chunk Size

**Best Practice** (from Rubato docs):
- If audio API provides fixed buffer size, use that as resampler chunk size
- For variable API buffer sizes, use power-of-2 near average chunk size
- Shared buffer should be large enough to avoid blocking on disk I/O

**Example Configuration**:
```rust
// For 256-sample audio buffer at 48 kHz
let chunk_size = 256;
let resampler = FftFixedInOut::<f32>::new(
    48000, // input rate
    96000, // output rate
    chunk_size,
    2, // channels
)?;
```

### 3.3 Audible Latency Threshold

- **3-10ms**: Threshold where latency becomes audible/annoying
- **<6ms**: Imperceptible for most users (256 samples at 44.1 kHz)
- **>20ms**: Noticeable delay, problematic for live monitoring

---

## 4. Player-Specific Implementations

### 4.1 Audacity

**Architecture**:
- Uses libsamplerate for sample rate conversion
- Custom dither implementation (RPDF, TPDF, Shaped)
- Separate quality profiles for real-time vs export

**Quality Preferences**:
- Real-time conversion: Faster settings for playback
- High-quality conversion: Best quality for export
- Both configurable independently

**Key Issues Found**:
- [Issue #3025](https://github.com/audacity/audacity/issues/3025): Shaped dither is sample-rate specific (44.1 kHz)
- [Issue #1584](https://github.com/audacity/audacity/issues/1584): Dither applied on every effect (should be once on export)
- Bit-perfect import/export requires careful dither management

### 4.2 VLC

**Resampler Options**:
- `samplerate` (libsamplerate)
- `soxr` (SoX Resampler)
- `speex_resampler` (SpeexDSP)
- `ugly` (low quality, fast)

**Recommended Configuration**:
```
audio-resampler=soxr
# OR
audio-resampler=speex_resampler
speex-resampler-quality=10
```

**Filter Chain**:
- Modular audio filter architecture
- Resampler invoked automatically when sample rates differ
- Source: [filters.c](https://github.com/videolan/vlc/blob/master/src/audio_output/filters.c)

### 4.3 MPD (Music Player Daemon)

**Resampler Plugins**:
- `libsamplerate`: Older, slower
- `soxr`: Recommended (10x faster, better quality)

**Configuration** (`mpd.conf`):
```conf
# Simple configuration
audio_output_format "192000:24:2"
samplerate_converter "soxr very high"

# Advanced SoXR configuration
resampler {
    plugin "soxr"
    quality "custom"
    precision "32"
    phase_response "50"
    passband_end "95.45"
    stopband_begin "100"
    attenuation "30"
    flags "0"
}
```

**Quality Presets**:
- `"soxr very high"`: Best quality, highest CPU
- `"soxr high"`: Default, excellent balance
- `"soxr medium"`: Good quality, lower CPU
- `"soxr low"`: Fast, acceptable quality

**Selective Resampling**:
MPD can resample only specific sample rates (e.g., 44.1 kHz → 88.2 kHz but leave 48 kHz unchanged).

**Sources**:
- [MPD SoXR Discussion](https://www.runeaudio.com/forum/mpd-soxr-resampling-t996.html)
- [Bitlab's MPD SoXR Guide](https://www.bitlab.nl/page_id=435)

### 4.4 Clementine

**Audio Engine**: GStreamer-based

**Issues Found**:
- [Issue #6132](https://github.com/clementine-player/clementine/issues/6132): GStreamer resampling quality options not exposed
- GStreamer API contains quality settings, but Clementine doesn't expose them
- Only target sample rate configurable

**Buffer Settings**:
- Configurable in Settings → Playback
- Users report buffer settings don't always affect behavior
- Multiple buffer underrun issues reported

**Limitation**:
Less control over resampling quality compared to MPD/VLC.

---

## 5. Rust Audio Ecosystem

### 5.1 Symphonia Integration

**Sources**:
- [Symphonia GitHub](https://github.com/pdeljanov/Symphonia)
- [Resampling Discussion #131](https://github.com/pdeljanov/Symphonia/discussions/131)

**Key Points**:
- Symphonia decodes at native file sample rate
- **No built-in resampling** (by design)
- Recommended external resamplers for Rust:
  - **rubato**: Pure Rust, real-time safe, best integration
  - **samplerate-rs**: libsamplerate bindings
  - **libspeex-rs**: Speex bindings
  - **soxr-rs**: SoXR bindings

**Bit Depth Handling**:
- Decoded samples in native format (memory efficient)
- `AudioBufferRef` for direct access
- `SampleBuffer`/`RawSampleBuffer` for format conversion
- Convenience methods for f32 conversion in real-time

### 5.2 Recommended Rust Stack

**For Soul Player**:
```
Symphonia (decode) → rubato (resample) → cpal (output)
```

**Advantages**:
- Pure Rust, no C dependencies
- Real-time safe (zero allocations during processing)
- Excellent performance
- Cross-platform
- Type-safe

**Configuration Example**:
```rust
use rubato::{FftFixedInOut, Resampler};

// Create resampler (do this ONCE, not in audio callback)
let resampler = FftFixedInOut::<f32>::new(
    44100,  // input sample rate
    48000,  // output sample rate
    512,    // chunk size (match buffer size)
    2,      // channels
)?;

// Pre-allocate output buffer
let mut output_buffer = resampler.output_buffer_allocate(true);

// In audio callback (real-time safe)
resampler.process_into_buffer(
    &input_chunks,
    &mut output_buffer,
    None
)?;
```

---

## 6. Key Takeaways & Recommendations

### 6.1 For Soul Player Implementation

#### Resampling:
1. **Use rubato** (pure Rust, real-time safe)
   - FftFixedInOut for synchronous resampling
   - Configure sinc parameters for quality/performance tradeoff

2. **Expose quality settings**:
   - High Quality (longer sinc, cubic interpolation)
   - Balanced (medium sinc, cubic interpolation)
   - Fast (shorter sinc, linear interpolation)

3. **Buffer size**: **512 samples** (good balance for music playback)
   - At 48 kHz: 10.7ms latency (imperceptible)
   - At 44.1 kHz: 11.6ms latency (imperceptible)

4. **Pre-allocate buffers**: Initialize resampler outside audio callback

#### Bit Depth Conversion:
1. **Process internally at 32-bit float**
2. **Apply TPDF dither** when reducing to 16-bit
3. **Dither only once** at final output stage
4. **Skip dither** for 24-bit output (noise floor too low)

#### Configuration:
```rust
pub enum ResamplingQuality {
    High,      // rubato with long sinc, cubic interpolation
    Balanced,  // rubato with medium sinc, cubic interpolation
    Fast,      // rubato with short sinc, linear interpolation
}

pub enum DitherType {
    None,      // For 24/32-bit output
    TPDF,      // For 16-bit output (recommended)
    Shaped,    // For 16-bit output (advanced, sample-rate dependent)
}
```

### 6.2 Missing in Current Implementation

1. **Quality presets**: Expose resampling quality to users
2. **Dithering**: Implement TPDF for 16-bit output devices
3. **Buffer size configuration**: Allow users to tune latency/stability tradeoff
4. **Metrics**: Log actual SNR, latency, CPU usage for monitoring

### 6.3 Industry Standards Summary

| Component | Standard Choice | Alternative | Notes |
|-----------|----------------|-------------|-------|
| Resampler | SoXR | libsamplerate | SoXR is 10x faster |
| Rust Resampler | rubato | samplerate-rs (bindings) | rubato is pure Rust, real-time safe |
| Dither Type | TPDF | Shaped | TPDF for CD mastering, Shaped for ultimate quality |
| Buffer Size | 256-512 samples | 1024+ for mixing | 512 is sweet spot for playback |
| Bit Depth Processing | 32-bit float | 64-bit float | 32-bit sufficient for music |

---

## 7. Additional Resources

### Technical Papers:
- [Digital Audio Resampling](https://ccrma.stanford.edu/~jos/resample/resample.pdf) - Julius O. Smith III, Stanford
- [The Quest For The Perfect Resampler](http://ldesoras.free.fr/doc/articles/resampler-en.pdf) - Laurent De Soras
- [Dithering and Noise Shaping](http://audio.rightmark.org/lukin/dither/dither.pdf) - Alexey Lukin

### Comparison Tools:
- [Audio Sample Rate Converters Comparison](https://lastique.github.io/src_test/) - Objective measurements of SRC quality

### Libraries Documentation:
- [libsamplerate API](https://libsndfile.github.io/libsamplerate/api.html)
- [SoXR Documentation](http://sox.sourceforge.net/)
- [rubato API Docs](https://docs.rs/rubato)
- [r8brain README](https://github.com/avaneev/r8brain-free-src/blob/master/README.md)

### Forums & Discussions:
- [Hydrogen Audio Forums](https://hydrogenaud.io/) - Audiophile discussions on resampling
- [KVR Audio DSP Forum](https://www.kvraudio.com/forum/viewforum.php?f=33) - Audio programming community

---

## 8. Implementation Checklist

For integrating into Soul Player:

- [ ] Add rubato dependency to Cargo.toml
- [ ] Create `ResamplingQuality` enum and configuration
- [ ] Implement TPDF dither for 16-bit output
- [ ] Pre-allocate resampler and buffers outside audio callback
- [ ] Expose quality settings in UI (High/Balanced/Fast)
- [ ] Add buffer size configuration (256/512/1024)
- [ ] Test with various sample rates (44.1, 48, 88.2, 96, 192 kHz)
- [ ] Measure CPU usage and latency for each quality preset
- [ ] Document tradeoffs in user-facing help/tooltips
- [ ] Add integration tests for dithering correctness
- [ ] Benchmark resampling performance vs alternatives

---

## Sources

All information synthesized from the following sources:

### Audacity
- [Audacity GitHub Repository](https://github.com/audacity/audacity)
- [Dither.cpp Implementation](https://github.com/spinlockirqsave/audio/blob/master/audacity-read-only/src/Dither.cpp)
- [Shaped Dither Issue #3025](https://github.com/audacity/audacity/issues/3025)
- [Dither Application Issue #1584](https://github.com/audacity/audacity/issues/1584)
- [Audacity Dither Manual](https://manual.audacityteam.org/man/dither.html)
- [Quality Preferences Manual](https://manual.audacityteam.org/man/quality_preferences.html)

### VLC
- [VLC GitHub Repository](https://github.com/videolan/vlc)
- [Audio Output Filters](https://github.com/videolan/vlc/blob/master/src/audio_output/filters.c)
- [VLC Best Settings Gist](https://gist.github.com/ageis/c79ada44c8208f688298bb8437c1d69e)
- [Arch Linux Bug #41414](https://bugs.archlinux.org/task/41414)

### MPD
- [MPD GitHub Repository](https://github.com/MusicPlayerDaemon/MPD)
- [MPD SoXR Discussion](https://www.runeaudio.com/forum/mpd-soxr-resampling-t996.html)
- [Bitlab's MPD SoXR Guide](https://www.bitlab.nl/page_id=435)
- [Snakeoil OS MPD Resampling](https://www.snakeoil-os.net/forums/Thread-How-to-resample-with-MPD)

### Clementine
- [Clementine GitHub Repository](https://github.com/clementine-player/clementine)
- [Resampling Quality Issue #6132](https://github.com/clementine-player/clementine/issues/6132)
- [Buffer Issues](https://github.com/clementine-player/Clementine/issues/157)

### Resampling Libraries
- [libsamplerate Homepage](https://libsndfile.github.io/libsamplerate/)
- [libsamplerate Quality Specs](https://libsndfile.github.io/libsamplerate/quality.html)
- [SoXR GitHub](https://github.com/chirlu/soxr)
- [r8brain-free-src GitHub](https://github.com/avaneev/r8brain-free-src)
- [rubato GitHub](https://github.com/HEnquist/rubato)
- [rubato Documentation](https://docs.rs/rubato)
- [Audio Sample Rate Converters Comparison](https://lastique.github.io/src_test/)

### Symphonia
- [Symphonia GitHub](https://github.com/pdeljanov/Symphonia)
- [Symphonia Resampling Discussion #131](https://github.com/pdeljanov/Symphonia/discussions/131)
- [Symphonia Documentation](https://docs.rs/symphonia)

### Dithering
- [TPDF Dither Technical](https://robin-prillwitz.de/misc/tpdf/tpdf.html)
- [Wikipedia: Dither](https://en.wikipedia.org/wiki/Dither)
- [Audio Sorcerer: Dithering](https://audiosorcerer.com/post/what-is-audio-dithering/)
- [Digital Sound & Music: Dither Mathematics](https://digitalsoundandmusic.com/5-3-7-the-mathematics-of-dithering-and-noise-shaping/)
- [Prism Sound Dither Guide](https://www.prismsound.com/music_recording/products_subs/orpheus/online_manual/tech_dither.htm)

### Buffer Sizing
- [Sweetwater Buffer Size Guide](https://www.sweetwater.com/sweetcare/articles/which-buffer-size-setting-should-i-use-in-my-daw/)
- [Gig Performer Latency Guide](https://gigperformer.com/audio-latency-buffer-size-and-sample-rate-explained)
- [Focusrite Buffer Explanation](https://support.focusrite.com/hc/en-gb/articles/115004120965-Sample-Rate-Bit-Depth-and-Buffer-Size-Explained)
- [Native Instruments Optimization](https://support.native-instruments.com/hc/en-us/articles/360001102997-Optimizing-the-Settings-for-Your-Native-Instruments-Audio-Interface)

### General Audio Processing
- [Rubber Band Library](https://breakfastquay.com/rubberband/) (time stretching)
- [Wikipedia: Sample Rate Conversion](https://en.wikipedia.org/wiki/Sample-rate_conversion)
- [Wikipedia: Audio Bit Depth](https://en.wikipedia.org/wiki/Audio_bit_depth)

---

**Document Version**: 1.0
**Last Updated**: 2026-02-11
**Author**: Claude Code Research
