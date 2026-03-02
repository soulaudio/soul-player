/**
 * Playwright global teardown — kills the Tauri app process and removes temp files.
 */

import { rmSync } from 'fs';

export default async function globalTeardown() {
  // Kill by name — more reliable than by PID on Windows
  try {
    const { execSync } = await import('child_process');
    execSync('powershell -Command "Stop-Process -Name soul-player-desktop -Force -ErrorAction SilentlyContinue"', { stdio: 'ignore' });
    console.log('[Playwright Teardown] Killed soul-player-desktop');
  } catch { /* already gone */ }

  const dir = process.env.PLAYWRIGHT_TEST_DIR;
  if (dir) {
    try {
      rmSync(dir, { recursive: true, force: true });
      console.log('[Playwright Teardown] ✓ Cleaned up temp files');
    } catch (err) {
      console.error('[Playwright Teardown] Cleanup failed:', err);
    }
  }
}
