"""
analyze_dsd_audio.py — WASAPI loopback audio quality analyser

Captures audio from the first available WASAPI loopback device and checks
for silence gaps (--mode silence) or RMS level drops (--mode rms).

Exit codes:
  0  pass — no violations detected
  1  fail — first violation description printed to stdout
  2  error — missing dependency, no loopback device, or capture failure
"""

import argparse
import struct
import sys
import wave


def parse_args():
    p = argparse.ArgumentParser(
        description='Capture WASAPI loopback audio and check for quality issues.',
    )
    p.add_argument('--duration', type=float, required=True,
                   help='Capture duration in seconds.')
    p.add_argument('--output', type=str, default=None,
                   help='Optional path to write captured audio as a WAV file.')
    p.add_argument('--mode', choices=['silence', 'rms'], default='silence',
                   help='Analysis mode: silence gap detection or RMS drop detection.')
    p.add_argument('--silence-threshold-ms', type=float, default=50.0,
                   help='Silence gap length (ms) that triggers a failure (default 50).')
    p.add_argument('--rms-window-ms', type=float, default=100.0,
                   help='Window size (ms) for RMS drop detection (default 100).')
    p.add_argument('--rms-max-drop-db', type=float, default=12.0,
                   help='Maximum RMS drop in dB below baseline before failure (default 12).')
    return p.parse_args()


def import_pyaudiowpatch():
    try:
        import pyaudiowpatch as pyaudio
        return pyaudio
    except ImportError:
        print('ERROR: pyaudiowpatch is not installed. '
              'Install with: pip install pyaudiowpatch', flush=True)
        sys.exit(2)


def get_loopback_device(pyaudio):
    devices = pyaudio.get_loopback_device_info_list()
    if not devices:
        print('ERROR: No WASAPI loopback devices found. '
              'Ensure a default audio output device is active.', flush=True)
        sys.exit(2)
    return devices[0]


def capture_audio(pyaudio, device_info, duration_secs):
    """Capture audio from the loopback device and return raw PCM bytes."""
    sample_rate = int(device_info['defaultSampleRate'])
    channels = int(device_info['maxInputChannels'])
    # Clamp channels to stereo — loopback usually reports 2
    if channels == 0:
        channels = 2

    chunk_size = 1024
    frames = []

    pa = pyaudio.PyAudio()
    try:
        stream = pa.open(
            format=pyaudio.paInt16,
            channels=channels,
            rate=sample_rate,
            input=True,
            input_device_index=int(device_info['index']),
            frames_per_buffer=chunk_size,
        )
        total_frames = int(sample_rate * duration_secs)
        collected = 0
        while collected < total_frames:
            to_read = min(chunk_size, total_frames - collected)
            try:
                data = stream.read(to_read, exception_on_overflow=False)
            except Exception as exc:
                stream.stop_stream()
                stream.close()
                pa.terminate()
                print(f'ERROR: Capture read failed: {exc}', flush=True)
                sys.exit(2)
            frames.append(data)
            collected += to_read
        stream.stop_stream()
        stream.close()
    except Exception as exc:
        pa.terminate()
        print(f'ERROR: Could not open loopback stream: {exc}', flush=True)
        sys.exit(2)
    pa.terminate()

    return b''.join(frames), sample_rate, channels


def write_wav(path, pcm_bytes, sample_rate, channels):
    with wave.open(path, 'wb') as wf:
        wf.setnchannels(channels)
        wf.setsampwidth(2)  # 16-bit
        wf.setframerate(sample_rate)
        wf.writeframes(pcm_bytes)


def pcm_to_float_mono(pcm_bytes, channels):
    """Convert raw 16-bit PCM to a flat list of mono float samples in [-1, 1]."""
    n_samples = len(pcm_bytes) // 2  # 2 bytes per int16 sample
    samples = struct.unpack(f'<{n_samples}h', pcm_bytes)
    # Mix down to mono by averaging channels
    mono = []
    for i in range(0, n_samples, channels):
        chunk = samples[i:i + channels]
        mono.append(sum(chunk) / (len(chunk) * 32768.0))
    return mono


def compute_rms(samples):
    """Compute RMS of a list of float samples."""
    if not samples:
        return 0.0
    mean_sq = sum(s * s for s in samples) / len(samples)
    return mean_sq ** 0.5


def rms_to_dbfs(rms):
    """Convert linear RMS to dBFS. Returns -120.0 for silence."""
    if rms < 1e-10:
        return -120.0
    import math
    return 20.0 * math.log10(rms)


def analyse_silence(mono_samples, sample_rate, threshold_ms):
    """
    Detect contiguous silence gaps >= threshold_ms.

    A window is silent when its RMS is below -60 dBFS.
    Window size: 20ms.

    Returns (passed, description) where description is None on pass.
    """
    window_size = max(1, int(sample_rate * 0.020))  # 20ms
    silence_threshold_rms = 10 ** (-60.0 / 20.0)   # -60 dBFS

    threshold_windows = threshold_ms / 20.0         # windows needed to breach threshold

    gap_windows = 0
    gap_start_ms = None

    i = 0
    window_idx = 0
    while i < len(mono_samples):
        window = mono_samples[i:i + window_size]
        rms = compute_rms(window)
        time_ms = window_idx * 20.0

        if rms < silence_threshold_rms:
            if gap_windows == 0:
                gap_start_ms = time_ms
            gap_windows += 1
            gap_duration_ms = gap_windows * 20.0
            if gap_duration_ms >= threshold_ms:
                return (
                    False,
                    f'Silence gap of {gap_duration_ms:.0f}ms detected at {gap_start_ms:.0f}ms '
                    f'(threshold: {threshold_ms:.0f}ms)',
                )
        else:
            gap_windows = 0
            gap_start_ms = None

        i += window_size
        window_idx += 1

    return True, None


def analyse_rms_drops(mono_samples, sample_rate, window_ms, max_drop_db):
    """
    Detect windows where RMS drops > max_drop_db below the baseline.

    Baseline: median RMS of the first 1 second of audio.
    Window size: window_ms.

    Returns (passed, description) where description is None on pass.
    """
    import math

    baseline_samples = mono_samples[:sample_rate]  # first 1s
    baseline_window_size = max(1, int(sample_rate * 0.100))  # 100ms windows for baseline

    baseline_rms_values = []
    for i in range(0, len(baseline_samples), baseline_window_size):
        w = baseline_samples[i:i + baseline_window_size]
        if w:
            baseline_rms_values.append(compute_rms(w))

    if not baseline_rms_values:
        return False, 'ERROR: Not enough audio to compute baseline RMS'

    baseline_rms_values.sort()
    mid = len(baseline_rms_values) // 2
    if len(baseline_rms_values) % 2 == 0:
        baseline_rms = (baseline_rms_values[mid - 1] + baseline_rms_values[mid]) / 2.0
    else:
        baseline_rms = baseline_rms_values[mid]

    if baseline_rms < 1e-10:
        # Baseline is silence — cannot perform meaningful RMS drop analysis
        return False, 'Baseline audio is silence — cannot perform RMS drop analysis'

    baseline_db = rms_to_dbfs(baseline_rms)

    analysis_window_size = max(1, int(sample_rate * window_ms / 1000.0))

    for idx, i in enumerate(range(0, len(mono_samples), analysis_window_size)):
        window = mono_samples[i:i + analysis_window_size]
        if not window:
            continue
        w_rms = compute_rms(window)
        w_db = rms_to_dbfs(w_rms)
        drop = baseline_db - w_db
        time_ms = idx * window_ms
        if drop > max_drop_db:
            return (
                False,
                f'RMS drop of {drop:.1f}dB detected at {time_ms:.0f}ms '
                f'(baseline: {baseline_db:.1f}dBFS, window: {w_db:.1f}dBFS, '
                f'threshold: {max_drop_db:.1f}dB)',
            )

    return True, None


def main():
    args = parse_args()

    pyaudio = import_pyaudiowpatch()
    device_info = get_loopback_device(pyaudio)

    print(
        f'Capturing {args.duration}s from loopback device: '
        f'{device_info.get("name", "unknown")} '
        f'(index {device_info.get("index", "?")})',
        flush=True,
    )

    pcm_bytes, sample_rate, channels = capture_audio(pyaudio, device_info, args.duration)

    if args.output:
        try:
            write_wav(args.output, pcm_bytes, sample_rate, channels)
            print(f'Audio saved to: {args.output}', flush=True)
        except Exception as exc:
            print(f'WARNING: Could not write WAV file: {exc}', flush=True)

    mono_samples = pcm_to_float_mono(pcm_bytes, channels)

    if args.mode == 'silence':
        passed, description = analyse_silence(
            mono_samples,
            sample_rate,
            args.silence_threshold_ms,
        )
    else:  # rms
        passed, description = analyse_rms_drops(
            mono_samples,
            sample_rate,
            args.rms_window_ms,
            args.rms_max_drop_db,
        )

    if passed:
        print(f'PASS: No violations detected in {args.duration}s capture.', flush=True)
        sys.exit(0)
    else:
        print(f'FAIL: {description}', flush=True)
        sys.exit(1)


if __name__ == '__main__':
    main()
