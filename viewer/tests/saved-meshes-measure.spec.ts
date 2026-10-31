import { expect, test, type Page } from '@playwright/test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// The viewer draws the doors and windows of Revit 2023 to 2027 files from the
// meshes Revit saved for them, with a toggle that keeps their boxes. The
// owner's condition for that default (2026-10-30) was that the viewer's
// memory and load time are measured in Playwright on the reference models.
// This spec opens each model with the option on and off, each in a fresh
// browser context, and records what the open cost:
//
//   loadMs           the click to the loaded model, as the viewer measures it
//   openMs           the wasm open call alone, where the saved graphics are read
//   wasmMemoryBytes  the wasm linear memory after the whole parse; it only
//                    grows, so it is the peak against the 4 GiB a tab holds
//   savedMeshBodies  doors and windows drawn from saved meshes
//
// The result goes to measurements/saved-meshes.json and, as a table, to the
// log and the job summary; Deploy viewer uploads the JSON. It runs on its own
// (RVT_VIEWER_MEASURE=1, one worker) so another spec does not compete for the
// CPU, and it asserts behaviour only: the timings are for reading, not pinned.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const corpus = path.resolve(__dirname, '../../_project_corpus/Revit');
const outFile =
  process.env.RVT_VIEWER_MEASURE_OUT ?? path.resolve(__dirname, '../measurements/saved-meshes.json');
// Deploy viewer fetches every model and sets this, so a model missing there
// fails; elsewhere it is a skip.
const required = process.env.RVT_VIEWER_REQUIRE_JOURNEYS === '1';

// Chromium rounds the JavaScript heap sizes it reports unless told otherwise.
test.use({ launchOptions: { args: ['--enable-precise-memory-info'] } });

interface Model {
  file: string;
  release: number;
  loadTimeout: number;
  /** Its doors and windows are known to draw from saved meshes (Measure run 40932136360). */
  drawsSavedMeshes?: boolean;
}

// Core Interior first: it is the largest, and its memory answers the question
// of the 4 GiB a tab can hold.
const models: Model[] = [
  { file: '2024_Core_Interior.rvt', release: 2024, loadTimeout: 300_000, drawsSavedMeshes: true },
  { file: 'Revit_IFC5_Einhoven.rvt', release: 2023, loadTimeout: 120_000 },
  ...['Architecture', 'Mechanical', 'Plumbing', 'Electrical'].map((discipline) => ({
    file: `RE1-${discipline}.rvt`,
    release: 2025,
    loadTimeout: 120_000,
  })),
];

interface Diagnostics {
  input: { revit_version: number };
  exported: {
    building_elements: number;
    by_ifc_type: Record<string, number>;
    body_sources?: Record<string, number>;
    bounding_box_bodies?: number;
  };
}

interface Measurement {
  model: string;
  release: number;
  fileBytes: number;
  savedMeshes: boolean;
  wallMs: number;
  loadMs: number;
  openMs: number;
  workerMs: number;
  wasmMemoryBytes: number;
  mainHeapBytes: number | null;
  buildingElements: number;
  doors: number;
  windows: number;
  savedMeshBodies: number;
  boundingBoxBodies: number;
}

interface Report {
  generatedAt: string;
  browser: string;
  cpus: number;
  memoryBytes: number;
  measurements: Measurement[];
}

async function waitLoaded(page: Page, model: Model): Promise<void> {
  await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: model.loadTimeout });
  await expect(page.locator('#status')).not.toContainText(/error|unreachable|failed/i);
  await expect(page.locator('#load-overlay')).toBeHidden();
}

/** Reads what the viewer shows about the model now open. */
async function read(
  page: Page,
  model: Model,
  savedMeshes: boolean,
  wallMs: number,
): Promise<Measurement> {
  const stats = JSON.parse(
    (await page.locator('#status').getAttribute('data-load-stats')) ?? 'null',
  ) as {
    loadMs: number;
    openMs: number;
    workerMs: number;
    wasmMemoryBytes: number;
  } | null;
  expect(stats, 'the viewer reports what an open cost').not.toBeNull();
  const diagnostics = JSON.parse(
    (await page.locator('#diagnostics-json').textContent()) ?? 'null',
  ) as Diagnostics;
  expect(diagnostics.input.revit_version).toBe(model.release);
  const mainHeapBytes = await page.evaluate(
    () => (performance as unknown as { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize ?? null,
  );
  const { exported } = diagnostics;
  return {
    model: model.file,
    release: model.release,
    fileBytes: fs.statSync(path.join(corpus, model.file)).size,
    savedMeshes,
    wallMs,
    loadMs: stats!.loadMs,
    openMs: stats!.openMs,
    workerMs: stats!.workerMs,
    wasmMemoryBytes: stats!.wasmMemoryBytes,
    mainHeapBytes,
    buildingElements: exported.building_elements,
    doors: exported.by_ifc_type.IFCDOOR ?? 0,
    windows: exported.by_ifc_type.IFCWINDOW ?? 0,
    savedMeshBodies: exported.body_sources?.saved_graphics_mesh ?? 0,
    boundingBoxBodies: exported.bounding_box_bodies ?? 0,
  };
}

function loadReport(): Report | null {
  try {
    return JSON.parse(fs.readFileSync(outFile, 'utf8')) as Report;
  } catch {
    return null;
  }
}

// Written after every open, so a failure later in the run, which also ends
// the worker's copy of this module, leaves the earlier results.
async function record(page: Page, measurement: Measurement): Promise<void> {
  const report: Report = loadReport() ?? {
    generatedAt: '',
    browser: '',
    cpus: os.cpus().length,
    memoryBytes: os.totalmem(),
    measurements: [],
  };
  report.generatedAt = new Date().toISOString();
  report.browser = `Chromium ${page.context().browser()?.version() ?? 'unknown'}`;
  report.measurements = report.measurements
    .filter((m) => !(m.model === measurement.model && m.savedMeshes === measurement.savedMeshes))
    .concat(measurement);
  fs.mkdirSync(path.dirname(outFile), { recursive: true });
  fs.writeFileSync(outFile, `${JSON.stringify(report, null, 2)}\n`);
}

const mib = (bytes: number | null): string =>
  bytes === null ? 'n/a' : (bytes / 2 ** 20).toFixed(0);
const seconds = (ms: number): string => (ms / 1000).toFixed(1);

function tableRows(report: Report): string[][] {
  const order = new Map(models.map((m, i) => [m.file, i]));
  const rows = [...report.measurements]
    .sort(
      (a, b) =>
        (order.get(a.model) ?? 0) - (order.get(b.model) ?? 0) ||
        Number(a.savedMeshes) - Number(b.savedMeshes),
    )
    .map((m) => [
      m.model,
      String(m.release),
      mib(m.fileBytes),
      m.savedMeshes ? 'on' : 'off',
      seconds(m.loadMs),
      seconds(m.openMs),
      mib(m.wasmMemoryBytes),
      mib(m.mainHeapBytes),
      `${m.savedMeshBodies} of ${m.doors + m.windows}`,
      String(m.boundingBoxBodies),
    ]);
  return [
    [
      'Model',
      'Revit',
      'File MiB',
      'Saved meshes',
      'Load s',
      'Open s',
      'Wasm MiB',
      'Page heap MiB',
      'Doors and windows from meshes',
      'Box bodies',
    ],
    ...rows,
  ];
}

function plainTable(rows: string[][]): string {
  const widths = rows[0].map((_, c) => Math.max(...rows.map((r) => r[c].length)));
  return rows
    .map((r) => r.map((cell, c) => (c < 2 || c === 3 ? cell.padEnd(widths[c]) : cell.padStart(widths[c]))).join('  '))
    .join('\n');
}

function markdownTable(rows: string[][]): string {
  const line = (r: string[]) => `| ${r.join(' | ')} |`;
  return [line(rows[0]), line(rows[0].map(() => '---')), ...rows.slice(1).map(line)].join('\n');
}

test.afterAll(() => {
  const report = loadReport();
  if (!report) return;
  const rows = tableRows(report);
  console.log(
    `\nSaved meshes on and off, one open each (${report.browser}, ${report.cpus} CPUs, ${mib(report.memoryBytes)} MiB)\n${plainTable(rows)}\n`,
  );
  if (process.env.GITHUB_STEP_SUMMARY) {
    fs.appendFileSync(
      process.env.GITHUB_STEP_SUMMARY,
      `### Saved meshes on and off, one open each\n\n${report.browser}, ${report.cpus} CPUs, ${mib(report.memoryBytes)} MiB\n\n${markdownTable(rows)}\n`,
    );
  }
});

for (const [index, model] of models.entries()) {
  const source = path.join(corpus, model.file);
  const modelTest = required || fs.existsSync(source) ? test : test.skip;

  modelTest(`${model.file} opens with and without saved meshes`, async ({ browser }) => {
    test.setTimeout(900_000);
    expect(fs.existsSync(source), `${source} was not fetched`).toBe(true);

    // Each open in its own context: a new worker, wasm instance and HTTP
    // cache. The order alternates so the first open of a browser's life, which
    // pays for its own warm-up, is not always the same setting.
    const runs = new Map<boolean, Measurement>();
    for (const savedMeshes of index % 2 === 0 ? [false, true] : [true, false]) {
      const context = await browser.newContext();
      try {
        const page = await context.newPage();
        await page.goto(savedMeshes ? '/' : '/?savedMeshes=0');
        await expect(page.locator('#status')).toHaveText(/Ready/);
        // ?savedMeshes=0 starts the toggle off.
        await expect(page.locator('#saved-meshes')).toBeChecked({ checked: savedMeshes });

        const started = Date.now();
        await page.locator('#file-input').setInputFiles(source);
        await waitLoaded(page, model);
        const measurement = await read(page, model, savedMeshes, Date.now() - started);
        await record(page, measurement);
        runs.set(savedMeshes, measurement);
      } finally {
        await context.close();
      }
    }

    const off = runs.get(false)!;
    const on = runs.get(true)!;
    // Off keeps every box.
    expect(off.savedMeshBodies).toBe(0);
    // On draws doors and windows only, and replaces boxes one for one: the
    // elements are the same, and the boxes that went are the meshes that came.
    expect(on.savedMeshBodies).toBeLessThanOrEqual(on.doors + on.windows);
    expect(on.buildingElements).toBe(off.buildingElements);
    expect(off.boundingBoxBodies - on.boundingBoxBodies).toBe(on.savedMeshBodies);
    if (model.drawsSavedMeshes) expect(on.savedMeshBodies).toBeGreaterThan(0);
  });
}

const coreInterior = models[0];
const coreInteriorSource = path.join(corpus, coreInterior.file);
const toggleTest = required || fs.existsSync(coreInteriorSource) ? test : test.skip;

toggleTest('the toggle reopens the open file with the option changed', async ({ page }) => {
  test.setTimeout(900_000);
  expect(fs.existsSync(coreInteriorSource), `${coreInteriorSource} was not fetched`).toBe(true);

  await page.goto('/');
  await expect(page.locator('#status')).toHaveText(/Ready/);
  const toggle = page.locator('#saved-meshes');
  await expect(toggle).toBeChecked();
  await expect(toggle).toBeEnabled();

  // Turning it before a file is open only sets the option for the next one.
  await toggle.uncheck();
  await toggle.check();
  await expect(page.locator('#status')).toHaveText(/Ready/);

  await page.locator('#file-input').setInputFiles(coreInteriorSource);
  await waitLoaded(page, coreInterior);
  const withMeshes = await read(page, coreInterior, true, 0);
  expect(withMeshes.savedMeshBodies).toBeGreaterThan(0);

  // Off reopens the same file with its boxes.
  await toggle.uncheck();
  await expect(toggle).toBeDisabled();
  await expect(page.locator('#load-overlay')).toBeVisible();
  await waitLoaded(page, coreInterior);
  await expect(toggle).toBeEnabled();
  await expect(toggle).not.toBeChecked();
  await expect(page.locator('#file-meta')).toContainText(coreInterior.file);
  const withoutMeshes = await read(page, coreInterior, false, 0);
  expect(withoutMeshes.savedMeshBodies).toBe(0);
  expect(withoutMeshes.boundingBoxBodies - withMeshes.boundingBoxBodies).toBe(
    withMeshes.savedMeshBodies,
  );

  // On again draws the same meshes.
  await toggle.check();
  await waitLoaded(page, coreInterior);
  await expect(toggle).toBeChecked();
  const again = await read(page, coreInterior, true, 0);
  expect(again.savedMeshBodies).toBe(withMeshes.savedMeshBodies);
  await expect(
    page.locator('#status-panel .status-row', {
      has: page.locator('.status-label', { hasText: /^Shapes$/ }),
    }),
  ).toContainText(`${again.savedMeshBodies} from the meshes Revit saved`);
});
