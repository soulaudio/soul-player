import { execSync } from 'child_process';

export default async function globalTeardown() {
  try {
    execSync('powershell -Command "Stop-Process -Name soul-player-desktop -Force -ErrorAction SilentlyContinue"', { stdio: 'ignore' });
  } catch { /* ignore */ }
}
