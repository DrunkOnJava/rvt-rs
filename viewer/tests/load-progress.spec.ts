import { expect, test, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// #1195: while a model decodes, the loading card shows the decoder's own
// progress (the stages `rvt-gltf --progress` prints) instead of a sweep and a
// guess from the file size. The worker posts each report from inside the wasm
// call, so the page sees them while the decode runs. The page keeps every
// report on #status as `data-load-progress`, with what the card showed once it
// had applied that report (its step, the bar's value, its hint), so these
// tests read the card's behaviour from that record rather than sampling the
// page, which a decode 3 s into its partition stage would make a coin flip:
//
//   - on the tier-1 fixture, which opens in milliseconds: the stages, their
//     counts and the steps after the decode;
//   - on Core Interior (17 to 24 s on Deploy viewer's runner): the bar is
//     determinate for the counted stage at the percent of each report, sweeps
//     for the stretch the decoder reports nothing in, and says "Building the
//     model" once. A screenshot is taken while the partition stage runs, and it
//     is the only thing that depends on timing, so it is best effort: what it
//     shows is written beside it.

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
  card?: { step: string; value: number | null; hint: string };
}

async function loadEvents(page: Page): Promise<LoadEvent[]> {
  const raw = await page.locator('#status').getAttribute('data-load-progress');
  expect(raw, 'the page keeps what the worker reported').not.toBeNull();
  return JSON.parse(raw!) as LoadEvent[];
}

/** Records every text #status takes, so what a live region was told can be counted. */
async function logStatusText(page: Page): Promise<void> {
  await page.evaluate(() => {
    const status = document.getElementById('status')!;
    const log: string[] = [];
    (window as unknown as { __statusLog: string[] }).__statusLog = log;
    new MutationObserver(() => log.push(status.textContent ?? '')).observe(status, {
      childList: true,
      characterData: true,
      subtree: true,
    });
  });
}

async function statusLog(page: Page): Promise<string[]> {
  return page.evaluate(() => (window as unknown as { __statusLog: string[] }).__statusLog);
}

/**
 * The stages whose last report hands over to something the decoder reports
 * nothing in, so the card names that and sweeps (AFTER_STAGE in main.ts).
 */
const HAS_AFTER = ['partition-scan', 'saved-meshes'];

/**
 * A report the card gives a determinate bar: a total above one, and not the end
 * of a stage with something after it. The end of a stage with nothing after it
 * (the candidate scan's 100%) is still a count, and the bar is full.
 */
function isCounted(event: LoadEvent): boolean {
  return (
    event.stage !== undefined &&
    event.total !== null &&
    event.total !== undefined &&
    event.total > 1 &&
    !(event.done! >= event.total && HAS_AFTER.includes(event.stage))
  );
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
    expect(event.card, 'the card applied it').toBeDefined();
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
  await logStatusText(page);
  await page.locator('#file-input').setInputFiles(coreInterior);

  // The screenshot is the only thing here that depends on timing. Wait, polling
  // every animation frame, for the bar to be a quarter full or more while the
  // partition stage runs, and take it at once; a decode that is past that by
  // the time the page looks, or a screenshot that lands after it, only means
  // no picture, which is written down rather than failed on.
  const screenshot: { taken: boolean; cardAtCapture: unknown; note: string } = {
    taken: false,
    cardAtCapture: null,
    note: '',
  };
  try {
    await page.waitForFunction(
      () => Number(document.getElementById('load-rail')?.getAttribute('aria-valuenow')) >= 25,
      null,
      { polling: 'raf', timeout: 120_000 },
    );
    screenshot.cardAtCapture = await page.evaluate(() => ({
      step: document.getElementById('load-step')?.textContent,
      value: document.getElementById('load-rail')?.getAttribute('aria-valuenow'),
      hint: document.getElementById('load-hint')?.textContent,
    }));
    await page.locator('#load-overlay').screenshot({
      path: path.join(outDir, 'load-progress-core-interior.png'),
      timeout: 10_000,
    });
    screenshot.taken = true;
    screenshot.note =
      'taken when the bar first read 25% or more; the card may have moved on while the picture was taken, so cardAtCapture is what the page said just before';
  } catch (err) {
    screenshot.note = `no screenshot: ${(err as Error).message.split('\n')[0]}`;
  }

  await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: 500_000 });
  const events = await loadEvents(page);
  fs.writeFileSync(
    path.join(outDir, 'load-progress-core-interior.json'),
    `${JSON.stringify({ screenshot, events }, null, 2)}\n`,
  );

  // Every report the decoder made was applied by the card, and the bar says
  // what the report said: the percent of its count where it has one, and a
  // sweep where it has none or the stage has ended.
  const reports = events.filter((event) => event.stage !== undefined);
  expect(reports.length).toBeGreaterThan(0);
  for (const event of reports) {
    expect(event.card, `${event.stage} ${event.done}/${event.total} was applied`).toBeDefined();
    if (isCounted(event)) {
      expect(event.card!.value, `${event.stage} ${event.done}/${event.total}`).toBe(
        Math.round((event.done! / event.total!) * 100),
      );
      expect(event.card!.hint).toMatch(/^\d+% · /);
    } else {
      expect(event.card!.value, `${event.stage} ${event.done}/${event.total} sweeps`).toBeNull();
      expect(event.card!.hint).toBe('');
    }
  }

  // The partition stage is counted on this project: the bar was determinate at
  // more than one value, rising within the stage, and the card named the stage
  // and what it counted.
  const counted = reports.filter(isCounted);
  expect(counted.length, 'reports with a count').toBeGreaterThanOrEqual(2);
  const values = counted.map((event) => event.card!.value!);
  expect(new Set(values).size, `fills: ${values.join(', ')}`).toBeGreaterThanOrEqual(2);
  for (const value of values) {
    expect(value).toBeGreaterThanOrEqual(0);
    expect(value).toBeLessThanOrEqual(100);
  }
  const partition = counted.filter((event) => event.stage === 'partition-scan');
  expect(partition.length, 'the partition streams are counted').toBeGreaterThanOrEqual(2);
  const rising = partition.map((event) => event.card!.value!);
  expect([...rising].sort((a, b) => a - b), 'the partition bar rises').toEqual(rising);
  expect(
    new Set(rising).size,
    `partition fills: ${rising.join(', ')}`,
  ).toBeGreaterThanOrEqual(2);
  for (const event of partition) {
    expect(event.card!.step).toBe('Reading partition streams');
    expect(event.card!.hint).toMatch(new RegExp(`^\\d+% · ${event.done} of ${event.total} streams$`));
  }

  // The stretch after the last stream is inflated reports nothing until the
  // saved graphics. The card names it, and says so once: #status is a live
  // region, and it was told "Building the model" a single time.
  const building = reports.filter((event) => event.card!.step === 'Building the model');
  expect(building.length, 'reports that left the card on Building the model').toBeGreaterThan(0);
  for (const event of building) expect(event.card!.value).toBeNull();
  const log = await statusLog(page);
  expect(
    log.filter((text) => text === 'Building the model'),
    `status text: ${log.join(' | ')}`,
  ).toHaveLength(1);
});
