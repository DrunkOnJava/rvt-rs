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
 * Selecting an element makes no request. This script is bundled into the
 * page's own (`?worker&inline`: the worker starts from a blob URL the page
 * makes, which the Content-Security-Policy's `worker-src` allows with `blob:`),
 * so there is no chunk to fetch; and the decoder's wasm arrives already
 * compiled, the module worker.ts compiled at the open, so `initSync` starts it
 * from memory and nothing is fetched. A page taken offline after it has loaded
 * still reads parameters.
 *
 * Protocol:
 *   main → worker: { type: 'read', file: File, ids: number[], module: WebAssembly.Module,
 *                    lock: string | null, limits?: OpenLimitsOptions }
 *                  `limits` are the ones the file was opened under; `lock` names a
 *                  Web Lock the worker holds for as long as it lives, so that the
 *                  page learns the worker has gone without answering (a worker
 *                  the browser ended raises no event).
 *   worker → main: { type: 'started' } once the lock is held
 *   worker → main: { type: 'result', result, readMs, wasmMemoryBytes }
 *                  `result` is what the `savedParameters` binding returns,
 *                  parsed (rvt-wasm/src/lib.rs); `readMs` the milliseconds from
 *                  the request to the answer, the file's bytes and the wasm's
 *                  start included, and `wasmMemoryBytes` this instance's linear
 *                  memory at the end.
 *   worker → main: { type: 'error', message, outOfMemory }
 */

import { initSync, savedParameters } from '../pkg/rvt.js';
import { PANIC_PREFIX, outOfMemory } from './wasm-failure';

interface OpenLimitsOptions {
  largeModel?: boolean;
  maxStreamBytes?: number;
  maxInflateBytes?: number;
}

interface ReadMsg {
  type: 'read';
  file: File;
  ids: number[];
  module: WebAssembly.Module;
  lock: string | null;
  limits?: OpenLimitsOptions;
}

const send = (msg: unknown): void => {
  (self as unknown as { postMessage: (m: unknown) => void }).postMessage(msg);
};

// The panic hook the bindings install writes `rvt-rs wasm panic: <info>` to
// `console.error` before the trap, as worker.ts reads it.
let lastPanic: string | null = null;
const nativeConsoleError = console.error.bind(console);
console.error = (...args: unknown[]): void => {
  const first = args[0];
  if (typeof first === 'string' && first.startsWith(PANIC_PREFIX)) lastPanic = first;
  nativeConsoleError(...args);
};

/** Takes the lock and never lets go: the browser releases it when this worker ends, for any reason. */
function holdLock(name: string): Promise<void> {
  return new Promise<void>((held) => {
    void navigator.locks.request(name, () => {
      held();
      return new Promise<never>(() => {});
    });
  });
}

self.addEventListener('message', async (ev: MessageEvent<ReadMsg>) => {
  const msg = ev.data;
  if (msg.type !== 'read') return;
  const startedAt = performance.now();
  try {
    if (msg.lock !== null && typeof navigator !== 'undefined' && navigator.locks) {
      await holdLock(msg.lock);
    }
    send({ type: 'started' });
    const wasm = initSync({ module: msg.module });
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
