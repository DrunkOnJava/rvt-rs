/*
 * Heavy-lifting worker (VW1-19). Parses the RVT bytes and builds the
 * scene graph off the main thread so the UI stays responsive even
 * when a family file has tens of thousands of instances.
 *
 * Protocol:
 *   main → worker: { type: 'parse', bytes: Uint8Array }
 *   worker → main: { type: 'progress', step: string }
 *   worker → main: { type: 'summary', summary }   (VW1-20 partial)
 *   worker → main: { type: 'ready', model, scene, glb, types }
 *   worker → main: { type: 'error', message: string }
 */

import init, {
  openRvtBytesWithDiagnostics,
  buildSceneGraph,
  modelToGlb,
  distinctIfcTypes,
  buildSchedule,
  quickSummary,
} from '../pkg/rvt.js';

type ParseMsg = { type: 'parse'; bytes: Uint8Array };

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
  if (msg.type !== 'parse') return;

  try {
    send({ type: 'progress', step: 'initializing wasm' });
    await init();

    // VW1-20 — progressive streaming. Emit the cheap metadata
    // first (sub-second even on hundreds-of-MB files) so the UI
    // can populate the top bar while the expensive full-model
    // parse continues.
    send({ type: 'progress', step: 'reading file metadata' });
    send({ type: 'summary', summary: quickSummary(msg.bytes) });

    send({ type: 'progress', step: 'parsing container' });
    const exportResult = openRvtBytesWithDiagnostics(msg.bytes) as {
      model: unknown;
      diagnostics: unknown;
    };
    const model = exportResult.model;

    send({ type: 'progress', step: 'building scene graph' });
    const scene = buildSceneGraph(model);
    const types = distinctIfcTypes(scene);

    send({ type: 'progress', step: 'rendering glTF' });
    const glb = modelToGlb(model);

    send({ type: 'progress', step: 'building schedule' });
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
