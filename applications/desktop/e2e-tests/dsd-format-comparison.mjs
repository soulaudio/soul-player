/**
 * DSD Format Comparison — Full Audio Quality Analysis
 *
 * Plays a DSD track and a PCM (FLAC/WAV) track through Soul Player,
 * captures actual output via WASAPI loopback, then runs comprehensive
 * waveform + spectral analysis and generates a visual report.
 *
 * Signals compared:
 *   cap_dsd  — Soul Player DSD playback (WASAPI loopback)
 *   cap_pcm  — Soul Player PCM playback (WASAPI loopback)
 *   ref_dsd  — ffmpeg DSD→PCM decode (ground truth)
 *   ref_pcm  — ffmpeg PCM decode      (ground truth)
 *
 * This answers:
 *   • DSD cap vs DSD ref  → how faithful is Soul Player's DSD decoder?
 *   • PCM cap vs PCM ref  → how transparent is the playback chain?
 *   • DSD cap vs PCM cap  → what does DSD sound like vs FLAC?
 *
 * Requirements:
 *   pip install pyaudiowpatch scipy numpy matplotlib
 *   ffmpeg in PATH
 *   WASAPI loopback device 14 = "Speakers (Realtek(R) Audio) [Loopback]"
 *
 * Usage:
 *   node dsd-format-comparison.mjs
 *   node dsd-format-comparison.mjs --capture-secs 15
 *   node dsd-format-comparison.mjs --dsd-id 326 --pcm-id 500
 *
 * Outputs → captures/analysis/
 *   analysis-<timestamp>.png   — waveforms, PSD, spectrograms, metrics table
 *   metrics-<timestamp>.json   — all numerical results
 */

import { chromium } from '@playwright/test';
import { spawnSync }  from 'child_process';
import { existsSync, mkdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname  = dirname(fileURLToPath(import.meta.url));
const CAPTURES   = join(__dirname, 'captures');
const ANALYSIS   = join(CAPTURES, 'analysis');
mkdirSync(ANALYSIS, { recursive: true });

// ── Config ────────────────────────────────────────────────────────────────────
const CDP_URL      = process.env.SOUL_CDP_URL ?? 'http://localhost:9222';
const WASAPI_DEV   = 14;     // "Speakers (Realtek(R) Audio) [Loopback]"
const CAPTURE_SR   = 48000;

const args    = process.argv.slice(2);
const getArg  = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args[i + 1] : d; };
const SECS    = parseFloat(getArg('--capture-secs', '12'));
const DSD_ID  = getArg('--dsd-id', null);
const PCM_ID  = getArg('--pcm-id', null);
const WARMUP  = 2000; // ms after play_queue before capture starts

// ── Helpers ───────────────────────────────────────────────────────────────────
const invoke = (page, cmd, params = {}) =>
  page.evaluate(({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params), { cmd, params });

async function connectCdp() {
  for (let i = 0; i < 10; i++) {
    try {
      const browser = await chromium.connectOverCDP(CDP_URL);
      const ctx  = browser.contexts()[0];
      const page = ctx.pages().find(
        p => (p.url().includes('tauri.localhost') || p.url().includes('localhost:1420'))
             && !p.url().includes('splash')
      );
      if (page) return { browser, page };
      await browser.close();
    } catch {}
    console.log(`  Waiting for Soul Player (${i + 1}/10)...`);
    await new Promise(r => setTimeout(r, 1500));
  }
  throw new Error('Could not connect to Soul Player via CDP');
}

function findTracks(allTracks) {
  const DSD_FMTS = new Set(['dsf','dff','dsdiff','DSF','DFF','DSDIFF']);
  const PCM_FMTS = new Set(['flac','wav','aiff','aif','FLAC','WAV','AIFF','AIF']);

  const dsdPool = allTracks.filter(t =>
    DSD_FMTS.has(t.file_format) && t.duration_seconds >= 30 && t.file_path && existsSync(t.file_path)
  ).sort((a, b) => a.duration_seconds - b.duration_seconds);

  const pcmPool = allTracks.filter(t =>
    PCM_FMTS.has(t.file_format) && t.duration_seconds >= 30 && t.file_path && existsSync(t.file_path)
  );

  if (dsdPool.length === 0) throw new Error('No DSD tracks with real files found in library');
  if (pcmPool.length === 0) throw new Error('No FLAC/WAV tracks found in library');

  const dsd = DSD_ID
    ? dsdPool.find(t => String(t.id) === DSD_ID) ?? dsdPool[0]
    : dsdPool[0];

  // Prefer PCM from the same artist, then same album title, then any
  const pcm = PCM_ID
    ? pcmPool.find(t => String(t.id) === PCM_ID) ?? pcmPool[0]
    : pcmPool.find(t => t.artist_name === dsd.artist_name)
      ?? pcmPool.find(t => t.album_title === dsd.album_title)
      ?? pcmPool[0];

  return { dsd, pcm };
}

function ffmpegRef(inPath, outPath) {
  const r = spawnSync('ffmpeg', [
    '-y', '-i', inPath,
    '-t', String(SECS + 2),
    '-ar', '48000', '-ac', '2', outPath,
  ], { encoding: 'utf8', stdio: 'pipe' });
  if (r.status !== 0) throw new Error(`ffmpeg failed:\n${r.stderr?.slice(-600)}`);
}

function captureWasapi(outPath) {
  const OUT = outPath.replace(/\\/g, '\\\\');
  const py = `
import pyaudiowpatch as pyaudio, wave, numpy as np, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
DEVICE=${WASAPI_DEV}; RATE=${CAPTURE_SR}; CHUNK=1024; DUR=${SECS}
OUT=r"${OUT}"
p=pyaudio.PyAudio()
s=p.open(format=pyaudio.paInt16,channels=2,rate=RATE,input=True,input_device_index=DEVICE,frames_per_buffer=CHUNK)
frames=[]
for _ in range(int(RATE/CHUNK*DUR)): frames.append(s.read(CHUNK,exception_on_overflow=False))
s.stop_stream(); s.close(); p.terminate()
raw=b''.join(frames)
with wave.open(OUT,'wb') as wf:
    wf.setnchannels(2); wf.setsampwidth(2); wf.setframerate(RATE); wf.writeframes(raw)
arr=np.frombuffer(raw,dtype=np.int16).astype(np.float32)/32768.0
rms=float(20*np.log10(np.sqrt(np.mean(arr**2))+1e-12))
print(f"RMS={rms:.1f} dBFS")
`;
  const r = spawnSync('python', ['-c', py], {
    encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, PYTHONIOENCODING: 'utf-8' },
  });
  if (r.status !== 0 || r.stderr?.includes('Traceback'))
    throw new Error(`WASAPI capture failed:\n${r.stderr}`);
  return r.stdout.trim();
}

async function playTrack(page, track) {
  await invoke(page, 'play_queue', {
    queue: [{
      trackId:         String(track.id),
      title:           track.title,
      artist:          track.artist_name || 'Unknown Artist',
      album:           track.album_title || null,
      albumId:         track.album_id    || null,
      filePath:        track.file_path   || '',
      durationSeconds: track.duration_seconds || null,
      trackNumber:     track.track_number    || null,
      coverArtPath:    null,
    }],
    startIndex: 0,
  });
  await new Promise(r => setTimeout(r, WARMUP));
}

// ── Python analysis: metrics + 4-row visual report ────────────────────────────
function runAnalysis({ capDsd, capPcm, refDsd, refPcm, dsdLabel, pcmLabel, outPng, outJson }) {
  const esc = p => p.replace(/\\/g, '\\\\');
  const py = `
import sys, io, json
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
import numpy as np
import scipy.io.wavfile as wav
import scipy.signal   as sig
import scipy.fft      as fft
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot    as plt
import matplotlib.gridspec  as gridspec

# ── load & normalise to float32 mono @ 48 kHz ─────────────────────────────────
TARGET_SR = ${CAPTURE_SR}
CUTOFF    = 20000

def load(path, label):
    sr, raw = wav.read(path)
    f = (raw / 32768.0  if raw.dtype.kind == 'i' and raw.itemsize == 2 else
         raw / 2147483648.0 if raw.dtype.kind == 'i' else
         raw.astype(np.float32))
    ch = f[:, 0] if f.ndim == 2 else f
    ch = ch.astype(np.float32)
    if sr != TARGET_SR:
        n  = int(len(ch) * TARGET_SR / sr)
        ch = sig.resample(ch, n).astype(np.float32)
    return {'label': label, 'ch': ch}

cap_dsd = load(r"${esc(capDsd)}", "${dsdLabel} — DSD Capture")
cap_pcm = load(r"${esc(capPcm)}", "${pcmLabel} — PCM Capture")
ref_dsd = load(r"${esc(refDsd)}", "${dsdLabel} — ffmpeg DSD ref")
ref_pcm = load(r"${esc(refPcm)}", "${pcmLabel} — ffmpeg PCM ref")

# Trim all to shortest
n = min(len(cap_dsd['ch']), len(cap_pcm['ch']), len(ref_dsd['ch']), len(ref_pcm['ch']),
        TARGET_SR * ${Math.round(SECS)})
for d in (cap_dsd, cap_pcm, ref_dsd, ref_pcm):
    d['ch'] = d['ch'][:n]

# ── metrics ────────────────────────────────────────────────────────────────────
def metrics(ch):
    rms_db   = float(20 * np.log10(np.sqrt(np.mean(ch**2)) + 1e-12))
    peak_db  = float(20 * np.log10(np.max(np.abs(ch))      + 1e-12))
    crest_db = peak_db - rms_db

    # Noise floor: RMS of the quietest 10% of 100 ms frames
    fs = max(1, int(TARGET_SR * 0.1))
    frames = [ch[i:i+fs] for i in range(0, len(ch) - fs, fs)]
    if frames:
        frame_rms = sorted(20 * np.log10(np.sqrt(np.mean(f**2)) + 1e-12) for f in frames)
        nf = float(np.mean(frame_rms[:max(1, len(frame_rms) // 10)]))
    else:
        nf = rms_db

    # Spectral centroid + band fractions (20 Hz – 20 kHz)
    nn     = min(TARGET_SR * 4, len(ch))
    freqs  = fft.rfftfreq(nn, 1 / TARGET_SR)
    spec   = np.abs(fft.rfft(ch[:nn] * np.hanning(nn)))
    mask   = (freqs >= 20) & (freqs <= CUTOFF)
    denom  = float(np.sum(spec[mask]) + 1e-12)
    centroid = float(np.sum(freqs[mask] * spec[mask]) / denom)
    total  = float(np.sum(spec[mask]**2) + 1e-12)
    def frac(lo, hi):
        m = mask & (freqs >= lo) & (freqs < hi)
        return float(np.sum(spec[m]**2) / total) if m.any() else 0.0

    return dict(
        rms_db=rms_db, peak_db=peak_db, crest_db=crest_db,
        noise_floor_db=nf, snr_db=rms_db - nf,
        centroid_hz=centroid,
        sub=frac(20, 200), bass=frac(200, 2000),
        mid=frac(2000, 8000), hi=frac(8000, 20000),
    )

for d in (cap_dsd, cap_pcm, ref_dsd, ref_pcm):
    d['m'] = metrics(d['ch'])

# ── PSD ────────────────────────────────────────────────────────────────────────
def psd(ch):
    f, p = sig.welch(ch, TARGET_SR, nperseg=8192, window='hann')
    return f, 10 * np.log10(p + 1e-20)

for d in (cap_dsd, cap_pcm, ref_dsd, ref_pcm):
    d['pf'], d['pp'] = psd(d['ch'])

# ── spectrogram ────────────────────────────────────────────────────────────────
def spec_data(ch):
    f, t, S = sig.spectrogram(ch, TARGET_SR, nperseg=2048, noverlap=1792, window='hann')
    return f, t, 10 * np.log10(S + 1e-20)

cap_dsd['sf'], cap_dsd['st'], cap_dsd['S'] = spec_data(cap_dsd['ch'])
cap_pcm['sf'], cap_pcm['st'], cap_pcm['S'] = spec_data(cap_pcm['ch'])

# ── plot setup ─────────────────────────────────────────────────────────────────
BG    = '#12121e'
PANEL = '#1a1a2e'
GRID  = '#2a2a4a'
TEXT  = '#d4d4e8'
COLORS = ['#00d4ff', '#ff6b35', '#7fff7f', '#ffcc00']  # dsd-cap, pcm-cap, dsd-ref, pcm-ref

fig = plt.figure(figsize=(22, 18), facecolor=BG)
fig.suptitle(
    'Soul Player — DSD vs PCM Audio Quality Analysis\\n'
    f'DSD: ${dsdLabel}   |   PCM: ${pcmLabel}',
    color=TEXT, fontsize=13, fontweight='bold', y=0.99,
)
gs = gridspec.GridSpec(4, 3, figure=fig, hspace=0.5, wspace=0.35,
                       left=0.06, right=0.97, top=0.95, bottom=0.05)

def ax_style(ax, title, xlabel, ylabel, xlim=None, ylim=None):
    ax.set_facecolor(PANEL)
    ax.set_title(title, color=TEXT, fontsize=8.5, pad=4, fontweight='bold')
    ax.set_xlabel(xlabel, color=TEXT, fontsize=7.5)
    ax.set_ylabel(ylabel, color=TEXT, fontsize=7.5)
    ax.tick_params(colors=TEXT, labelsize=6.5)
    ax.grid(True, color=GRID, linewidth=0.5, alpha=0.8)
    for sp in ax.spines.values(): sp.set_edgecolor('#333366')
    if xlim: ax.set_xlim(*xlim)
    if ylim: ax.set_ylim(*ylim)

t_axis = np.linspace(0, n / TARGET_SR, n)
dur    = n / TARGET_SR

# ── Row 0: waveforms (all 4 signals) ──────────────────────────────────────────
for col, (d, color) in enumerate([
    (cap_dsd, COLORS[0]),
    (cap_pcm, COLORS[1]),
    (ref_dsd, COLORS[2]),
]):
    ax = fig.add_subplot(gs[0, col])
    ax.plot(t_axis, d['ch'], color=color, linewidth=0.25, alpha=0.75)
    # RMS envelope
    env_w = TARGET_SR // 50
    n_env = len(d['ch']) // env_w
    if n_env > 1:
        env = [np.sqrt(np.mean(d['ch'][i*env_w:(i+1)*env_w]**2)) for i in range(n_env)]
        t_env = np.linspace(0, dur, n_env)
        ax.fill_between(t_env, [-e for e in env], env, color=color, alpha=0.25)
    ax_style(ax, d['label'], 'Time (s)', 'Amplitude', xlim=(0, dur), ylim=(-1.1, 1.1))
    ax.text(0.01, 0.97,
            f"RMS {d['m']['rms_db']:.1f} dBFS   Crest {d['m']['crest_db']:.1f} dB   "
            f"NF {d['m']['noise_floor_db']:.1f} dBFS",
            transform=ax.transAxes, color=TEXT, fontsize=6, va='top', alpha=0.85)

# ── Row 1: PSD full (0–20 kHz) + PSD zoomed (10–24 kHz) + PSD all 4 overlay ──
ax_psd = fig.add_subplot(gs[1, :2])
for d, color, ls in [
    (cap_dsd, COLORS[0], '-'),
    (cap_pcm, COLORS[1], '-'),
    (ref_dsd, COLORS[2], '--'),
    (ref_pcm, COLORS[3], '--'),
]:
    mk = d['pf'] <= CUTOFF
    ax_psd.plot(d['pf'][mk] / 1000, d['pp'][mk],
                color=color, linewidth=0.8, linestyle=ls,
                label=d['label'], alpha=0.9)
ax_style(ax_psd, 'Power Spectral Density (0–20 kHz)', 'Frequency (kHz)', 'Power (dB/Hz)')
ax_psd.legend(fontsize=6.5, facecolor=PANEL, edgecolor='#333366', labelcolor=TEXT, ncol=2)

ax_hf = fig.add_subplot(gs[1, 2])
for d, color, ls in [
    (cap_dsd, COLORS[0], '-'),
    (cap_pcm, COLORS[1], '-'),
    (ref_dsd, COLORS[2], '--'),
    (ref_pcm, COLORS[3], '--'),
]:
    mk = (d['pf'] >= 10000) & (d['pf'] <= TARGET_SR // 2)
    ax_hf.plot(d['pf'][mk] / 1000, d['pp'][mk],
               color=color, linewidth=0.8, linestyle=ls, alpha=0.9,
               label=d['label'].split(' — ')[1])
ax_style(ax_hf, 'PSD: High-Freq Detail (10–24 kHz)', 'Frequency (kHz)', 'Power (dB/Hz)',
         xlim=(10, TARGET_SR / 2 / 1000))
ax_hf.legend(fontsize=6, facecolor=PANEL, edgecolor='#333366', labelcolor=TEXT)

# ── Row 2: Spectrograms DSD cap + PCM cap ─────────────────────────────────────
vmin = min(np.percentile(cap_dsd['S'], 10), np.percentile(cap_pcm['S'], 10))
vmax = max(np.percentile(cap_dsd['S'], 99), np.percentile(cap_pcm['S'], 99))

for col, d in enumerate([cap_dsd, cap_pcm]):
    ax = fig.add_subplot(gs[2, col])
    im = ax.pcolormesh(d['st'], d['sf'] / 1000, d['S'],
                       shading='gouraud', cmap='inferno', vmin=vmin, vmax=vmax)
    ax_style(ax, f"Spectrogram: {d['label']}", 'Time (s)', 'Frequency (kHz)',
             ylim=(0, CUTOFF / 1000))
    cb = plt.colorbar(im, ax=ax, format='%.0f', pad=0.02)
    cb.ax.tick_params(colors=TEXT, labelsize=6)
    cb.set_label('dB', color=TEXT, fontsize=6)

# ── Row 2, Col 2: PSD difference DSD-cap minus PCM-cap ───────────────────────
ax_diff = fig.add_subplot(gs[2, 2])
common_f = cap_dsd['pf']
mk = common_f <= CUTOFF
diff = cap_dsd['pp'] - cap_pcm['pp']
pos  = np.maximum(diff, 0)
neg  = np.minimum(diff, 0)
ax_diff.fill_between(common_f[mk] / 1000, 0, diff[mk],
                     where=diff[mk] >= 0, color=COLORS[0], alpha=0.5, label='DSD louder')
ax_diff.fill_between(common_f[mk] / 1000, 0, diff[mk],
                     where=diff[mk] <  0, color=COLORS[1], alpha=0.5, label='PCM louder')
ax_diff.axhline(0, color=TEXT, linewidth=0.5, alpha=0.5)
ax_style(ax_diff, 'PSD Δ: DSD Capture − PCM Capture', 'Frequency (kHz)', 'ΔPower (dB)')
ax_diff.legend(fontsize=7, facecolor=PANEL, edgecolor='#333366', labelcolor=TEXT)

# ── Row 3: Metrics comparison table ───────────────────────────────────────────
ax_tbl = fig.add_subplot(gs[3, :])
ax_tbl.set_facecolor(PANEL)
ax_tbl.axis('off')

signals = [cap_dsd, cap_pcm, ref_dsd, ref_pcm]
short   = ['DSD Cap', 'PCM Cap', 'DSD Ref', 'PCM Ref']
metric_keys = [
    ('RMS (dBFS)',    'rms_db',         '{:.1f}'),
    ('Peak (dBFS)',   'peak_db',        '{:.1f}'),
    ('Crest (dB)',    'crest_db',       '{:.1f}'),
    ('Noise Flr (dBFS)', 'noise_floor_db', '{:.1f}'),
    ('SNR (dB)',      'snr_db',         '{:.1f}'),
    ('Centroid (Hz)', 'centroid_hz',    '{:.0f}'),
    ('Sub 20-200Hz',  'sub',            '{:.3f}'),
    ('Bass 200-2kHz', 'bass',           '{:.3f}'),
    ('Mid 2-8kHz',    'mid',            '{:.3f}'),
    ('Hi 8-20kHz',    'hi',             '{:.3f}'),
]

col_labels = ['Metric'] + short
rows = []
for label, key, fmt in metric_keys:
    row = [label] + [fmt.format(d['m'][key]) for d in signals]
    rows.append(row)

# Extra: comparison rows
centroid_dsd_ref_diff = abs(cap_dsd['m']['centroid_hz'] - ref_dsd['m']['centroid_hz'])
centroid_dsd_pcm_diff = abs(cap_dsd['m']['centroid_hz'] - cap_pcm['m']['centroid_hz'])
rms_dsd_ref_diff      = abs(cap_dsd['m']['rms_db']      - ref_dsd['m']['rms_db'])
rms_pcm_ref_diff      = abs(cap_pcm['m']['rms_db']      - ref_pcm['m']['rms_db'])

rows.append(['DSD cap vs DSD ref: centroid Δ',
             f"{centroid_dsd_ref_diff:.0f} Hz", '—', '—', '—'])
rows.append(['DSD cap vs PCM cap: centroid Δ',
             f"{centroid_dsd_pcm_diff:.0f} Hz", '—', '—', '—'])
rows.append(['DSD chain fidelity (RMS Δ vs ref)',
             f"{rms_dsd_ref_diff:.1f} dB", '—', '—', '—'])
rows.append(['PCM chain fidelity (RMS Δ vs ref)',
             '—', f"{rms_pcm_ref_diff:.1f} dB", '—', '—'])

tbl = ax_tbl.table(cellText=rows, colLabels=col_labels, loc='center', cellLoc='center')
tbl.auto_set_font_size(False)
tbl.set_fontsize(7.5)
tbl.scale(1, 1.35)
for (r, c), cell in tbl.get_celld().items():
    bg = '#222244' if r == 0 else (PANEL if r % 2 == 0 else '#161628')
    cell.set_facecolor(bg)
    cell.set_edgecolor('#333366')
    cell.set_text_props(color=TEXT)
    if c == 0 and r > 0:
        cell.set_text_props(ha='left', color='#a0a0c8')

ax_tbl.set_title('Full Metrics Comparison', color=TEXT, fontsize=9,
                  fontweight='bold', pad=6)

plt.savefig(r"${esc(outPng)}", dpi=150, bbox_inches='tight',
            facecolor=BG, edgecolor='none')
print(f"[analysis] Plot saved: ${esc(outPng)}")

# ── JSON output ────────────────────────────────────────────────────────────────
results = {
    'tracks': {'dsd': '${dsdLabel}', 'pcm': '${pcmLabel}'},
    'dsd_capture':   cap_dsd['m'],
    'pcm_capture':   cap_pcm['m'],
    'dsd_reference': ref_dsd['m'],
    'pcm_reference': ref_pcm['m'],
    'comparisons': {
        'centroid_dsd_cap_vs_dsd_ref_hz': centroid_dsd_ref_diff,
        'centroid_dsd_cap_vs_pcm_cap_hz': centroid_dsd_pcm_diff,
        'rms_dsd_cap_vs_dsd_ref_db':      rms_dsd_ref_diff,
        'rms_pcm_cap_vs_pcm_ref_db':      rms_pcm_ref_diff,
    },
    'verdict': {
        'dsd_fidelity': 'PASS' if rms_dsd_ref_diff < 12 and centroid_dsd_ref_diff < 2500 else 'WARN',
        'pcm_fidelity': 'PASS' if rms_pcm_ref_diff <  6 else 'WARN',
        'dsd_non_silent': 'PASS' if cap_dsd['m']['rms_db'] > -60 else 'FAIL',
        'pcm_non_silent': 'PASS' if cap_pcm['m']['rms_db'] > -60 else 'FAIL',
    },
}
with open(r"${esc(outJson)}", 'w') as f:
    json.dump(results, f, indent=2)
print(json.dumps(results, indent=2))
`;

  const r = spawnSync('python', ['-c', py], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, PYTHONIOENCODING: 'utf-8' },
    timeout: 180_000,
  });
  if (r.status !== 0 || r.stderr?.includes('Traceback'))
    throw new Error(`Analysis failed:\n${r.stderr}`);
  if (r.stderr) process.stderr.write(r.stderr);
  process.stdout.write(r.stdout);
}

// ── Main ──────────────────────────────────────────────────────────────────────
async function main() {
  // Dependency checks
  const ffmpegOk = spawnSync('ffmpeg', ['-version'], { stdio: 'pipe' }).status === 0;
  const pyOk     = spawnSync('python',
    ['-c', 'import pyaudiowpatch,scipy,numpy,matplotlib'], { stdio: 'pipe' }).status === 0;
  if (!ffmpegOk) throw new Error('ffmpeg not found in PATH');
  if (!pyOk)     throw new Error('pip install pyaudiowpatch scipy numpy matplotlib');

  console.log(`\nConnecting to Soul Player at ${CDP_URL}...`);
  const { browser, page } = await connectCdp();
  console.log('Connected.\n');

  console.log('Fetching track list...');
  const allTracks = await invoke(page, 'get_all_tracks');
  console.log(`Library: ${allTracks.length} total tracks`);

  const { dsd, pcm } = findTracks(allTracks);

  console.log(`\n  DSD: "${dsd.title}" — ${dsd.artist_name}`);
  console.log(`       ${dsd.file_format} @ ${dsd.sample_rate?.toLocaleString()} Hz | ${dsd.duration_seconds.toFixed(0)}s`);
  console.log(`       ${dsd.file_path}`);
  console.log(`\n  PCM: "${pcm.title}" — ${pcm.artist_name}`);
  console.log(`       ${pcm.file_format} @ ${pcm.sample_rate?.toLocaleString()} Hz / ${pcm.bit_depth || '?'}-bit | ${pcm.duration_seconds.toFixed(0)}s`);
  console.log(`       ${pcm.file_path}`);

  const ts      = Date.now();
  const capDsd  = join(ANALYSIS, `cap-dsd-${ts}.wav`);
  const capPcm  = join(ANALYSIS, `cap-pcm-${ts}.wav`);
  const refDsd  = join(ANALYSIS, `ref-dsd-${ts}.wav`);
  const refPcm  = join(ANALYSIS, `ref-pcm-${ts}.wav`);
  const outPng  = join(ANALYSIS, `analysis-${ts}.png`);
  const outJson = join(ANALYSIS, `metrics-${ts}.json`);

  // ffmpeg references (fast, offline)
  console.log('\n  [1/5] Generating ffmpeg reference for DSD...');
  ffmpegRef(dsd.file_path, refDsd);
  console.log(`        → ${refDsd}`);

  console.log('  [2/5] Generating ffmpeg reference for PCM...');
  ffmpegRef(pcm.file_path, refPcm);
  console.log(`        → ${refPcm}`);

  // DSD capture
  console.log(`\n  [3/5] Playing DSD track, capturing ${SECS}s via WASAPI loopback...`);
  await playTrack(page, dsd);
  const dsdRms = captureWasapi(capDsd);
  console.log(`        ${dsdRms}  → ${capDsd}`);
  await new Promise(r => setTimeout(r, 600));

  // PCM capture
  console.log(`\n  [4/5] Playing PCM track, capturing ${SECS}s via WASAPI loopback...`);
  await playTrack(page, pcm);
  const pcmRms = captureWasapi(capPcm);
  console.log(`        ${pcmRms}  → ${capPcm}`);

  await invoke(page, 'stop_playback').catch(() => {});
  await browser.close();

  // Analysis
  const dsdLabel = `${dsd.title} (${dsd.file_format})`.replace(/["\n]/g, ' ');
  const pcmLabel = `${pcm.title} (${pcm.file_format})`.replace(/["\n]/g, ' ');

  console.log('\n  [5/5] Running spectral analysis + generating plots...\n');
  runAnalysis({ capDsd, capPcm, refDsd, refPcm, dsdLabel, pcmLabel, outPng, outJson });

  console.log(`\n  Done.`);
  console.log(`  Plot:    ${outPng}`);
  console.log(`  Metrics: ${outJson}\n`);
}

main().catch(e => { console.error(e); process.exit(1); });
