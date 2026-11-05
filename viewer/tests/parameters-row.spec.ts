import { expect, test, type Page } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const coreInteriorPath = path.resolve(__dirname, '../public/demos/2024_Core_Interior.rvt');
const coreInteriorTest = fs.existsSync(coreInteriorPath) ? test : test.skip;

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
const sampleTest = required || sample !== null ? test : test.skip;

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
        key: row.querySelector('.k')?.textContent ?? '',
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
    // The sample first; the largest reference model when none of the sample's
    // elements is one.
    await open(page, sample!);
    let from = path.basename(sample!);
    let found = await none(oracle());
    if (found === undefined && fs.existsSync(coreInteriorPath)) {
      await open(page, coreInteriorPath);
      from = path.basename(coreInteriorPath);
      found = await none(rvtElements(coreInteriorPath));
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

// The page reads on selection while a second instance of the size the open's
// worker held (a read has taken 0.74 to 1.04 times that) still fits the
// ceiling it judges an open by, and asks first above that. The open's memory
// is measured here and the ceiling set a little either side of twice it, so
// the rule is held at its edge, not at the extremes of the default 4 GiB.
sampleTest(
  'a read is made on selection while a second instance of the open fits the ceiling, and asked about when it does not',
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

    // A ceiling a little over twice the open's memory: still read on selection.
    await open(page, sample!, `?memoryCeilingBytes=${2 * opened + 8 * MiB}`);
    expect(Math.abs((await openedWasmBytes(page)) - opened), 'the open held what it held').toBeLessThanOrEqual(2 * MiB);
    await select(page, pick!.index);
    const over = await settled(page);
    expect(over.state).toBe('ready');
    await expect(page.locator('#read-saved-parameters')).toHaveCount(0);

    // A little under twice it: asked first, and then read as the command line has it.
    await open(page, sample!, `?memoryCeilingBytes=${2 * opened - 8 * MiB}`);
    expect(Math.abs((await openedWasmBytes(page)) - opened), 'the open held what it held').toBeLessThanOrEqual(2 * MiB);
    await select(page, pick!.index);
    const asked = await settled(page);
    expect(asked.state).toBe('on-request');
    expect(asked.rows).toHaveLength(0);
    await expect(page.locator('#info-parameters')).toContainText('as much memory as opening it did');
    await page.locator('#read-saved-parameters').click();
    const read = await settled(page);
    expect(read.state).toBe('ready');
    expect(differences(read.elementId!, read, expected.get(read.elementId!) ?? [])).toEqual([]);
  },
);

// An element selected again is not read again: the page keeps each answer for
// the open file, a bounded few, and starts afresh when a file is opened.
sampleTest(
  'an element selected again is not read again, and a file opened again starts a new map',
  async ({ page }) => {
    test.setTimeout(300_000);
    const expected = oracle();
    await page.addInitScript(() => {
      const reads = window as unknown as { __parameterReads: number };
      reads.__parameterReads = 0;
      const RealWorker = window.Worker;
      window.Worker = class extends RealWorker {
        constructor(url: string | URL, options?: WorkerOptions) {
          super(url, options);
          if (String(url).includes('parameters-worker')) reads.__parameterReads += 1;
        }
      } as typeof Worker;
    });
    const reads = () =>
      page.evaluate(() => (window as unknown as { __parameterReads: number }).__parameterReads);

    await open(page, sample!);
    const [a, b] = await withParameters(page, expected, 2);
    await select(page, a!.index);
    const firstA = await settled(page);
    expect(firstA.elementId).toBe(a!.id);
    expect(await reads()).toBe(1);
    await select(page, b!.index);
    await settled(page);
    expect(await reads()).toBe(2);

    // Both again: from the map, with the answer they had.
    await select(page, a!.index);
    const againA = await settled(page);
    await select(page, b!.index);
    const againB = await settled(page);
    expect(await reads(), 'selecting two elements again read the file').toBe(2);
    expect(againA.readMs, 'the answer kept is the one read').toBe(firstA.readMs);
    expect(againA.rows.map((row) => row.parameter)).toEqual(firstA.rows.map((row) => row.parameter));
    expect(differences(a!.id, againA, expected.get(a!.id)!)).toEqual([]);
    expect(differences(b!.id, againB, expected.get(b!.id)!)).toEqual([]);
    expect(againB.cacheElements).toBe(2);
    expect(againB.cacheCharacters!).toBeGreaterThan(0);

    // The same file opened again: nothing is carried over.
    const before = await page.locator('#status').getAttribute('data-load-stats');
    await page.locator('#file-input').setInputFiles(sample!);
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
    expect(await reads(), 'a file opened again kept an answer').toBe(3);
    expect(afresh.cacheElements).toBe(1);
    expect(differences(a!.id, afresh, expected.get(a!.id)!)).toEqual([]);
  },
);

// The map is bounded: with room for two, a third element's answer pushes out
// the least recently selected, which is then read again, and the other is not.
sampleTest('the answers kept are bounded, the least recently selected going first', async ({ page }) => {
  test.setTimeout(300_000);
  const expected = oracle();
  await page.addInitScript(() => {
    const reads = window as unknown as { __parameterReads: number };
    reads.__parameterReads = 0;
    const RealWorker = window.Worker;
    window.Worker = class extends RealWorker {
      constructor(url: string | URL, options?: WorkerOptions) {
        super(url, options);
        if (String(url).includes('parameters-worker')) reads.__parameterReads += 1;
      }
    } as typeof Worker;
  });
  const reads = () =>
    page.evaluate(() => (window as unknown as { __parameterReads: number }).__parameterReads);
  await open(page, sample!, '?parametersRemembered=2');
  const [a, b, c] = await withParameters(page, expected, 3);
  for (const element of [a!, b!, c!]) {
    await select(page, element.index);
    await settled(page);
  }
  expect(await reads()).toBe(3);
  await select(page, c!.index);
  const keptC = await settled(page);
  expect(await reads(), 'the newest answer was kept').toBe(3);
  expect(keptC.cacheElements, 'room for two').toBe(2);
  await select(page, a!.index);
  const evictedA = await settled(page);
  expect(await reads(), 'the oldest answer was pushed out and read again').toBe(4);
  expect(differences(a!.id, evictedA, expected.get(a!.id)!)).toEqual([]);
});

/** Serves the parameters worker's script with a fault in front of it, for its first request only. */
async function failFirstRead(page: Page, panic: string): Promise<() => number> {
  const prelude = `(() => {
    const panic = ${JSON.stringify(panic)};
    const failing = () => {
      console.error(panic);
      throw new WebAssembly.RuntimeError('unreachable');
    };
    const wrapInstance = (instance) => {
      const exports = { ...instance.exports, savedParameters: failing };
      return new Proxy(instance, {
        get(target, key) {
          if (key === 'exports') return exports;
          const value = Reflect.get(target, key, target);
          return typeof value === 'function' ? value.bind(target) : value;
        },
      });
    };
    const wrap = (result) =>
      result instanceof WebAssembly.Instance
        ? wrapInstance(result)
        : { ...result, instance: wrapInstance(result.instance) };
    for (const name of ['instantiateStreaming', 'instantiate']) {
      const real = WebAssembly[name];
      WebAssembly[name] = async (...args) => wrap(await real.apply(WebAssembly, args));
    }
  })();`;
  let requests = 0;
  let served = 0;
  await page.context().route(/\/assets\/parameters-worker-[^/?]+\.js(\?.*)?$/, async (route) => {
    requests += 1;
    if (requests > 1) {
      await route.continue();
      return;
    }
    const response = await route.fetch();
    served += 1;
    await route.fulfill({ response, body: `${prelude}\n${await response.text()}` });
  });
  return () => served;
}

sampleTest(
  'a read that fails says so, and Try again reads the element',
  async ({ page }) => {
    test.setTimeout(300_000);
    const expected = oracle();
    const served = await failFirstRead(
      page,
      'rvt-rs wasm panic: panicked at rvt-wasm/src/lib.rs:1:1:\nindex out of bounds: the len is 3 but the index is 7',
    );
    await open(page, sample!);
    const withParameters = (await treeElements(page)).find(
      ({ id }) => id !== null && (expected.get(id)?.length ?? 0) > 0,
    );
    expect(withParameters, 'an element rvt-elements lists parameters for').toBeDefined();

    await select(page, withParameters!.index);
    const failed = await settled(page);
    await expect.poll(served, 'the patched worker script was served').toBeGreaterThan(0);
    expect(failed.state).toBe('error');
    // The panic's own words, and not a claim about memory.
    expect(failed.text).toContain('index out of bounds: the len is 3');
    expect(failed.text).not.toMatch(/ran out of memory/i);
    expect(failed.rows).toHaveLength(0);
    // The rest of the panel is where it was.
    await expect(page.locator('#info .info-group-identity')).toContainText('ElementId');

    await page.locator('#retry-saved-parameters').click();
    const retried = await settled(page);
    expect(retried.state).toBe('ready');
    expect(differences(retried.elementId!, retried, expected.get(retried.elementId!) ?? [])).toEqual([]);
  },
);

sampleTest(
  'a file of a release whose parameters are not read says so (the answer replaced)',
  async ({ page }) => {
    // No file of a release outside the range the reader covers is in the
    // viewer's job, so the worker's answer to the page is replaced with the
    // binding's own shape for one: `native` false, the release, no elements.
    // The range in the message is the real binding's.
    test.setTimeout(300_000);
    await page.addInitScript(() => {
      const RealWorker = window.Worker;
      window.Worker = class extends RealWorker {
        constructor(url: string | URL, options?: WorkerOptions) {
          super(url, options);
          if (!String(url).includes('parameters-worker')) return;
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
                const result = { ...data.result, release: 2013, native: false, elements: {} };
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
    expect(shown.text).toMatch(/from Revit 2015 to 2027 files/);
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
