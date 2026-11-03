import { expect, test, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// #1195: while a model decodes, the loading card shows the decoder's own
// progress (the stages `rvt-gltf --progress` prints) instead of a sweep and a
// guess from the file size. The worker posts each report from inside the wasm
// call, so the page sees them while the decode runs; the page keeps the whole
// list on #status as `data-load-progress`.
//
//   - on the tier-1 fixture, which opens in milliseconds, the list is checked:
//     the stages, their counts, and the steps after the decode;
//   - on Core Interior (24 s on Deploy viewer's runner), the card itself is
//     watched while it loads: the bar must turn determinate and fill, and the
//     card must name the stages. A screenshot of the bar part-way and the
//     samples go to measurements/, which Deploy viewer uploads.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const tier1 = path.resolve(
  __dirname,
  '../../corpus/tier1/architectural-2024/architectural-2024.rvt',
);
const corpus = path.resolve(__dirname, '../../_project_corpus/Revit');
const coreInterior = path.join(corpus, '2024_Core_Interior.rvt');
const outDir = path.resolve(__dirname, '../measurements');
// Deploy viewer fetches every model and sets this, so a model missing there
// fails; elsewhere it is a skip.
const required = process.env.RVT_VIEWER_REQUIRE_JOURNEYS === '1';

const STAGES = [
  'schema-parse',
  'candidate-scan',
  'element-decode',
  'partition-scan',
  'saved-graphics',
  'saved-meshes',
];

interface LoadEvent {
  t: number;
  stage?: string;
  done?: number;
  total?: number | null;
  memoryBytes?: number;
  step?: string;
}

async function loadEvents(page: Page): Promise<LoadEvent[]> {
  const raw = await page.locator('#status').getAttribute('data-load-progress');
  expect(raw, 'the page keeps what the worker reported').not.toBeNull();
  return JSON.parse(raw!) as LoadEvent[];
}

test('the decoder reports its stages to the page, and the steps after it follow', async ({
  page,
}) => {
  expect(fs.existsSync(tier1), `${tier1} is committed`).toBe(true);
  await page.goto('/');
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await page.locator('#file-input').setInputFiles(tier1);
  await expect(page.locator('#status')).toHaveText(/Loaded/);

  const events = await loadEvents(page);
  const stages = events.filter((event) => event.stage !== undefined);
  expect(stages.length, 'the decoder reported').toBeGreaterThan(0);
  // The walk begins with the schema, and every report names a stage the
  // decoder has, with a count that is not past its total.
  expect(stages[0].stage).toBe('schema-parse');
  for (const event of stages) {
    expect(STAGES, `stage ${event.stage}`).toContain(event.stage);
    if (event.total !== null && event.total !== undefined) {
      expect(event.done!, `${event.stage} ${event.done} of ${event.total}`).toBeLessThanOrEqual(
        event.total,
      );
    }
    expect(event.memoryBytes, 'the wasm memory at that moment').toBeGreaterThan(0);
  }
  // Within a stage the count only grows until the stage starts over.
  let previous: LoadEvent | null = null;
  for (const event of stages) {
    if (previous && previous.stage === event.stage && event.done !== 0) {
      expect(event.done!, `${event.stage} went back`).toBeGreaterThanOrEqual(previous.done!);
    }
    previous = event;
  }
  // The element decode reaches its end: the walk finished.
  expect(
    stages.some(
      (event) =>
        event.stage === 'element-decode' &&
        event.total !== null &&
        event.total !== undefined &&
        event.done === event.total,
    ),
    'element-decode ended',
  ).toBe(true);

  // The steps after the decode come after every report of it, in time.
  const steps = events.filter((event) => event.step !== undefined).map((event) => event.step);
  expect(steps).toEqual(
    expect.arrayContaining(['Building scene graph', 'Rendering glTF', 'Building schedule']),
  );
  const lastStage = Math.max(...stages.map((event) => event.t));
  const sceneGraph = events.find((event) => event.step === 'Building scene graph')!;
  expect(sceneGraph.t).toBeGreaterThanOrEqual(lastStage);
});

const coreInteriorTest = required || fs.existsSync(coreInterior) ? test : test.skip;

coreInteriorTest('the loading card fills while Core Interior decodes', async ({ page }) => {
  test.setTimeout(600_000);
  expect(fs.existsSync(coreInterior), `${coreInterior} was not fetched`).toBe(true);
  fs.mkdirSync(outDir, { recursive: true });

  await page.goto('/');
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await page.locator('#file-input').setInputFiles(coreInterior);

  interface Sample {
    t: number;
    step: string;
    hint: string;
    determinate: boolean;
    value: number | null;
  }
  const samples: Sample[] = [];
  const started = Date.now();
  let screenshot = false;
  const deadline = started + 500_000;
  for (;;) {
    expect(Date.now(), 'Core Interior loads').toBeLessThan(deadline);
    const loaded = await page.locator('#status').getAttribute('data-load-stats');
    if (loaded !== null) break;
    const sample = await page.evaluate(() => {
      const rail = document.getElementById('load-rail')!;
      const now = rail.getAttribute('aria-valuenow');
      return {
        step: document.getElementById('load-step')!.textContent ?? '',
        hint: document.getElementById('load-hint')!.textContent ?? '',
        determinate: rail.classList.contains('is-determinate'),
        value: now === null ? null : Number(now),
        visible: !document.getElementById('load-overlay')!.classList.contains('hidden'),
      };
    });
    if (sample.visible) {
      samples.push({ ...sample, t: Date.now() - started });
      if (!screenshot && sample.determinate && sample.value! >= 25 && sample.value! <= 75) {
        await page.locator('#load-overlay').screenshot({
          path: path.join(outDir, 'load-progress-core-interior.png'),
        });
        screenshot = true;
      }
    }
    await page.waitForTimeout(200);
  }
  await expect(page.locator('#status')).toHaveText(/Loaded/);
  fs.writeFileSync(
    path.join(outDir, 'load-progress-core-interior.json'),
    `${JSON.stringify({ samples, events: await loadEvents(page) }, null, 2)}\n`,
  );

  // The card named the decoder's stages, not just "Opening".
  const labels = new Set(samples.map((sample) => sample.step));
  const stageLabels = [...labels].filter((label) =>
    /^(Reading the schema|Scanning for elements|Decoding elements|Reading partition streams|Reading saved graphics|Drawing doors and windows from saved meshes|Building the model|Finishing the model)$/.test(
      label,
    ),
  );
  expect(stageLabels.length, `stages named on the card: ${[...labels].join(' | ')}`).toBeGreaterThanOrEqual(2);

  // The bar turned determinate, at more than one fill, and a fill was
  // somewhere between empty and full.
  const fills = new Set(samples.filter((s) => s.determinate).map((s) => s.value));
  expect(fills.size, `fills seen: ${[...fills].join(', ')}`).toBeGreaterThanOrEqual(2);
  expect(
    [...fills].some((value) => value !== null && value > 0 && value < 100),
    'a part-way fill',
  ).toBe(true);
  // A determinate bar says what it counts.
  const counted = samples.filter((s) => s.determinate && s.value! > 0 && s.value! < 100);
  expect(counted.every((s) => /^\d+% · /.test(s.hint))).toBe(true);
  expect(screenshot, 'a screenshot of the bar part-way').toBe(true);
});
