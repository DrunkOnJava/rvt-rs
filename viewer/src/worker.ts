/*
 * Heavy-lifting worker (VW1-19). Parses the RVT bytes and builds the
 * scene graph off the main thread so the UI stays responsive even
 * when a family file has tens of thousands of instances.
 *
 * Protocol:
 *   main → worker: { type: 'parse', bytes: Uint8Array, mode?: string,
 *                    limits?: OpenLimitsOptions }
 *                  `limits` are the reader's resource limits; absent means the
 *                  defaults (the large-model retry sends `largeModel: true`).
 *   main → worker: { type: 'info', index: number }
 *   worker → main: { type: 'progress', step: string }
 *   worker → main: { type: 'stage', stage, done, total, wasmMemoryBytes } (#1195)
 *                  the decoder's own progress, as `rvt-gltf --progress` prints
 *                  it: `stage` is schema-parse, candidate-scan, element-decode,
 *                  partition-scan, saved-graphics or saved-meshes, `done` and
 *                  `total` are counts (`total` null while the stage has none),
 *                  and `wasmMemoryBytes` is the wasm linear memory at that
 *                  moment. The decoder reports a stage when it begins, when it
 *                  ends and once per whole percent in between.
 *   worker → main: { type: 'summary', summary, document } (VW1-20 partial;
 *                   `document` is the FileMetadata or null)
 *   worker → main: { type: 'ready', model, scene, glb, types, diagnostics, stats }
 *                  `stats` is { openMs, workerMs, wasmMemoryBytes }: the
 *                  milliseconds spent in the wasm open (saved meshes included)
 *                  and in the whole parse, and the wasm linear memory after it,
 *                  which only grows, so it is the peak against the 4 GiB a tab
 *                  can hold.
 *   worker → main: { type: 'info', index, panel }
 *   worker → main: { type: 'error', message: string, tooLarge?: TooLarge }
 *                  `tooLarge` is set when the failure is the wasm instance
 *                  running out of memory (see `outOfMemory`), with what the
 *                  worker knew when it stopped: the wasm memory reached, the
 *                  last decoder report (or null once the open call returned),
 *                  the worker step it was in, and the engine's own message.
 */

import init, {
  elementInfoPanel,
  openRvtBytesWithDiagnosticsModeAndOpenLimits,
  buildSceneGraph,
  modelToGlb,
  distinctIfcTypes,
  buildSchedule,
  fileMetadata,
  quickSummary,
} from '../pkg/rvt.js';

interface OpenLimitsOptions {
  largeModel?: boolean;
  maxStreamBytes?: number;
  maxInflateBytes?: number;
  /** Draw doors and windows from their saved meshes; the binding's default is true. */
  savedMeshes?: boolean;
}

/** The decoder's last report before a stop. */
interface StageReport {
  stage: string;
  done: number;
  total: number | null;
}

/** What the worker reports when a decode stops for want of memory. */
interface TooLarge {
  /** The wasm linear memory when it stopped: how far the decode got. */
  wasmMemoryBytes: number;
  /** The decoder's last report inside the open call, or null outside it. */
  report: StageReport | null;
  /** The worker step it was in when it stopped, or null inside the open call. */
  step: string | null;
  /** The engine's own message (the panic's, when there was one), shown in the status panel. */
  detail: string;
}

type ParseMsg =
  | { type: 'parse'; bytes: Uint8Array; mode?: string; limits?: OpenLimitsOptions }
  | { type: 'info'; index: number };

// Retained so `info` requests can re-run element_info_panel (host /
// hosted relationships, M4-04) without shipping a panel per element.
let parsedModel: unknown = null;

// The panic hook the bindings install writes `rvt-rs wasm panic: <info>` to
// `console.error` before the trap. A trap with none of those first is not a
// panic: an allocation the wasm memory could not grow to hold aborts the
// instance without running the hook, and reaches here as a bare
// `RuntimeError: unreachable` (V8), `unreachable executed` (Firefox) or
// `Unreachable code should not be executed` (Safari).
const PANIC_PREFIX = 'rvt-rs wasm panic:';
let lastPanic: string | null = null;
const nativeConsoleError = console.error.bind(console);
console.error = (...args: unknown[]): void => {
  const first = args[0];
  if (typeof first === 'string' && first.startsWith(PANIC_PREFIX)) lastPanic = first;
  nativeConsoleError(...args);
};

/** Panic messages and engine errors that mean an allocation could not be made. */
const OUT_OF_MEMORY_MESSAGE =
  /capacity overflow|memory allocation of \d+ bytes failed|out of memory|cannot allocate|array buffer allocation failed|invalid (typed )?array length|maximum memory size exceeded|invalid string length/i;

/**
 * True when `err` is the wasm instance, or the engine around it, running out
 * of memory. wasm32 holds at most 4 GiB, a `Vec` at most 2 GiB, and a failed
 * allocation aborts without a message, so the evidence is: a bare
 * `unreachable` trap with no panic message before it, or a message that names
 * an allocation. Any other panic is a bug and keeps its message.
 */
function outOfMemory(err: unknown, panic: string | null): boolean {
  const message = err instanceof Error ? err.message : String(err);
  if (panic !== null) return OUT_OF_MEMORY_MESSAGE.test(panic);
  if (err instanceof WebAssembly.RuntimeError) {
    return /unreach/i.test(message) || OUT_OF_MEMORY_MESSAGE.test(message);
  }
  return OUT_OF_MEMORY_MESSAGE.test(message) && !(err instanceof TypeError);
}

// DedicatedWorkerGlobalScope.postMessage has a slightly shifty
// TS signature across lib.dom.d.ts versions — strictly-typed
// `self` narrows it to zero-arg. Wrap once in a helper so every
// callsite goes through the same local type.
const send = (msg: unknown, transfer?: Transferable[]): void => {
  (
    self as unknown as {
      postMessage: (m: unknown, transfer?: Transferable[]) => void;
    }
  ).postMessage(msg, transfer);
};

self.addEventListener('message', async (ev: MessageEvent<ParseMsg>) => {
  const msg = ev.data;
  if (msg.type === 'info') {
    let panel: unknown = null;
    try {
      if (parsedModel !== null) panel = elementInfoPanel(parsedModel, msg.index);
    } catch {
      // A panel we cannot build is a missing relationship section,
      // not a failed load — reply with null rather than throwing.
      panel = null;
    }
    send({ type: 'info', index: msg.index, panel });
    return;
  }
  if (msg.type !== 'parse') return;

  let wasmMemory: WebAssembly.Memory | null = null;
  let report: StageReport | null = null;
  let currentStep: string | null = null;
  // A step the page is told about, and the one a stop is then blamed on.
  const step = (text: string): void => {
    currentStep = text;
    send({ type: 'progress', step: text });
  };
  try {
    step('Initializing wasm');
    const startedAt = performance.now();
    const wasm = await init();
    wasmMemory = wasm.memory;

    // VW1-20 — progressive streaming. Emit the cheap metadata
    // first (sub-second even on hundreds-of-MB files) so the UI
    // can populate the top bar while the expensive full-model
    // parse continues.
    step('Reading file metadata');
    // Worksharing / last-saved identity is a nicety: a file whose
    // BasicFileInfo text block is unreadable still parses.
    let document: unknown = null;
    try {
      document = fileMetadata(msg.bytes);
    } catch {
      document = null;
      // A panic this swallowed is not the one a later stop is judged by.
      lastPanic = null;
    }
    send({ type: 'summary', summary: quickSummary(msg.bytes), document });

    const qualityMode = (msg.mode ?? 'scaffold').trim() || 'scaffold';
    const limitsNote = msg.limits?.largeModel ? ' · large-model limits' : '';
    const meshesNote = msg.limits?.savedMeshes === false ? ' · door and window boxes' : '';
    step(`Parsing container · IFC bar ${qualityMode}${limitsNote}${meshesNote}`);
    const openStartedAt = performance.now();
    const onStage = (stage: string, done: number, total: number | null): void => {
      report = { stage, done, total };
      currentStep = null;
      send({ type: 'stage', stage, done, total, wasmMemoryBytes: wasm.memory.buffer.byteLength });
    };
    // Only a panic from the open call itself bears on how it stopped.
    lastPanic = null;
    const exportResult = openRvtBytesWithDiagnosticsModeAndOpenLimits(
      msg.bytes,
      qualityMode,
      msg.limits ?? null,
      onStage,
    ) as {
      model: unknown;
      diagnostics: unknown;
    };
    const openMs = performance.now() - openStartedAt;
    // The open call is over: a stop after it is in the worker's own steps.
    report = null;
    const model = exportResult.model;
    parsedModel = model;

    step('Building scene graph');
    const scene = buildSceneGraph(model);
    const types = distinctIfcTypes(scene);

    step('Rendering glTF');
    const glb = modelToGlb(model);

    step('Building schedule');
    const schedule = buildSchedule(model);

    // Read last, so the memory is what the whole pipeline needed.
    const stats = {
      openMs,
      workerMs: performance.now() - startedAt,
      wasmMemoryBytes: wasm.memory.buffer.byteLength,
    };

    send(
      {
        type: 'ready',
        model,
        scene,
        types,
        glb,
        schedule,
        diagnostics: exportResult.diagnostics,
        stats,
      },
      // Cast for lib.dom.d.ts variants that parameterise
      // ArrayBufferView over ArrayBufferLike: the underlying
      // ArrayBuffer is a Transferable in every runtime we target.
      [glb.buffer as ArrayBuffer],
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    if (outOfMemory(err, lastPanic)) {
      const tooLarge: TooLarge = {
        wasmMemoryBytes: wasmMemory?.buffer.byteLength ?? 0,
        report,
        step: report ? null : currentStep,
        detail: lastPanic ?? (err instanceof Error ? `${err.name}: ${err.message}` : message),
      };
      send({ type: 'error', message, tooLarge });
      return;
    }
    // A panic's text is on console.error, and the trap carries only
    // `unreachable`: say the panic's, which is the failure's own message.
    send({ type: 'error', message: lastPanic ?? message });
  }
});
