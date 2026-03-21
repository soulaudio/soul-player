/**
 * DSD Format Comparison E2E Test
 *
 * Validates Soul Player DSD audio quality by:
 *   1. Capturing DSD and PCM (FLAC/WAV) playback via WASAPI loopback
 *   2. Generating ffmpeg references for both
 *   3. Running comprehensive spectral comparison
 *   4. Attaching a visual report (PNG) as a test artifact
 *
 * Skips gracefully when:
 *   - No real DSD or PCM files are found (test DB scenario)
 *   - ffmpeg / Python deps not installed
 *   - WASAPI loopback device unavailable
 *
 * Run: npx playwright test tests/playwright/dsd-format-comparison.spec.js \
 *        --config playwright.prod.config.js
 */

import { test, expect, chromium } from '@playwright/test';
import { spawnSync } from 'child_process';
import { mkdirSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname  = dirname(fileURLToPath(import.meta.url));
const CAPTURES   = join(__dirname, '..', '..', 'captures');
const ANALYSIS   = join(CAPTURES, 'analysis');
const CDP_URL    = process.env.SOUL_CDP_URL ?? 'http://localhost:9222';

const WASAPI_DEV  = 14;
const CAPTURE_SR  = 48000;
const CAPTURE_SECS = 12;
const WARMUP_MS   = 2000;

// Quality thresholds
const MAX_CENTROID_DIFF_DSD_REF_HZ = 2500;
const MAX_RMS_DIFF_DSD_REF_DB      = 12;
const MAX_RMS_DIFF_PCM_REF_DB      = 12;
const MIN_SIGNAL_RMS_DB            = -60;

// ── module-level state (shared across tests in the suite) ─────────────────────
let browser, page, skipReason;
let dsdTrack, pcmTrack;
let capDsd, capPcm, refDsd, refPcm, outPng, outJson;
let metrics = null;

// ── helpers ───────────────────────────────────────────────────────────────────
const invoke = (pg, cmd, params = {}) =>
  pg.evaluate(({ cmd, params }) => window.__TAURI_INTERNALS__.invoke(cmd, params), { cmd, params });

function findTracks(allTracks) {
  const DSD_FMTS = new Set(['dsf','dff','dsdiff','DSF','DFF','DSDIFF']);
  const PCM_FMTS = new Set(['flac','wav','aiff','aif','FLAC','WAV','AIFF','AIF']);

  const dsdPool = allTracks.filter(t =>
    DSD_FMTS.has(t.file_format) && t.duration_seconds >= 30
    && t.file_path && existsSync(t.file_path)
  ).sort((a, b) => a.duration_seconds - b.duration_seconds);

  const pcmPool = allTracks.filter(t =>
    PCM_FMTS.has(t.file_format) && t.duration_seconds >= 30
    && t.file_path && existsSync(t.file_path)
  );

  if (!dsdPool.length || !pcmPool.length) return null;

  const dsd = dsdPool[0];
  const pcm = pcmPool.find(t => t.artist_name === dsd.artist_name)
    ?? pcmPool.find(t => t.album_title === dsd.album_title)
    ?? pcmPool[0];

  return { dsd, pcm };
}

function ffmpegRef(inPath, outPath) {
  const r = spawnSync('ffmpeg', [
    '-y', '-i', inPath,
    '-t', String(CAPTURE_SECS + 2),
    '-ar', String(CAPTURE_SR), '-ac', '2', outPath,
  ], { encoding: 'utf8', stdio: 'pipe' });
  if (r.status !== 0) throw new Error(`ffmpeg failed: ${r.stderr?.slice(-400)}`);
}

function captureWasapi(outPath) {
  const OUT = outPath.replace(/\\/g, '\\\\');
  const py = `
import pyaudiowpatch as pyaudio, wave, numpy as np
DEVICE=${WASAPI_DEV}; RATE=${CAPTURE_SR}; CHUNK=1024; DUR=${CAPTURE_SECS}
p=pyaudio.PyAudio()
s=p.open(format=pyaudio.paInt16,channels=2,rate=RATE,input=True,input_device_index=DEVICE,frames_per_buffer=CHUNK)
frames=[]
for _ in range(int(RATE/CHUNK*DUR)): frames.append(s.read(CHUNK,exception_on_overflow=False))
s.stop_stream(); s.close(); p.terminate()
raw=b''.join(frames)
with wave.open(r"${OUT}",'wb') as wf:
    wf.setnchannels(2); wf.setsampwidth(2); wf.setframerate(RATE); wf.writeframes(raw)
arr=np.frombuffer(raw,dtype=np.int16).astype(np.float32)/32768.0
import sys,io; sys.stdout=io.TextIOWrapper(sys.stdout.buffer,encoding='utf-8',errors='replace')
print(f"{20*np.log10(float(np.sqrt(np.mean(arr**2)))+1e-12):.2f}")
`;
  const r = spawnSync('python', ['-c', py], {
    encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, PYTHONIOENCODING: 'utf-8' },
  });
  if (r.status !== 0 || r.stderr?.includes('Traceback'))
    throw new Error(`WASAPI capture failed: ${r.stderr}`);
  return parseFloat(r.stdout.trim());
}

async function playTrack(pg, track) {
  await invoke(pg, 'play_queue', {
    queue: [{
      trackId:         String(track.id),
      title:           track.title,
      artist:          track.artist_name    || 'Unknown Artist',
      album:           track.album_title    || null,
      albumId:         track.album_id       || null,
      filePath:        track.file_path      || '',
      durationSeconds: track.duration_seconds || null,
      trackNumber:     track.track_number   || null,
      coverArtPath:    null,
    }],
    startIndex: 0,
  });
  await pg.waitForTimeout(WARMUP_MS);
}

function runAnalysis() {
  const esc = p => p.replace(/\\/g, '\\\\');
  const dsdLabel = `${dsdTrack.title} (${dsdTrack.file_format})`.replace(/["\n]/g, ' ');
  const pcmLabel = `${pcmTrack.title} (${pcmTrack.file_format})`.replace(/["\n]/g, ' ');

  const py = `
import sys, io, json
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
import numpy as np
import scipy.io.wavfile as wav
import scipy.signal     as sig
import scipy.fft        as fft
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot   as plt
import matplotlib.gridspec as gridspec

TARGET_SR = ${CAPTURE_SR}
CUTOFF    = 20000

def load(path):
    sr, raw = wav.read(path)
    f = (raw / 32768.0 if raw.dtype.kind == 'i' and raw.itemsize == 2 else
         raw / 2147483648.0 if raw.dtype.kind == 'i' else raw.astype(np.float32))
    ch = (f[:, 0] if f.ndim == 2 else f).astype(np.float32)
    if sr != TARGET_SR:
        ch = sig.resample(ch, int(len(ch) * TARGET_SR / sr)).astype(np.float32)
    return ch

cap_dsd = load(r"${esc(capDsd)}")
cap_pcm = load(r"${esc(capPcm)}")
ref_dsd = load(r"${esc(refDsd)}")
ref_pcm = load(r"${esc(refPcm)}")

n = min(len(cap_dsd), len(cap_pcm), len(ref_dsd), len(ref_pcm), TARGET_SR * ${CAPTURE_SECS})
cap_dsd, cap_pcm, ref_dsd, ref_pcm = (x[:n] for x in (cap_dsd, cap_pcm, ref_dsd, ref_pcm))

def metrics(ch):
    rms_db   = float(20 * np.log10(np.sqrt(np.mean(ch**2)) + 1e-12))
    peak_db  = float(20 * np.log10(np.max(np.abs(ch))      + 1e-12))
    fs = max(1, int(TARGET_SR * 0.1))
    frames = [ch[i:i+fs] for i in range(0, len(ch) - fs, fs)]
    nf     = float(np.mean(sorted(
        20 * np.log10(np.sqrt(np.mean(f**2)) + 1e-12) for f in frames
    )[:max(1, len(frames) // 10)])) if frames else rms_db
    nn    = min(TARGET_SR * 4, len(ch))
    freqs = fft.rfftfreq(nn, 1 / TARGET_SR)
    spec  = np.abs(fft.rfft(ch[:nn] * np.hanning(nn)))
    mask  = (freqs >= 20) & (freqs <= CUTOFF)
    centroid = float(np.sum(freqs[mask] * spec[mask]) / (np.sum(spec[mask]) + 1e-12))
    total = float(np.sum(spec[mask]**2) + 1e-12)
    def frac(lo, hi):
        m = mask & (freqs >= lo) & (freqs < hi)
        return float(np.sum(spec[m]**2) / total) if m.any() else 0.0
    return dict(rms_db=rms_db, peak_db=peak_db, crest_db=peak_db-rms_db,
                noise_floor_db=nf, snr_db=rms_db-nf, centroid_hz=centroid,
                sub=frac(20,200), bass=frac(200,2000), mid=frac(2000,8000), hi=frac(8000,20000))

m_cd = metrics(cap_dsd); m_cp = metrics(cap_pcm)
m_rd = metrics(ref_dsd); m_rp = metrics(ref_pcm)

def psd(ch):
    f, p = sig.welch(ch, TARGET_SR, nperseg=8192, window='hann')
    return f, 10 * np.log10(p + 1e-20)
pf_cd, pp_cd = psd(cap_dsd); pf_cp, pp_cp = psd(cap_pcm)
pf_rd, pp_rd = psd(ref_dsd); pf_rp, pp_rp = psd(ref_pcm)

def specdata(ch):
    f, t, S = sig.spectrogram(ch, TARGET_SR, nperseg=2048, noverlap=1792, window='hann')
    return f, t, 10 * np.log10(S + 1e-20)
sf_d, st_d, Sdsd = specdata(cap_dsd)
sf_p, st_p, Spcm = specdata(cap_pcm)

BG=  '#12121e'; PANEL='#1a1a2e'; GRID='#2a2a4a'; TEXT='#d4d4e8'
C = ['#00d4ff','#ff6b35','#7fff7f','#ffcc00']
fig = plt.figure(figsize=(22, 18), facecolor=BG)
fig.suptitle(
    'Soul Player — DSD vs PCM Full Spectral Analysis\\n'
    f'DSD: ${dsdLabel}   |   PCM: ${pcmLabel}',
    color=TEXT, fontsize=12, fontweight='bold', y=0.99)
gs = gridspec.GridSpec(4, 3, figure=fig, hspace=0.5, wspace=0.35,
                       left=0.06, right=0.97, top=0.95, bottom=0.05)

def ax_style(ax, title, xl, yl, xlim=None, ylim=None):
    ax.set_facecolor(PANEL)
    ax.set_title(title, color=TEXT, fontsize=8, pad=4, fontweight='bold')
    ax.set_xlabel(xl, color=TEXT, fontsize=7)
    ax.set_ylabel(yl, color=TEXT, fontsize=7)
    ax.tick_params(colors=TEXT, labelsize=6)
    ax.grid(True, color=GRID, linewidth=0.5, alpha=0.8)
    for sp in ax.spines.values(): sp.set_edgecolor('#333366')
    if xlim: ax.set_xlim(*xlim)
    if ylim: ax.set_ylim(*ylim)

t_ax = np.linspace(0, n/TARGET_SR, n); dur = n/TARGET_SR
labels3 = [f'${dsdLabel}\\nDSD Capture', f'${pcmLabel}\\nPCM Capture', f'${dsdLabel}\\nffmpeg ref']
for col,(ch,color,label) in enumerate([(cap_dsd,C[0],labels3[0]),(cap_pcm,C[1],labels3[1]),(ref_dsd,C[2],labels3[2])]):
    ax = fig.add_subplot(gs[0,col])
    ax.plot(t_ax, ch, color=color, linewidth=0.25, alpha=0.7)
    ew = TARGET_SR//50; ne = len(ch)//ew
    if ne>1:
        env=[np.sqrt(np.mean(ch[i*ew:(i+1)*ew]**2)) for i in range(ne)]
        te=np.linspace(0,dur,ne); ax.fill_between(te,[-e for e in env],env,color=color,alpha=0.25)
    m = [m_cd,m_cp,m_rd][col]
    ax.text(0.01,0.97,f"RMS {m['rms_db']:.1f}  Crest {m['crest_db']:.1f}  NF {m['noise_floor_db']:.1f} dBFS",
            transform=ax.transAxes,color=TEXT,fontsize=5.5,va='top')
    ax_style(ax, label, 'Time (s)', 'Amplitude', xlim=(0,dur), ylim=(-1.1,1.1))

ax_psd = fig.add_subplot(gs[1,:2])
for (pf,pp,label,color,ls) in [(pf_cd,pp_cd,'DSD Cap',C[0],'-'),(pf_cp,pp_cp,'PCM Cap',C[1],'-'),
                                 (pf_rd,pp_rd,'DSD Ref',C[2],'--'),(pf_rp,pp_rp,'PCM Ref',C[3],'--')]:
    mk = pf <= CUTOFF
    ax_psd.plot(pf[mk]/1000, pp[mk], color=color, linewidth=0.8, linestyle=ls, label=label, alpha=0.9)
ax_style(ax_psd, 'Power Spectral Density 0–20 kHz', 'Frequency (kHz)', 'dB/Hz')
ax_psd.legend(fontsize=6.5, facecolor=PANEL, edgecolor='#333366', labelcolor=TEXT, ncol=2)

ax_hf = fig.add_subplot(gs[1,2])
for (pf,pp,label,color,ls) in [(pf_cd,pp_cd,'DSD Cap',C[0],'-'),(pf_cp,pp_cp,'PCM Cap',C[1],'-'),
                                 (pf_rd,pp_rd,'DSD Ref',C[2],'--'),(pf_rp,pp_rp,'PCM Ref',C[3],'--')]:
    mk = (pf>=10000) & (pf<=TARGET_SR//2)
    ax_hf.plot(pf[mk]/1000, pp[mk], color=color, linewidth=0.8, linestyle=ls, label=label, alpha=0.9)
ax_style(ax_hf, 'PSD: High-Freq Detail 10–24 kHz', 'Frequency (kHz)', 'dB/Hz',
         xlim=(10, TARGET_SR/2/1000))
ax_hf.legend(fontsize=6, facecolor=PANEL, edgecolor='#333366', labelcolor=TEXT)

vmin = min(np.percentile(Sdsd,10), np.percentile(Spcm,10))
vmax = max(np.percentile(Sdsd,99), np.percentile(Spcm,99))
for col,(sf,st,S,label) in enumerate([(sf_d,st_d,Sdsd,'DSD Capture'),(sf_p,st_p,Spcm,'PCM Capture')]):
    ax = fig.add_subplot(gs[2,col])
    im = ax.pcolormesh(st,sf/1000,S,shading='gouraud',cmap='inferno',vmin=vmin,vmax=vmax)
    ax_style(ax, f'Spectrogram: {label}', 'Time (s)', 'Frequency (kHz)', ylim=(0,CUTOFF/1000))
    cb = plt.colorbar(im,ax=ax,format='%.0f',pad=0.02); cb.ax.tick_params(colors=TEXT,labelsize=6)
    cb.set_label('dB',color=TEXT,fontsize=6)

ax_df = fig.add_subplot(gs[2,2])
diff = pp_cd - pp_cp; mk = pf_cd<=CUTOFF
ax_df.fill_between(pf_cd[mk]/1000,0,diff[mk],where=diff[mk]>=0,color=C[0],alpha=0.5,label='DSD louder')
ax_df.fill_between(pf_cd[mk]/1000,0,diff[mk],where=diff[mk]<0, color=C[1],alpha=0.5,label='PCM louder')
ax_df.axhline(0,color=TEXT,linewidth=0.5,alpha=0.5)
ax_style(ax_df,'PSD Δ: DSD − PCM Capture','Frequency (kHz)','ΔPower (dB)')
ax_df.legend(fontsize=7,facecolor=PANEL,edgecolor='#333366',labelcolor=TEXT)

ax_t = fig.add_subplot(gs[3,:])
ax_t.set_facecolor(PANEL); ax_t.axis('off')
col_labels = ['Metric','DSD Capture','PCM Capture','DSD ffmpeg ref','PCM ffmpeg ref']
rows = [
    ['RMS (dBFS)',        f"{m_cd['rms_db']:.1f}",         f"{m_cp['rms_db']:.1f}",         f"{m_rd['rms_db']:.1f}",   f"{m_rp['rms_db']:.1f}"],
    ['Peak (dBFS)',       f"{m_cd['peak_db']:.1f}",        f"{m_cp['peak_db']:.1f}",        f"{m_rd['peak_db']:.1f}",  f"{m_rp['peak_db']:.1f}"],
    ['Crest (dB)',        f"{m_cd['crest_db']:.1f}",       f"{m_cp['crest_db']:.1f}",       f"{m_rd['crest_db']:.1f}", f"{m_rp['crest_db']:.1f}"],
    ['Noise floor (dBFS)',f"{m_cd['noise_floor_db']:.1f}", f"{m_cp['noise_floor_db']:.1f}", f"{m_rd['noise_floor_db']:.1f}", f"{m_rp['noise_floor_db']:.1f}"],
    ['SNR (dB)',          f"{m_cd['snr_db']:.1f}",         f"{m_cp['snr_db']:.1f}",         f"{m_rd['snr_db']:.1f}",   f"{m_rp['snr_db']:.1f}"],
    ['Centroid (Hz)',     f"{m_cd['centroid_hz']:.0f}",    f"{m_cp['centroid_hz']:.0f}",    f"{m_rd['centroid_hz']:.0f}", f"{m_rp['centroid_hz']:.0f}"],
    ['Sub 20-200 Hz',     f"{m_cd['sub']:.3f}",  f"{m_cp['sub']:.3f}",  f"{m_rd['sub']:.3f}",  f"{m_rp['sub']:.3f}"],
    ['Bass 200-2kHz',     f"{m_cd['bass']:.3f}", f"{m_cp['bass']:.3f}", f"{m_rd['bass']:.3f}", f"{m_rp['bass']:.3f}"],
    ['Mid 2-8kHz',        f"{m_cd['mid']:.3f}",  f"{m_cp['mid']:.3f}",  f"{m_rd['mid']:.3f}",  f"{m_rp['mid']:.3f}"],
    ['Hi 8-20kHz',        f"{m_cd['hi']:.3f}",   f"{m_cp['hi']:.3f}",   f"{m_rd['hi']:.3f}",   f"{m_rp['hi']:.3f}"],
    ['DSD cap vs ref: centroid Δ', f"{abs(m_cd['centroid_hz']-m_rd['centroid_hz']):.0f} Hz", '—','—','—'],
    ['DSD cap vs ref: RMS Δ',      f"{abs(m_cd['rms_db']-m_rd['rms_db']):.1f} dB",          '—','—','—'],
    ['PCM cap vs ref: RMS Δ',      '—', f"{abs(m_cp['rms_db']-m_rp['rms_db']):.1f} dB",     '—','—'],
]
tbl = ax_t.table(cellText=rows, colLabels=col_labels, loc='center', cellLoc='center')
tbl.auto_set_font_size(False); tbl.set_fontsize(7.5); tbl.scale(1, 1.3)
for (r,c),cell in tbl.get_celld().items():
    bg = '#222244' if r==0 else (PANEL if r%2==0 else '#161628')
    cell.set_facecolor(bg); cell.set_edgecolor('#333366'); cell.set_text_props(color=TEXT)
    if c==0 and r>0: cell.set_text_props(ha='left',color='#a0a0c8')
ax_t.set_title('Full Metrics Comparison',color=TEXT,fontsize=9,fontweight='bold',pad=6)

plt.savefig(r"${esc(outPng)}", dpi=150, bbox_inches='tight', facecolor=BG, edgecolor='none')
print(f"[plot] {r'${esc(outPng)}'}")

results = {
    'tracks': {'dsd': '${dsdLabel}', 'pcm': '${pcmLabel}'},
    'dsd_capture':   m_cd,
    'pcm_capture':   m_cp,
    'dsd_reference': m_rd,
    'pcm_reference': m_rp,
    'comparisons': {
        'centroid_dsd_cap_vs_dsd_ref_hz': abs(m_cd['centroid_hz'] - m_rd['centroid_hz']),
        'centroid_dsd_cap_vs_pcm_cap_hz': abs(m_cd['centroid_hz'] - m_cp['centroid_hz']),
        'rms_dsd_cap_vs_dsd_ref_db':      abs(m_cd['rms_db'] - m_rd['rms_db']),
        'rms_pcm_cap_vs_pcm_ref_db':      abs(m_cp['rms_db'] - m_rp['rms_db']),
    },
}
with open(r"${esc(outJson)}",'w') as f: json.dump(results,f,indent=2)
print(json.dumps(results, indent=2))
`;

  const r = spawnSync('python', ['-c', py], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    env: { ...process.env, PYTHONIOENCODING: 'utf-8' },
    timeout: 180_000,
  });
  if (r.status !== 0 || r.stderr?.includes('Traceback'))
    throw new Error(`Analysis failed: ${r.stderr}`);
  if (r.stderr) process.stderr.write(r.stderr);
  return JSON.parse(r.stdout.slice(r.stdout.indexOf('{')));
}

// ── Suite ─────────────────────────────────────────────────────────────────────
test.describe('DSD Format Comparison', () => {

  test.beforeAll(async () => {
    mkdirSync(ANALYSIS, { recursive: true });

    const ffOk = spawnSync('ffmpeg',  ['-version'], { stdio: 'pipe' }).status === 0;
    const pyOk = spawnSync('python',
      ['-c', 'import pyaudiowpatch,scipy,numpy,matplotlib'], { stdio: 'pipe' }).status === 0;
    if (!ffOk) { skipReason = 'ffmpeg not in PATH';                                       return; }
    if (!pyOk) { skipReason = 'pip install pyaudiowpatch scipy numpy matplotlib needed'; return; }

    browser = await chromium.connectOverCDP(CDP_URL);
    const ctx = browser.contexts()[0];
    page = ctx.pages().find(
      p => (p.url().includes('tauri.localhost') || p.url().includes('localhost:1420'))
           && !p.url().includes('splash')
    );
    if (!page) { skipReason = 'Main app window not found via CDP'; return; }

    const allTracks = await invoke(page, 'get_all_tracks');
    const found = findTracks(allTracks);
    if (!found) {
      skipReason = 'Need both DSD and PCM tracks with real files (production library)';
      return;
    }
    ({ dsd: dsdTrack, pcm: pcmTrack } = found);

    const ts = Date.now();
    capDsd  = join(ANALYSIS, `cap-dsd-${ts}.wav`);
    capPcm  = join(ANALYSIS, `cap-pcm-${ts}.wav`);
    refDsd  = join(ANALYSIS, `ref-dsd-${ts}.wav`);
    refPcm  = join(ANALYSIS, `ref-pcm-${ts}.wav`);
    outPng  = join(ANALYSIS, `analysis-${ts}.png`);
    outJson = join(ANALYSIS, `metrics-${ts}.json`);

    ffmpegRef(dsdTrack.file_path, refDsd);
    ffmpegRef(pcmTrack.file_path, refPcm);
  });

  test.afterAll(async () => {
    if (page)    await invoke(page, 'stop_playback').catch(() => {});
    if (browser) await browser.close();
  });

  // ── 1. Track discovery ──────────────────────────────────────────────────────
  test('1. DSD and PCM tracks found with valid files', () => {
    if (skipReason) test.skip(true, skipReason);
    expect(dsdTrack).toBeTruthy();
    expect(pcmTrack).toBeTruthy();
    expect(existsSync(dsdTrack.file_path)).toBe(true);
    expect(existsSync(pcmTrack.file_path)).toBe(true);
    console.log(`DSD: "${dsdTrack.title}" — ${dsdTrack.artist_name} [${dsdTrack.file_format} @ ${dsdTrack.sample_rate?.toLocaleString()} Hz]`);
    console.log(`PCM: "${pcmTrack.title}" — ${pcmTrack.artist_name} [${pcmTrack.file_format} @ ${pcmTrack.sample_rate?.toLocaleString()} Hz / ${pcmTrack.bit_depth || '?'}-bit]`);
  });

  // ── 2. ffmpeg references ────────────────────────────────────────────────────
  test('2. ffmpeg decodes both tracks to PCM reference', () => {
    if (skipReason) test.skip(true, skipReason);
    expect(existsSync(refDsd)).toBe(true);
    expect(existsSync(refPcm)).toBe(true);
  });

  // ── 3. DSD capture ──────────────────────────────────────────────────────────
  test('3. WASAPI captures non-silent audio during DSD playback', async () => {
    if (skipReason) test.skip(true, skipReason);
    await playTrack(page, dsdTrack);
    const rms = captureWasapi(capDsd);
    console.log(`DSD capture RMS: ${rms.toFixed(1)} dBFS`);
    expect(rms).toBeGreaterThan(MIN_SIGNAL_RMS_DB);
    expect(existsSync(capDsd)).toBe(true);
  });

  // ── 4. PCM capture ──────────────────────────────────────────────────────────
  test('4. WASAPI captures non-silent audio during PCM playback', async () => {
    if (skipReason) test.skip(true, skipReason);
    await page.waitForTimeout(600);
    await playTrack(page, pcmTrack);
    const rms = captureWasapi(capPcm);
    console.log(`PCM capture RMS: ${rms.toFixed(1)} dBFS`);
    expect(rms).toBeGreaterThan(MIN_SIGNAL_RMS_DB);
    expect(existsSync(capPcm)).toBe(true);
  });

  // ── 5. Full spectral analysis + visual report ───────────────────────────────
  test('5. Spectral analysis: DSD vs PCM vs ffmpeg references', async () => {
    if (skipReason) test.skip(true, skipReason);

    metrics = runAnalysis();

    const c = metrics.comparisons;
    console.log(`\n  DSD cap vs DSD ref: centroid Δ = ${c.centroid_dsd_cap_vs_dsd_ref_hz.toFixed(0)} Hz  (max ${MAX_CENTROID_DIFF_DSD_REF_HZ} Hz)`);
    console.log(`  DSD cap vs PCM cap: centroid Δ = ${c.centroid_dsd_cap_vs_pcm_cap_hz.toFixed(0)} Hz`);
    console.log(`  DSD chain fidelity (RMS Δ vs ref): ${c.rms_dsd_cap_vs_dsd_ref_db.toFixed(1)} dB  (max ${MAX_RMS_DIFF_DSD_REF_DB} dB)`);
    console.log(`  PCM chain fidelity (RMS Δ vs ref): ${c.rms_pcm_cap_vs_pcm_ref_db.toFixed(1)} dB  (max ${MAX_RMS_DIFF_PCM_REF_DB} dB)`);

    // DSD fidelity vs ffmpeg reference
    expect(c.centroid_dsd_cap_vs_dsd_ref_hz).toBeLessThan(MAX_CENTROID_DIFF_DSD_REF_HZ);
    expect(c.rms_dsd_cap_vs_dsd_ref_db).toBeLessThan(MAX_RMS_DIFF_DSD_REF_DB);

    // PCM playback chain should be more transparent
    expect(c.rms_pcm_cap_vs_pcm_ref_db).toBeLessThan(MAX_RMS_DIFF_PCM_REF_DB);

    // Both captures must have signal
    expect(metrics.dsd_capture.rms_db).toBeGreaterThan(MIN_SIGNAL_RMS_DB);
    expect(metrics.pcm_capture.rms_db).toBeGreaterThan(MIN_SIGNAL_RMS_DB);

    // Band ratios: only check where ref has ≥5% energy
    const MIN_FRAC = 0.05;
    for (const band of ['sub', 'bass', 'mid', 'hi']) {
      const refFrac = metrics.dsd_reference[band];
      if (refFrac < MIN_FRAC) continue;
      const capFrac = metrics.dsd_capture[band];
      const ratio   = capFrac / (refFrac + 1e-9);
      console.log(`  ${band}: cap=${capFrac.toFixed(3)}  ref=${refFrac.toFixed(3)}  ratio=${ratio.toFixed(2)}x`);
      expect(ratio).toBeGreaterThan(0.05);
      expect(ratio).toBeLessThan(20);
    }

    // Attach visual report
    if (existsSync(outPng)) {
      await test.info().attach('dsd-format-comparison.png', {
        path: outPng, contentType: 'image/png',
      });
    }
    if (existsSync(outJson)) {
      await test.info().attach('metrics.json', {
        path: outJson, contentType: 'application/json',
      });
    }

    console.log(`\n  Analysis plot: ${outPng}`);
  });

  // ── 6. DSD position advances (playback not stalled) ────────────────────────
  test('6. DSD playback position advances correctly', async () => {
    if (skipReason) test.skip(true, skipReason);
    await playTrack(page, dsdTrack);
    const pos1 = await invoke(page, 'get_position');
    await page.waitForTimeout(3000);
    const pos2 = await invoke(page, 'get_position');
    console.log(`Position: ${pos1?.toFixed(2)}s → ${pos2?.toFixed(2)}s  (Δ=${(pos2-pos1).toFixed(2)}s in 3s)`);
    expect(pos2).toBeGreaterThan((pos1 ?? 0) + 1.0);
    expect(pos2).toBeLessThan((pos1 ?? 0) + 6.0);
  });

});
