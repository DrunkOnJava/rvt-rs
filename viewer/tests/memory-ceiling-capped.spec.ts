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

/** The card the decoder leaves while it works in the stretch it reports nothing in (AFTER_STAGE in main.ts). */
const BUILDING = 'Building the model';

/** One report of the decoder as the page keeps it on #status: its stage, and the wasm memory and the card at that moment. */
interface Report {
  stage: string;
  done: number;
  total: number | null;
  memoryBytes: number;
  card: { step: string; value: number | null; hint: string };
}

/** The decoder's own reports, in order, without the worker's steps around them. */
async function decoderReports(page: Page): Promise<Report[]> {
  const raw = await page.locator('#status').getAttribute('data-load-progress');
  expect(raw, 'the page keeps what the worker reported').not.toBeNull();
  return (JSON.parse(raw!) as Array<Partial<Report>>).filter(
    (event): event is Report => event.stage !== undefined,
  );
}

function sameReport(a: Report, b: Report): boolean {
  return (
    a.stage === b.stage &&
    a.done === b.done &&
    a.total === b.total &&
    a.memoryBytes === b.memoryBytes &&
    a.card.step === b.card.step
  );
}

// The stretch after the last partition stream is inflated: the decoder reports
// nothing until it has built the model, and the card names it "Building the
// model" (load-progress.spec.ts). A stop there must say so. Where in the open
// the memory grows is the allocator's, and it has moved between stretches
// without the peak changing (#1374: on Core Interior the wasm memory is 275.4
// MiB at the last stream, 309.1 MiB at the end of the walk and 325.4 MiB at
// the end of the open. Before #1370 the last 16.3 MiB were taken before the
// saved graphics were reported; after it they are taken inside the saved
// meshes, and a cap tuned to the old profile, 310 MiB, then stopped in the
// meshes: the viewer measurement of Deploy viewer runs 41870855196 at
// f1f80d9e and 41885589164 at 0ca59127). So the cap is not tuned: the test
// opens the model once at the real ceiling to measure its profile, and puts
// the cap between the memory at the first report that leaves the card on
// "Building the model" and the memory at the first report after those. The
// allocation that crosses the cap is then in the stretch, whatever the
// profile, and the capped open must have stopped there: its reports are the
// measured ones up to the last, which still reads "Building the model". The
// profile, the cap and the capped open's reports are written to
// memory-ceiling-cap-core-interior.json in the measurement artifact.
coreInteriorTest(
  'a stop in the stretch the decoder reports nothing in says it was building the model',
  async ({ baseURL }) => {
    test.setTimeout(600_000);
    const wasmPages = 65_536;
    const record = (content: Record<string, unknown>): void =>
      fs.writeFileSync(
        path.join(outDir, 'memory-ceiling-cap-core-interior.json'),
        `${JSON.stringify(content, null, 2)}\n`,
      );
    let profile: Report[] = [];
    await underCap(baseURL, wasmPages, '', coreInterior, async (page) => {
      await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: 300_000 });
      profile = await decoderReports(page);
      record({ profile });
    });
    const start = profile.findIndex((report) => report.card.step === BUILDING);
    expect(start, `a report left the card on "${BUILDING}"`).toBeGreaterThanOrEqual(0);
    const end = profile.findIndex((report, i) => i > start && report.card.step !== BUILDING);
    expect(end, `a report follows the ones on "${BUILDING}"`).toBeGreaterThan(start);
    const before = profile[start];
    const after = profile[end];
    const grown = after.memoryBytes - before.memoryBytes;
    expect(
      grown,
      `the open grows the wasm memory between its report of ${before.stage} ${before.done}/${before.total} ` +
        `(${(before.memoryBytes / MIB).toFixed(1)} MiB) and its report of ${after.stage} ` +
        `${after.done}/${after.total} (${(after.memoryBytes / MIB).toFixed(1)} MiB), the stretch it reports nothing in; ` +
        'with no growth there is nothing for a cap to stop in',
    ).toBeGreaterThanOrEqual(MIB);
    const capPages = Math.floor((before.memoryBytes + grown / 2) / PAGE_BYTES);
    const capBytes = capPages * PAGE_BYTES;
    expect(capBytes).toBeGreaterThan(before.memoryBytes);
    expect(capBytes).toBeLessThan(after.memoryBytes);

    await underCap(baseURL, capPages, `?memoryCeilingBytes=${capBytes}`, coreInterior, async (page) => {
      await expect(page.locator('#status')).toHaveAttribute('data-load-failure', 'too-large', {
        timeout: 240_000,
      });
      // The open followed its measured profile up to the stop, and the last
      // thing the decoder reported was in the stretch.
      const stopped = await decoderReports(page);
      record({ before, after, capBytes, profile, stopped });
      expect(stopped.length, 'reports before the stop').toBeGreaterThan(start);
      expect(stopped.length, 'reports before the stop').toBeLessThanOrEqual(end);
      stopped.forEach((report, i) => {
        expect(sameReport(report, profile[i]), `report ${i}: ${JSON.stringify(report)}`).toBe(true);
        expect(report.memoryBytes).toBeLessThanOrEqual(capBytes);
      });
      const last = stopped[stopped.length - 1];
      expect(last.card.step).toBe(BUILDING);
      const body = page.locator('#scaffold-note-body');
      await expect(body).toContainText(', while building the model,');
      await expect(body).toContainText(`, while ${last.card.step.toLowerCase()},`);
      await expect(body).not.toContainText('saved meshes');
      await expect.poll(() => page.workers().length, 'the worker is ended').toBe(0);
    });
  },
);
