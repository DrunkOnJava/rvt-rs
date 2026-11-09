/*
 * Heavy-lifting worker (VW1-19). Parses the RVT bytes and builds the
 * scene graph off the main thread so the UI stays responsive even
 * when a family file has tens of thousands of instances.
 *
 * Protocol:
 *   main → worker: { type: 'parse', bytes: Uint8Array, mode?: string,
 *                    limits?: OpenLimitsOptions, wasmUrl: string,
 *                    wasmModule?: WebAssembly.Module }
 *                  The worker is bundled into the page and starts from a blob
 *                  URL (`?worker&inline`), so it fetches no script and resolves
 *                  no URL itself: `wasmUrl` is where the page says the wasm is,
 *                  fetched only when `wasmModule` (the page's, from an earlier
 *                  open) is absent.
 *                  `limits` are the reader's resource limits; absent means the
 *                  defaults (the large-model retry sends `largeModel: true`).
 *                  `limits.phase` (a phase's name or ElementId) exports that
 *                  project phase, as `rvt-ifc --phase` does; absent is the
 *                  project's last.
 *   main → worker: { type: 'info', index: number }
 *   worker → main: { type: 'progress', step: string }
 *   worker → main: { type: 'wasm', module } the decoder's compiled wasm, which
 *                  the page hands to the workers that read saved parameters,
 *                  so that selecting an element fetches nothing
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
 *   worker → main: { type: 'ready', model, scene, glb, types, diagnostics, stats,
 *                    phases }
 *                  `phases` is the project's phases in order, `{ id, name? }`,
 *                  the last being the default export's; empty where the export
 *                  applies no phase.
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
import { PANIC_PREFIX, outOfMemory } from './wasm-failure';

interface OpenLimitsOptions {
  largeModel?: boolean;
  maxStreamBytes?: number;
  maxInflateBytes?: number;
  /** Draw every boxed element from its saved mesh; the binding's default is true. */
  savedMeshes?: boolean;
  /** The project phase to export, by name or ElementId; the binding's default is the last. */
  phase?: string | number;
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
  | {
      type: 'parse';
      bytes: Uint8Array;
      mode?: string;
      limits?: OpenLimitsOptions;
      /** Where the wasm is, as the page resolved it: this worker's own base is a blob URL. */
      wasmUrl: string;
      /** The compiled wasm, once the page has one: then nothing is fetched. */
      wasmModule?: WebAssembly.Module;
    }
  | { type: 'info'; index: number };

// Retained so `info` requests can re-run element_info_panel (host /
// hosted relationships, M4-04) without shipping a panel per element.
let parsedModel: unknown = null;

// The panic hook the bindings install writes `rvt-rs wasm panic: <info>` to
// `console.error` before the trap; wasm-failure.ts says how a stop without one
// reads, and what counts as running out of memory.
let lastPanic: string | null = null;
const nativeConsoleError = console.error.bind(console);
console.error = (...args: unknown[]): void => {
  const first = args[0];
  if (typeof first === 'string' && first.startsWith(PANIC_PREFIX)) lastPanic = first;
  nativeConsoleError(...args);
};

/**
 * The decoder's wasm, fetched once and compiled once, by the first open. The
 * page keeps the compiled module and hands it to every later open and to each
 * worker that reads an element's saved parameters (parameters-worker.ts), and
 * uses it for its own exports, so that nothing is requested once the first
 * file is open: those start from this module and fetch nothing.
 */
async function compileWasm(url: string): Promise<WebAssembly.Module> {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`could not load the decoder: ${response.status}`);
  try {
    return await WebAssembly.compileStreaming(response.clone());
  } catch {
    // A server that does not say `application/wasm` refuses to stream it.
    return WebAssembly.compile(await response.arrayBuffer());
  }
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
    const wasmModule = msg.wasmModule ?? (await compileWasm(msg.wasmUrl));
    const wasm = await init({ module_or_path: wasmModule });
    wasmMemory = wasm.memory;
    // The first open's module is the page's from here on.
    if (!msg.wasmModule) send({ type: 'wasm', module: wasmModule });

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
    const meshesNote = msg.limits?.savedMeshes === false ? ' · boxes kept' : '';
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
      phases?: unknown[];
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
        phases: exportResult.phases ?? [],
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
