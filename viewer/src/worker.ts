/*
 * Heavy-lifting worker (VW1-19). Parses the RVT bytes and builds the
 * scene graph off the main thread so the UI stays responsive even
 * when a family file has tens of thousands of instances.
 *
 * Protocol:
 *   main → worker: { type: 'parse', bytes: Uint8Array }
 *   worker → main: { type: 'progress', step: string }
 *   worker → main: { type: 'ready', model, scene, glb, types }
 *   worker → main: { type: 'error', message: string }
 */

import init, {
  openRvtBytes,
  buildSceneGraph,
  modelToGlb,
  distinctIfcTypes,
  buildSchedule,
  quickSummary,
} from '../pkg/rvt.js';

type ParseMsg = { type: 'parse'; bytes: Uint8Array };

self.addEventListener('message', async (ev: MessageEvent<ParseMsg>) => {
  const msg = ev.data;
  if (msg.type !== 'parse') return;

  try {
    (self as unknown as { postMessage: (m: unknown) => void }).postMessage({
      type: 'progress',
      step: 'initializing wasm',
    });
    await init();

    // VW1-20 — progressive streaming. Emit the cheap metadata
    // first (sub-second even on hundreds-of-MB files) so the UI
    // can populate the top bar / storey list / version while the
    // expensive full-model parse continues.
    (self as unknown as { postMessage: (m: unknown) => void }).postMessage({
      type: 'progress',
      step: 'reading file metadata',
    });
    const summary = quickSummary(msg.bytes);
    (self as unknown as { postMessage: (m: unknown) => void }).postMessage({
      type: 'summary',
      summary,
    });

    (self as unknown as { postMessage: (m: unknown) => void }).postMessage({
      type: 'progress',
      step: 'parsing container',
    });
    const model = openRvtBytes(msg.bytes);

    (self as unknown as { postMessage: (m: unknown) => void }).postMessage({
      type: 'progress',
      step: 'building scene graph',
    });
    const scene = buildSceneGraph(model);
    const types = distinctIfcTypes(scene);

    (self as unknown as { postMessage: (m: unknown) => void }).postMessage({
      type: 'progress',
      step: 'rendering glTF',
    });
    const glb = modelToGlb(model);

    (self as unknown as { postMessage: (m: unknown) => void }).postMessage({
      type: 'progress',
      step: 'building schedule',
    });
    const schedule = buildSchedule(model);

    (self as unknown as { postMessage: (m: unknown) => void }).postMessage(
      { type: 'ready', model, scene, types, glb, schedule },
      { transfer: [glb.buffer] },
    );
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    (self as unknown as { postMessage: (m: unknown) => void }).postMessage({
      type: 'error',
      message,
    });
  }
});
