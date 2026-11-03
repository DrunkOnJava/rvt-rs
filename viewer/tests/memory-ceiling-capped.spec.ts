import { expect, test } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// #1195: the stop. wasm32 addresses at most 4 GiB, and a failed allocation ends
// the wasm instance with a bare `RuntimeError: unreachable` and no panic
// message. No model that large is in CI, so the ceiling is lowered instead of
// the model raised, and the stop is real: this file runs in a Chromium whose
// V8 caps wasm memory at 16 MiB (`--js-flags=--wasm-max-mem-pages=256`, 64 KiB
// a page), where Einhoven's 23 MiB peak (saved-meshes-measure.spec.ts) cannot
// be reached. The wasm, its allocator and the worker are the shipped ones;
// only the ceiling is lower. `?memoryCeilingBytes=` tells the page what the
// ceiling is, so its message names 16 MiB. The warning before the decode is
// memory-ceiling.spec.ts. `test.use` with launch options has to be top level,
// which is why this is a file of its own.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const einhoven = path.resolve(__dirname, '../../_project_corpus/Revit/Revit_IFC5_Einhoven.rvt');
const outDir = path.resolve(__dirname, '../measurements');
// Deploy viewer fetches every model and sets this, so a model missing there
// fails; elsewhere it is a skip.
const required = process.env.RVT_VIEWER_REQUIRE_JOURNEYS === '1';

const CAP_PAGES = 256;
const CAP_BYTES = CAP_PAGES * 64 * 1024;

test.use({ launchOptions: { args: [`--js-flags=--wasm-max-mem-pages=${CAP_PAGES}`] } });

const modelTest = required || fs.existsSync(einhoven) ? test : test.skip;

modelTest('a model past it is reported as too large, not as unreachable', async ({ page }) => {
  expect(fs.existsSync(einhoven), `${einhoven} was not fetched`).toBe(true);
  fs.mkdirSync(outDir, { recursive: true });
  await page.goto(`/?memoryCeilingBytes=${CAP_BYTES}`);
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await page.locator('#file-input').setInputFiles(einhoven);

  // The stop is named in the status line, and the page says so in the
  // viewport, with what to use instead.
  await expect(page.locator('#status')).toHaveAttribute('data-load-failure', 'too-large');
  await expect(page.locator('#status')).toHaveText(/^Too large for the browser · stopped at /);
  const note = page.locator('#scaffold-note');
  await expect(note).toBeVisible();
  await expect(note.locator('#scaffold-note-title')).toHaveText('Too large for the browser');
  const body = note.locator('#scaffold-note-body');
  await expect(body).toContainText('Revit_IFC5_Einhoven.rvt');
  await expect(body).toContainText('would not give this tab more than');
  await expect(body).toContainText('16 MiB');
  await expect(body).toContainText('rvt-ifc');
  await expect(body).toContainText('rvt-gltf');
  await expect(body).toContainText('--large-model');
  await expect(page.locator('#status-panel')).toContainText('Too large for the browser');

  // Nothing on the page shows the engine's message, and the loading card
  // is gone.
  await expect(page.locator('#load-overlay')).toBeHidden();
  const text = await page.locator('body').innerText();
  expect(text).not.toMatch(/unreachable|RuntimeError/);
  await expect(page.locator('#dropzone')).toBeVisible();
  await expect(page.locator('#export-ifc')).toBeDisabled();

  // What the worker reported before it stopped is kept, with the memory at
  // each report, which never passes the cap.
  const events = JSON.parse(
    (await page.locator('#status').getAttribute('data-load-progress')) ?? '[]',
  ) as Array<{ stage?: string; memoryBytes?: number }>;
  expect(events.length).toBeGreaterThan(0);
  for (const event of events) {
    if (event.memoryBytes !== undefined) expect(event.memoryBytes).toBeLessThanOrEqual(CAP_BYTES);
  }
  await page.locator('#viewport').screenshot({ path: path.join(outDir, 'too-large.png') });
});
