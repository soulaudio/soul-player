import { chromium } from 'playwright';

(async () => {
  const browser = await chromium.launch({ headless: false });
  const page = await browser.newPage();

  await page.goto('http://localhost:3001/');

  // Wait for demo to load
  await page.waitForTimeout(3000);

  // Find volume control
  console.log('\n=== VOLUME CONTROL INSPECTION ===');
  const volumeControl = await page.locator('div:has(> input[aria-label="Volume"])').first();
  if (await volumeControl.count() > 0) {
    const box = await volumeControl.boundingBox();
    console.log('Volume control bounding box:', box);

    const styles = await volumeControl.evaluate(el => {
      const computed = window.getComputedStyle(el);
      return {
        width: computed.width,
        height: computed.height,
        display: computed.display,
        position: computed.position,
        minWidth: computed.minWidth,
        backgroundColor: computed.backgroundColor
      };
    });
    console.log('Volume control computed styles:', styles);

    // Check the track element
    const track = await page.locator('div[style*="backgroundColor"]').first();
    if (await track.count() > 0) {
      const trackBox = await track.boundingBox();
      console.log('Track bounding box:', trackBox);

      const trackStyles = await track.evaluate(el => {
        const computed = window.getComputedStyle(el);
        return {
          width: computed.width,
          height: computed.height,
          backgroundColor: computed.backgroundColor,
          display: computed.display
        };
      });
      console.log('Track computed styles:', trackStyles);
    }
  } else {
    console.log('Volume control not found!');
  }

  // Check version
  console.log('\n=== VERSION INSPECTION ===');
  const versionText = await page.locator('text=/v.*Demo/i').first().textContent();
  console.log('Version displayed:', versionText);

  // Check console logs
  page.on('console', msg => {
    if (msg.text().includes('MockBackendProvider') || msg.text().includes('getVersion')) {
      console.log('Browser console:', msg.text());
    }
  });

  console.log('\nPress Ctrl+C to exit...');

  // Keep browser open for manual inspection
  await page.waitForTimeout(60000);

  await browser.close();
})();
