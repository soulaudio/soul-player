/**
 * Playwright production setup — launches the app against the REAL production
 * database so we can take screenshots of the actual music library.
 *
 * Does NOT seed any test data. Does NOT set DATABASE_PATH (app uses default).
 */

import { spawn, execSync } from 'child_process';
import { existsSync, rmSync } from 'fs';
import { join, dirname } from 'path';
import { homedir } from 'os';
import { fileURLToPath } from 'url';
import { chromium } from '@playwright/test';

const __dirname = dirname(fileURLToPath(import.meta.url));
export const CDP_PORT = 9223; // different port to avoid clash with test suite

function getAppPath() {
  const workspaceRoot = join(__dirname, '..', '..', '..');
  const candidates = [
    join(workspaceRoot, 'target', 'release', 'soul-player-desktop.exe'),
    join(workspaceRoot, 'target', 'release', 'soul-player-desktop'),
    join(workspaceRoot, 'target', 'debug', 'soul-player-desktop.exe'),
    join(workspaceRoot, 'target', 'debug', 'soul-player-desktop'),
  ];
  for (const p of candidates) {
    if (existsSync(p)) return p;
  }
  return candidates[0];
}

export default async function globalSetup() {
  const appPath = getAppPath();
  if (!existsSync(appPath)) {
    throw new Error(`[Prod Setup] App binary not found: ${appPath}`);
  }

  // Kill any existing instance
  try {
    execSync('powershell -Command "Stop-Process -Name soul-player-desktop -Force -ErrorAction SilentlyContinue"', { stdio: 'ignore' });
    await new Promise(r => setTimeout(r, 1500));
  } catch { /* nothing running */ }

  // Clear WebView2 cache
  const webView2CacheDir = join(
    process.env.LOCALAPPDATA || join(homedir(), 'AppData', 'Local'),
    'com.soulaudio.player', 'EBWebView', 'Default', 'Cache'
  );
  try {
    if (existsSync(webView2CacheDir)) {
      rmSync(webView2CacheDir, { recursive: true, force: true });
    }
  } catch { /* ignore */ }

  // Ensure dev server is running
  const devServerReady = await fetch('http://localhost:1420').then(r => r.ok).catch(() => false);
  if (!devServerReady) {
    const desktopDir = join(__dirname, '..', '..', '..');
    const devServer = spawn('yarn', ['workspace', 'soul-player-desktop', 'dev'], {
      cwd: desktopDir, stdio: 'ignore', shell: true, detached: false,
    });
    process.env.PLAYWRIGHT_PROD_DEV_SERVER_PID = String(devServer.pid);
    const deadline = Date.now() + 30_000;
    let ready = false;
    while (Date.now() < deadline) {
      await new Promise(r => setTimeout(r, 1000));
      ready = await fetch('http://localhost:1420').then(r => r.ok).catch(() => false);
      if (ready) break;
    }
    if (!ready) throw new Error('[Prod Setup] Dev server did not start within 30s');
  }

  console.log(`[Prod Setup] Launching: ${appPath} (production DB, port ${CDP_PORT})`);

  // Build env without DATABASE_PATH so the app uses the real production DB
  const appEnv = { ...process.env };
  delete appEnv.DATABASE_PATH;
  appEnv.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = `--remote-debugging-port=${CDP_PORT} --disable-cache`;
  appEnv.PLAYWRIGHT_TEST_DIR = 'prod-screenshot'; // skip pre-warm

  // Launch WITHOUT DATABASE_PATH — uses real production DB
  const app = spawn(appPath, [], {
    env: appEnv,
    stdio: 'ignore',
    detached: false,
  });

  process.env.PLAYWRIGHT_PROD_APP_PID = String(app.pid);
  process.env.SOUL_CDP_URL = `http://localhost:${CDP_PORT}`;

  const cdpUrl = `http://localhost:${CDP_PORT}`;
  const deadline = Date.now() + 30_000;
  let ready = false;
  while (Date.now() < deadline) {
    await new Promise(r => setTimeout(r, 500));
    try {
      const res = await fetch(`${cdpUrl}/json/version`);
      if (res.ok) { ready = true; break; }
    } catch { /* not ready */ }
  }
  if (!ready) throw new Error('[Prod Setup] CDP endpoint did not become ready within 30s');

  // Wait for main window
  const browser = await chromium.connectOverCDP(cdpUrl);
  try {
    const readyDeadline = Date.now() + 60_000;
    let mainPage = null;
    while (Date.now() < readyDeadline) {
      const pages = browser.contexts().flatMap(c => c.pages());
      for (const p of pages) {
        try {
          const el = await p.$('[data-testid="nav-albums"]');
          if (el) { mainPage = p; break; }
        } catch { /* not this page */ }
      }
      if (mainPage) break;
      await new Promise(r => setTimeout(r, 500));
    }
    if (!mainPage) throw new Error('[Prod Setup] Main window never became ready');
    console.log('[Prod Setup] ✓ App ready with production DB');
  } finally {
    await browser.close();
  }
}
