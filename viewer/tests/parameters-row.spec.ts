import { chromium, expect, test, type Page } from '@playwright/test';
import { execFileSync, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const coreInteriorPath = path.resolve(__dirname, '../public/demos/2024_Core_Interior.rvt');
const coreInteriorTest = fs.existsSync(coreInteriorPath) ? test : test.skip;
// RE1 Mechanical (Drshelden/IFC-ECS, MIT), which viewer-test.yml fetches beside
// the magnetar models.
const re1MechanicalPath = path.resolve(__dirname, '../../_project_corpus/Revit/RE1-Mechanical.rvt');

// B49 (#35): Core Interior's export writes Revit's parameter values into its
// Pset_ property sets, so the status panel's Parameters row reports how many
// instead of saying the parameter table is not read.
coreInteriorTest(
  'the Parameters row counts the Revit parameter values Core Interior exports',
  async ({ page }) => {
    test.slow();
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/Ready/);
    await page.locator('[data-demo-id="core-interior-2024"]').click();
    await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: 300_000 });

    const row = page.locator('#status-panel .status-row', {
      has: page.locator('.status-label', { hasText: /^Parameters$/ }),
    });
    await expect(row).toHaveCount(1);
    await expect(row.locator('.status-value')).toHaveText(/^[1-9][\d,]* /);
    await expect(row.locator('.status-value')).not.toContainText('not read yet');
    // RE-207 reads the rest of each element's parameters, on selection.
    await expect(row.locator('.status-value')).not.toContainText('still to be decoded');
    await expect(row.locator('.status-value')).toContainText('select an element to read its saved parameters');
  },
);

// #35: the info panel lists an element's saved parameters (RE-207), read when it
// is selected by a worker of its own (parameters-worker.ts). The sample is the
// file the suite already opens, Einhoven, and the oracle is the command line:
// viewer-test.yml builds rvt-elements beside gen-fixture, the test runs it on
// the same file, and the entries the panel holds for an element must be the
// entries it prints for that ElementId, whole.

const sample = resolveSamplePath();
const required = process.env.RVT_VIEWER_REQUIRE_JOURNEYS === '1';
// The oracle is the command line: viewer-test.yml builds rvt-elements and puts it on PATH. A
// local run without it skips these (a local run builds nothing, see viewer/README.md); CI fails.
const cliOnPath = spawnSync('rvt-elements', ['--help']).error === undefined;
const sampleTest = required || (sample !== null && cliOnPath) ? test : test.skip;

/** One saved parameter, as rvt-elements prints it. */
interface Parameter {
  name: string | null;
  parameter_id: number;
  kind: string;
  storage_type: string;
  value: unknown;
  si_value: number | null;
  si_unit: string | null;
  level: 'instance' | 'type';
}

interface CliElement {
  id: number | null;
  class_name: string;
  parameters: Parameter[];
}

/** What the panel's parameters group holds. */
interface Shown {
  state: string;
  elementId: number | null;
  readMs: number | null;
  wasmMemoryBytes: number | null;
  cacheElements: number | null;
  cacheCharacters: number | null;
  rows: Array<{ parameter: Parameter; key: string; text: string; inFold: boolean }>;
  headings: string[];
  text: string;
}

function resolveSamplePath(): string | null {
  const candidates = [
    process.env.RVT_VIEWER_SAMPLE,
    path.resolve(__dirname, '../../_project_corpus/Revit/Revit_IFC5_Einhoven.rvt'),
  ].filter((candidate): candidate is string => Boolean(candidate));
  for (const candidate of candidates) {
    const resolved = path.isAbsolute(candidate)
      ? candidate
      : path.resolve(process.cwd(), candidate);
    if (fs.existsSync(resolved)) return resolved;
  }
  return null;
}

/** `rvt-elements FILE`: what the command line prints, by ElementId. */
function rvtElements(file: string): Map<number, Parameter[]> {
  let stdout: string;
  try {
    stdout = execFileSync('rvt-elements', [file, '--no-typed'], {
      encoding: 'utf8',
      maxBuffer: 1024 * 1024 * 1024,
    });
  } catch (err) {
    throw new Error(
      `rvt-elements could not be run (viewer-test.yml builds it beside gen-fixture and puts it on PATH): ${(err as Error).message}`,
    );
  }
  const byId = new Map<number, Parameter[]>();
  for (const element of JSON.parse(stdout) as CliElement[]) {
    if (element.id !== null && !byId.has(element.id)) byId.set(element.id, element.parameters);
  }
  return byId;
}

let einhovenOracle: Map<number, Parameter[]> | undefined;
function oracle(): Map<number, Parameter[]> {
  einhovenOracle ??= rvtElements(sample!);
  return einhovenOracle;
}

async function open(page: Page, file: string, query = ''): Promise<void> {
  await page.goto(`/${query}`);
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await page.locator('#file-input').setInputFiles(file);
  await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: 300_000 });
}

/** The tree's elements: entity index, the ElementId its name ends in (if it does) and its IFC type. */
async function treeElements(
  page: Page,
): Promise<Array<{ index: number; id: number | null; type: string }>> {
  const rows = await page
    .locator('.tree-node.tree-element[data-entity-index]')
    .evaluateAll((nodes) =>
      nodes.map((node) => ({
        index: Number((node as HTMLElement).dataset.entityIndex),
        title: (node as HTMLElement).title,
        type: (node as HTMLElement).dataset.ifcType ?? '',
      })),
    );
  return rows.map(({ index, title, type }) => {
    const name = title.split(' · ')[0] ?? '';
    const id = /[:-](\d+)(?::\d+)?$/.exec(name)?.[1];
    return { index, id: id === undefined ? null : Number(id), type };
  });
}

/** Selects an entity as a click on its tree row does, whether or not the row is open. */
async function select(page: Page, index: number): Promise<void> {
  await page
    .locator(`.tree-node.tree-element[data-entity-index="${index}"]`)
    .evaluate((row) => (row as HTMLElement).click());
}

/** The group once its read has ended, in whatever state. */
async function settled(page: Page): Promise<Shown> {
  const box = page.locator('#info-parameters');
  await expect(box).toHaveAttribute(
    'data-state',
    /^(ready|empty|unsupported|error|on-request|no-id)$/,
    { timeout: 180_000 },
  );
  return box.evaluate((el) => {
    const group = el as HTMLElement;
    const number = (value: string | undefined) => (value === undefined ? null : Number(value));
    return {
      state: group.dataset.state ?? '',
      elementId: number(group.dataset.elementId),
      readMs: number(group.dataset.readMs),
      wasmMemoryBytes: number(group.dataset.wasmMemoryBytes),
      cacheElements: number(group.dataset.cacheElements),
      cacheCharacters: number(group.dataset.cacheCharacters),
      rows: [...group.querySelectorAll<HTMLElement>('[data-parameter]')].map((row) => ({
        parameter: JSON.parse(row.dataset.parameter ?? 'null') as Parameter,
        key: row.querySelector('.k .info-parameter-name')?.textContent ?? '',
        text: row.querySelector('.v')?.textContent ?? '',
        inFold: row.closest('.info-parameters-unnamed') !== null,
      })),
      headings: [...group.querySelectorAll('.info-parameters-subtitle')].map(
        (heading) => heading.textContent ?? '',
      ),
      text: group.textContent ?? '',
    };
  });
}

/** A value with the keys of every object in one order, so JSON written in two orders compares equal. */
function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
        .map(([key, item]) => [key, canonical(item)]),
    );
  }
  return value;
}

/** What differs between the panel's rows and the command line's entries for an element. */
function differences(id: number, shown: Shown, expected: Parameter[]): string[] {
  const out: string[] = [];
  const order = (list: Parameter[]) => list.map((p) => JSON.stringify(canonical(p))).sort();
  const fromPanel = shown.rows.map((row) => row.parameter);
  if (JSON.stringify(order(fromPanel)) !== JSON.stringify(order(expected))) {
    const panel = new Set(order(fromPanel));
    const cli = new Set(order(expected));
    out.push(
      `${id}: panel ${fromPanel.length} entries, rvt-elements ${expected.length}; only in the panel: ${[...panel].filter((p) => !cli.has(p)).slice(0, 3)}; only in rvt-elements: ${[...cli].filter((p) => !panel.has(p)).slice(0, 3)}`,
    );
  }
  for (const level of ['instance', 'type'] as const) {
    const count = expected.filter((p) => p.level === level).length;
    const heading = shown.headings.find((h) => h.startsWith(level === 'instance' ? 'Instance' : 'Type'));
    if (count === 0 && heading !== undefined) out.push(`${id}: a ${level} heading with no ${level} parameter`);
    if (count > 0 && heading !== `${level === 'instance' ? 'Instance' : 'Type'} parameters · ${count}`) {
      out.push(`${id}: ${level} heading is ${JSON.stringify(heading)}, rvt-elements has ${count}`);
    }
  }
  for (const row of shown.rows) {
    const p = row.parameter;
    const named = p.name !== null && p.name !== '';
    const key = named ? p.name : `${p.kind === 'built_in' ? 'Built-in' : 'Parameter'} ${p.parameter_id}`;
    if (row.key !== key) out.push(`${id}: ${p.parameter_id} is labelled ${JSON.stringify(row.key)}, not ${JSON.stringify(key)}`);
    if (row.inFold !== !named) out.push(`${id}: ${p.parameter_id} is ${row.inFold ? '' : 'not '}behind the fold`);
    if (typeof p.value === 'string' && p.value !== '' && row.text !== p.value) {
      out.push(`${id}: ${key} shows ${JSON.stringify(row.text)}, the value is ${JSON.stringify(p.value)}`);
    }
    if (typeof p.value === 'boolean' && row.text !== (p.value ? 'Yes' : 'No')) {
      out.push(`${id}: ${key} shows ${JSON.stringify(row.text)}, the value is ${p.value}`);
    }
    const si = p.si_value?.toLocaleString('en-US', { maximumFractionDigits: 6, useGrouping: false });
    if (p.si_unit !== null && si !== undefined && !row.text.includes(si)) {
      out.push(`${id}: ${key} shows ${JSON.stringify(row.text)} without its SI value ${p.si_value} ${p.si_unit}`);
    }
    if (row.text === '') out.push(`${id}: ${key} shows nothing`);
  }
  return out;
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.floor(sorted.length / 2)] ?? 0;
}

/** Writes figures where the run's summary and log show them. */
function report(lines: string[]): void {
  const text = lines.join('\n');
  console.log(text);
  if (process.env.GITHUB_STEP_SUMMARY) {
    fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${text}\n\n`);
  }
}

sampleTest(
  'selecting an element lists the saved parameters rvt-elements prints for it',
  async ({ page }) => {
    test.setTimeout(600_000);
    const expected = oracle();
    await open(page, sample!);
    const loadStats = JSON.parse(
      (await page.locator('#status').getAttribute('data-load-stats')) ?? 'null',
    ) as { openMs: number; loadMs: number; wasmMemoryBytes: number };

    const all = await treeElements(page);
    expect(all.length, 'the sample has elements in its tree').toBeGreaterThan(0);
    // Every element of a small model; an even sample of a large one.
    const step = Math.max(1, Math.ceil(all.length / 60));
    const chosen = all.filter((_, i) => i % step === 0);

    const found: string[] = [];
    const census = { ready: 0, empty: 0, other: 0, notInCli: 0, entries: 0, instance: 0, type: 0, named: 0 };
    const readMs: number[] = [];
    const wasmMemory: number[] = [];
    for (const { index } of chosen) {
      await select(page, index);
      const shown = await settled(page);
      if (shown.elementId === null) {
        // An entity whose Tag is no ElementId says so, and is not looked up.
        census.other += 1;
        if (shown.state !== 'no-id') found.push(`entity ${index}: state ${shown.state} with no ElementId`);
        continue;
      }
      const cli = expected.get(shown.elementId);
      if (cli === undefined) {
        // Not an element rvt-elements lists: nothing to hold the panel to.
        census.notInCli += 1;
        continue;
      }
      if (shown.readMs !== null) readMs.push(shown.readMs);
      if (shown.wasmMemoryBytes !== null) wasmMemory.push(shown.wasmMemoryBytes);
      if (cli.length === 0) {
        census.empty += 1;
        if (shown.state !== 'empty' || shown.rows.length > 0) {
          found.push(`${shown.elementId}: rvt-elements prints none; the panel is ${shown.state} with ${shown.rows.length} rows`);
        }
        continue;
      }
      census.ready += 1;
      if (shown.state !== 'ready') {
        found.push(`${shown.elementId}: rvt-elements prints ${cli.length}; the panel is ${shown.state}: ${shown.text.slice(0, 200)}`);
        continue;
      }
      census.entries += cli.length;
      census.instance += cli.filter((p) => p.level === 'instance').length;
      census.type += cli.filter((p) => p.level === 'type').length;
      census.named += cli.filter((p) => p.name !== null && p.name !== '').length;
      found.push(...differences(shown.elementId, shown, cli));
    }

    report([
      `Saved parameters in the panel, ${path.basename(sample!)}: ${chosen.length} of ${all.length} tree elements selected; ` +
        `${census.ready} hold parameters (${census.entries} entries: ${census.instance} instance, ${census.type} type, ${census.named} named), ` +
        `${census.empty} hold none, ${census.notInCli} are not in rvt-elements' list, ${census.other} have no ElementId.`,
      `Read cost (a worker of its own, per element, the file's bytes and the wasm's start included): median ${Math.round(median(readMs))} ms, ` +
        `most ${Math.round(Math.max(0, ...readMs))} ms; that instance's wasm memory at the end: median ${Math.round(median(wasmMemory) / 2 ** 20)} MiB, ` +
        `most ${Math.round(Math.max(0, ...wasmMemory) / 2 ** 20)} MiB. The open's worker, for the same file: ${Math.round(loadStats.wasmMemoryBytes / 2 ** 20)} MiB of wasm memory, ` +
        `${Math.round(loadStats.openMs)} ms in the open call, ${Math.round(loadStats.loadMs)} ms to Loaded.`,
    ]);
    expect(found, 'the panel against rvt-elements').toEqual([]);
    // Not vacuous: elements with parameters were compared. The empty state has
    // a test of its own, which looks for an element to show it on.
    expect(census.ready, 'elements holding parameters that were compared').toBeGreaterThan(0);
  },
);

sampleTest(
  'an element rvt-elements prints no parameters for shows the empty state',
  async ({ page }) => {
    test.setTimeout(900_000);
    const none = async (expected: Map<number, Parameter[]>) =>
      (await treeElements(page)).find(({ id }) => id !== null && expected.get(id)?.length === 0);
    // The sample first, then the reference models this job holds, until one
    // has such an element. Einhoven had one in its ArcWall matches, which were
    // its curtain wall's panels and grid lines (RE-440) and are no longer
    // written; every element its export writes now has parameters, and so has
    // every element of Core Interior's. RE1 Mechanical has 25 (v0.22.0's
    // rvt-elements against its rvt-ifc export, 2026-11-13).
    await open(page, sample!);
    let from = path.basename(sample!);
    let found = await none(oracle());
    for (const fallback of [coreInteriorPath, re1MechanicalPath]) {
      if (found !== undefined || !fs.existsSync(fallback)) continue;
      await open(page, fallback);
      from = path.basename(fallback);
      found = await none(rvtElements(fallback));
    }
    expect(found, 'an element of the tree that rvt-elements prints no parameters for').toBeDefined();

    await select(page, found!.index);
    const shown = await settled(page);
    report([`The empty state, on element ${found!.id} of ${from}: ${shown.state}: ${shown.text}`]);
    expect(shown.state).toBe('empty');
    expect(shown.elementId).toBe(found!.id);
    expect(shown.rows).toHaveLength(0);
    expect(shown.text).toMatch(/no saved parameters|no record of this element/);
  },
);

/** The tree elements of the sample that rvt-elements lists parameters for, one for each ElementId. */
async function withParameters(
  page: Page,
  expected: Map<number, Parameter[]>,
  count: number,
): Promise<Array<{ index: number; id: number }>> {
  const seen = new Set<number>();
  const out: Array<{ index: number; id: number }> = [];
  for (const { index, id } of await treeElements(page)) {
    if (id === null || (expected.get(id)?.length ?? 0) === 0 || seen.has(id)) continue;
    seen.add(id);
    out.push({ index, id });
    if (out.length === count) break;
  }
  expect(out.length, `elements rvt-elements lists parameters for`).toBe(count);
  return out;
}

const MiB = 2 ** 20;
async function openedWasmBytes(page: Page): Promise<number> {
  const stats = JSON.parse(
    (await page.locator('#status').getAttribute('data-load-stats')) ?? 'null',
  ) as { wasmMemoryBytes: number };
  return stats.wasmMemoryBytes;
}

// The page reads on selection while the wasm memory it holds is within what a read
// has been measured beside (1 GiB, a quarter of the 4 GiB ceiling the page judges an
// open by), and asks first above that. The open's memory is measured here and the
// ceiling set so that the limit is a little either side of it, so the rule is held at
// its edge, not at the extremes of the default.
sampleTest(
  'a read is made on selection within what has been measured, and asked about above it',
  async ({ page }) => {
    test.setTimeout(300_000);
    const expected = oracle();
    await open(page, sample!);
    const opened = await openedWasmBytes(page);
    const [pick] = await withParameters(page, expected, 1);

    // The default ceiling, 4 GiB: read on selection, no button.
    await select(page, pick!.index);
    const onTheDefault = await settled(page);
    expect(onTheDefault.state).toBe('ready');
    await expect(page.locator('#read-saved-parameters')).toHaveCount(0);

    // A limit (a quarter of the ceiling) 8 MiB over the open's memory: still read on selection.
    await open(page, sample!, `?memoryCeilingBytes=${4 * opened + 32 * MiB}`);
    expect(Math.abs((await openedWasmBytes(page)) - opened), 'the open held what it held').toBeLessThanOrEqual(2 * MiB);
    await select(page, pick!.index);
    const over = await settled(page);
    expect(over.state).toBe('ready');
    await expect(page.locator('#read-saved-parameters')).toHaveCount(0);

    // A limit 8 MiB under it: asked first, and then read as the command line has it.
    await open(page, sample!, `?memoryCeilingBytes=${4 * opened - 32 * MiB}`);
    expect(Math.abs((await openedWasmBytes(page)) - opened), 'the open held what it held').toBeLessThanOrEqual(2 * MiB);
    await select(page, pick!.index);
    const asked = await settled(page);
    expect(asked.state).toBe('on-request');
    expect(asked.rows).toHaveLength(0);
    await expect(page.locator('#info-parameters')).toContainText('as much memory as opening it did');
    await expect(page.locator('#info-parameters')).toContainText('clicking may close this tab');
    await page.locator('#read-saved-parameters').click();
    const read = await settled(page);
    expect(read.state).toBe('ready');
    expect(differences(read.elementId!, read, expected.get(read.elementId!) ?? [])).toEqual([]);
    // The button was removed by the click: focus is where the result is.
    await expect(page.locator('#info-parameters .info-parameters-body')).toBeFocused();
  },
);

// An element selected again is not read again: the page keeps each answer for
// the open file, a bounded few, and starts afresh when a file is opened.
sampleTest(
  'an element selected again is not read again, and another file opened starts a new map',
  async ({ page }, testInfo) => {
    test.setTimeout(300_000);
    const expected = oracle();
    await watchReadWorkers(page);

    await open(page, sample!);
    const [a, b] = await withParameters(page, expected, 2);
    await select(page, a!.index);
    const firstA = await settled(page);
    expect(firstA.elementId).toBe(a!.id);
    expect(await reads(page)).toBe(1);
    await select(page, b!.index);
    await settled(page);
    expect(await reads(page)).toBe(2);

    // Both again: from the map, with the answer they had.
    await select(page, a!.index);
    const againA = await settled(page);
    await select(page, b!.index);
    const againB = await settled(page);
    expect(await reads(page), 'selecting two elements again read the file').toBe(2);
    expect(againA.readMs, 'the answer kept is the one read').toBe(firstA.readMs);
    expect(againA.rows.map((row) => row.parameter)).toEqual(firstA.rows.map((row) => row.parameter));
    expect(differences(a!.id, againA, expected.get(a!.id)!)).toEqual([]);
    expect(differences(b!.id, againB, expected.get(b!.id)!)).toEqual([]);
    expect(againB.cacheElements).toBe(2);
    expect(againB.cacheCharacters!).toBeGreaterThan(0);

    // Another file opened (a copy under another name: the browser raises no
    // change when the file input is given the file it already holds): nothing
    // is carried over.
    const before = await page.locator('#status').getAttribute('data-load-stats');
    const copy = testInfo.outputPath('Einhoven-copy.rvt');
    fs.copyFileSync(sample!, copy);
    await page.locator('#file-input').setInputFiles(copy);
    // The open's figures are removed when it starts and written when it ends.
    await expect
      .poll(
        async () => {
          const now = await page.locator('#status').getAttribute('data-load-stats');
          return now !== null && now !== before;
        },
        { message: 'the file was opened again', timeout: 300_000 },
      )
      .toBe(true);
    await expect(page.locator('#status')).toHaveText(/Loaded/);
    await select(page, a!.index);
    const afresh = await settled(page);
    expect(await reads(page), 'another file opened kept an answer').toBe(3);
    expect(afresh.cacheElements).toBe(1);
    expect(differences(a!.id, afresh, expected.get(a!.id)!)).toEqual([]);
    fs.rmSync(copy, { force: true });
  },
);

// The map is bounded: with room for two, a third element's answer pushes out
// the least recently selected, which is then read again, and the other is not.
sampleTest('the answers kept are bounded, the least recently selected going first', async ({ page }) => {
  test.setTimeout(300_000);
  const expected = oracle();
  await watchReadWorkers(page);
  await open(page, sample!, '?parametersRemembered=2');
  const [a, b, c] = await withParameters(page, expected, 3);
  for (const element of [a!, b!, c!]) {
    await select(page, element.index);
    await settled(page);
  }
  expect(await reads(page)).toBe(3);
  await select(page, c!.index);
  const keptC = await settled(page);
  expect(await reads(page), 'the newest answer was kept').toBe(3);
  expect(keptC.cacheElements, 'room for two').toBe(2);
  await select(page, a!.index);
  const evictedA = await settled(page);
  expect(await reads(page), 'the oldest answer was pushed out and read again').toBe(4);
  expect(differences(a!.id, evictedA, expected.get(a!.id)!)).toEqual([]);
});

// The map is bounded in characters too (#1887): with a budget no answer fits in,
// only the answer just read is kept. It is not read again when it is selected
// once more, and the one before it, pushed out by the budget, is.
sampleTest('the answers kept are bounded in characters, the answer just read always staying', async ({ page }) => {
  test.setTimeout(300_000);
  const expected = oracle();
  await watchReadWorkers(page);
  await open(page, sample!, '?parametersRememberedCharacters=1');
  const [a, b] = await withParameters(page, expected, 2);
  await select(page, a!.index);
  await settled(page);
  await select(page, b!.index);
  const keptB = await settled(page);
  expect(await reads(page)).toBe(2);
  expect(keptB.cacheElements, 'only the answer just read fits the budget').toBe(1);
  await select(page, b!.index);
  await settled(page);
  expect(await reads(page), 'the answer just read was kept').toBe(2);
  await select(page, a!.index);
  const evictedA = await settled(page);
  expect(await reads(page), 'the older answer was pushed out by the budget and read again').toBe(3);
  expect(differences(a!.id, evictedA, expected.get(a!.id)!)).toEqual([]);
});

/**
 * Shapes the first `times` workers that read saved parameters. `fail` makes `savedParameters`
 * fail the way worker-failures.spec.ts fails the open's: a panic message on console.error before
 * the trap, or (`panic` null) a bare `unreachable` with none, which is how V8 words a failed
 * allocation. `delayMs` holds the file back that long, so that a read is still running when the
 * test acts. The workers are bundled into the page and start from blob URLs, so the shaping is
 * put in front of a blob's script as the page makes its URL; the open's worker is the first the
 * page starts, so `skip` (1 unless given) leaves it as shipped.
 */
async function shapeReads(
  page: Page,
  shape: { fail?: { panic: string | null }; delayMs?: number; skip?: number; times: number },
): Promise<void> {
  await page.addInitScript((shape) => {
    const prelude = `(() => {
      const fail = ${JSON.stringify(shape.fail ?? null)};
      const delayMs = ${JSON.stringify(shape.delayMs ?? 0)};
      if (delayMs > 0) {
        const real = Blob.prototype.arrayBuffer;
        Blob.prototype.arrayBuffer = async function () {
          await new Promise((resolve) => setTimeout(resolve, delayMs));
          return real.call(this);
        };
      }
      if (fail !== null) {
        const failing = () => {
          if (fail.panic !== null) console.error(fail.panic);
          throw new WebAssembly.RuntimeError('unreachable');
        };
        const exportsOf = Object.getOwnPropertyDescriptor(WebAssembly.Instance.prototype, 'exports').get;
        const Real = WebAssembly.Instance;
        WebAssembly.Instance = class extends Real {
          constructor(module, imports) {
            super(module, imports);
            Object.defineProperty(this, 'exports', {
              value: { ...exportsOf.call(this), savedParameters: failing },
            });
          }
        };
      }
    })();\n`;
    const skip = shape.skip ?? 1;
    let seen = 0;
    let used = 0;
    const real = URL.createObjectURL.bind(URL);
    URL.createObjectURL = (object: Blob | MediaSource) => {
      if (object instanceof Blob && /javascript/.test(object.type)) {
        seen += 1;
        if (seen > skip && used < shape.times) {
          used += 1;
          return real(new Blob([prelude, object], { type: object.type }));
        }
      }
      return real(object);
    };
  }, shape);
}

/** Counts the workers the page starts to read saved parameters, and keeps them, and notes when each has started. */
async function watchReadWorkers(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const seen = window as unknown as {
      __parameterReads: number;
      __parameterStarted: number;
      __parameterWorkers: Worker[];
    };
    seen.__parameterReads = 0;
    seen.__parameterStarted = 0;
    seen.__parameterWorkers = [];
    const RealWorker = window.Worker;
    window.Worker = class extends RealWorker {
      constructor(url: string | URL, options?: WorkerOptions) {
        super(url, options);
        if (options?.name !== 'saved-parameters') return;
        seen.__parameterReads += 1;
        seen.__parameterWorkers.push(this);
        this.addEventListener('message', (ev: MessageEvent) => {
          if ((ev.data as { type?: string } | null)?.type === 'started') seen.__parameterStarted += 1;
        });
      }
    } as typeof Worker;
  });
}

const reads = (page: Page) =>
  page.evaluate(() => (window as unknown as { __parameterReads: number }).__parameterReads);

sampleTest('a read that fails says so, and Try again reads the element', async ({ page }) => {
  test.setTimeout(300_000);
  const expected = oracle();
  await shapeReads(page, {
    fail: {
      panic:
        'rvt-rs wasm panic: panicked at rvt-wasm/src/lib.rs:1:1:\nindex out of bounds: the len is 3 but the index is 7',
    },
    times: 1,
  });
  await open(page, sample!);
  const [pick] = await withParameters(page, expected, 1);

  await select(page, pick!.index);
  const failed = await settled(page);
  expect(failed.state).toBe('error');
  // The panic's own words, and not a claim about memory.
  expect(failed.text).toContain('index out of bounds: the len is 3');
  expect(failed.text).not.toMatch(/ran out of memory/i);
  expect(failed.rows).toHaveLength(0);
  // The rest of the panel is where it was.
  await expect(page.locator('#info .info-group-identity')).toContainText('ElementId');

  // Try again: the button is gone when it is pressed, and focus is where the result is.
  await page.locator('#retry-saved-parameters').click();
  const retried = await settled(page);
  expect(retried.state).toBe('ready');
  expect(differences(retried.elementId!, retried, expected.get(retried.elementId!) ?? [])).toEqual([]);
  await expect(page.locator('#info-parameters .info-parameters-body')).toBeFocused();
  // A failure that is not memory does not stop the file being read on selection.
  await expect(page.locator('#read-saved-parameters')).toHaveCount(0);
});

// A bare unreachable is how V8 words a failed allocation: the page says the tab ran out of
// memory reading the file a second time, and from then on reads that file only when asked, so
// that it does not fail on every selection.
sampleTest(
  'a read that runs out of memory says so, and the file is then read only when asked',
  async ({ page }) => {
    test.setTimeout(300_000);
    const expected = oracle();
    await shapeReads(page, { fail: { panic: null }, times: 1 });
    await open(page, sample!);
    const [a, b] = await withParameters(page, expected, 2);

    await select(page, a!.index);
    const failed = await settled(page);
    expect(failed.state).toBe('error');
    expect(failed.text).toContain('ran out of memory reading the file a second time');
    expect(failed.text).toContain('read only when asked from now on');
    expect(failed.text).toContain('may close this tab');
    expect(failed.text).not.toMatch(/unreachable executed|Unreachable code/);

    // Another element: asked about, not read and failed again.
    await select(page, b!.index);
    const asked = await settled(page);
    expect(asked.state).toBe('on-request');
    expect(asked.text).toContain('The last read of this file ran out of memory');
    await page.locator('#read-saved-parameters').click();
    const read = await settled(page);
    expect(read.state).toBe('ready');
    expect(differences(read.elementId!, read, expected.get(read.elementId!) ?? [])).toEqual([]);
    await expect(page.locator('#info-parameters .info-parameters-body')).toBeFocused();
  },
);

// A browser ends a worker that runs out of memory with no event and no message. The page
// finds out from the lock the worker held, so "Reading" cannot show for ever.
sampleTest('a worker that ends without a word is found, and loading does not show for ever', async ({ page }) => {
  test.setTimeout(300_000);
  const expected = oracle();
  // A read of Einhoven takes a quarter of a second, which the test cannot be sure to beat when
  // it ends the worker (run 42122623374 ended a worker that had answered): the file is held
  // back four seconds, so the worker is still reading.
  await shapeReads(page, { delayMs: 4_000, times: 99 });
  await watchReadWorkers(page);
  await open(page, sample!);
  const [a, b] = await withParameters(page, expected, 2);

  await select(page, a!.index);
  await expect
    .poll(() => page.evaluate(() => (window as unknown as { __parameterStarted: number }).__parameterStarted), {
      message: 'the worker started',
    })
    .toBe(1);
  await page.evaluate(() => (window as unknown as { __parameterWorkers: Worker[] }).__parameterWorkers[0]!.terminate());
  const stopped = await settled(page);
  expect(stopped.state).toBe('error');
  expect(stopped.text).toContain('stopped without answering');
  expect(stopped.text).toContain('read only when asked from now on');

  await select(page, b!.index);
  expect((await settled(page)).state).toBe('on-request');
});

// Nothing is requested once the page has loaded and the first file is open, and a page taken
// offline then still does everything: an element's saved parameters are read, an IFC, a plan and
// a glTF export are made (the main thread's wasm starts from the module the open compiled, with
// the glue in the page), and the file is opened again (the saved-mesh toggle reopens it: the
// worker is in the page's bundle and starts from that module). Every request is recorded, the
// workers' included.
sampleTest(
  'selecting, exporting and opening again make no request, and an offline page does them',
  async ({ page }) => {
    test.setTimeout(300_000);
    const expected = oracle();
    await open(page, sample!);
    const requests: string[] = [];
    await page.context().route('**/*', async (route) => {
      requests.push(route.request().url());
      await route.abort('internetdisconnected');
    });
    page.on('request', (request) => requests.push(request.url()));
    await page.context().setOffline(true);

    const picks = await withParameters(page, expected, 3);
    for (const pick of picks) {
      await select(page, pick.index);
      const shown = await settled(page);
      expect(shown.state, `element ${pick.id}: ${shown.text.slice(0, 200)}`).toBe('ready');
      expect(differences(pick.id, shown, expected.get(pick.id)!)).toEqual([]);
    }

    // The exports: the IFC and the plan run wasm on the main thread.
    for (const [selector, extension] of [
      ['#export-ifc', /\.ifc$/],
      ['#export-svg', /\.svg$/],
      ['#export-glb', /\.glb$/],
    ] as const) {
      const [download] = await Promise.all([
        page.waitForEvent('download'),
        page.locator(selector).click(),
      ]);
      expect(download.suggestedFilename(), selector).toMatch(extension);
      expect(fs.statSync((await download.path())!).size, `${selector} wrote nothing`).toBeGreaterThan(0);
    }

    // Opened again: the toggle reopens the open file.
    const before = await page.locator('#status').getAttribute('data-load-stats');
    await page.locator('#saved-meshes').click();
    await expect
      .poll(
        async () => {
          const now = await page.locator('#status').getAttribute('data-load-stats');
          return now !== null && now !== before;
        },
        { message: 'the file was opened again', timeout: 300_000 },
      )
      .toBe(true);
    await expect(page.locator('#status')).toHaveText(/Loaded/);
    await select(page, picks[0]!.index);
    const again = await settled(page);
    expect(again.state).toBe('ready');
    expect(differences(picks[0]!.id, again, expected.get(picks[0]!.id)!)).toEqual([]);

    // A blob: URL is the page's own memory, not the network.
    expect(
      requests.filter((url) => /^https?:/.test(url)),
      'requests made after the first file was open',
    ).toEqual([]);
  },
);

// A read for an element nobody is looking at is ended, and the element whose read is running
// keeps it when selected again. The reads are held back four seconds, so that one is still
// running when the test acts.
sampleTest('a read for an element no longer shown is ended, and the shown one is not started over', async ({ page }) => {
  test.setTimeout(300_000);
  const expected = oracle();
  await shapeReads(page, { delayMs: 4_000, times: 99 });
  await watchReadWorkers(page);
  await open(page, sample!);
  const [a, b, c] = await withParameters(page, expected, 3);

  // The same element twice at once: one read.
  await select(page, a!.index);
  await select(page, a!.index);
  await settled(page);
  expect(await reads(page), 'selecting an element again restarted its read').toBe(1);

  // Another element selected while a read runs ends it: its answer is never kept.
  await select(page, c!.index);
  await expect
    .poll(() => page.evaluate(() => (window as unknown as { __parameterStarted: number }).__parameterStarted), {
      message: 'the second worker started',
    })
    .toBe(2);
  await select(page, b!.index);
  await settled(page);
  const afterB = await reads(page);
  await select(page, c!.index);
  const again = await settled(page);
  expect(await reads(page), 'the ended read was kept, or not ended').toBe(afterB + 1);
  expect(differences(c!.id, again, expected.get(c!.id)!)).toEqual([]);
});

// A container (a storey, a category) shows no parameters, so a read still running is ended.
sampleTest('a read is ended when a storey is selected instead', async ({ page }) => {
  test.setTimeout(300_000);
  const expected = oracle();
  await shapeReads(page, { delayMs: 4_000, times: 99 });
  await watchReadWorkers(page);
  await open(page, sample!);
  const [a] = await withParameters(page, expected, 1);
  await select(page, a!.index);
  await expect
    .poll(() => page.evaluate(() => (window as unknown as { __parameterStarted: number }).__parameterStarted), {
      message: 'the worker started',
    })
    .toBe(1);
  await page.locator('.tree-node.tree-storey').first().evaluate((row) => (row as HTMLElement).click());
  await expect(page.locator('#info-parameters')).toHaveCount(0);
  await select(page, a!.index);
  await settled(page);
  expect(await reads(page), 'the read of an element no longer shown was kept').toBe(2);
});

// A file of a release whose parameters are not read: no such file is in the viewer's job, so the
// worker's answer to the page is replaced with the binding's shape for one. The range in the
// message is the real binding's.
sampleTest(
  'a file of a release whose parameters are not read says so (the answer replaced)',
  async ({ page }) => {
    test.setTimeout(300_000);
    await page.addInitScript(() => {
      const RealWorker = window.Worker;
      window.Worker = class extends RealWorker {
        constructor(url: string | URL, options?: WorkerOptions) {
          super(url, options);
          if (options?.name !== 'saved-parameters') return;
          const add = this.addEventListener.bind(this) as (...args: unknown[]) => void;
          (this as unknown as { addEventListener: unknown }).addEventListener = (
            type: string,
            listener: (ev: Event) => void,
            options?: unknown,
          ) => {
            if (type !== 'message') return add(type, listener, options);
            return add(
              type,
              (ev: Event) => {
                const data = (ev as MessageEvent).data as { type: string; result: object };
                if (data.type !== 'result') return listener(ev);
                const result = {
                  ...data.result,
                  release: 2013,
                  release_read: false,
                  release_experimental: false,
                  elements: {},
                };
                return listener(new MessageEvent('message', { data: { ...data, result } }));
              },
              options,
            );
          };
        }
      } as typeof Worker;
    });
    await open(page, sample!);
    const first = (await treeElements(page)).find(({ id }) => id !== null);
    expect(first, 'an element with an ElementId').toBeDefined();
    await select(page, first!.index);
    const shown = await settled(page);
    expect(shown.state).toBe('unsupported');
    expect(shown.rows).toHaveLength(0);
    expect(shown.text).toMatch(/This is a Revit 2013 file/);
    expect(shown.text).toContain('Revit 2015 to 2027 (2015 to 2022 experimentally)');
  },
);

// Revit 2015 to 2022 are read experimentally (RE-281), and the panel says so from the
// release: the answer is replaced with one for a release of that range.
sampleTest(
  'a release read only experimentally says so beside its parameters (the answer replaced)',
  async ({ page }) => {
    test.setTimeout(300_000);
    const expected = oracle();
    await page.addInitScript(() => {
      const RealWorker = window.Worker;
      window.Worker = class extends RealWorker {
        constructor(url: string | URL, options?: WorkerOptions) {
          super(url, options);
          if (options?.name !== 'saved-parameters') return;
          const add = this.addEventListener.bind(this) as (...args: unknown[]) => void;
          (this as unknown as { addEventListener: unknown }).addEventListener = (
            type: string,
            listener: (ev: Event) => void,
            options?: unknown,
          ) => {
            if (type !== 'message') return add(type, listener, options);
            return add(
              type,
              (ev: Event) => {
                const data = (ev as MessageEvent).data as { type: string; result: object };
                if (data.type !== 'result') return listener(ev);
                const result = { ...data.result, release: 2019, release_experimental: true };
                return listener(new MessageEvent('message', { data: { ...data, result } }));
              },
              options,
            );
          };
        }
      } as typeof Worker;
    });
    await open(page, sample!);
    const [pick] = await withParameters(page, expected, 1);
    await select(page, pick!.index);
    const shown = await settled(page);
    expect(shown.state).toBe('ready');
    expect(shown.text).toContain('This is a Revit 2019 file. Saved parameters are read from files of Revit 2015 to 2022 experimentally');
  },
);

// A read that really runs the tab out of memory. A Chromium whose V8 caps wasm memory
// (memory-ceiling-capped.spec.ts) cannot fail a read of Einhoven that its open survived: a read
// takes less than the open there (17 MiB against 23). On the BIM4LCA office it takes more, so a
// cap between the two opens it and stops the read: the real failure of "a second instance beside
// the open". The page says so, and reads that file only when asked. Both figures are measured
// here in an uncapped browser, since any change to the open moves them, and a run with no room
// between them says NOT MEASURED instead of failing.
const office = process.env.RVT_VIEWER_OFFICE;
const officeTest = office ? test : test.skip;
officeTest(
  'a read that really runs the tab out of memory is reported, and the file is then read only when asked',
  async ({ baseURL }) => {
    test.setTimeout(1_500_000);
    const uncapped = await chromium.launch();
    let openBytes = 0;
    let readBytes = 0;
    try {
      const page = await (await uncapped.newContext({ baseURL })).newPage();
      await page.goto('/');
      await expect(page.locator('#status')).toHaveText(/Ready/);
      await page.locator('#file-input').setInputFiles(office!);
      await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: 300_000 });
      openBytes = (
        JSON.parse((await page.locator('#status').getAttribute('data-load-stats')) ?? '{}') as {
          wasmMemoryBytes: number;
        }
      ).wasmMemoryBytes;
      const [first] = (await treeElements(page)).filter(({ id }) => id !== null);
      await select(page, first!.index);
      const read = (await settled(page)).wasmMemoryBytes;
      if (read === null) {
        report(['NOT MEASURED: the read reported no wasm memory figure.']);
        test.skip(true, 'NOT MEASURED: the read reported no wasm memory figure');
        return;
      }
      readBytes = read;
    } finally {
      await uncapped.close();
    }
    const room = readBytes - openBytes;
    const summary = `the open's peak ${Math.round(openBytes / MiB)} MiB, a read's ${Math.round(readBytes / MiB)} MiB`;
    if (room < 8 * MiB) {
      report([`NOT MEASURED: no wasm cap fits between ${summary}.`]);
      test.skip(true, `NOT MEASURED: ${summary}`);
      return;
    }
    const capPages = Math.ceil((openBytes + room / 2) / (64 * 1024));
    report([`The cap between ${summary}: ${Math.round((capPages * 64 * 1024) / MiB)} MiB.`]);
    const browser = await chromium.launch({ args: [`--js-flags=--wasm-max-mem-pages=${capPages}`] });
    try {
      const context = await browser.newContext({ baseURL });
      const page = await context.newPage();
      await page.goto('/');
      await expect(page.locator('#status')).toHaveText(/Ready/);
      await page.locator('#file-input').setInputFiles(office!);
      await expect(page.locator('#status')).toHaveText(/Loaded|Ran out of memory/, { timeout: 900_000 });
      await expect(page.locator('#status')).toHaveText(/Loaded/);
      const [a, b] = (await treeElements(page)).filter(({ id }) => id !== null).slice(0, 2);
      expect(a && b, 'two elements with an ElementId').toBeTruthy();

      await select(page, a!.index);
      const failed = await settled(page);
      report([`A read under a ${Math.round((capPages * 64 * 1024) / MiB)} MiB wasm cap on the office: ${failed.state}: ${failed.text}`]);
      expect(failed.state).toBe('error');
      expect(failed.text).toMatch(/ran out of memory reading the file a second time|stopped without answering/);

      await select(page, b!.index);
      const asked = await settled(page);
      expect(asked.state).toBe('on-request');
    } finally {
      await browser.close();
    }
  },
);

// The same hold on the largest reference model, a few elements of different
// categories, and what a read costs there.
coreInteriorTest(
  'Core Interior: the saved parameters of elements of several categories are what rvt-elements prints',
  async ({ page }) => {
    test.setTimeout(900_000);
    const expected = rvtElements(coreInteriorPath);
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/Ready/);
    await page.locator('[data-demo-id="core-interior-2024"]').click();
    await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: 300_000 });
    const loadStats = JSON.parse(
      (await page.locator('#status').getAttribute('data-load-stats')) ?? 'null',
    ) as { openMs: number; loadMs: number; wasmMemoryBytes: number };

    const chosen = new Map<string, { index: number; id: number }>();
    for (const { index, id, type } of await treeElements(page)) {
      if (id === null || (expected.get(id)?.length ?? 0) === 0 || chosen.has(type)) continue;
      chosen.set(type, { index, id });
      if (chosen.size === 4) break;
    }
    expect(chosen.size, 'categories with an element rvt-elements lists parameters for').toBeGreaterThan(1);

    const found: string[] = [];
    const lines: string[] = [];
    for (const [type, { index }] of chosen) {
      await select(page, index);
      const shown = await settled(page);
      expect(shown.state, `${type}`).toBe('ready');
      found.push(...differences(shown.elementId!, shown, expected.get(shown.elementId!) ?? []));
      lines.push(
        `${type} ${shown.elementId}: ${shown.rows.length} entries in ${shown.readMs} ms, that read's wasm memory ${Math.round((shown.wasmMemoryBytes ?? 0) / 2 ** 20)} MiB`,
      );
    }
    report([
      `Saved parameters on 2024_Core_Interior.rvt (${Math.round(fs.statSync(coreInteriorPath).size / 2 ** 20)} MiB): ${lines.join('; ')}. ` +
        `The open's worker: ${Math.round(loadStats.wasmMemoryBytes / 2 ** 20)} MiB of wasm memory, ${Math.round(loadStats.openMs)} ms in the open call, ${Math.round(loadStats.loadMs)} ms to Loaded.`,
    ]);
    expect(found, 'the panel against rvt-elements').toEqual([]);
  },
);
