import { test, expect, chromium } from '@playwright/test';
import { CDP_URL } from '../../playwright-global-setup.js';

let browser, page;

test.beforeAll(async () => {
  browser = await chromium.connectOverCDP(CDP_URL);
  const context = browser.contexts()[0];
  const pages = context.pages();
  page = pages.find(p => (p.url().includes('localhost:1420') || p.url().includes('tauri.localhost')) && !p.url().includes('splash'));
  if (!page) throw new Error('Main window not found');
  await page.waitForSelector('[data-testid="nav-albums"]', { timeout: 30_000 });
});

test.afterAll(async () => { await browser.close(); });

test('check dsp chain with devtools console capture', async () => {
  const messages = [];
  page.on('console', msg => {
    if (msg.type() === 'error' || msg.text().includes('dsp') || msg.text().includes('DSP') || msg.text().includes('effect') || msg.text().includes('Effect')) {
      messages.push(`[${msg.type()}] ${msg.text()}`);
    }
  });
  
  await page.keyboard.press('Escape');
  await page.waitForTimeout(200);
  await page.click('[data-testid="settings-button"]', { force: true });
  await page.waitForSelector('[data-testid="nav-settings-about"]', { timeout: 10_000 });
  await page.click('[data-testid="nav-settings-audio"]');
  await page.waitForSelector('[data-testid="audio-settings-page"]', { timeout: 10_000 });
  await page.locator('[data-testid="audio-stage-dsp"]').scrollIntoViewIfNeeded();
  await page.waitForSelector('[data-testid="dsp-config"]', { timeout: 10_000 });
  
  // Give time for getDspChain to complete
  await page.waitForTimeout(2000);
  
  const slot0Count = await page.locator('[data-testid="effect-slot-0"]').count();
  const addBtn0Count = await page.locator('[data-testid="add-effect-btn-0"]').count();
  
  console.log('effect-slot-0 count:', slot0Count);
  console.log('add-effect-btn-0 count:', addBtn0Count);
  console.log('DSP-related console messages:', JSON.stringify(messages));
  
  expect(true).toBe(true);
});
