/**
 * Audio Settings E2E tests — Playwright CDP
 *
 * Tests the audio settings page end-to-end:
 *   1. Audio settings page loads with expected pipeline sections
 *   2. Pipeline overview (stage navigation bar) is visible
 *   3. Volume leveling mode defaults to "Disabled"
 *   4. Volume leveling mode can be changed and the change is reflected in the UI
 *   5. Preamp slider is visible when a leveling mode is active
 *   6. Reset to defaults button opens a confirmation dialog
 *
 * Navigation pattern
 * ──────────────────
 * The desktop app uses React Router with SettingsLayout + SettingsSidebar.
 * nav-settings-audio is the Link element that navigates to /settings/audio.
 * We wait for nav-settings-about to confirm the settings panel is open before
 * clicking nav-settings-audio, mirroring the pattern in updater.spec.js.
 */

import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

// ── CDP connection shared across all tests in this file ────────────────────

let browser;
let page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];

  const pages = context.pages();
  page = pages.find(
    (p) =>
      (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) &&
      !p.url().includes('splash')
  );

  if (!page) throw new Error('Main window not found in CDP context');

  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => {
  await browser.close();
});

// Navigate to settings → Audio page before each test
test.beforeEach(async () => {
  // Dismiss any lingering overlay from a previous test
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);

  // Open settings panel — force:true handles any residual backdrop overlay
  await page.click('[data-testid="settings-button"]', { force: true });

  // Wait for the sidebar to be visible (any nav item confirms it)
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 10_000 });

  // Navigate to the Audio section
  await page.click('[data-testid="nav-settings-audio"]');

  // Wait for the audio settings page container
  await page.waitForSelector('[data-testid="audio-settings-page"]', { timeout: 10_000 });
});

test.afterEach(async () => {
  // Dismiss any dialog that might have been opened during the test
  await page.keyboard.press('Escape').catch(() => {});
  await page.waitForTimeout(200);
});

// ── Tests ──────────────────────────────────────────────────────────────────

test('audio settings page loads with pipeline stages and output section', async () => {
  // The top-level audio settings container
  const audioPage = page.locator('[data-testid="audio-settings-page"]');
  await expect(audioPage).toBeVisible();

  // Three pipeline stages
  await expect(page.locator('[data-testid="audio-stage-resampling"]')).toBeVisible();
  await expect(page.locator('[data-testid="audio-stage-dsp"]')).toBeVisible();
  await expect(page.locator('[data-testid="audio-stage-volume-leveling"]')).toBeVisible();

  await page.screenshot({ path: 'screenshots/audio-settings-page.png' });
});

test('pipeline overview navigation bar is visible', async () => {
  // The clickable stage overview bar at the top of the page
  const overview = page.locator('[data-testid="audio-pipeline-overview"]');
  await expect(overview).toBeVisible();

  // Overview must contain the three pipeline stage labels
  await expect(overview.getByText(/resample/i)).toBeVisible();
  await expect(overview.getByText(/dsp/i)).toBeVisible();
  await expect(overview.getByText(/leveling/i)).toBeVisible();

  await page.screenshot({ path: 'screenshots/audio-pipeline-overview.png' });
});

test('volume leveling defaults to Disabled mode', async () => {
  // Scroll to the volume leveling stage first
  await page.locator('[data-testid="audio-stage-volume-leveling"]').scrollIntoViewIfNeeded();

  // The "Disabled" mode button must have aria-pressed="true" by default
  const disabledBtn = page.locator('[data-testid="volume-leveling-mode-disabled"]');
  await expect(disabledBtn).toBeVisible();
  await expect(disabledBtn).toHaveAttribute('aria-pressed', 'true');

  // The other modes must not be selected
  await expect(page.locator('[data-testid="volume-leveling-mode-replaygain_track"]')).toHaveAttribute('aria-pressed', 'false');
  await expect(page.locator('[data-testid="volume-leveling-mode-replaygain_album"]')).toHaveAttribute('aria-pressed', 'false');
  await expect(page.locator('[data-testid="volume-leveling-mode-ebu_r128"]')).toHaveAttribute('aria-pressed', 'false');
});

test('volume leveling mode can be changed to ReplayGain Track', async () => {
  await page.locator('[data-testid="audio-stage-volume-leveling"]').scrollIntoViewIfNeeded();

  // Click ReplayGain (Track) mode
  const rgTrackBtn = page.locator('[data-testid="volume-leveling-mode-replaygain_track"]');
  await expect(rgTrackBtn).toBeVisible();
  await rgTrackBtn.click();

  // The clicked mode must become selected
  await expect(rgTrackBtn).toHaveAttribute('aria-pressed', 'true');

  // The Disabled mode must no longer be selected
  await expect(page.locator('[data-testid="volume-leveling-mode-disabled"]')).toHaveAttribute('aria-pressed', 'false');

  // After enabling a mode the preamp slider must appear
  const preampSlider = page.locator('[data-testid="volume-leveling-preamp-slider"]');
  await expect(preampSlider).toBeVisible();

  await page.screenshot({ path: 'screenshots/audio-volume-leveling-rg-track.png' });

  // Cleanup — reset back to Disabled so state does not bleed between tests
  await page.locator('[data-testid="volume-leveling-mode-disabled"]').click();
  await expect(page.locator('[data-testid="volume-leveling-mode-disabled"]')).toHaveAttribute('aria-pressed', 'true');
});

test('preamp slider is only shown when a leveling mode is active', async () => {
  await page.locator('[data-testid="audio-stage-volume-leveling"]').scrollIntoViewIfNeeded();

  // With Disabled selected the preamp slider must NOT be visible
  const preampSlider = page.locator('[data-testid="volume-leveling-preamp-slider"]');
  // Ensure we are in the default Disabled state first
  const disabledBtn = page.locator('[data-testid="volume-leveling-mode-disabled"]');
  if ((await disabledBtn.getAttribute('aria-pressed')) !== 'true') {
    await disabledBtn.click();
    await expect(disabledBtn).toHaveAttribute('aria-pressed', 'true');
  }
  await expect(preampSlider).not.toBeVisible();

  // Enable EBU R128 — preamp slider must appear
  await page.locator('[data-testid="volume-leveling-mode-ebu_r128"]').click();
  await expect(preampSlider).toBeVisible();

  // Slider must be within its defined range [-12, 12]
  const min = await preampSlider.getAttribute('min');
  const max = await preampSlider.getAttribute('max');
  expect(Number(min)).toBe(-12);
  expect(Number(max)).toBe(12);

  // Cleanup
  await page.locator('[data-testid="volume-leveling-mode-disabled"]').click();
});

test('reset-to-defaults button opens a confirmation dialog', async () => {
  // The reset button is in the page header area — visible without scrolling
  const resetBtn = page.locator('[data-testid="audio-reset-button"]');
  await expect(resetBtn).toBeVisible();

  await resetBtn.click();

  // A ConfirmDialog must appear after clicking reset.
  // ConfirmDialog renders with data-testid="confirm-dialog" by default.
  const dialog = page.locator('[data-testid="confirm-dialog"]');
  await expect(dialog).toBeVisible({ timeout: 5_000 });

  // The dialog must describe a destructive/reset action in its title or body
  await expect(dialog).toContainText(/reset/i);

  await page.screenshot({ path: 'screenshots/audio-reset-dialog.png' });

  // Dismiss the dialog via Escape so afterEach is clean
  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible({ timeout: 3_000 });
});

test('settings sidebar nav items for audio section are accessible', async () => {
  // Verify all expected nav items in the settings sidebar are present and clickable.
  // This exercises the SettingsSidebar navigation structure.

  const navItems = [
    'nav-settings-appearance',
    'nav-settings-musicData',
    'nav-settings-audio',
    'nav-settings-shortcuts',
    'nav-settings-about',
  ];

  for (const testId of navItems) {
    const navItem = page.locator(`[data-testid="${testId}"]`);
    await expect(navItem).toBeVisible();
  }

  // Clicking audio nav while already on audio page keeps the page visible
  await page.click('[data-testid="nav-settings-audio"]');
  await expect(page.locator('[data-testid="audio-settings-page"]')).toBeVisible({ timeout: 5_000 });
});
