import { expect, test } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// #1195: a model too large for the browser. wasm32 addresses at most 4 GiB, an
// open takes about ten times the file's size, and a failed allocation ends the
// wasm instance with a bare `RuntimeError: unreachable`. The viewer warns
// before the decode when the file's size says it may not fit, and says
// plainly that it did not fit when the instance stops.
//
// No file of several hundred MiB is in CI, so the ceiling is lowered instead of
// the model raised. This file is the warning: it compares the estimate with
// `?memoryCeilingBytes=`, which defaults to 4 GiB. The stop itself is
// memory-ceiling-capped.spec.ts.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const tier1 = path.resolve(
  __dirname,
  '../../corpus/tier1/architectural-2024/architectural-2024.rvt',
);

test.describe('the warning before the decode', () => {
  test('a file whose estimate passes the ceiling is warned about, and still opens', async ({
    page,
  }) => {
    expect(fs.existsSync(tier1), `${tier1} is committed`).toBe(true);
    const fileBytes = fs.statSync(tier1).size;
    // The estimate is ten times the file; a ceiling under it warns. The real
    // one is 4 GiB, which a file of this size does not reach.
    const ceiling = fileBytes * 10 - 1;
    await page.goto(`/?memoryCeilingBytes=${ceiling}`);
    await expect(page.locator('#status')).toHaveText(/Ready/);
    await page.locator('#file-input').setInputFiles(tier1);

    // The card warns while it loads; the toast is what outlasts a quick open.
    await expect(page.locator('#toast-region .toast', { hasText: 'may be too large' })).toBeVisible();
    await expect(page.locator('#status')).toHaveText(/Loaded/);
    const stats = JSON.parse((await page.locator('#status').getAttribute('data-load-stats')) ?? 'null');
    expect(stats.memoryWarning).toBe(true);
    expect(stats.estimatedPeakBytes).toBe(fileBytes * 10);
    // A warning is not a refusal: the model opened.
    await expect(page.locator('#status')).not.toHaveAttribute('data-load-failure', /.+/);
    await expect(page.locator('#file-meta')).toContainText(/[1-9]\d* entities/);
  });

  test('a file whose estimate fits the ceiling is not warned about', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/Ready/);
    await page.locator('#file-input').setInputFiles(tier1);
    await expect(page.locator('#status')).toHaveText(/Loaded/);
    const stats = JSON.parse((await page.locator('#status').getAttribute('data-load-stats')) ?? 'null');
    expect(stats.memoryWarning).toBe(false);
    await expect(page.locator('#toast-region .toast', { hasText: 'may be too large' })).toHaveCount(0);
  });
});
