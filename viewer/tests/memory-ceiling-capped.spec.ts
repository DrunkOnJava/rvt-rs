import { chromium, expect, test, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// #1195: the stop. wasm32 addresses at most 4 GiB, and a failed allocation ends
// the wasm instance with a bare `RuntimeError: unreachable` and no panic
// message. No model that large is in CI, so the ceiling is lowered instead of
// the model raised, and the stop is real: each test launches a Chromium whose
// V8 caps wasm memory (`--js-flags=--wasm-max-mem-pages=N`, 64 KiB a page),
// below what the model's open needs (saved-meshes-measure.spec.ts has the
// peaks). The wasm, its allocator and the worker are the shipped ones; only the
// ceiling is lower. `?memoryCeilingBytes=` tells the page what that ceiling is,
// so its message names it; leaving it out tells the page the real 4 GiB, which
// a stop at a few MiB is a long way short of. The warning before the decode is
// memory-ceiling.spec.ts, and what the page does with failures the engine words
// differently, or that are not memory, is worker-failures.spec.ts. Each test has
// a browser of its own because the cap is a launch option.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const corpus = path.resolve(__dirname, '../../_project_corpus/Revit');
const einhoven = path.join(corpus, 'Revit_IFC5_Einhoven.rvt');
const coreInterior = path.join(corpus, '2024_Core_Interior.rvt');
const outDir = path.resolve(__dirname, '../measurements');
// Deploy viewer fetches every model and sets this, so a model missing there
// fails; elsewhere it is a skip.
const required = process.env.RVT_VIEWER_REQUIRE_JOURNEYS === '1';

const PAGE_BYTES = 64 * 1024;
const MIB = 2 ** 20;

/** Opens `file` in a Chromium capped at `pages` wasm pages, on the preview server, and runs `check`. */
async function underCap(
  baseURL: string | undefined,
  pages: number,
  query: string,
  file: string,
  check: (page: Page) => Promise<void>,
): Promise<void> {
  expect(fs.existsSync(file), `${file} was not fetched`).toBe(true);
  fs.mkdirSync(outDir, { recursive: true });
  const browser = await chromium.launch({
    args: [`--js-flags=--wasm-max-mem-pages=${pages}`],
  });
  try {
    const context = await browser.newContext({ baseURL });
    const page = await context.newPage();
    await page.goto(`/${query}`);
    await expect(page.locator('#status')).toHaveText(/Ready/);
    await page.locator('#file-input').setInputFiles(file);
    await check(page);
  } finally {
    await browser.close();
  }
}

/** The note is what a person sees: the empty state under it is over the viewport too. */
async function noteIsOnTop(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const card = document.querySelector('#scaffold-note .note-card')!.getBoundingClientRect();
    const top = document.elementFromPoint(card.x + card.width / 2, card.y + card.height / 2);
    return top?.closest('#scaffold-note') !== null;
  });
}

const einhovenTest = required || fs.existsSync(einhoven) ? test : test.skip;

einhovenTest(
  'a model past the ceiling is reported as too large, not as unreachable',
  async ({ baseURL }) => {
    const capPages = 256;
    const capBytes = capPages * PAGE_BYTES;
    await underCap(
      baseURL,
      capPages,
      `?memoryCeilingBytes=${capBytes}`,
      einhoven,
      async (page) => {
        // The stop is named in the status line, and the page says so in the
        // viewport, with what to use instead.
        await expect(page.locator('#status')).toHaveAttribute('data-load-failure', 'too-large');
        await expect(page.locator('#status')).toHaveText(/^Too large for the browser · stopped at /);
        const note = page.locator('#scaffold-note');
        await expect(note).toBeVisible();
        expect(await noteIsOnTop(page), 'the note is under the empty state').toBe(true);
        await expect(note.locator('#scaffold-note-title')).toHaveText('Too large for the browser');
        const body = note.locator('#scaffold-note-body');
        await expect(body).toContainText(/Revit_IFC5_Einhoven\.rvt \(\d+(\.\d)? KiB\)/);
        await expect(body).toContainText('would not give this tab more than');
        await expect(body).toContainText(/, while (reading partition streams|building the model),/);
        await expect(body).toContainText('needs more');
        await expect(body).toContainText('16 MiB');
        // What reads a larger file, as the code takes it: --large-model on the
        // two CLIs, and the two limits to raise in Python; not "any size".
        await expect(body).toContainText('rvt-ifc and rvt-gltf with --large-model');
        await expect(body).toContainText('max_stream_bytes and max_inflate_bytes');
        await expect(body).not.toContainText(/any size/i);
        await expect(page.locator('#status-panel')).toContainText('Too large for the browser');

        // The engine's own message is in the status panel, where it can be
        // read and reported, and nowhere a person reads first.
        await expect(
          page.locator('#status-panel .status-row', {
            has: page.locator('.status-label', { hasText: /^Engine message$/ }),
          }),
        ).toContainText(/RuntimeError: unreachable/);
        for (const where of ['#status', '#scaffold-note', '#toast-region']) {
          expect(await page.locator(where).innerText(), where).not.toMatch(/unreachable|RuntimeError/);
        }
        await expect(page.locator('#toast-region .toast.bad')).toContainText('too large');

        // The loading card is gone, nothing is left to export, and the worker
        // that held the memory is ended.
        await expect(page.locator('#load-overlay')).toBeHidden();
        await expect(page.locator('#dropzone')).toBeVisible();
        await expect(page.locator('#export-ifc')).toBeDisabled();
        await expect.poll(() => page.workers().length, 'the worker is ended').toBe(0);

        // What the worker reported before it stopped is kept, with the memory
        // at each report, which never passes the cap.
        const events = JSON.parse(
          (await page.locator('#status').getAttribute('data-load-progress')) ?? '[]',
        ) as Array<{ stage?: string; memoryBytes?: number }>;
        expect(events.length).toBeGreaterThan(0);
        for (const event of events) {
          if (event.memoryBytes !== undefined) expect(event.memoryBytes).toBeLessThanOrEqual(capBytes);
        }
        await page.locator('#viewport').screenshot({ path: path.join(outDir, 'too-large.png') });
      },
    );
  },
);

einhovenTest(
  'a stop far short of the ceiling is not called too large',
  async ({ baseURL }) => {
    // The same stop at 16 MiB, in a page that believes the real 4 GiB: the
    // decoder ran out of memory with 99.6% of a tab's to spare, on a file under
    // 1 MiB. That is what a damaged file looks like too, so the page says what
    // happened and does not say the file needs more memory than a tab has.
    await underCap(
      baseURL,
      256,
      '',
      einhoven,
      async (page) => {
        await expect(page.locator('#status')).toHaveAttribute('data-load-failure', 'out-of-memory');
        await expect(page.locator('#status')).toHaveText(/^Ran out of memory · stopped at /);
        const note = page.locator('#scaffold-note');
        await expect(note).toBeVisible();
        expect(await noteIsOnTop(page)).toBe(true);
        await expect(note.locator('#scaffold-note-title')).toHaveText('Ran out of memory');
        const body = note.locator('#scaffold-note-body');
        await expect(body).toContainText('a long way short of the 4 GiB a tab can hold');
        await expect(body).toContainText('may be damaged');
        await expect(body).not.toContainText('needs more');
        await expect(page.locator('#toast-region .toast.bad')).toContainText('ran out of memory');
        await expect(page.locator('#toast-region')).not.toContainText(/too large/i);
        await expect.poll(() => page.workers().length, 'the worker is ended').toBe(0);
      },
    );
  },
);

const coreInteriorTest = required || fs.existsSync(coreInterior) ? test : test.skip;

coreInteriorTest(
  'a stop in the stretch the decoder reports nothing in says it was building the model',
  async ({ baseURL }) => {
    test.setTimeout(300_000);
    // Core Interior takes 291.6 MiB for its partition streams, all of it by the
    // first inflate, and 324 MiB by the end of its open (saved-meshes-measure
    // and the reports of load-progress.spec.ts). A cap between them stops it
    // after the last stream and before the saved graphics, where the decoder
    // reports nothing and the card says "Building the model".
    const capMiB = 310;
    await underCap(
      baseURL,
      (capMiB * MIB) / PAGE_BYTES,
      `?memoryCeilingBytes=${capMiB * MIB}`,
      coreInterior,
      async (page) => {
        await expect(page.locator('#status')).toHaveAttribute('data-load-failure', 'too-large', {
          timeout: 240_000,
        });
        await expect(page.locator('#scaffold-note-body')).toContainText(
          ', while building the model,',
        );
        await expect.poll(() => page.workers().length, 'the worker is ended').toBe(0);
      },
    );
  },
);
