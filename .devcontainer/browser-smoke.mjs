import { spawn } from 'node:child_process';
import { open, writeFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const require = createRequire(new URL('../viewer/package.json', import.meta.url));
const { chromium } = require('@playwright/test');
const evidence = process.argv[2];
if (!evidence || !path.isAbsolute(evidence)) throw new Error('Absolute evidence directory required');
const port = Number(process.env.RVT_VIEWER_PORT ?? 4173);
const url = `http://127.0.0.1:${port}`;
const log = await open(path.join(evidence, 'preview.log'), 'w');
const vite = path.join(path.dirname(require.resolve('vite/package.json')), 'bin/vite.js');
const server = spawn(process.execPath, [vite, 'preview', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], {
  cwd: path.join(root, 'viewer'), stdio: ['ignore', log.fd, log.fd],
});
let browser;
try {
  const deadline = Date.now() + 30_000;
  for (;;) {
    if (server.exitCode !== null) throw new Error(`Preview exited: ${server.exitCode}; inspect preview.log`);
    try { if ((await fetch(url)).ok) break; } catch {}
    if (Date.now() >= deadline) throw new Error('Preview did not become ready');
    await new Promise(resolve => setTimeout(resolve, 250));
  }
  browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  page.on('pageerror', error => errors.push(String(error)));
  await page.goto(url);
  await page.locator('input[type=file]').setInputFiles(path.join(evidence, 'smoke.rvt'));
  await page.getByText(/Opened Revit 2024/).waitFor({ timeout: 120_000 });
  await page.screenshot({ path: path.join(evidence, 'viewer-smoke.png'), fullPage: true });
  await writeFile(path.join(evidence, 'browser.json'), JSON.stringify({
    url, title: await page.title(), text: await page.locator('body').innerText(), errors,
    fixture: 'Synthetic, scaffold-only; does not validate real Revit geometry',
  }, null, 2));
  if (errors.length) throw new Error(`Browser errors: ${errors.join('; ')}`);
} finally {
  await browser?.close();
  server.kill('SIGTERM');
  await log.close();
}
