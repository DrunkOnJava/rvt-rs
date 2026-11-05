import { expect, test } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

type NetworkPhase =
  | 'initial-static'
  | 'opening-sample'
  | 'post-open'
  | 'selecting'
  | 'exporting'
  | 'reopening';

interface NetworkRecord {
  url: string;
  method: string;
  resourceType: string;
  phase: NetworkPhase;
  initiator: string;
}

interface CdpStackFrame {
  url?: string;
  lineNumber?: number;
  columnNumber?: number;
  functionName?: string;
}

interface CdpInitiator {
  type?: string;
  url?: string;
  lineNumber?: number;
  columnNumber?: number;
  stack?: {
    callFrames?: CdpStackFrame[];
  };
}

interface CdpRequestWillBeSent {
  request: {
    method: string;
    url: string;
  };
  type?: string;
  initiator?: CdpInitiator;
}

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const observationMs = Number(process.env.RVT_VIEWER_NETWORK_IDLE_MS ?? '1500');
const samplePath = resolveSamplePath();

test.skip(
  samplePath === null,
  'Set RVT_VIEWER_SAMPLE, run pnpm run stage:demos, or check out phi-ag/rvt under ../_corpus to run the viewer network invariant test.',
);

test('opens a sample without external or post-open network requests', async ({ page, baseURL }) => {
  const records: NetworkRecord[] = [];
  let phase: NetworkPhase = 'initial-static';
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('Network.enable');
  cdp.on('Network.requestWillBeSent', (event: CdpRequestWillBeSent) => {
    if (!isNetworkUrl(event.request.url)) return;
    records.push({
      url: event.request.url,
      method: event.request.method,
      resourceType: event.type ?? 'unknown',
      phase,
      initiator: describeInitiator(event.initiator),
    });
  });

  await page.goto('/');
  await page.waitForLoadState('networkidle');
  await expect(page.locator('#status')).toHaveText(/Ready/);

  phase = 'opening-sample';
  await page.locator('#file-input').setInputFiles(samplePath!);
  await expect(page.locator('#status')).toHaveText(/Loaded/);

  phase = 'post-open';
  await page.waitForTimeout(observationMs);

  // Selecting an element reads its saved parameters in a worker of its own
  // (#35). It makes no request: the worker is in the page's bundle and starts
  // from a blob URL, and the wasm it runs is the module the open compiled. The
  // step is not optional: a sample with no element to select would let it pass
  // by skipping it.
  phase = 'selecting';
  const elements = page.locator('.tree-node.tree-element[data-entity-index]');
  expect(await elements.count(), 'the sample has elements to select').toBeGreaterThan(0);
  await elements.first().evaluate((row) => (row as HTMLElement).click());
  await expect(page.locator('#info-parameters')).toHaveAttribute(
    'data-state',
    /^(ready|empty|unsupported|error|on-request|no-id)$/,
    { timeout: 120_000 },
  );
  await page.waitForTimeout(observationMs);

  // The exports run wasm on the main thread, which starts from the module the
  // open compiled with the glue already in the page: an IFC, a plan and a glTF
  // export fetch nothing, however late they are clicked.
  phase = 'exporting';
  for (const selector of ['#export-ifc', '#export-svg', '#export-glb']) {
    const [download] = await Promise.all([
      page.waitForEvent('download'),
      page.locator(selector).click(),
    ]);
    expect((await download.path()) !== null, `${selector} made no file`).toBe(true);
  }
  await page.waitForTimeout(observationMs);

  // Opening the file again (the saved-mesh toggle does) starts the worker from
  // the page's bundle and from the module it already holds.
  phase = 'reopening';
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
  await page.waitForTimeout(observationMs);

  const base = new URL(baseURL ?? 'http://127.0.0.1:4173');
  const disallowed = records.filter((record) => isDisallowed(record, base));
  expect(disallowed, formatFailure(disallowed, records)).toEqual([]);

  // The decoder's wasm is fetched once, by the open, and not before it (nothing
  // is fetched to boot the page) and not again for an element: a read starts
  // from the module the open compiled. The CDP session sees the page's own
  // requests; the offline and request-recording test of parameters-row.spec.ts
  // is what sees the workers'.
  const wasm = records.filter((record) => new URL(record.url).pathname.endsWith('.wasm'));
  expect(
    wasm.filter((record) => record.phase === 'initial-static'),
    'the wasm was fetched to boot the page',
  ).toEqual([]);
  expect(wasm.length, 'the wasm was fetched more than once').toBeLessThanOrEqual(1);
});

function resolveSamplePath(): string | null {
  const candidates = [
    process.env.RVT_VIEWER_SAMPLE,
    path.resolve(__dirname, '../public/demos/synthetic-mvp.rvt'),
    path.resolve(__dirname, '../../_corpus/examples/Autodesk/racbasicsamplefamily-2024.rfa'),
  ].filter((candidate): candidate is string => Boolean(candidate));

  for (const candidate of candidates) {
    const resolved = path.isAbsolute(candidate) ? candidate : path.resolve(process.cwd(), candidate);
    if (fs.existsSync(resolved)) return resolved;
  }
  return null;
}

function isNetworkUrl(url: string): boolean {
  return /^(https?|wss?):\/\//i.test(url);
}

function isDisallowed(record: NetworkRecord, base: URL): boolean {
  if (record.phase !== 'initial-static' && record.phase !== 'opening-sample') return true;

  const url = new URL(record.url);
  if (url.origin !== base.origin) return true;

  return !isAllowedStaticRequest(url);
}

function isAllowedStaticRequest(url: URL): boolean {
  if (url.pathname === '/' || url.pathname.endsWith('/index.html')) return true;
  if (url.pathname.endsWith('/favicon.ico')) return true;
  if (url.pathname.includes('/demos/')) {
    return /\.(json|svg|png|jpe?g|webp|rvt|rfa|rte|rft|ifc)$/i.test(url.pathname);
  }
  if (!url.pathname.includes('/assets/')) return false;
  return /\.(css|js|mjs|wasm|map)$/i.test(url.pathname);
}

function describeInitiator(initiator: CdpInitiator | undefined): string {
  if (!initiator) return 'unknown';
  const type = initiator.type ?? 'unknown';
  const stackFrame = initiator.stack?.callFrames?.find((frame) => frame.url);
  if (stackFrame) return `${type} ${formatLocation(stackFrame)}`;

  if (initiator.url) {
    return `${type} ${formatLocation({
      url: initiator.url,
      lineNumber: initiator.lineNumber,
      columnNumber: initiator.columnNumber,
    })}`;
  }

  return type;
}

function formatLocation(frame: CdpStackFrame): string {
  const line = typeof frame.lineNumber === 'number' ? frame.lineNumber + 1 : 0;
  const column = typeof frame.columnNumber === 'number' ? frame.columnNumber + 1 : 0;
  const name = frame.functionName ? `${frame.functionName} ` : '';
  return `${name}${frame.url ?? 'unknown'}:${line}:${column}`;
}

function formatFailure(disallowed: NetworkRecord[], allRecords: NetworkRecord[]): string {
  if (disallowed.length === 0) return '';

  const rendered = disallowed.map(
    (record) =>
      `${record.phase} ${record.method} ${record.url} (${record.resourceType}; initiator ${record.initiator})`,
  );
  const context = allRecords.map(
    (record) =>
      `${record.phase} ${record.method} ${record.url} (${record.resourceType}; initiator ${record.initiator})`,
  );
  return [
    'Viewer privacy invariant failed: browser network traffic escaped the allowed static-load window.',
    'Disallowed requests:',
    ...rendered,
    'All observed HTTP/WebSocket requests:',
    ...context,
  ].join('\n');
}
