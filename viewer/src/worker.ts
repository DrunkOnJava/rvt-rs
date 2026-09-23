/*
 * Heavy-lifting worker (VW1-19). Parses the RVT bytes and builds the
 * scene graph off the main thread so the UI stays responsive even
 * when a family file has tens of thousands of instances.
 *
 * Protocol:
 *   main → worker: { type: 'parse', bytes: Uint8Array, mode?: string }
 *   main → worker: { type: 'info', index: number }
 *   worker → main: { type: 'progress', step: string }
 *   worker → main: { type: 'summary', summary, document } (VW1-20 partial;
 *                   `document` is the FileMetadata or null)
 *   worker → main: { type: 'ready', model, scene, glb, types, diagnostics }
 *   worker → main: { type: 'info', index, panel }
 *   worker → main: { type: 'error', message: string }
 */

import init, {
  elementInfoPanel,
  openRvtBytesWithDiagnosticsMode,
  buildSceneGraph,
  modelToGlb,
  distinctIfcTypes,
  buildSchedule,
  fileMetadata,
  quickSummary,
} from '../pkg/rvt.js';

type ParseMsg =
  | { type: 'parse'; bytes: Uint8Array; mode?: string }
  | { type: 'info'; index: number };

// Retained so `info` requests can re-run element_info_panel (host /
// hosted relationships, M4-04) without shipping a panel per element.
let parsedModel: unknown = null;

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

  try {
    send({ type: 'progress', step: 'Initializing wasm' });
    await init();

    // VW1-20 — progressive streaming. Emit the cheap metadata
    // first (sub-second even on hundreds-of-MB files) so the UI
    // can populate the top bar while the expensive full-model
    // parse continues.
    send({ type: 'progress', step: 'Reading file metadata' });
    // Worksharing / last-saved identity is a nicety: a file whose
    // BasicFileInfo text block is unreadable still parses.
    let document: unknown = null;
    try {
      document = fileMetadata(msg.bytes);
    } catch {
      document = null;
    }
    send({ type: 'summary', summary: quickSummary(msg.bytes), document });

    const qualityMode = (msg.mode ?? 'scaffold').trim() || 'scaffold';
    send({ type: 'progress', step: `Parsing container · IFC bar ${qualityMode}` });
    const exportResult = openRvtBytesWithDiagnosticsMode(msg.bytes, qualityMode) as {
      model: unknown;
      diagnostics: unknown;
    };
    const model = exportResult.model;
    parsedModel = model;

    send({ type: 'progress', step: 'Building scene graph' });
    const scene = buildSceneGraph(model);
    const types = distinctIfcTypes(scene);

    send({ type: 'progress', step: 'Rendering glTF' });
    const glb = modelToGlb(model);

    send({ type: 'progress', step: 'Building schedule' });
    const schedule = buildSchedule(model);

    send(
      {
        type: 'ready',
        model,
        scene,
        types,
        glb,
        schedule,
        diagnostics: exportResult.diagnostics,
      },
      // Cast for lib.dom.d.ts variants that parameterise
      // ArrayBufferView over ArrayBufferLike: the underlying
      // ArrayBuffer is a Transferable in every runtime we target.
      [glb.buffer as ArrayBuffer],
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    send({ type: 'error', message });
  }
});
