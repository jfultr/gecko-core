// Opens an article from file:// in Chromium and Firefox (light and dark) and checks it.
// Usage: node verify.mjs <article.html> [--screenshot out-dir]
// Checks: no console errors or page errors, no network request except the two font hosts,
// the page also renders with JavaScript disabled, and (if present) every .code-host editor
// shows tokens. Needs the browsers once: `npx playwright install chromium firefox`
// (Linux may also need system libraries: `sudo npx playwright install-deps`).
// Run through `articles.py verify <article.html>`.
import { chromium, firefox } from 'playwright';
import path from 'node:path';
import fs from 'node:fs';

const args = process.argv.slice(2);
const file = path.resolve(args[0] ?? '');
const shotDir = args.includes('--screenshot') ? path.resolve(args[args.indexOf('--screenshot') + 1]) : null;
if (!args[0] || !fs.existsSync(file)) {
  console.error('usage: node verify.mjs <article.html> [--screenshot dir]');
  process.exit(2);
}
const FONT_HOSTS = ['https://fonts.googleapis.com/', 'https://fonts.gstatic.com/'];
let failed = false;
const fail = (msg) => { failed = true; console.log('  FAIL', msg); };

for (const [name, type] of [['chromium', chromium], ['firefox', firefox]]) {
  for (const scheme of ['light', 'dark']) {
    const browser = await type.launch();
    const page = await browser.newPage({ colorScheme: scheme, viewport: { width: 820, height: 900 } });
    const problems = [];
    const external = new Set();
    page.on('console', (m) => { if (['error', 'warning'].includes(m.type())) problems.push(`console ${m.type()}: ${m.text().slice(0, 160)}`); });
    page.on('pageerror', (e) => problems.push(`page error: ${String(e).slice(0, 160)}`));
    page.on('request', (r) => {
      const u = r.url();
      if (/^(file|blob|data|about):/.test(u)) return;
      if (!FONT_HOSTS.some((h) => u.startsWith(h))) external.add(u.slice(0, 80));
    });
    const t0 = Date.now();
    await page.goto('file://' + file);
    const hasEditors = (await page.locator('pre > code[data-lang]').count()) + (await page.locator('.code-host').count()) > 0;
    if (hasEditors) {
      try { await page.waitForSelector('.code-host .view-line span', { timeout: 20000 }); }
      catch { problems.push('code blocks were not turned into editors within 20 s'); }
    } else {
      await page.waitForTimeout(1000);
    }
    const ms = Date.now() - t0;
    const info = await page.evaluate(() => ({
      editors: document.querySelectorAll('.code-host').length,
      tokenColors: new Set([...document.querySelectorAll('.code-host .view-line span span')].map((s) => getComputedStyle(s).color)).size,
    }));
    for (const u of external) problems.push(`network request outside fonts: ${u}`);
    console.log(`${name} ${scheme}: ${ms} ms, editors ${info.editors}, token colors ${info.tokenColors}`);
    for (const p of problems) fail(p);
    if (shotDir) {
      fs.mkdirSync(shotDir, { recursive: true });
      await page.screenshot({ path: path.join(shotDir, `${path.basename(file, '.html')}-${name}-${scheme}.png`), fullPage: false });
    }
    await browser.close();
  }
}
const b = await chromium.launch();
const ctx = await b.newContext({ javaScriptEnabled: false });
const page = await ctx.newPage();
await page.goto('file://' + file);
const visibleText = await page.evaluate(() => document.body.innerText.length);
console.log(`no JavaScript: ${visibleText} characters of visible text`);
if (visibleText < 500) fail('page shows almost no text without JavaScript');
await b.close();
console.log(failed ? 'VERIFY FAILED' : 'VERIFY OK');
process.exit(failed ? 1 : 0);
