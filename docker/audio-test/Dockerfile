# Docker image for audio testing with PulseAudio virtual devices
# Provides isolated environment for testing audio device switching,
# playback to virtual sinks, recording from virtual sources, and glitch detection.
#
# Usage:
#   docker build -t soul-audio-test:latest .
#   docker run --rm -d --name audio-test soul-audio-test:latest

FROM rust:1.75-bookworm

# Install PulseAudio, ALSA, and audio testing tools
RUN apt-get update && apt-get install -y \
    # PulseAudio daemon and utilities
    pulseaudio \
    pulseaudio-utils \
    # ALSA utilities and development headers
    alsa-utils \
    libpulse-dev \
    libasound2-dev \
    # Audio analysis and manipulation
    sox \
    libsox-fmt-all \
    # FFmpeg for audio processing
    ffmpeg \
    # Process utilities
    procps \
    # pkg-config for Rust build scripts
    pkg-config \
    # For audio analysis scripts
    bc \
    && rm -rf /var/lib/apt/lists/*

# Create directories for PulseAudio
RUN mkdir -p /run/pulse \
    && mkdir -p /etc/pulse \
    && mkdir -p /var/lib/pulse \
    && mkdir -p /run/user/1000/pulse

# Configure PulseAudio for headless operation with virtual devices
# Create a custom default.pa with multiple virtual sinks and sources
RUN cat > /etc/pulse/default.pa << 'EOF'
#!/usr/bin/pulseaudio -nF

# Load required modules
.ifexists module-udev-detect.so
load-module module-udev-detect
.else
load-module module-detect
.endif

# Virtual audio outputs (null sinks that discard audio)
load-module module-null-sink sink_name=virtual_output_1 sink_properties=device.description="Virtual_Output_1" format=s16le rate=44100 channels=2
load-module module-null-sink sink_name=virtual_output_2 sink_properties=device.description="Virtual_Output_2" format=s16le rate=48000 channels=2
load-module module-null-sink sink_name=virtual_output_3 sink_properties=device.description="Virtual_Output_3" format=s24le rate=96000 channels=2
load-module module-null-sink sink_name=virtual_output_hires sink_properties=device.description="Virtual_HiRes_Output" format=s32le rate=192000 channels=2

# Virtual audio inputs (sources from the monitor of each sink)
# These allow "recording" what's being played to the sinks
load-module module-virtual-source source_name=virtual_input_1 master=virtual_output_1.monitor source_properties=device.description="Virtual_Input_1"
load-module module-virtual-source source_name=virtual_input_2 master=virtual_output_2.monitor source_properties=device.description="Virtual_Input_2"
load-module module-virtual-source source_name=virtual_input_3 master=virtual_output_3.monitor source_properties=device.description="Virtual_Input_3"

# Pipe sink for capturing audio to file (useful for verification)
load-module module-pipe-sink sink_name=pipe_output file=/tmp/audio_pipe format=s16le rate=44100 channels=2

# Native protocol for local connections
load-module module-native-protocol-unix auth-anonymous=1

# Set default sink
set-default-sink virtual_output_1

# Set default source
set-default-source virtual_input_1
EOF

# Configure client behavior
RUN cat > /etc/pulse/client.conf << 'EOF'
# PulseAudio client configuration for container use
default-sink = virtual_output_1
default-source = virtual_input_1
autospawn = no
EOF

# Configure daemon settings
RUN cat > /etc/pulse/daemon.conf << 'EOF'
# PulseAudio daemon configuration for container testing

# Run without any real-time priorities (container may not have permissions)
high-priority = no
realtime-scheduling = no

# Logging for debugging
log-target = stderr
log-level = info

# Buffer sizes for testing
default-fragments = 4
default-fragment-size-msec = 25

# Exit behavior
exit-idle-time = -1
daemon-startup = no
EOF

# Create ALSA configuration to use PulseAudio
RUN cat > /etc/asound.conf << 'EOF'
# ALSA configuration to use PulseAudio
pcm.!default {
    type pulse
    fallback "sysdefault"
    hint {
        show on
        description "Default ALSA Output (PulseAudio)"
    }
}

ctl.!default {
    type pulse
    fallback "sysdefault"
}

# Direct access to virtual devices through ALSA
pcm.virtual1 {
    type pulse
    device "virtual_output_1"
}

pcm.virtual2 {
    type pulse
    device "virtual_output_2"
}

pcm.virtual3 {
    type pulse
    device "virtual_output_3"
}
EOF

# Set up non-root user for PulseAudio (best practice)
RUN useradd -m -u 1000 testuser \
    && mkdir -p /home/testuser/.config/pulse \
    && chown -R testuser:testuser /home/testuser \
    && chown -R testuser:testuser /run/user/1000

# Create startup script that initializes audio subsystem
RUN cat > /usr/local/bin/start-audio.sh << 'EOF'
#!/bin/bash
set -e

echo "=== Starting Audio Test Container ==="

# Ensure runtime directory exists with correct permissions
mkdir -p /run/user/1000/pulse
chown -R testuser:testuser /run/user/1000

# Create the audio pipe if it doesn't exist
rm -f /tmp/audio_pipe
mkfifo /tmp/audio_pipe 2>/dev/null || true

# Start PulseAudio as testuser
echo "Starting PulseAudio..."
su - testuser -c "pulseaudio --start --log-target=stderr --log-level=info --exit-idle-time=-1 --disallow-exit" || {
    echo "Failed to start PulseAudio, trying fallback..."
    pulseaudio --start --log-target=stderr --exit-idle-time=-1 --disallow-exit 2>&1 || true
}

# Wait for PulseAudio to be ready
echo "Waiting for PulseAudio to initialize..."
for i in $(seq 1 30); do
    if su - testuser -c "pactl info" >/dev/null 2>&1; then
        echo "PulseAudio ready!"
        break
    fi
    sleep 0.5
done

# List available devices
echo ""
echo "=== Available Audio Devices ==="
su - testuser -c "pactl list sinks short" 2>/dev/null || pactl list sinks short
echo ""
su - testuser -c "pactl list sources short" 2>/dev/null || pactl list sources short
echo ""
echo "=== Audio Container Ready ==="

# Keep container running
exec tail -f /dev/null
EOF
RUN chmod +x /usr/local/bin/start-audio.sh

# Create helper scripts for audio testing
RUN cat > /usr/local/bin/audio-test-helpers.sh << 'EOF'
#!/bin/bash
# Audio testing helper functions

# Check for audio underruns by monitoring PulseAudio statistics
check_underruns() {
    local sink="${1:-virtual_output_1}"
    pactl list sinks | grep -A 20 "$sink" | grep -i underrun || echo "No underruns detected"
}

# Play a test tone to a specific sink
play_test_tone() {
    local sink="${1:-virtual_output_1}"
    local frequency="${2:-440}"
    local duration="${3:-1}"

    # Generate and play sine wave
    sox -n -t pulseaudio "$sink" synth "$duration" sine "$frequency"
}

# Record audio from a source
record_audio() {
    local source="${1:-virtual_input_1}"
    local output="${2:-/tmp/recorded.wav}"
    local duration="${3:-2}"

    parecord -d "$source" --file-format=wav "$output" &
    local pid=$!
    sleep "$duration"
    kill $pid 2>/dev/null || true
    echo "Recorded to $output"
}

# Detect audio glitches by analyzing waveform
detect_glitches() {
    local wavfile="${1:-/tmp/recorded.wav}"

    if [ ! -f "$wavfile" ]; then
        echo "Error: File not found: $wavfile"
        return 1
    fi

    # Use sox stats to check for clipping and silence
    sox "$wavfile" -n stats 2>&1 | grep -E "(Pk lev|Flat factor|Pk count|Bit-depth)"
}

# Switch default sink
switch_sink() {
    local sink="${1:-virtual_output_1}"
    pactl set-default-sink "$sink"
    echo "Default sink set to: $sink"
}

# Get device info
get_device_info() {
    echo "=== Sinks (Output Devices) ==="
    pactl list sinks
    echo ""
    echo "=== Sources (Input Devices) ==="
    pactl list sources
}

# Verify audio pipeline is working
verify_pipeline() {
    local sink="${1:-virtual_output_1}"

    echo "Testing audio pipeline to $sink..."

    # Play a short tone and check for errors
    if sox -n -t pulseaudio "$sink" synth 0.5 sine 1000 2>&1; then
        echo "Pipeline OK: Audio played successfully"
        return 0
    else
        echo "Pipeline ERROR: Failed to play audio"
        return 1
    fi
}
EOF
RUN chmod +x /usr/local/bin/audio-test-helpers.sh

# Environment variables for PulseAudio
ENV PULSE_SERVER=unix:/run/user/1000/pulse/native
ENV XDG_RUNTIME_DIR=/run/user/1000
ENV HOME=/home/testuser

# Switch to testuser for running tests
USER testuser
WORKDIR /app

# Default command
CMD ["/usr/local/bin/start-audio.sh"]
