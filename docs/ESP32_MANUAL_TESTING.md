# ESP32 Manual Testing Checklist

## Overview

Due to hardware limitations, ESP32 firmware testing in CI is limited to build verification and QEMU-based unit tests. **All hardware I/O features require manual testing on real hardware before each release.**

## Why Manual Testing is Required

**QEMU Limitations**:
- ❌ No I2S/SPI/I2C/UART peripheral simulation
- ❌ No audio DAC output testing
- ❌ No SD card read/write simulation
- ❌ No e-ink display rendering
- ❌ No WiFi/Bluetooth radio hardware
- ❌ No real-time timing accuracy
- ❌ No power consumption measurement

**What CI Tests**:
- ✅ Firmware compiles successfully
- ✅ Binary size within limits
- ✅ QEMU CPU/memory unit tests
- ✅ Static code analysis

## Required Hardware

For complete testing, you'll need:

```
✓ ESP32-S3 development board
✓ I2S DAC (e.g., PCM5102)
✓ SD card (SDMMC interface)
✓ E-ink display (SPI)
✓ WiFi access point
✓ Bluetooth audio device (for testing BT streaming)
✓ Physical buttons/controls
✓ Power measurement tools (optional)
✓ USB cable for flashing/monitoring
```

## Testing Environment Setup

### 1. Install ESP-IDF Toolchain

```bash
# Install espup
cargo install espup

# Install ESP toolchain
espup install

# Source environment
. $HOME/export-esp.sh
```

### 2. Flash Firmware

```bash
# Navigate to firmware directory
cd applications/firmware

# Build firmware
cargo build --release

# Flash to device
espflash flash target/xtensa-esp32s3-espidf/release/soul-player-esp32

# Monitor serial output
espflash monitor
```

## Pre-Release Testing Checklist

### Core Functionality

#### 1. Audio Playback (DAC Output)

**Test**: Verify audio output quality and formats

- [ ] **MP3 Playback**
  - [ ] Play 44.1kHz MP3 file
  - [ ] Play 48kHz MP3 file
  - [ ] Verify no audio glitches or stuttering
  - [ ] Check left/right channel balance

- [ ] **FLAC Playback**
  - [ ] Play 44.1kHz FLAC file
  - [ ] Play 96kHz FLAC file (if supported)
  - [ ] Verify lossless quality

- [ ] **OGG/Vorbis Playback**
  - [ ] Play OGG file
  - [ ] Verify playback quality

- [ ] **WAV Playback**
  - [ ] Play 16-bit WAV file
  - [ ] Play 24-bit WAV file (if supported)

- [ ] **Sample Rate Conversion**
  - [ ] Test automatic resampling to DAC rate
  - [ ] Verify no audio artifacts

- [ ] **Audio Controls**
  - [ ] Volume up/down (0-100%)
  - [ ] Volume at 0% (mute)
  - [ ] Volume at 100% (no clipping)
  - [ ] Play/pause/resume
  - [ ] Stop playback
  - [ ] Next/previous track

**Expected Behavior**:
- Clean audio output with no pops, clicks, or stuttering
- Smooth volume transitions
- Correct left/right channel separation
- Proper sample rate conversion

**Common Issues**:
- Buffer underruns causing audio glitches
- I2S timing issues causing popping
- Incorrect channel mapping (left/right swapped)

#### 2. SD Card Operations

**Test**: File system read/write reliability

- [ ] **Card Detection**
  - [ ] Insert SD card → detected
  - [ ] Remove SD card → detected
  - [ ] Hot-swap during idle (should not crash)

- [ ] **File Reading**
  - [ ] Read small file (< 1MB)
  - [ ] Read medium file (10-50MB)
  - [ ] Read large file (> 100MB)
  - [ ] Read multiple files in sequence
  - [ ] Navigate folder structure

- [ ] **File Writing**
  - [ ] Write playlist
  - [ ] Write configuration file
  - [ ] Write metadata cache
  - [ ] Append to log file

- [ ] **Error Handling**
  - [ ] Corrupted file → graceful error
  - [ ] Missing file → graceful error
  - [ ] Full card → error message

- [ ] **Performance**
  - [ ] Read speed sufficient for 320kbps MP3
  - [ ] No audio stuttering during playback
  - [ ] Acceptable seek time (< 2 seconds)

**Expected Behavior**:
- Reliable card detection
- No file corruption on writes
- Consistent read performance
- Graceful handling of errors

**Common Issues**:
- SDMMC bus timing issues
- File system corruption on power loss
- Slow seek performance with large files

#### 3. Display Rendering (E-ink)

**Test**: UI rendering and refresh

- [ ] **Initial Boot**
  - [ ] Splash screen displays correctly
  - [ ] No partial updates or ghosting

- [ ] **Now Playing Screen**
  - [ ] Track title displays
  - [ ] Artist name displays
  - [ ] Album name displays
  - [ ] Progress bar updates
  - [ ] Time remaining updates

- [ ] **Menu Navigation**
  - [ ] Menu items render correctly
  - [ ] Selection highlight visible
  - [ ] Scrolling smooth (no artifacts)

- [ ] **Album Art (if supported)**
  - [ ] Album art loads
  - [ ] Correct aspect ratio
  - [ ] No visual corruption

- [ ] **Partial vs Full Refresh**
  - [ ] Partial refresh for progress bar
  - [ ] Full refresh for screen changes
  - [ ] No ghosting buildup over time

**Expected Behavior**:
- Clear, readable text
- No ghosting after full refresh
- Acceptable refresh times (< 1 second for full, < 200ms for partial)

**Common Issues**:
- E-ink ghosting from too many partial updates
- SPI bus timing causing visual artifacts
- Memory issues with album art rendering

#### 4. WiFi Connectivity

**Test**: Network operations

- [ ] **WiFi Connection**
  - [ ] Connect to WPA2 network
  - [ ] Connect to WPA3 network (if supported)
  - [ ] Reconnect after disconnect
  - [ ] Handle incorrect password gracefully

- [ ] **Network Operations**
  - [ ] Resolve DNS
  - [ ] HTTP GET request
  - [ ] HTTPS GET request (if supported)
  - [ ] Download file from server

- [ ] **Server Sync**
  - [ ] Connect to Soul Server
  - [ ] Authenticate user
  - [ ] Sync playlist changes
  - [ ] Download track metadata
  - [ ] Stream audio from server

- [ ] **Stability**
  - [ ] WiFi stays connected during playback
  - [ ] Reconnects if connection drops
  - [ ] No memory leaks during long sessions

**Expected Behavior**:
- Reliable connection establishment
- Stable connection during use
- Automatic reconnection on dropout
- No crashes or freezes

**Common Issues**:
- WiFi driver instability
- TLS/SSL memory issues
- Connection timeouts
- DNS resolution failures

#### 5. Bluetooth (Optional)

**Test**: Bluetooth audio streaming

- [ ] **BT Pairing**
  - [ ] Device discoverable
  - [ ] Pair with phone/headphones
  - [ ] Remember paired devices
  - [ ] Unpair device

- [ ] **Audio Streaming**
  - [ ] Stream audio to BT headphones
  - [ ] Audio quality acceptable
  - [ ] No stuttering or dropouts
  - [ ] Latency acceptable

- [ ] **Controls**
  - [ ] Volume control from BT device
  - [ ] Play/pause from BT device
  - [ ] Track navigation from BT device

**Expected Behavior**:
- Reliable pairing
- Stable audio streaming
- Low latency (< 200ms)

**Common Issues**:
- Bluetooth stack instability
- Audio buffer underruns
- High latency
- Pairing failures

#### 6. Physical Controls

**Test**: Button inputs and encoders

- [ ] **Button Mapping**
  - [ ] Play/pause button works
  - [ ] Next/previous track buttons work
  - [ ] Volume up/down buttons work
  - [ ] Menu button works
  - [ ] Back button works

- [ ] **Button Timing**
  - [ ] Single press registered
  - [ ] Long press registered
  - [ ] Double press (if supported)
  - [ ] No button bouncing issues

- [ ] **Rotary Encoder (if present)**
  - [ ] Clockwise rotation
  - [ ] Counter-clockwise rotation
  - [ ] Encoder button press
  - [ ] Smooth scrolling

**Expected Behavior**:
- Immediate button response
- No missed presses
- No double-triggering
- Smooth encoder rotation

**Common Issues**:
- Button debouncing needed
- Interrupt handling issues
- Encoder direction reversed

#### 7. OTA Updates

**Test**: Firmware updates over WiFi

- [ ] **OTA Discovery**
  - [ ] Check for updates
  - [ ] Display available version
  - [ ] Show update notes

- [ ] **OTA Download**
  - [ ] Download firmware binary
  - [ ] Verify checksum/signature
  - [ ] Display download progress

- [ ] **OTA Installation**
  - [ ] Flash new firmware
  - [ ] Reboot to new version
  - [ ] Verify new version running
  - [ ] Rollback on failure

- [ ] **Safety**
  - [ ] Handle WiFi disconnect during download
  - [ ] Handle power loss during flash (if possible)
  - [ ] Don't brick device on bad firmware

**Expected Behavior**:
- Reliable update process
- Clear progress indication
- Automatic rollback on failure
- User data preserved

**Common Issues**:
- Partition table issues
- Insufficient flash space
- Checksum verification failures
- Bricking due to incomplete flash

### Performance Testing

#### 8. Power Consumption

**Test**: Battery life and power modes

- [ ] **Idle Power**
  - [ ] Measure idle current draw
  - [ ] Test deep sleep mode
  - [ ] Test light sleep mode

- [ ] **Playback Power**
  - [ ] Measure current during MP3 playback
  - [ ] Measure current during FLAC playback
  - [ ] Measure with WiFi enabled
  - [ ] Measure with WiFi disabled

- [ ] **Battery Life**
  - [ ] Estimate battery life (e.g., 10 hours playback)
  - [ ] Test low battery warning
  - [ ] Test shutdown at critical battery

**Expected Behavior**:
- Reasonable battery life (> 8 hours playback)
- Low idle power consumption
- Effective sleep modes

**Tools**:
- Multimeter with current measurement
- Power profiler (e.g., Nordic PPK2)
- Battery with known capacity

**Common Issues**:
- High idle current (peripherals not sleeping)
- WiFi consuming too much power
- Display refresh draining battery

#### 9. Stress Testing

**Test**: Long-term stability

- [ ] **Playback Endurance**
  - [ ] Play entire album (1 hour+)
  - [ ] Play multiple albums (4+ hours)
  - [ ] Overnight playback test (8+ hours)
  - [ ] No crashes or freezes

- [ ] **Memory Leaks**
  - [ ] Check free heap over time
  - [ ] No gradual memory loss
  - [ ] No heap fragmentation

- [ ] **Temperature**
  - [ ] ESP32 temperature under load
  - [ ] DAC temperature
  - [ ] No thermal throttling

**Expected Behavior**:
- Stable operation for 24+ hours
- Consistent memory usage
- Safe operating temperatures (< 80°C)

**Tools**:
- Serial monitor for heap statistics
- IR thermometer for temperature
- Long-term automated playback script

### Edge Cases and Error Handling

#### 10. Error Scenarios

**Test**: Graceful degradation

- [ ] **File Errors**
  - [ ] Corrupted MP3 → skip to next track
  - [ ] Missing metadata → display "Unknown"
  - [ ] Unsupported format → show error

- [ ] **Network Errors**
  - [ ] WiFi disconnect → show warning
  - [ ] Server unreachable → use local mode
  - [ ] Sync failure → retry gracefully

- [ ] **Hardware Errors**
  - [ ] SD card removed → pause playback
  - [ ] DAC disconnect → error message
  - [ ] Display fault → continue audio

- [ ] **Resource Exhaustion**
  - [ ] Out of memory → free cache
  - [ ] Disk full → show warning
  - [ ] Queue overflow → limit queue size

**Expected Behavior**:
- No crashes on errors
- Clear error messages to user
- Automatic recovery when possible

## Testing Workflow

### Before Each Release

1. **Build and Flash**
   ```bash
   cd applications/firmware
   cargo build --release
   espflash flash target/xtensa-esp32s3-espidf/release/soul-player-esp32
   ```

2. **Complete Core Checklist**
   - Work through sections 1-6 systematically
   - Document any issues found
   - Re-test after fixes

3. **Perform Stress Tests**
   - Run overnight playback test
   - Monitor serial output for errors
   - Check memory usage

4. **Test OTA Update**
   - Flash previous version
   - Update to new version via OTA
   - Verify successful update

5. **Document Results**
   - Record test date
   - Note firmware version tested
   - List any known issues

### Test Report Template

```markdown
## ESP32 Firmware Test Report

**Version**: v0.1.0
**Date**: 2025-01-10
**Tester**: [Your Name]
**Hardware**: ESP32-S3 DevKit + PCM5102 DAC + 16GB SD Card

### Core Functionality
- [x] Audio Playback - All formats working
- [x] SD Card Operations - No issues
- [x] Display Rendering - Occasional ghosting
- [x] WiFi Connectivity - Stable
- [ ] Bluetooth - Not tested (hardware unavailable)
- [x] Physical Controls - All working
- [x] OTA Updates - Successful

### Performance
- Battery life: ~9 hours (MP3 @ 128kbps)
- Idle current: 45mA
- Playback current: 120mA

### Issues Found
1. E-ink ghosting after 50+ partial updates (minor)
2. WiFi reconnect takes 5-10 seconds (acceptable)

### Recommendation
✅ **APPROVED for release** - Minor issues documented
```

## Automated Logging

### Serial Monitor Output

Monitor for these messages during testing:

```
✓ Good Signs:
[INFO] Audio buffer filled
[INFO] WiFi connected
[INFO] SD card mounted
[INFO] Playback started

✗ Warning Signs:
[WARN] Buffer underrun
[WARN] WiFi disconnected
[ERROR] SD card read failed
[ERROR] Out of memory
```

### Logging Script

```bash
# Capture serial output to file
espflash monitor 2>&1 | tee test-session-$(date +%Y%m%d-%H%M%S).log

# Monitor for errors in real-time
espflash monitor 2>&1 | grep -E "(ERROR|WARN|PANIC)"
```

## Known Limitations (Document These)

Items that are known not to work or have limitations:

```
- [ ] High-res audio (> 96kHz) not supported
- [ ] Bluetooth audio streaming (not implemented yet)
- [ ] Album art display (limited memory)
- [ ] Gapless playback (timing issues)
- [ ] Equalizer (CPU limited)
```

## Regression Testing

When fixing bugs, re-test these specific scenarios:

```
Bug #42: SD card corruption on power loss
→ Test: Remove power during write, verify no corruption

Bug #15: WiFi memory leak
→ Test: Connect/disconnect 100 times, check heap

Bug #8: Audio glitches during UI updates
→ Test: Scroll through menus during playback
```

## Future Enhancements

### Hardware-in-the-Loop (HIL) Testing

For automated hardware testing, consider:

```
- Self-hosted GitHub Actions runner with USB ESP32
- Automated audio quality testing (THD+N measurement)
- Automated power consumption logging
- Automated stress testing (24+ hours)
```

**Cost**: Medium (requires dedicated hardware + maintenance)
**Benefit**: Catch hardware regressions in CI

### Remote Test Lab

Alternative approach:

```
- Multiple ESP32 devices with different DAC/display combos
- Remote access for team members
- Scheduled automated test runs
```

**Cost**: High (complex infrastructure)
**Benefit**: Test on multiple hardware variants

## Resources

- [ESP-IDF Programming Guide](https://docs.espressif.com/projects/esp-idf/en/latest/)
- [ESP32-S3 Datasheet](https://www.espressif.com/sites/default/files/documentation/esp32-s3_datasheet_en.pdf)
- [I2S Audio Guide](https://docs.espressif.com/projects/esp-idf/en/latest/esp32s3/api-reference/peripherals/i2s.html)
- [SDMMC Driver](https://docs.espressif.com/projects/esp-idf/en/latest/esp32s3/api-reference/storage/sdmmc.html)

## Contact

For issues or questions about ESP32 testing:
- Open a GitHub issue with `[ESP32]` prefix
- Tag `@firmware-maintainer` in discussions
- Check #esp32-testing channel on Discord
