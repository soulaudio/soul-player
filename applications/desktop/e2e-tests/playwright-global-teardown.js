/**
 * Playwright global teardown — kills the Tauri app process and removes temp files.
 */

import { rmSync } from 'fs';

export default async function globalTeardown() {
  const pid = process.env.PLAYWRIGHT_APP_PID;
  if (pid) {
    try {
      process.kill(parseInt(pid));
      console.log(`[Playwright Teardown] Killed app PID ${pid}`);
    } catch { /* already gone */ }
  }

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
