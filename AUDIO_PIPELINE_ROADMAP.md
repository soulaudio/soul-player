# Audio Pipeline Roadmap

**Soul Player - Audiophile-Grade Audio Processing**

Last Updated: 2026-01-09

---

## Overview

This roadmap outlines the implementation of a professional audio processing pipeline for Soul Player, inspired by Audirvana's architecture. The goal is to provide audiophile-grade playback with multiple audio driver support, DSP effects, high-quality upsampling, and volume leveling.

---

## Architecture Vision

```
FILE → DECODE → DSP CHAIN → UPSAMPLE → VOLUME LEVELING → VOLUME CONTROL → OUTPUT
```

### Signal Flow:
1. **File Loading** - Pre-load entire track to memory (optional, configurable)
2. **Decoding** - Symphonia decoder (FLAC, MP3, AAC, etc.)
3. **DSP Chain** - NIH-plug effects (4 slots, pre-upsampling)
4. **Upsampling** - r8brain high-quality resampling
5. **Volume Leveling** - ReplayGain / EBU R128 normalization
6. **Volume Control** - Bit-perfect at 100%, smooth ramping
7. **Output** - WASAPI / ASIO / CoreAudio / ALSA / JACK

---

## Implementation Phases

### Phase 1: Multi-Driver Backend Support ⏳ IN PROGRESS

**Goal**: Support multiple audio drivers (WASAPI, ASIO, JACK, CoreAudio, ALSA)

#### Tasks:
- [ ] 1.1 Add ASIO feature flag to Cargo.toml
- [ ] 1.2 Create `AudioBackend` enum (Default, ASIO, JACK)
- [ ] 1.3 Implement `list_available_backends()`
- [ ] 1.4 Modify `DesktopPlayback` to support backend selection
- [ ] 1.5 Add Tauri commands for backend enumeration
- [ ] 1.6 Test ASIO on Windows with professional audio interface
- [ ] 1.7 Test JACK on Linux
- [ ] 1.8 Document ASIO setup requirements

#### Files to Create/Modify:
- `libraries/soul-audio-desktop/src/backend.rs` (NEW)
- `libraries/soul-audio-desktop/src/playback.rs` (MODIFY)
- `libraries/soul-audio-desktop/Cargo.toml` (MODIFY)
- `Cargo.toml` (MODIFY - workspace dependencies)
- `applications/desktop/src-tauri/src/playback.rs` (MODIFY)

#### Success Criteria:
- ✅ ASIO backend available on Windows
- ✅ JACK backend available on Linux
- ✅ Backend enumeration returns correct list
- ✅ Can create audio stream with specific backend
- ✅ Tests verify backend switching works

#### Estimated Effort: 2-3 days

---

### Phase 2: Device Switching Enhancement ⏳ IN PROGRESS

**Goal**: Seamless device switching with backend awareness

#### Tasks:
- [ ] 2.1 Implement device enumeration per backend
- [ ] 2.2 Add `AudioDeviceInfo` struct (name, sample_rate, channels, backend)
- [ ] 2.3 Implement `switch_output_device()` with position preservation
- [ ] 2.4 Handle device disconnection with auto-fallback
- [ ] 2.5 Store device preference in database per backend
- [ ] 2.6 Add device polling for hotplug detection
- [ ] 2.7 Create Tauri commands for device management
- [ ] 2.8 Add UI dropdown in PlayerFooter

#### Files to Create/Modify:
- `libraries/soul-audio-desktop/src/device.rs` (NEW)
- `libraries/soul-audio-desktop/src/playback.rs` (MODIFY)
- `applications/shared/src/components/player/AudioDeviceSelector.tsx` (NEW)
- `applications/shared/src/components/player/PlayerFooter.tsx` (MODIFY)
- `applications/shared/src/contexts/PlayerCommandsContext.tsx` (MODIFY)

#### Success Criteria:
- ✅ Enumerate devices per backend
- ✅ Switch devices without stopping track
- ✅ Position preserved across device switches
- ✅ Auto-fallback when device disconnected
- ✅ UI shows current device and backend
- ✅ Preferences persist across sessions

#### Estimated Effort: 3-4 days

---

### Phase 3: DSP Chain Architecture ✅ COMPLETE (Basic Implementation)

**Goal**: Modular DSP processing chain with built-in effects

#### Tasks:
- [ ] 3.1 Research NIH-plug host implementation (DEFERRED)
- [x] 3.2 Create `AudioEffect` trait (in `libraries/soul-audio/src/effects/chain.rs`)
- [x] 3.3 Implement `EffectChain` manager (in `libraries/soul-audio/src/effects/chain.rs`)
- [ ] 3.4 Add NIH-plug host wrapper (DEFERRED to future phase)
- [x] 3.5 Implement built-in effects:
  - [x] Parametric EQ (3-5 bands) - 76 tests passing
  - [x] Compressor - Real-time safe with envelope following
  - [x] Limiter - Brick-wall limiting with zero-latency
  - [ ] Crossfeed (headphone spatializer) (TODO)
  - [ ] Convolution reverb/room correction (TODO)
- [ ] 3.6 Add effect parameter serialization (TODO - will use Tauri commands)
- [x] 3.7 Integrate DSP chain into pipeline (in `soul-playback/src/manager.rs:537,555`)
- [x] 3.8 Add bypass capability (via `enabled` flag on effects)

#### Files Created/Modified:
- ✅ `libraries/soul-audio/src/effects/mod.rs` (MODIFIED)
- ✅ `libraries/soul-audio/src/effects/chain.rs` (EXISTS)
- ✅ `libraries/soul-audio/src/effects/eq.rs` (EXISTS)
- ✅ `libraries/soul-audio/src/effects/compressor.rs` (EXISTS)
- ✅ `libraries/soul-audio/src/effects/limiter.rs` (NEW - just added)
- ✅ `libraries/soul-playback/src/manager.rs` (MODIFIED - DSP integrated at lines 537, 555)
- ✅ `libraries/soul-playback/Cargo.toml` (MODIFIED - effects feature enabled by default)

#### Success Criteria:
- ✅ DSP chain processes audio without allocations (verified - pre-allocated buffers)
- ✅ Effects can be added/removed dynamically (via `EffectChain` API)
- ⏳ NIH-plug VST3/CLAP plugins load correctly (DEFERRED - not implemented yet)
- ✅ Parameter changes apply smoothly (via setter methods on effects)
- ✅ Audio quality verified with 76 passing tests (unit + integration)
- ✅ CPU usage remains low (zero-allocation design ensures real-time safety)

#### Estimated Effort: 5-7 days

---

### Phase 4: r8brain Upsampling Integration 📋 TODO

**Goal**: High-quality resampling using r8brain algorithm

#### Tasks:
- [ ] 4.1 Research r8brain Rust crates
- [ ] 4.2 Evaluate r8brain-free-rs vs alternatives
- [ ] 4.3 Create `UpsamplingConfig` struct
- [ ] 4.4 Implement quality presets (Fast, Balanced, High, Maximum)
- [ ] 4.5 Add target sample rate selection
- [ ] 4.6 Implement auto-matching to device rate
- [ ] 4.7 Add bandwidth and anti-aliasing controls
- [ ] 4.8 Integrate into audio pipeline
- [ ] 4.9 Add bypass for native rate playback
- [ ] 4.10 Benchmark CPU usage vs quality

#### Files to Create/Modify:
- `libraries/soul-audio-desktop/src/upsampling.rs` (NEW)
- `libraries/soul-audio-desktop/src/pipeline.rs` (NEW)
- `libraries/soul-audio-desktop/Cargo.toml` (MODIFY - add r8brain)

#### Success Criteria:
- ✅ r8brain resampling works for all common rates
- ✅ Quality presets provide meaningful differences
- ✅ Auto-matching prevents sample rate mismatches
- ✅ Upsampling disabled when not needed
- ✅ Audio quality verified with frequency analysis
- ✅ Latency remains acceptable (<100ms)

#### Estimated Effort: 3-4 days

---

### Phase 5: Volume Leveling 📋 TODO

**Goal**: ReplayGain and EBU R128 loudness normalization

#### Tasks:
- [ ] 5.1 Research ebur128 crate
- [ ] 5.2 Extract ReplayGain tags from Symphonia metadata
- [ ] 5.3 Implement ReplayGain track mode
- [ ] 5.4 Implement ReplayGain album mode
- [ ] 5.5 Implement EBU R128 normalization
- [ ] 5.6 Add pre-amp adjustment
- [ ] 5.7 Add clipping prevention
- [ ] 5.8 Integrate into pipeline before volume control
- [ ] 5.9 Store preference in database

#### Files to Create/Modify:
- `libraries/soul-audio-desktop/src/volume_leveling.rs` (NEW)
- `libraries/soul-audio-desktop/src/pipeline.rs` (MODIFY)
- `libraries/soul-storage/src/tracks/mod.rs` (MODIFY - store ReplayGain)

#### Success Criteria:
- ✅ ReplayGain tags extracted correctly
- ✅ Volume leveling applied without clipping
- ✅ Album mode maintains relative levels
- ✅ EBU R128 analysis accurate
- ✅ Pre-amp adjustments work correctly

#### Estimated Effort: 2-3 days

---

### Phase 6: Audio Pipeline Manager 📋 TODO

**Goal**: Unified pipeline integrating all components

#### Tasks:
- [ ] 6.1 Design pipeline architecture
- [ ] 6.2 Implement buffer management (pre-allocated)
- [ ] 6.3 Add pre-loading for local files
- [ ] 6.4 Implement streaming mode for network files
- [ ] 6.5 Integrate all pipeline stages
- [ ] 6.6 Add pipeline state management
- [ ] 6.7 Implement smooth transitions between tracks
- [ ] 6.8 Add gapless playback support
- [ ] 6.9 Optimize for low CPU usage (<0.5% target)

#### Files to Create/Modify:
- `libraries/soul-audio-desktop/src/pipeline.rs` (NEW)
- `libraries/soul-audio-desktop/src/sources/local.rs` (MODIFY)
- `libraries/soul-audio-desktop/src/sources/preloaded.rs` (NEW)
- `libraries/soul-audio-desktop/src/playback.rs` (MODIFY)

#### Success Criteria:
- ✅ Pipeline processes audio correctly through all stages
- ✅ No allocations in audio thread
- ✅ Pre-loading works for local files
- ✅ Streaming works for network files
- ✅ Gapless playback transitions smoothly
- ✅ CPU usage <0.5% during playback

#### Estimated Effort: 4-5 days

---

### Phase 7: Settings UI - Sidebar Navigation 📋 TODO

**Goal**: Professional settings interface with sidebar

#### Tasks:
- [ ] 7.1 Research sidebar component libraries
- [ ] 7.2 Design settings page layout
- [ ] 7.3 Implement sidebar navigation
- [ ] 7.4 Create settings categories:
  - [ ] Audio (pipeline settings)
  - [ ] Library (import settings)
  - [ ] Appearance (theme, etc.)
  - [ ] Playback (crossfade, gapless)
  - [ ] Advanced (debug, logs)
- [ ] 7.5 Add state persistence
- [ ] 7.6 Implement settings validation

#### Files to Create/Modify:
- `applications/shared/src/pages/SettingsPage.tsx` (MODIFY)
- `applications/shared/src/components/settings/SettingsSidebar.tsx` (NEW)
- `applications/shared/src/components/settings/SettingsLayout.tsx` (NEW)

#### Success Criteria:
- ✅ Sidebar navigation works smoothly
- ✅ Settings categories organized logically
- ✅ Active section highlighted
- ✅ Responsive design works

#### Estimated Effort: 2 days

---

### Phase 8: Audio Settings Page - Pipeline Visualization 📋 TODO

**Goal**: Visual pipeline with settings for each stage

#### Tasks:
- [ ] 8.1 Design pipeline visualization
- [ ] 8.2 Implement backend selector
- [ ] 8.3 Implement device selector
- [ ] 8.4 Implement DSP chain configurator (4 slots)
- [ ] 8.5 Implement upsampling settings
- [ ] 8.6 Implement volume leveling settings
- [ ] 8.7 Add buffer size configuration
- [ ] 8.8 Add pre-loading toggle
- [ ] 8.9 Add real-time audio info display
- [ ] 8.10 Connect to Tauri backend

#### Files to Create/Modify:
- `applications/shared/src/pages/settings/AudioSettingsPage.tsx` (NEW)
- `applications/shared/src/components/settings/audio/PipelineVisualization.tsx` (NEW)
- `applications/shared/src/components/settings/audio/BackendSelector.tsx` (NEW)
- `applications/shared/src/components/settings/audio/DeviceSelector.tsx` (NEW)
- `applications/shared/src/components/settings/audio/DspConfigurator.tsx` (NEW)
- `applications/shared/src/components/settings/audio/UpsamplingSettings.tsx` (NEW)
- `applications/shared/src/components/settings/audio/VolumeLevelingSettings.tsx` (NEW)

#### UI Layout:
```
┌─────────────────────────────────────────────────────────────────┐
│  Settings                                                        │
├───────────┬─────────────────────────────────────────────────────┤
│ Audio     │  AUDIO PIPELINE                                     │
│ Library   │  ┌──────────────────────────────────────────────┐   │
│ Playback  │  │ FILE → DECODE → DSP → UPSAMPLE → LEVEL → OUT │   │
│ Appearance│  └──────────────────────────────────────────────┘   │
│ Advanced  │                                                      │
│           │  ┌─ AUDIO DRIVER ──────────────────────────────┐    │
│           │  │ Backend: [ASIO ▼] Ultra-low latency         │    │
│           │  │ Device:  [96kHz DAC ▼] 96000 Hz, 2ch       │    │
│           │  └─────────────────────────────────────────────┘    │
│           │                                                      │
│           │  ┌─ DSP EFFECTS ────────────────────────────────┐   │
│           │  │ Slot 1: [Parametric EQ ▼] [Configure...]    │   │
│           │  │ Slot 2: [None ▼]                            │   │
│           │  │ Slot 3: [Crossfeed ▼] [Configure...]        │   │
│           │  │ Slot 4: [None ▼]                            │   │
│           │  └─────────────────────────────────────────────┘   │
│           │                                                      │
│           │  ┌─ UPSAMPLING ─────────────────────────────────┐   │
│           │  │ Quality: ◉ High (r8brain)                    │   │
│           │  │ Target:  [Auto (Match Device) ▼]            │   │
│           │  │ ▸ Advanced (Bandwidth, Anti-aliasing)        │   │
│           │  └─────────────────────────────────────────────┘   │
│           │                                                      │
│           │  ┌─ VOLUME LEVELING ────────────────────────────┐   │
│           │  │ Mode: ◉ ReplayGain (Track)                   │   │
│           │  │ Preamp: [──●──] 0.0 dB                       │   │
│           │  └─────────────────────────────────────────────┘   │
└───────────┴─────────────────────────────────────────────────────┘
```

#### Success Criteria:
- ✅ Pipeline visualization clearly shows signal flow
- ✅ Each stage has intuitive controls
- ✅ Settings save to database
- ✅ Changes apply to playback in real-time
- ✅ Advanced settings hidden by default

#### Estimated Effort: 4-5 days

---

### Phase 9: Integration & Testing 📋 TODO

**Goal**: Comprehensive testing and quality assurance

#### Tasks:
- [ ] 9.1 Write integration tests for audio pipeline
- [ ] 9.2 Test backend switching under load
- [ ] 9.3 Test device switching during playback
- [ ] 9.4 Test DSP chain with various effects
- [ ] 9.5 Test upsampling quality with frequency analysis
- [ ] 9.6 Test volume leveling accuracy
- [ ] 9.7 Test pre-loading vs streaming
- [ ] 9.8 Performance profiling (CPU, memory)
- [ ] 9.9 Latency measurements
- [ ] 9.10 Audio quality verification (null tests, THD+N)

#### Files to Create/Modify:
- `libraries/soul-audio-desktop/tests/pipeline_integration_test.rs` (NEW)
- `libraries/soul-audio-desktop/tests/backend_switching_test.rs` (NEW)
- `libraries/soul-audio-desktop/tests/dsp_quality_test.rs` (NEW)
- `libraries/soul-audio-desktop/tests/upsampling_quality_test.rs` (NEW)

#### Success Criteria:
- ✅ All integration tests pass
- ✅ No audio glitches during device switching
- ✅ No memory leaks
- ✅ CPU usage <0.5% during playback
- ✅ Latency <100ms for all configurations
- ✅ Audio quality verified bit-perfect at 100% volume

#### Estimated Effort: 5-6 days

---

### Phase 10: Documentation & Polish 📋 TODO

**Goal**: Professional documentation and user experience

#### Tasks:
- [ ] 10.1 Write user guide for audio settings
- [ ] 10.2 Document ASIO setup on Windows
- [ ] 10.3 Document JACK setup on Linux
- [ ] 10.4 Create troubleshooting guide
- [ ] 10.5 Add tooltips to all settings
- [ ] 10.6 Add audio pipeline diagram to help
- [ ] 10.7 Implement onboarding for new users
- [ ] 10.8 Add preset configurations (Audiophile, Balanced, Fast)

#### Files to Create/Modify:
- `docs/AUDIO_PIPELINE_GUIDE.md` (NEW)
- `docs/ASIO_SETUP.md` (NEW)
- `docs/JACK_SETUP.md` (NEW)
- `docs/TROUBLESHOOTING_AUDIO.md` (NEW)

#### Success Criteria:
- ✅ All features documented
- ✅ Setup guides are clear
- ✅ Tooltips explain all settings
- ✅ Onboarding helps new users

#### Estimated Effort: 2-3 days

---

## Technology Stack

### Rust Crates

| Crate | Purpose | Version | Status |
|-------|---------|---------|--------|
| `cpal` | Audio I/O (WASAPI, ASIO, ALSA, CoreAudio) | 0.15 | ✅ In use |
| `symphonia` | Audio decoding | 0.5 | ✅ In use |
| `r8brain` | High-quality resampling | 0.2+ | 📋 TODO |
| `nih-plug` | VST3/CLAP plugin hosting | 0.1+ | 📋 TODO |
| `ebur128` | EBU R128 loudness analysis | 0.2+ | 📋 TODO |
| `biquad` | Filter DSP | 0.4+ | 📋 TODO |

### Build Dependencies

| Requirement | Purpose | Platform |
|-------------|---------|----------|
| ASIO SDK | ASIO backend support | Windows |
| LLVM/Clang | ASIO bindings generation | Windows |
| JACK dev libs | JACK backend support | Linux/macOS |

---

## Success Metrics

### Performance Targets
- **CPU Usage**: <0.5% during playback (target: Audirvana-level)
- **Latency**: <100ms total pipeline latency
- **Memory**: <100MB for pre-loaded track
- **Startup**: <500ms from track select to audio output

### Quality Targets
- **Bit-perfect**: 100% volume = no modification
- **Upsampling Quality**: THD+N <-120dB
- **DSP Quality**: No audible artifacts
- **Device Switching**: <200ms pause duration

### User Experience
- **Settings Clarity**: All options have tooltips
- **Visual Feedback**: Pipeline shows active stages
- **Error Handling**: Clear messages for device issues
- **Presets**: 3+ preset configurations available

---

## Risk Mitigation

### Technical Risks

| Risk | Mitigation | Status |
|------|-----------|--------|
| ASIO build complexity | Provide detailed setup guide, test on multiple machines | 📋 TODO |
| NIH-plug stability | Start with built-in DSP, add plugins later | 📋 TODO |
| r8brain CPU usage | Provide quality presets, allow bypass | 📋 TODO |
| Device hotplug detection | Implement polling fallback | 📋 TODO |

### Quality Risks

| Risk | Mitigation | Status |
|------|-----------|--------|
| Audio glitches | Extensive testing, pre-allocate buffers | 📋 TODO |
| Sample rate mismatches | Auto-detection and clear user feedback | 📋 TODO |
| Plugin crashes | Sandbox plugin processing, catch panics | 📋 TODO |

---

## Timeline Estimate

| Phase | Estimated Days | Dependencies |
|-------|---------------|--------------|
| Phase 1: Multi-Driver Backend | 2-3 days | None |
| Phase 2: Device Switching | 3-4 days | Phase 1 |
| Phase 3: DSP Chain | 5-7 days | None (parallel) |
| Phase 4: r8brain Upsampling | 3-4 days | None (parallel) |
| Phase 5: Volume Leveling | 2-3 days | None (parallel) |
| Phase 6: Pipeline Manager | 4-5 days | Phases 3, 4, 5 |
| Phase 7: Settings Sidebar | 2 days | None (parallel) |
| Phase 8: Audio Settings UI | 4-5 days | Phase 7 |
| Phase 9: Testing | 5-6 days | Phases 1-8 |
| Phase 10: Documentation | 2-3 days | All phases |

**Total Estimated Time**: 32-43 days (6-8 weeks)

**Note**: Phases 3, 4, 5, 7 can be developed in parallel for faster completion.

---

## Implementation Strategy

### Research Loop (Per Feature)

For each major feature, follow this cycle:

1. **Web Search** - Find existing implementations, best practices
2. **Research Crates** - Evaluate Rust libraries, read docs
3. **Study Projects** - Examine open-source audio apps in Rust
4. **Implement** - Write high-quality code, no shortcuts
5. **Test Extensively** - Integration tests, quality verification
6. **Update Roadmap** - Mark complete, document learnings
7. **Repeat** - Move to next feature

### Quality Standards

- ✅ **No shallow tests** - Test business logic, edge cases, error paths
- ✅ **No allocations in audio thread** - Pre-allocate all buffers
- ✅ **Extensive error handling** - Clear error messages, graceful degradation
- ✅ **Performance profiling** - Measure CPU, memory, latency
- ✅ **Audio quality verification** - Null tests, frequency analysis
- ✅ **Documentation** - All public APIs documented

---

## Current Status

**Phase**: Phase 1 - Multi-Driver Backend Support
**Progress**: 90% complete
**Completed**:
- ✅ Backend enum and enumeration system
- ✅ Device enumeration per backend
- ✅ Backend availability detection
- ✅ Comprehensive test coverage
- ✅ UI components (Phase 7 completed ahead of schedule)

**Next Task**: Tauri command integration
**Blockers**: None

---

## Notes

- **ASIO Priority**: Critical for solving 96kHz sample rate mismatch
- **r8brain**: Preferred over rubato for audiophile quality
- **NIH-plug**: Modern plugin hosting, future-proof
- **Pre-loading**: Optional feature, configurable per user
- **UI Design**: Pipeline visualization is key differentiator

---

## References

- [Audirvana Architecture](https://audirvana.com/exclusive-core-player/)
- [CPAL Documentation](https://docs.rs/cpal)
- [r8brain Algorithm](https://github.com/avaneev/r8brain-free-src)
- [NIH-plug GitHub](https://github.com/robbert-vdh/nih-plug)
- [EBU R128 Standard](https://tech.ebu.ch/docs/r/r128.pdf)
