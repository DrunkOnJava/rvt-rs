/*
 * Reads the saved parameters of one element (#35, RE-207), in a worker of its
 * own and then ends.
 *
 * The page opens this worker when an element is selected and terminates it as
 * soon as it has answered, or when another element is selected first. The
 * worker that opened the file (worker.ts) is not asked: a read is a pass over
 * the file that blocks its worker for as long as it takes, and the
 * wasm linear memory of an instance only grows. A worker that is ended gives
 * every byte of both back, and the file is read again from the `File` the page
 * already holds, so nothing of it stays resident beyond what the page held
 * before this existed.
 *
 * Protocol:
 *   main → worker: { type: 'read', file: File, ids: number[], limits?: OpenLimitsOptions }
 *                  `limits` are the ones the file was opened under.
 *   worker → main: { type: 'result', result, readMs, wasmMemoryBytes }
 *                  `result` is what the `savedParameters` binding returns,
 *                  parsed (rvt-wasm/src/lib.rs); `readMs` the milliseconds from
 *                  the request to the answer, the file's bytes and the wasm's
 *                  start included, and `wasmMemoryBytes` this instance's linear
 *                  memory at the end.
 *   worker → main: { type: 'error', message, outOfMemory }
 */

import init, { savedParameters } from '../pkg/rvt.js';

interface OpenLimitsOptions {
  largeModel?: boolean;
  maxStreamBytes?: number;
  maxInflateBytes?: number;
}

interface ReadMsg {
  type: 'read';
  file: File;
  ids: number[];
  limits?: OpenLimitsOptions;
}

const send = (msg: unknown): void => {
  (self as unknown as { postMessage: (m: unknown) => void }).postMessage(msg);
};

// The panic hook the bindings install writes `rvt-rs wasm panic: <info>` to
// `console.error` before the trap, as worker.ts reads it.
const PANIC_PREFIX = 'rvt-rs wasm panic:';
let lastPanic: string | null = null;
const nativeConsoleError = console.error.bind(console);
console.error = (...args: unknown[]): void => {
  const first = args[0];
  if (typeof first === 'string' && first.startsWith(PANIC_PREFIX)) lastPanic = first;
  nativeConsoleError(...args);
};

/** Messages that mean an allocation could not be made (worker.ts, `outOfMemory`). */
const OUT_OF_MEMORY_MESSAGE =
  /capacity overflow|memory allocation of \d+ bytes failed|out of memory|cannot allocate|array buffer allocation failed|invalid (typed )?array length|maximum memory size exceeded|invalid string length/i;

function outOfMemory(err: unknown, panic: string | null): boolean {
  const message = err instanceof Error ? err.message : String(err);
  if (panic !== null) return OUT_OF_MEMORY_MESSAGE.test(panic);
  if (err instanceof WebAssembly.RuntimeError) {
    return /unreach/i.test(message) || OUT_OF_MEMORY_MESSAGE.test(message);
  }
  return OUT_OF_MEMORY_MESSAGE.test(message) && !(err instanceof TypeError);
}

self.addEventListener('message', async (ev: MessageEvent<ReadMsg>) => {
  const msg = ev.data;
  if (msg.type !== 'read') return;
  const startedAt = performance.now();
  try {
    const wasm = await init();
    const bytes = new Uint8Array(await msg.file.arrayBuffer());
    lastPanic = null;
    const json = savedParameters(bytes, new Float64Array(msg.ids), msg.limits ?? null);
    send({
      type: 'result',
      result: JSON.parse(json) as unknown,
      readMs: performance.now() - startedAt,
      wasmMemoryBytes: wasm.memory.buffer.byteLength,
    });
  } catch (err) {
    const raw = err instanceof Error ? err.message : String(err);
    send({
      type: 'error',
      message: lastPanic ?? raw,
      outOfMemory: outOfMemory(err, lastPanic),
    });
  }
});
