import { chromium } from 'playwright';

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();

  // Navigate to the site
  await page.goto('http://localhost:3001');
  await page.waitForTimeout(2000);

  const themes = ['dark', 'light', 'ocean'];

  for (const theme of themes) {
    console.log(`\n=== Inspecting ${theme.toUpperCase()} theme ===\n`);

    // Click the theme button (desktop)
    const themeButton = page.locator(`button:has-text("${theme.charAt(0).toUpperCase() + theme.slice(1)}")`);
    if (await themeButton.count() > 0) {
      await themeButton.click();
      await page.waitForTimeout(1000);
    }

    // Take screenshot
    await page.screenshot({ path: `/mnt/d/dev/soulaudio/soul-player/applications/marketing/theme-${theme}.png`, fullPage: true });

    // Get colors
    const heroSection = await page.locator('[data-hero-section]').first();
    const soulPlayerText = await page.locator('h2:has-text("Soul Player")').first();
    const themeLabel = await page.locator('[data-theme-label]').first();
    const mainText = await page.locator('[data-main-text]').first();
    const backdrop = await page.locator('[data-demo-backdrop]').first();

    // Extract computed styles
    const heroStyles = await heroSection.evaluate(el => {
      const computed = window.getComputedStyle(el);
      return {
        backgroundColor: computed.backgroundColor,
      };
    });

    const soulPlayerStyles = await soulPlayerText.evaluate(el => {
      const computed = window.getComputedStyle(el);
      return {
        color: computed.color,
        textShadow: computed.textShadow,
      };
    });

    const themeLabelStyles = await themeLabel.evaluate(el => {
      const computed = window.getComputedStyle(el);
      return {
        color: computed.color,
      };
    });

    const mainTextStyles = await mainText.evaluate(el => {
      const computed = window.getComputedStyle(el);
      return {
        color: computed.color,
      };
    });

    const backdropStyles = await backdrop.evaluate(el => {
      const computed = window.getComputedStyle(el);
      return {
        background: computed.background,
      };
    });

    console.log('Hero Section Background:', heroStyles.backgroundColor);
    console.log('Soul Player Text Color:', soulPlayerStyles.color);
    console.log('Soul Player Text Shadow:', soulPlayerStyles.textShadow);
    console.log('"Pick your theme" Label Color:', themeLabelStyles.color);
    console.log('Main Text Color:', mainTextStyles.color);
    console.log('Backdrop Gradient:', backdropStyles.background.substring(0, 150) + '...');
    console.log(`Screenshot saved: theme-${theme}.png`);
  }

  await browser.close();
})();
