import { expect, test, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// #1195: what the page says when the wasm call fails in ways a lowered memory
// ceiling cannot produce in Chromium: another engine's wording of the same trap,
// a panic that is not a memory failure, and a stop after the open call. The
// page's worker is the shipped one and the wasm is the shipped one; the test
// serves the worker's script with a few lines in front of it that patch
// `WebAssembly.instantiateStreaming` and `instantiate` so that one export of the
// instance, the one named, fails the way the case says. The page's
// Content-Security-Policy allows workers from 'self' only, so the script is
// served from its own address, by a route, not from a blob. Nothing is added to
// what ships.
//
//   - "unreachable executed" (Firefox) and "Unreachable code should not be
//     executed" (Safari), with no panic message before them, are the same
//     failed allocation V8 calls `unreachable`;
//   - a panic message on console.error before the trap is a panic, which is a
//     bug, not memory, and keeps its own message;
//   - an allocation that fails after the open call returned is blamed on the
//     worker's step, not on the decoder.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const tier1 = path.resolve(
  __dirname,
  '../../corpus/tier1/architectural-2024/architectural-2024.rvt',
);

interface Fault {
  /** The wasm export that fails. */
  fn: string;
  /** What the panic hook would have written to console.error first, if anything. */
  panic: string | null;
  /** The message of the `WebAssembly.RuntimeError` the engine throws. */
  error: string;
}

/** Fails one export of the worker's wasm instance; returns how many times the patched worker script was served. */
async function failWasm(page: Page, fault: Fault): Promise<() => number> {
  const prelude = `(() => {
    const fault = ${JSON.stringify(fault)};
    const failing = () => {
      if (fault.panic !== null) console.error(fault.panic);
      throw new WebAssembly.RuntimeError(fault.error);
    };
    const wrapInstance = (instance) => {
      const exports = { ...instance.exports, [fault.fn]: failing };
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
  let served = 0;
  await page.context().route(/\/assets\/worker-[^/?]+\.js(\?.*)?$/, async (route) => {
    const response = await route.fetch();
    served += 1;
    await route.fulfill({ response, body: `${prelude}\n${await response.text()}` });
  });
  return () => served;
}

async function open(page: Page, query = ''): Promise<void> {
  expect(fs.existsSync(tier1), `${tier1} is committed`).toBe(true);
  await page.goto(`/${query}`);
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await page.locator('#file-input').setInputFiles(tier1);
}

function engineRow(page: Page) {
  return page.locator('#status-panel .status-row', {
    has: page.locator('.status-label', { hasText: /^Engine message$/ }),
  });
}

const OPEN = 'openRvtBytesWithDiagnosticsModeAndOpenLimits';

test('a panic that is not a memory failure keeps its own message', async ({ page }) => {
  const panic =
    'rvt-rs wasm panic: panicked at src/walker.rs:1440:9:\nindex out of bounds: the len is 3 but the index is 7';
  const served = await failWasm(page, { fn: OPEN, panic, error: 'unreachable' });
  await open(page);
  // A route that never saw the worker would leave the page unbroken.
  await expect.poll(served, 'the patched worker script was served').toBeGreaterThan(0);

  // An error, in the panic's own words; not a claim about memory.
  await expect(page.locator('#status')).toHaveText(/^Error: rvt-rs wasm panic: .*index out of bounds/s);
  await expect(page.locator('#status-panel')).toContainText('index out of bounds: the len is 3');
  await expect(page.locator('#toast-region .toast.bad')).toContainText('index out of bounds');
  await expect(page.locator('#status')).not.toHaveAttribute('data-load-failure', /.+/);
  await expect(page.locator('#scaffold-note')).toBeHidden();
  for (const where of ['#status', '#status-panel', '#toast-region']) {
    expect(await page.locator(where).innerText(), where).not.toMatch(
      /too large|ran out of memory|needs more/i,
    );
  }
  await expect(page.locator('#load-overlay')).toBeHidden();
  await expect.poll(() => page.workers().length, 'the worker is ended').toBe(0);
});

test('a bare unreachable in Firefox wording is a memory failure', async ({ page }) => {
  const served = await failWasm(page, { fn: OPEN, panic: null, error: 'unreachable executed' });
  // A ceiling the page's memory is within reach of, so the stop reads as near it.
  await open(page, '?memoryCeilingBytes=1048576');
  // A route that never saw the worker would leave the page unbroken.
  await expect.poll(served, 'the patched worker script was served').toBeGreaterThan(0);

  await expect(page.locator('#status')).toHaveAttribute('data-load-failure', 'too-large');
  await expect(page.locator('#status')).toHaveText(/^Too large for the browser · stopped at /);
  await expect(engineRow(page)).toContainText('RuntimeError: unreachable executed');
  await expect(page.locator('#scaffold-note-title')).toHaveText('Too large for the browser');
  await expect(page.locator('#scaffold-note-body')).not.toContainText(/unreachable/i);
});

test('a bare unreachable in Safari wording is a memory failure, hedged far from the ceiling', async ({
  page,
}) => {
  const served = await failWasm(page, {
    fn: OPEN,
    panic: null,
    error: 'Unreachable code should not be executed (evaluating ...)',
  });
  await open(page);
  // A route that never saw the worker would leave the page unbroken.
  await expect.poll(served, 'the patched worker script was served').toBeGreaterThan(0);

  // The real 4 GiB, and a stop at a few MiB on a 20 KiB file: not claimed to
  // be too large, and the page says so.
  await expect(page.locator('#status')).toHaveAttribute('data-load-failure', 'out-of-memory');
  await expect(page.locator('#scaffold-note-title')).toHaveText('Ran out of memory');
  await expect(page.locator('#scaffold-note-body')).toContainText('may be damaged');
  await expect(page.locator('#scaffold-note-body')).not.toContainText('needs more');
  await expect(engineRow(page)).toContainText('Unreachable code should not be executed');
});

test('an allocation that fails after the open call is blamed on the step it was in', async ({
  page,
}) => {
  const served = await failWasm(page, { fn: 'buildSceneGraph', panic: null, error: 'unreachable' });
  await open(page, '?memoryCeilingBytes=1048576');
  // A route that never saw the worker would leave the page unbroken.
  await expect.poll(served, 'the patched worker script was served').toBeGreaterThan(0);

  await expect(page.locator('#status')).toHaveAttribute('data-load-failure', 'too-large');
  await expect(page.locator('#scaffold-note-body')).toContainText(', while building the scene graph,');
  await expect(page.locator('#scaffold-note-body')).not.toContainText('partition');
  // The decoder finished: its reports are all there, and none is the blame.
  const events = JSON.parse(
    (await page.locator('#status').getAttribute('data-load-progress')) ?? '[]',
  ) as Array<{ stage?: string; step?: string }>;
  expect(events.some((event) => event.stage !== undefined)).toBe(true);
  expect(events.some((event) => event.step === 'Building scene graph')).toBe(true);
  await expect.poll(() => page.workers().length, 'the worker is ended').toBe(0);
});
