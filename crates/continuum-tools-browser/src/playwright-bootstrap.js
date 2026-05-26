// Playwright bootstrap script — invoked by PlaywrightRunner inside the sandbox.
// Reads the invocation payload from argv[2] as JSON, drives Playwright,
// writes results as JSON to stdout.

const { chromium } = require('playwright');

async function main() {
  const payload = JSON.parse(process.argv[2] || '{}');
  const { url, action = 'navigate', screenshot = false } = payload;

  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();

  try {
    if (action === 'navigate' && url) {
      await page.goto(url, { waitUntil: 'networkidle' });
    }

    const result = { ok: true, url: page.url(), title: await page.title() };

    if (screenshot) {
      result.screenshot = await page.screenshot({ encoding: 'base64', type: 'png' });
    }

    console.log(JSON.stringify(result));
  } catch (err) {
    console.error(JSON.stringify({ ok: false, error: err.message }));
    process.exit(1);
  } finally {
    await browser.close();
  }
}

main();
