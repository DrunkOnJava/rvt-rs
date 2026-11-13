/*
 * rvt-rs browser viewer — main thread.
 *
 * Responsibilities:
 *   - drag-and-drop / file-picker intake (VW1-23)
 *   - spin up the parse worker (VW1-19)
 *   - Three.js scene + orbit controls + GLTFLoader (VW1-03)
 *   - scene-tree / category / info panels
 *   - status line + error surfacing
 *
 * Kept deliberately dependency-light: Three.js + the wasm package,
 * plus a single on-page CSS block. No React, no UI framework.
 */

import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import supportMatrix from '../../docs/support-matrix.json';
import * as wasmGlue from '../pkg/rvt.js';
import DecoderWorker from './worker.ts?worker&inline';
import ParametersWorker from './parameters-worker.ts?worker&inline';
import {
  elementIdOf,
  parameterIsNumeric,
  parameterKindText,
  parameterLabel,
  parameterValueText,
  releasesText,
  type SavedParameter,
  type SavedParametersReading,
  type SavedParametersResult,
} from './saved-parameters';

// ---------- DOM ----------
const $ = (id: string): HTMLElement => {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing element: #${id}`);
  return el;
};
const viewport = $('viewport');
const dropzone = $('dropzone');
const fileInput = $('file-input') as HTMLInputElement;
const pickBtn = $('pick-file');
$('app-version').textContent = `rvt-rs ${__RVT_VIEWER_VERSION__}`;
const statusEl = $('status');
const fileMetaEl = $('file-meta');
const treeEl = $('tree');
const treeFilterEl = $('tree-filter') as HTMLInputElement;
const treeFilterStatusEl = $('tree-filter-status');
const categoriesEl = $('categories');
const infoEl = $('info');
const scheduleEl = $('schedule-summary');
const scheduleTotalEl = $('schedule-total');
const scheduleGroupsEl = $('schedule-groups');
const statusPanelEl = $('status-panel');
const diagnosticsJsonEl = $('diagnostics-json');
const downloadDiagnosticsBtn = $('download-diagnostics') as HTMLButtonElement;
const downloadScheduleBtn = $('download-schedule') as HTMLButtonElement;
const downloadRoomsBtn = $('download-rooms') as HTMLButtonElement;
const exportGlbBtn = $('export-glb') as HTMLButtonElement;
const exportIfcBtn = $('export-ifc') as HTMLButtonElement;
const exportSvgBtn = $('export-svg') as HTMLButtonElement;
const exportQualityEl = $('export-quality');
const exportModeEl = $('export-mode') as HTMLSelectElement;
const savedMeshesEl = $('saved-meshes') as HTMLInputElement;
const exportPhaseEl = $('export-phase') as HTMLSelectElement;
const exportPhaseFieldEl = $('export-phase-field');
const exportPhaseHintEl = $('export-phase-hint');
const demoListEl = $('demo-list');
const demoAttributionEl = $('demo-attribution');
const loadOverlayEl = $('load-overlay');
const loadStepEl = $('load-step');
const loadElapsedEl = $('load-elapsed');
const loadHintEl = $('load-hint');
const loadRailEl = $('load-rail');
const loadWarningEl = $('load-warning');
const scaffoldNoteEl = $('scaffold-note');
const scaffoldNoteTitleEl = $('scaffold-note-title');
const scaffoldNoteBodyEl = $('scaffold-note-body');
const showRealProjectsBtn = $('show-real-projects') as HTMLButtonElement;
const dismissScaffoldNoteBtn = $('dismiss-scaffold-note') as HTMLButtonElement;
const toastRegionEl = $('toast-region');
const exportHintEl = $('export-hint');
const supportMatrixEl = $('support-matrix');
const supportHomeEl = $('support-profile-home') as HTMLDetailsElement;
const supportSlotEl = $('support-slot-canvas');
const supportGridEl = $('support-grid') as HTMLTableElement;
const supportLegendEl = $('support-grid-legend');

// ---------- Motion ----------
const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
function prefersReducedMotion(): boolean {
  return reducedMotionQuery.matches;
}

// ---------- Three.js scene ----------
const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 2000);
camera.position.set(60, 40, 60);
const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
viewport.appendChild(renderer.domElement);
const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;
controls.target.set(0, 0, 0);

// Physical light units: a matte face returns its colour times the light
// it receives over pi, so a face lit by both lights shows about its own
// category colour, and faces turned away fall off from there.
const hemi = new THREE.HemisphereLight(0xffffff, 0xffffff, 2.0);
scene.add(hemi);
const dir = new THREE.DirectionalLight(0xffffff, 1.6);
dir.position.set(50, 80, 50);
scene.add(dir);
let grid: THREE.GridHelper | null = null;
const axes = new THREE.AxesHelper(10);
scene.add(axes);

// ---------- Theme ----------
// The scene takes its colours from the design tokens (tokens.css,
// styles.css), so the canvas, grid and selection follow the page between
// the light and dark schemes.
function tokenColour(name: string): THREE.Color {
  const value = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  return new THREE.Color(value || '#808080');
}

function applySceneTheme(): void {
  scene.background = tokenColour('--viewer-canvas');
  hemi.color = tokenColour('--viewer-light-sky');
  hemi.groundColor = tokenColour('--viewer-light-ground');
  if (grid) {
    scene.remove(grid);
    grid.geometry.dispose();
    (grid.material as THREE.Material).dispose();
  }
  grid = new THREE.GridHelper(
    100,
    20,
    tokenColour('--viewer-grid'),
    tokenColour('--viewer-grid-soft'),
  );
  scene.add(grid);
  axes.setColors(tokenColour('--sx-red-6'), tokenColour('--sx-green-6'), tokenColour('--sx-blue-6'));
  highlightMaterial.color = tokenColour('--viewer-highlight');
  highlightMaterial.emissive = tokenColour('--viewer-highlight').multiplyScalar(0.25);
}

let currentModel: THREE.Group | null = null;

function resize(): void {
  const w = viewport.clientWidth;
  const h = viewport.clientHeight;
  if (w === 0 || h === 0) {
    return;
  }
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}
window.addEventListener('resize', resize);
// The viewport also changes size without a window resize: the responsive
// layout reflows the grid, and the sidebars change height as they fill.
if (typeof ResizeObserver !== 'undefined') {
  new ResizeObserver(resize).observe(viewport);
}
resize();

// A fitted camera that snaps into place reads as a glitch; one short
// decelerating move reads as the instrument finding the model. Orbit
// control is suspended for the duration so damping does not fight it.
interface CameraTween {
  fromPos: THREE.Vector3;
  toPos: THREE.Vector3;
  fromTarget: THREE.Vector3;
  toTarget: THREE.Vector3;
  startedAt: number;
  durationMs: number;
}
let cameraTween: CameraTween | null = null;

function advanceCameraTween(now: number): void {
  if (!cameraTween) return;
  const raw = (now - cameraTween.startedAt) / cameraTween.durationMs;
  const t = raw >= 1 ? 1 : raw;
  // Ease-out cubic — fast departure, long settle, no overshoot.
  const eased = 1 - Math.pow(1 - t, 3);
  camera.position.lerpVectors(cameraTween.fromPos, cameraTween.toPos, eased);
  controls.target.lerpVectors(cameraTween.fromTarget, cameraTween.toTarget, eased);
  if (t === 1) {
    cameraTween = null;
    controls.enabled = true;
  }
}

function tick(now: number = performance.now()): void {
  advanceCameraTween(now);
  controls.update();
  renderer.render(scene, camera);
  requestAnimationFrame(tick);
}
tick();

// ---------- Raycasting for element picking ----------
const raycaster = new THREE.Raycaster();
const pointer = new THREE.Vector2();

/** Whether `object` and everything above it is visible. */
function isDrawn(object: THREE.Object3D): boolean {
  for (let at: THREE.Object3D | null = object; at; at = at.parent) {
    if (!at.visible) return false;
  }
  return true;
}

/**
 * The entity drawn under a pointer position, if any: for a piece folded into
 * another element's body, the element that owns it.
 */
function pickEntity(ev: MouseEvent): number | null {
  if (!currentModel) return null;
  const rect = renderer.domElement.getBoundingClientRect();
  pointer.x = ((ev.clientX - rect.left) / rect.width) * 2 - 1;
  pointer.y = -((ev.clientY - rect.top) / rect.height) * 2 + 1;
  raycaster.setFromCamera(pointer, camera);
  // A raycast does not look at `visible`, so a hidden category would take the
  // click meant for what is drawn behind it.
  const hit = raycaster.intersectObject(currentModel, true).find((h) => isDrawn(h.object));
  if (!hit) return null;
  const userData = hit.object.userData as { entityIndex?: number };
  // A click on any loop of an element of separate loops picks the element.
  return userData.entityIndex === undefined ? null : writtenElementOf(userData.entityIndex);
}

// A click selects; a drag orbits without changing the selection.
let pointerDownAt: { x: number; y: number } | null = null;
const CLICK_SLOP_PX = 4;
renderer.domElement.addEventListener('pointerdown', (ev) => {
  pointerDownAt = ev.button === 0 ? { x: ev.clientX, y: ev.clientY } : null;
});
renderer.domElement.addEventListener('pointerup', (ev) => {
  const down = pointerDownAt;
  pointerDownAt = null;
  if (!down || Math.hypot(ev.clientX - down.x, ev.clientY - down.y) > CLICK_SLOP_PX) return;
  const idx = pickEntity(ev);
  if (idx !== null) selectEntity(idx);
});
// A double-click also brings the element into view.
renderer.domElement.addEventListener('dblclick', (ev) => {
  const idx = pickEntity(ev);
  if (idx === null) return;
  selectEntity(idx);
  frameEntities(new Set([idx]), entityLabel(idx));
});

// ---------- Selection highlight (M4-04) ----------
const highlightMaterial = new THREE.MeshStandardMaterial({ side: THREE.DoubleSide });
applySceneTheme();
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', applySceneTheme);
let highlighted: Array<{
  mesh: THREE.Mesh;
  material: THREE.Material | THREE.Material[];
}> = [];

function clearHighlight(): void {
  for (const entry of highlighted) entry.mesh.material = entry.material;
  highlighted = [];
  activeTypeHighlight = null;
  activeMaterialHighlight = null;
}

/**
 * Tint every mesh the predicate accepts. The highlight set has
 * always been a list, so lighting a whole IFC type or a whole
 * material costs nothing beyond a wider predicate.
 */
function highlightWhere(match: (data: MeshIdentity) => boolean): number {
  clearHighlight();
  if (!currentModel) return 0;
  currentModel.traverse((obj) => {
    const mesh = obj as THREE.Mesh;
    if (!mesh.isMesh) return;
    if (!match(obj.userData as MeshIdentity)) return;
    highlighted.push({ mesh, material: mesh.material });
    mesh.material = highlightMaterial;
  });
  return highlighted.length;
}

interface MeshIdentity {
  entityIndex?: number;
  ifcType?: string;
}

/** Which IFC type the schedule is currently lighting up, if any. */
let activeTypeHighlight: string | null = null;
/** Which material index the info panel is currently lighting up. */
let activeMaterialHighlight: number | null = null;

/**
 * Tint every mesh of element `idx` and of the pieces folded into its body, so
 * the scene agrees with the tree.
 */
function highlightEntity(idx: number): void {
  const indices = withBodyPieces([idx]);
  highlightWhere((data) => data.entityIndex !== undefined && indices.has(data.entityIndex));
}

/**
 * The pieces the IFC export folds into the body of element `owner` (#823). The
 * model lists the pieces of every owner; the export folds a piece only where
 * the owner's body and the piece's are both plain, and a piece with a body of
 * its own stays an element with its own tree row. The tree lists exactly the
 * elements the export writes, so a listed piece is not folded.
 */
function bodyPiecesOf(owner: number): number[] {
  const pieces = model?.body_pieces;
  const all = pieces instanceof Map ? pieces.get(owner) : pieces?.[String(owner)];
  return (all ?? []).filter((piece) => !listedEntities.has(piece));
}

/**
 * The element the IFC export writes for the drawn entity `idx`: `idx` itself
 * where the tree lists it, else the element that owns it where it is a piece
 * folded into that element's body. The info panel says the same in
 * `entity_index`.
 */
function writtenElementOf(idx: number): number {
  if (listedEntities.has(idx)) return idx;
  const pieces = model?.body_pieces;
  const owners: Array<[number | string, number[]]> =
    pieces instanceof Map ? [...pieces] : Object.entries(pieces ?? {});
  for (const [owner, list] of owners) {
    if (list.includes(idx)) return Number(owner);
  }
  return idx;
}

/**
 * `indices` and the pieces folded into the body of any owner among them. The
 * 3-D view draws each piece as a mesh of its own, but the tree, the panel and
 * the schedule know the owner only, so a selection, a highlight or a zoom of
 * the owner has to cover them.
 */
function withBodyPieces(indices: Iterable<number>): Set<number> {
  const out = new Set<number>(indices);
  for (const idx of [...out]) {
    for (const piece of bodyPiecesOf(idx)) out.add(piece);
  }
  return out;
}

/** Tint every mesh of one IFC type — the schedule's "show in view". */
function highlightIfcType(ifcType: string): number {
  const lit = highlightWhere((data) => data.ifcType === ifcType);
  activeTypeHighlight = lit > 0 ? ifcType : null;
  return lit;
}

/** Tint every element sharing a material — the info panel material row. */
function highlightByMaterial(materialIndex: number): number {
  const indices = entityIndicesWithMaterial(materialIndex);
  const lit = highlightWhere(
    (data) => data.entityIndex !== undefined && indices.has(data.entityIndex),
  );
  activeMaterialHighlight = lit > 0 ? materialIndex : null;
  return lit;
}

/**
 * Entities associated with a material, directly or as one of their
 * layers (RE-58). The model is already in memory on this thread, so the
 * lookup needs no worker round-trip.
 */
function entityIndicesWithMaterial(materialIndex: number): Set<number> {
  const out = new Set<number>();
  const sets = model?.material_layer_sets ?? [];
  const setsWithMaterial = new Set<number>();
  sets.forEach((set, index) => {
    if (set.layers.some((layer) => layer.material_index === materialIndex)) {
      setsWithMaterial.add(index);
    }
  });
  const entities = model?.entities ?? [];
  for (let i = 0; i < entities.length; i += 1) {
    const entity = entities[i];
    // The export associates an IfcSpace with no material (#1430), so a space
    // is not among the elements the panel counts and this lights up.
    if (entity?.ifc_type?.toUpperCase() === 'IFCSPACE') continue;
    if (entity?.material_index === materialIndex) out.add(i);
    const set = entity?.material_layer_set_index;
    if (set !== undefined && set !== null && setsWithMaterial.has(set)) out.add(i);
  }
  return out;
}

/** `true` once the scene carries pickable, tintable element meshes. */
function sceneSupportsHighlight(): boolean {
  if (!currentModel) return false;
  let found = false;
  currentModel.traverse((obj) => {
    if (found) return;
    if (!(obj as THREE.Mesh).isMesh) return;
    if ((obj.userData as MeshIdentity).entityIndex !== undefined) found = true;
  });
  return found;
}

/**
 * Single entry point for "select this element": tree row, 3-D
 * highlight, and the info panel (including its host relationships).
 * Host / hosted rows in the panel call this to jump across the
 * relationship.
 */
/** The element the tree and info panel describe, for F and the zoom button. */
let selectedIndex: number | null = null;

function selectEntity(idx: number): void {
  clearScheduleHighlightState();
  selectTreeRow(idx);
  highlightEntity(idx);
  showElementInfo(idx);
  selectedIndex = idx;
}

/** An entity's name as the tree shows it. */
function entityLabel(idx: number): string {
  return model?.entities?.[idx]?.name ?? findSceneNodeByIndex(sceneGraph, idx)?.name ?? 'element';
}

/** Every entity index under a tree row, the row's own included. */
function entityIndicesUnder(item: TreeItem): Set<number> {
  const out = new Set<number>();
  const walk = (n: TreeItem) => {
    if (n.entityIndex !== null) out.add(n.entityIndex);
    n.children.forEach(walk);
  };
  walk(item);
  return withBodyPieces(out);
}

/**
 * Bring some elements into view: F, a double-click in the tree or the
 * view, or the element panel's button. Reports when none of them has
 * drawn geometry rather than moving the camera to nothing.
 */
function frameEntities(indices: Set<number>, label: string): boolean {
  if (!currentModel) return false;
  const wanted = withBodyPieces(indices);
  const box = new THREE.Box3();
  currentModel.traverse((obj) => {
    const idx = (obj.userData as { entityIndex?: number }).entityIndex;
    if ((obj as THREE.Mesh).isMesh && idx !== undefined && wanted.has(idx)) {
      box.expandByObject(obj);
    }
  });
  if (box.isEmpty()) {
    setStatus(`${label} has no geometry to zoom to`);
    return false;
  }
  frameBox(box);
  setStatus(`Zoomed to ${label}`);
  return true;
}

function selectTreeRow(idx: number): void {
  document
    .querySelectorAll('.tree-node.selected')
    .forEach((el) => el.classList.remove('selected'));
  // The row may not be built yet: the tree builds a group's rows a block at a time.
  const item = treeItemByEntity.get(idx);
  if (item) ensureTreeRow(item);
  const row = treeEl.querySelector<HTMLElement>(`.tree-node[data-entity-index="${idx}"]`);
  if (!row) return;
  revealTreeRow(row);
  row.classList.add('selected');
  row.scrollIntoView({
    block: 'nearest',
    behavior: prefersReducedMotion() ? 'auto' : 'smooth',
  });
  focusTreeRow(row);
}

// ---------- Status ----------
function setStatus(text: string): void {
  statusEl.textContent = text;
}

/** Marks the status line as actively working (drives the sweep hairline). */
function setStatusBusy(busy: boolean): void {
  statusEl.classList.toggle('is-busy', busy);
}

// ---------- Transient confirmations ----------
type ToastKind = 'ok' | 'bad' | 'info';

/**
 * Non-blocking confirmation for the three moments worth interrupting
 * for: a load finished, an export downloaded, a load failed. #status
 * keeps the running commentary; this marks the transitions.
 */
function toast(message: string, kind: ToastKind = 'info'): void {
  const el = document.createElement('div');
  el.className = `toast ${kind}`;
  el.textContent = message;
  toastRegionEl.appendChild(el);

  const lifetimeMs = kind === 'bad' ? 7000 : 4500;
  window.setTimeout(() => {
    el.classList.add('leaving');
    window.setTimeout(() => el.remove(), prefersReducedMotion() ? 0 : 200);
  }, lifetimeMs);
}

// ---------- Viewport loading treatment ----------
/**
 * What the loading card says before the decoder has reported anything, and
 * all it says for a file whose stages are too quick to show: the throughput
 * observed when the 33.7 MB project decoded in about 3 s on the deployed site
 * (a laptop's; Deploy viewer's runner takes about 24 s for it). Once the
 * decoder reports a stage (#1195) the card shows that stage's own counts.
 */
const DECODE_BYTES_PER_SECOND = 11e6;

function loadExpectation(bytes: number): string {
  const size = formatBytes(bytes);
  if (bytes < 256 * 1024) return `${size}, usually instant`;
  const raw = bytes / DECODE_BYTES_PER_SECOND;
  const seconds = raw < 10 ? Math.max(1, Math.round(raw)) : Math.round(raw / 5) * 5;
  return `${size}, about ${seconds} s`;
}

/** The most memory a wasm32 instance can address, and so a tab can give the decoder. */
const WASM32_CEILING_BYTES = 4 * 2 ** 30;

/**
 * The multiple of its file's size an open has been measured to take at least:
 * Core Interior, the largest reference model (32 MiB), peaks at 324 MiB, 10.1
 * times, and the smaller ones 13 to 27 times (#1150, and the estimate columns
 * of tests/saved-meshes-measure.spec.ts). The multiple falls as files grow,
 * and the only larger evidence, native runs on two client models (#1075), is
 * 8.4 and 4.5 times, so for a file of hundreds of MiB 10 times can be too
 * much: the warning can fire on a file that fits, and the stop is the verdict.
 */
const PEAK_MEMORY_PER_FILE_BYTE = 10;

/** `?memoryCeilingBytes=` lowers the ceiling the warning compares with, for the browser test; a browser capped the same way (V8's `--wasm-max-mem-pages`) then really stops there. */
const memoryCeilingBytes =
  wholeBytesParam(new URLSearchParams(window.location.search), 'memoryCeilingBytes') ??
  WASM32_CEILING_BYTES;

function estimatePeakBytes(fileBytes: number): number {
  return fileBytes * PEAK_MEMORY_PER_FILE_BYTE;
}

/** Memory and file sizes in the binary units the 4 GiB ceiling is spoken in. */
function formatMemory(bytes: number): string {
  const trim = (value: number): number => Number(value.toFixed(1));
  if (bytes >= 2 ** 30) return `${trim(bytes / 2 ** 30)} GiB`;
  if (bytes >= 2 ** 20) return `${trim(bytes / 2 ** 20)} MiB`;
  return `${trim(bytes / 2 ** 10)} KiB`;
}

/**
 * What reads a file the browser cannot, as the command line and the Python
 * package really take it (`rvt::cli::open_limits`, `rvt::reader::OpenLimits`,
 * `rvt-py`): `--large-model` on rvt-ifc and rvt-gltf reads streams up to 2 GiB
 * and, on a 64-bit machine, inflates up to 8 GiB. Neither has a flag for
 * `max_file_bytes`, the 2 GiB cap on the whole file, so a larger file is
 * rejected whatever the flags. Python's `RevitFile` has no preset, only
 * `max_stream_bytes`, `max_inflate_bytes` and `max_file_bytes` to raise. All of
 * it needs a machine with the memory.
 */
const LARGER_FILE_ADVICE =
  'rvt-ifc and rvt-gltf with --large-model read streams up to 2 GiB and, on a 64-bit machine with the memory, inflate up to 8 GiB; ' +
  'they have no flag for the 2 GiB cap on the whole file, so a larger file is rejected whatever the flags. ' +
  'The Python package opens a file with a larger max_stream_bytes and max_inflate_bytes, and max_file_bytes for one over 2 GiB.';

/** The decoder's stages (`rvt::control::Stage`) as the card names them, and how it counts each. */
const DECODE_STAGES: Record<string, { label: string; count: (done: number, total: number) => string }> = {
  'schema-parse': { label: 'Reading the schema', count: () => '' },
  'candidate-scan': {
    label: 'Scanning for elements',
    count: (done, total) => `${formatMemory(done)} of ${formatMemory(total)}`,
  },
  'element-decode': {
    label: 'Decoding elements',
    count: (done, total) =>
      `${done.toLocaleString('en-US')} of ${total.toLocaleString('en-US')} candidates`,
  },
  'partition-scan': {
    label: 'Reading partition streams',
    count: (done, total) => `${done} of ${total} streams`,
  },
  'saved-graphics': { label: 'Reading saved graphics', count: () => '' },
  'saved-meshes': {
    label: 'Drawing family instances from saved meshes',
    count: (done, total) => `${done.toLocaleString('en-US')} of ${total.toLocaleString('en-US')} elements`,
  },
};

/**
 * What the decoder is doing after these stages end, when it reports nothing
 * until the next one: the walk's last stage hands over to building the
 * model, and the saved meshes to the diagnostics and the copy out of wasm.
 */
const AFTER_STAGE: Record<string, string> = {
  'partition-scan': 'Building the model',
  'saved-meshes': 'Finishing the model',
};

/**
 * What the card says for a decoder report, and whether it has a count to fill
 * the bar with. A stage that has ended and has something after it names that
 * instead, and sweeps. The failure message names the same thing, so a stop in
 * the silent stretch says it was building the model.
 */
function stageStep(
  stage: string,
  done: number,
  total: number | null,
): { label: string; counted: boolean } {
  const finished = total !== null && done >= total;
  const after = finished ? AFTER_STAGE[stage] : undefined;
  return {
    label: after ?? DECODE_STAGES[stage]?.label ?? stage,
    counted: !after && total !== null && total > 1,
  };
}

/** One thing the worker told the card, with the milliseconds since the open began. */
interface LoadEvent {
  t: number;
  stage?: string;
  done?: number;
  total?: number | null;
  /** The wasm linear memory at that moment. */
  memoryBytes?: number;
  step?: string;
  /** What the card showed once it had applied this report: tests read it instead of sampling the page. */
  card?: { step: string; value: number | null; hint: string };
}

let loadStartedAt = 0;
/** What the card says until the decoder first reports: the size-derived estimate. */
let loadExpectationText = '';
let loadTimer: number | null = null;
let loadEvents: LoadEvent[] = [];
let loadStage: string | null = null;
/** The last thing said on #status, a live region, so each is said once. */
let loadAnnounced = '';
/** Everything `announceLoad` has said on #status this open, in order, kept for the tests. */
let loadAnnouncements: string[] = [];
let loadMemoryWarned = false;
/** How many of each `keepLast` has dropped this open, so a record that was cut says so. */
let loadEventsDropped = 0;
let loadAnnouncementsDropped = 0;

/**
 * The most reports and announcements the page keeps of one open, for the tests
 * and the measurement; the oldest go first (#1887). The binding hands the page
 * one report per whole percent of a stage (`forward_progress` in
 * rvt-wasm/src/lib.rs), a few hundred in an open, so a file does not reach
 * this: it bounds what the page holds, and writes into one attribute, if the
 * decoder reports more often than that.
 */
const LOAD_RECORDS_KEPT = 2000;

/**
 * Adds `item` to `list`, which keeps its last `limit` items: it is cut in one
 * splice when it reaches twice that, so a push stays cheap. Returns how many
 * items the push dropped.
 */
function keepLast<T>(list: T[], item: T, limit: number): number {
  list.push(item);
  if (list.length < 2 * limit) return 0;
  const dropped = list.length - limit;
  list.splice(0, dropped);
  return dropped;
}

/** Says `text` on #status unless it was the last thing said. */
function announceLoad(text: string): void {
  if (text === loadAnnounced) return;
  loadAnnounced = text;
  loadAnnouncementsDropped += keepLast(loadAnnouncements, text, LOAD_RECORDS_KEPT);
  setStatus(text);
}

/** A determinate bar at `fraction`, or the indeterminate sweep for null. */
function setLoadFraction(fraction: number | null): void {
  if (fraction === null) {
    loadRailEl.classList.remove('is-determinate');
    loadRailEl.style.removeProperty('--load-fraction');
    loadRailEl.removeAttribute('aria-valuenow');
    return;
  }
  const clamped = Math.min(1, Math.max(0, fraction));
  loadRailEl.classList.add('is-determinate');
  loadRailEl.style.setProperty('--load-fraction', clamped.toFixed(3));
  loadRailEl.setAttribute('aria-valuenow', String(Math.round(clamped * 100)));
}

/**
 * The decoder reported `done` of `total` in `stage`. The bar is determinate
 * while the stage has a count to be a fraction of (a stage that reports 0 and
 * then 1 of 1 has none), and sweeps otherwise.
 */
function onDecodeStage(stage: string, done: number, total: number | null, memoryBytes: number): void {
  const { label, counted } = stageStep(stage, done, total);
  const info = DECODE_STAGES[stage];
  setLoadStep(label);
  setLoadFraction(counted && total !== null ? done / total : null);
  loadHintEl.textContent =
    counted && info && total !== null
      ? [`${Math.round((done / total) * 100)}%`, info.count(done, total)].filter(Boolean).join(' · ')
      : '';
  const value = loadRailEl.getAttribute('aria-valuenow');
  loadEventsDropped += keepLast(
    loadEvents,
    {
      t: Math.round(performance.now() - loadStartedAt),
      stage,
      done,
      total,
      memoryBytes,
      card: {
        step: loadStepEl.textContent ?? '',
        value: value === null ? null : Number(value),
        hint: loadHintEl.textContent ?? '',
      },
    },
    LOAD_RECORDS_KEPT,
  );
  announceLoad(label);
  loadStage = stage;
}

/** What the worker said, kept on #status for tests/load-progress.spec.ts and the measurement. */
function saveLoadEvents(): void {
  statusEl.dataset.loadProgress = JSON.stringify(loadEvents.slice(-LOAD_RECORDS_KEPT));
  statusEl.dataset.loadAnnounced = JSON.stringify(loadAnnouncements.slice(-LOAD_RECORDS_KEPT));
  // Written only when a record was cut, so a test that finds it knows the list is not whole.
  const dropped =
    loadEventsDropped + Math.max(0, loadEvents.length - LOAD_RECORDS_KEPT);
  const droppedAnnouncements =
    loadAnnouncementsDropped + Math.max(0, loadAnnouncements.length - LOAD_RECORDS_KEPT);
  if (dropped > 0) statusEl.dataset.loadProgressDropped = String(dropped);
  else delete statusEl.dataset.loadProgressDropped;
  if (droppedAnnouncements > 0) statusEl.dataset.loadAnnouncedDropped = String(droppedAnnouncements);
  else delete statusEl.dataset.loadAnnouncedDropped;
}

/**
 * Says before the decode when the file is likely to need more memory than a
 * tab has (#1195). It is an estimate from the file's size alone, so it
 * warns and the open goes ahead; the worker reports the real stop.
 */
function showMemoryWarning(file: File): void {
  const estimate = estimatePeakBytes(file.size);
  loadMemoryWarned = estimate >= memoryCeilingBytes;
  loadWarningEl.classList.toggle('hidden', !loadMemoryWarned);
  if (!loadMemoryWarned) return;
  loadWarningEl.textContent =
    `This file may be too large for the browser. An open has taken at least about ${PEAK_MEMORY_PER_FILE_BYTE} times its file's size in memory ` +
    `(the largest reference model took 10 times, smaller ones up to 27), so about ${formatMemory(estimate)} for ${formatMemory(file.size)}, ` +
    `past the ${formatMemory(memoryCeilingBytes)} a browser tab can hold. Larger files may take less per byte, so it is tried anyway. ` +
    'rvt-ifc and rvt-gltf with --large-model can read a larger one.';
  toast(`${file.name} may be too large for the browser. Trying anyway.`, 'info');
}

function startLoadOverlay(file: File): void {
  loadStartedAt = performance.now();
  loadEvents = [];
  loadStage = null;
  loadAnnounced = '';
  loadAnnouncements = [];
  loadEventsDropped = 0;
  loadAnnouncementsDropped = 0;
  delete statusEl.dataset.loadProgress;
  delete statusEl.dataset.loadAnnounced;
  delete statusEl.dataset.loadProgressDropped;
  delete statusEl.dataset.loadAnnouncedDropped;
  setLoadFraction(null);
  loadStepEl.textContent = `Opening ${file.name}`;
  loadExpectationText = loadExpectation(file.size);
  loadHintEl.textContent = loadExpectationText;
  loadElapsedEl.textContent = '0 s elapsed';
  loadOverlayEl.classList.remove('hidden');
  showMemoryWarning(file);
  // The options apply to the read in progress; a change reopens the file.
  savedMeshesEl.disabled = true;
  exportPhaseEl.disabled = true;
  setStatusBusy(true);
  if (loadTimer !== null) window.clearInterval(loadTimer);
  loadTimer = window.setInterval(() => {
    const seconds = Math.floor((performance.now() - loadStartedAt) / 1000);
    loadElapsedEl.textContent = `${seconds} s elapsed`;
  }, 250);
}

function setLoadStep(step: string): void {
  loadStepEl.textContent = step;
}

function stopLoadOverlay(): void {
  if (loadTimer !== null) {
    window.clearInterval(loadTimer);
    loadTimer = null;
  }
  loadOverlayEl.classList.add('hidden');
  loadWarningEl.classList.add('hidden');
  savedMeshesEl.disabled = false;
  exportPhaseEl.disabled = exportPhaseEl.options.length < 2;
  setStatusBusy(false);
}

// ---------- Scaffold-only empty state ----------
function hideScaffoldNote(): void {
  scaffoldNoteEl.classList.add('hidden');
  scaffoldNoteEl.classList.remove('is-failure');
}

/**
 * A scaffold decode produces a legal scene graph with no drawable
 * geometry. Without this the viewport is a bare grid and the result
 * looks broken rather than expected.
 */
function maybeShowScaffoldNote(diagnostics: ExportDiagnostics): void {
  const geometry = diagnostics.exported?.building_elements_with_geometry ?? 0;
  if (geometry > 0) {
    hideScaffoldNote();
    return;
  }
  scaffoldNoteTitleEl.textContent = 'Nothing to draw';
  if (unreadStreamCount(diagnostics) > 0) {
    scaffoldNoteBodyEl.textContent = largeModelRetryOffered(diagnostics)
      ? 'Part of the file was not read, so the model is missing what those streams hold. The File status panel can retry with large-model limits.'
      : 'Part of the file was not read even with the large-model limits, so the model is missing what those streams hold.';
    scaffoldNoteEl.classList.remove('hidden');
    return;
  }
  const level = diagnostics.confidence?.level ?? 'unknown';
  const score = diagnostics.confidence?.score;
  const pct = typeof score === 'number' ? ` at ${Math.round(score * 100)}% confidence` : '';
  const lines = [
    `This file decoded as ${exportQualityLabel(level).toLowerCase()}${pct} — schema, project metadata and a spatial scaffold, but no element geometry to draw.`,
  ];
  if (level === 'scaffold' || level === 'proxy_only') {
    lines.push(
      'A scaffold decode is the expected result for the synthetic tier1 demos, not a failure. The status panel, diagnostics and export bars all still work on it.',
    );
  }
  scaffoldNoteBodyEl.textContent = lines.join(' ');
  scaffoldNoteEl.classList.remove('hidden');
}

/** Reopens the gallery and puts the keyboard on the first real project. */
function revealRealProjects(): void {
  hideScaffoldNote();
  setDropzoneVisible(true);
  const card = demoListEl.querySelector<HTMLElement>('[data-demo-real="true"]');
  if (!card) {
    setStatus('Real-project demos are not staged in this build');
    return;
  }
  card.scrollIntoView({
    block: 'nearest',
    behavior: prefersReducedMotion() ? 'auto' : 'smooth',
  });
  card.focus();
  card.classList.add('is-flagged');
  window.setTimeout(() => card.classList.remove('is-flagged'), 2400);
}

showRealProjectsBtn.addEventListener('click', revealRealProjects);
dismissScaffoldNoteBtn.addEventListener('click', () => {
  hideScaffoldNote();
  viewport.focus();
});

// ---------- Worker ----------
type Worker_ = Worker & {
  postMessage: (data: unknown, transfer?: Transferable[]) => void;
};
let worker: Worker_ | null = null;
/** Ends the worker, and with it the wasm instance and the memory it held. */
function stopWorker(): void {
  if (worker) worker.terminate();
  worker = null;
}
/**
 * Where the decoder's wasm is, resolved against this script (a worker started
 * from a blob URL has no base to resolve it against). Only the first open
 * fetches it: the worker compiles it and hands the module to the page
 * (`decoderWasm`), and every later open, and every worker that reads saved
 * parameters, starts from that module.
 */
const wasmUrl = new URL('../pkg/rvt_bg.wasm', import.meta.url).href;
function resetWorker(): Worker_ {
  stopWorker();
  // Bundled into this script and started from a blob URL, so that opening the
  // file again (another phase, the saved-mesh toggle, the large-model retry)
  // fetches no worker script: nothing is requested once the page has loaded and
  // the first file is open.
  const w = new DecoderWorker({ name: 'decoder' }) as Worker_;
  worker = w;
  return w;
}

// ---------- Model / scene-graph state ----------
interface IfcModel {
  project_name?: string;
  description?: string;
  building_storeys?: Array<{ name: string; elevation_feet?: number }>;
  materials?: Array<{ name: string; color_packed?: number; transparency?: number }>;
  material_layer_sets?: Array<{ layers: Array<{ material_index?: number | null }> }>;
  /**
   * Revit's own GlobalIds by entity index (RE-48). serde-wasm-bindgen hands
   * a Rust map over as a `Map`; JSON gives an object keyed by string.
   */
  global_ids?: { elements?: Map<number, string> | Record<string, string> };
  /**
   * The further pieces of an element of separate loops that the IFC export
   * writes as items of the owner's body, by the owner's entity index (#823).
   * The pieces stay in `entities` and are drawn, as a `Map` from
   * serde-wasm-bindgen and an object keyed by string in JSON. The export
   * folds only the pieces whose body and the owner's are both plain: the
   * scene graph lists the others.
   */
  body_pieces?: Map<number, number[]> | Record<string, number[]>;
  entities?: Array<{
    kind?: string;
    name: string;
    ifc_type: string;
    guid?: string;
    /** The Revit ElementId, which the IFC export writes as the element's Tag. */
    type_guid?: string | null;
    material_index?: number | null;
    material_layer_set_index?: number | null;
    /** `Aggregate` entities: the whole and its parts (RE-39, RE-46). */
    whole?: number;
    parts?: number[];
  }>;
}
interface RelatedElement {
  entity_index: number;
  name: string;
  ifc_type: string;
}
/** One end of a wall that butt-joins another (RE-70) or stops against it at a T joint (RE-73). */
interface PanelJoin {
  end: string;
  runs_through: boolean;
  wall: RelatedElement;
}
interface PanelRow {
  label: string;
  value: string;
}
interface PanelProperty {
  name: string;
  value: string;
  kind: string;
  numeric: boolean;
}
interface PanelPropertyGroup {
  name: string;
  properties: PanelProperty[];
}
interface PanelStorey {
  index: number;
  name: string;
  elevation_feet: number;
  elevation_label: string;
}
interface PanelMaterial {
  index: number;
  name: string;
  element_count: number;
}
interface PanelLayer {
  label: string;
  value: string;
  material?: PanelMaterial | null;
}
interface PanelLayers {
  name: string;
  order?: string | null;
  rows: PanelLayer[];
  total: string;
}
interface ElementInfoPanel {
  /**
   * The element the panel describes: the one asked for or, for a piece the
   * export folds into another element's body, the element that owns it.
   */
  entity_index?: number;
  name: string;
  ifc_type: string;
  type_guid?: string | null;
  predefined_type?: string | null;
  storey?: PanelStorey | null;
  placement_rows?: PanelRow[];
  extent_rows?: PanelRow[];
  material?: PanelMaterial | null;
  layers?: PanelLayers | null;
  property_group?: PanelPropertyGroup | null;
  further_property_groups?: PanelPropertyGroup[];
  host?: RelatedElement | null;
  hosted?: RelatedElement[];
  joins?: PanelJoin[];
  missing?: string[];
}
interface ScheduleTypeGroup {
  ifc_type: string;
  count: number;
  entity_indices: number[];
  storeys: string[];
}
interface Schedule {
  rows?: unknown[];
  groups?: ScheduleTypeGroup[];
}
interface SceneNode {
  name: string;
  ifc_type: string;
  entity_index: number | null;
  storey_index?: number | null;
  children: SceneNode[];
}
interface ExportDiagnostics {
  schema_version?: number;
  mode?: string;
  input?: {
    revit_version?: number;
    original_path?: string;
    project_name?: string;
    stream_count?: number;
    has_basic_file_info?: boolean;
    has_part_atom?: boolean;
    has_formats_latest?: boolean;
    has_global_latest?: boolean;
    /** How the file's release is read (#871): its support-matrix row and that row's status. */
    release_support?: { status: string; capability: string | null } | null;
  };
  decoded?: {
    production_walker_elements?: number;
    diagnostic_proxy_candidates?: number;
    arcwall_records?: number;
    production_class_counts?: Record<string, number>;
    parameter_value_count?: number;
    mean_element_confidence?: number | null;
    elements_below_min_confidence?: number;
    min_element_confidence?: number;
  };
  confidence?: {
    level?: string;
    score?: number;
    has_project_metadata?: boolean;
    has_typed_elements?: boolean;
    has_geometry?: boolean;
    has_diagnostic_proxies?: boolean;
    warning_count?: number;
    /** RE-30 records found but not exported, in records of ElementIds the file does not declare; non-zero means the model may be incomplete. */
    unexported_element_records?: number;
  };
  /** A10 — measured when denominators exist; fractions stay null when unknown. */
  source_coverage?: {
    status?: 'unset' | 'unknown' | 'measured';
    notes?: string;
    decoded_element_fraction?: number | null;
    exported_element_fraction?: number | null;
    geometry_element_fraction?: number | null;
  };
  exported?: {
    building_elements?: number;
    building_elements_with_geometry?: number;
    storey_count?: number;
    material_count?: number;
    unit_assignment_count?: number;
    storey_names?: string[];
    material_names_sample?: string[];
    layered_element_count?: number;
    body_sources?: Record<string, number>;
    bounding_box_bodies?: number;
  };
  unsupported_features?: string[];
  skipped?: { reason?: string; count?: number; sample_names?: string[]; phase?: string }[];
  /** The rooms of an earlier phase written because the last phase holds none (#1359). */
  room_phase_fallback?: { phase?: string; rooms?: number };
  warnings?: string[];
  formats_latest_integrity?: {
    stream?: string;
    stored_bytes?: number;
    inflated_bytes?: number | null;
    page_boundary_detected?: boolean;
    checksum_tail_stripping?: string;
    integrity_status?: string;
    diagnostic_code?: string | null;
  };
}

let model: IfcModel | null = null;
let sceneGraph: SceneNode | null = null;
/** The entity indices of the nodes of `sceneGraph`: the elements the IFC export writes. */
let listedEntities = new Set<number>();
let distinctTypes: string[] = [];
let lastGlb: Uint8Array | null = null;
/** Retained so the schedule can re-render once the GLB is in the scene. */
let lastSchedule: Schedule | null = null;
let lastFileStem = 'model';
let currentDiagnostics: ExportDiagnostics | null = null;
const hiddenTypes = new Set<string>();

function selectedExportMode(): string {
  return exportModeEl.value || 'scaffold';
}

// ---------- Resource limits (#1075) ----------
/**
 * The options of one open, as the wasm binding
 * `openRvtBytesWithDiagnosticsModeAndOpenLimits` takes them: absent fields
 * keep the defaults, `largeModel` is `rvt-ifc --large-model`, an explicit
 * byte limit wins over it, and `savedMeshes` draws every element that would be
 * a box from the mesh Revit saved for it. The limits are the file's own, kept in
 * `currentOpenLimits`; `savedMeshes` follows the toggle on every open, so
 * `loadBytes` adds it.
 */
interface OpenLimitsOptions {
  largeModel?: boolean;
  maxStreamBytes?: number;
  maxInflateBytes?: number;
  savedMeshes?: boolean;
}

/** The limits the "Retry with large-model limits" button reopens the file with. */
const LARGE_MODEL_LIMITS: OpenLimitsOptions = { largeModel: true };

/** How each export warning about a stream not read whole begins (rvt::ifc::record_unread_streams). */
const UNREAD_STREAM_WARNING = 'Not read whole: ';

function wholeBytesParam(params: URLSearchParams, name: string): number | undefined {
  const raw = params.get(name);
  if (raw === null || !/^\d+$/.test(raw)) return undefined;
  const bytes = Number(raw);
  return Number.isSafeInteger(bytes) ? bytes : undefined;
}

/**
 * `?maxStreamBytes=` and `?maxInflateBytes=` open the first file under those
 * limits instead of the defaults, so a small file can reach the "Part of the
 * file not read" path the way a client model over 256 MiB does. The browser
 * test uses it on the 20 KiB tier-1 fixture, whose 236-byte BasicFileInfo a
 * 200-byte stream limit stops. Anything else in the query is ignored.
 */
function limitsFromQuery(): OpenLimitsOptions | undefined {
  const params = new URLSearchParams(window.location.search);
  const limits: OpenLimitsOptions = {};
  const maxStreamBytes = wholeBytesParam(params, 'maxStreamBytes');
  if (maxStreamBytes !== undefined) limits.maxStreamBytes = maxStreamBytes;
  const maxInflateBytes = wholeBytesParam(params, 'maxInflateBytes');
  if (maxInflateBytes !== undefined) limits.maxInflateBytes = maxInflateBytes;
  return Object.keys(limits).length > 0 ? limits : undefined;
}

/** Limits for the first open only; the query describes that file, not every later one. */
let firstOpenLimits: OpenLimitsOptions | undefined = limitsFromQuery();

/**
 * `?savedMeshes=0` (or `false`) starts the toggle off, so every open draws
 * family instances as their boxes until the toggle is turned on. Unlike the
 * limits it is not for the first open only: the toggle shows what applies, so
 * it must not turn itself back on after one file.
 */
function savedMeshesFromQuery(): boolean {
  const raw = new URLSearchParams(window.location.search).get('savedMeshes');
  return raw !== '0' && raw !== 'false';
}
savedMeshesEl.checked = savedMeshesFromQuery();

/**
 * One phase of the project (rvt-wasm's `phases`): its ElementId and, where the
 * file names it, its name.
 */
interface ProjectPhase {
  id: number;
  name?: string | null;
}

/**
 * The phase the open file is shown at: a phase's ElementId as text, or '' for
 * the project's last, which is the default (`rvt-ifc` without `--phase`).
 */
let currentPhase = '';

/**
 * The most phases the select lists (#1887). A project has a handful; a file
 * that names more has its first ones listed and its last, the default, and the
 * hint says how many there are.
 */
const PHASE_OPTIONS_SHOWN = 200;
/** The hint as the page ships it, which `renderPhases` adds a sentence to for a file with more phases than are listed. */
const exportPhaseHintText = exportPhaseHintEl.textContent ?? '';

/**
 * Lists the file's phases in the select, in the project's order, the last as
 * the default. A file with no phases (a family, or a release whose phases are
 * not read) hides the field; a project with one shows it fixed. Where the
 * default shows the rooms of an earlier phase because the last holds none
 * (`fallbackPhase`, the diagnostics' `room_phase_fallback`), the last says so.
 */
function renderPhases(phases: ProjectPhase[], fallbackPhase?: string): void {
  exportPhaseEl.replaceChildren();
  const listed =
    phases.length > PHASE_OPTIONS_SHOWN
      ? [...phases.slice(0, PHASE_OPTIONS_SHOWN - 1), phases[phases.length - 1]!]
      : phases;
  exportPhaseHintEl.textContent =
    listed.length < phases.length
      ? `${exportPhaseHintText} The file names ${phases.length.toLocaleString('en-US')} phases; the first ${PHASE_OPTIONS_SHOWN - 1} and the last are listed.`
      : exportPhaseHintText;
  listed.forEach((phase, index) => {
    const last = index === listed.length - 1;
    const label = phase.name ? phase.name : `ElementId ${phase.id}`;
    const option = document.createElement('option');
    option.value = last ? '' : String(phase.id);
    option.textContent = !last
      ? label
      : fallbackPhase === undefined
        ? `${label} (last, default)`
        : `${label} (last, default; no room in it, so the rooms of ${fallbackPhase} are shown)`;
    exportPhaseEl.append(option);
  });
  if (![...exportPhaseEl.options].some((option) => option.value === currentPhase)) {
    currentPhase = '';
  }
  exportPhaseEl.value = currentPhase;
  exportPhaseEl.disabled = phases.length < 2;
  exportPhaseFieldEl.classList.toggle('hidden', phases.length === 0);
  exportPhaseHintEl.classList.toggle('hidden', phases.length === 0);
}

/** The file now open, kept so the large-model retry can read it again. */
let currentFile: File | null = null;
/** The limits it was opened under; undefined means the defaults. */
let currentOpenLimits: OpenLimitsOptions | undefined;

// ---------- Load flow ----------
/** What the worker knew when a decode stopped for want of memory (worker.ts). */
interface TooLarge {
  wasmMemoryBytes: number;
  /** The decoder's last report inside the open call, or null outside it. */
  report: { stage: string; done: number; total: number | null } | null;
  /** The worker step it was in, or null inside the open call. */
  step: string | null;
  /** The engine's own message, or the panic's. */
  detail: string;
}

/** What the worker reports about one open (worker.ts). */
interface LoadStats {
  /** Milliseconds in the wasm open call, saved meshes included. */
  openMs: number;
  /** Milliseconds from wasm init to the last message. */
  workerMs: number;
  /** The wasm linear memory after the open, which only grows: the peak against the 4 GiB a tab holds. */
  wasmMemoryBytes: number;
}

async function loadBytes(file: File, limits?: OpenLimitsOptions): Promise<void> {
  const openLimits = limits ?? firstOpenLimits;
  const savedMeshes = savedMeshesEl.checked;
  firstOpenLimits = undefined;
  // The same file read again (a changed option, the large-model retry) keeps
  // its phase; another file starts at the project's last.
  if (file !== currentFile) {
    currentPhase = '';
    // The previous file's phases are not this one's choices.
    renderPhases([]);
  }
  const phase = currentPhase === '' ? undefined : Number(currentPhase);
  currentFile = file;
  currentOpenLimits = openLimits;
  delete statusEl.dataset.loadStats;
  delete statusEl.dataset.loadFailure;
  setStatus(
    openLimits?.largeModel
      ? `Reopening ${file.name} with large-model limits…`
      : `Reading ${formatBytes(file.size)}…`,
  );
  startLoadOverlay(file);
  hideScaffoldNote();
  model = null;
  sceneGraph = null;
  listedEntities = new Set<number>();
  resetTreeFilter();
  distinctTypes = [];
  lastGlb = null;
  lastSchedule = null;
  currentDiagnostics = null;
  // The last file's worker holds its wasm memory, and a large file's read, which
  // is next, must not have to fit beside it.
  stopWorker();
  exportGlbBtn.disabled = true;
  exportIfcBtn.disabled = true;
  exportSvgBtn.disabled = true;
  setExportHint('Exports unlock when the file is read');
  downloadDiagnosticsBtn.disabled = true;
  downloadScheduleBtn.disabled = true;
  downloadRoomsBtn.disabled = true;
  exportQualityEl.textContent = 'Quality pending';
  exportQualityEl.className = 'quality-pill';
  diagnosticsJsonEl.textContent = '';
  // A new file invalidates the panel and the schedule; leaving the
  // previous file's numbers up would read as if they still applied.
  pendingInfoIndex = null;
  resetSavedParameters();
  infoEl.removeAttribute('aria-busy');
  infoEl.textContent =
    'Select an element in the 3-D view or scene tree (Enter / Space on a tree row). Double-click it or press F to zoom to it.';
  scheduleTotalEl.textContent = 'Reading the file…';
  scheduleGroupsEl.innerHTML = '';
  clearHighlight();
  renderLoadingStatusPanel(file.name);
  let bytes: Uint8Array;
  try {
    bytes = new Uint8Array(await file.arrayBuffer());
  } catch (err) {
    const message = (err as Error).message ?? String(err);
    // A file the tab has no memory to hold is the same failure as one the
    // decoder has no memory to open.
    if (/allocation failed|out of memory|invalid array length/i.test(message)) {
      showTooLarge(file, { wasmMemoryBytes: 0, report: null, step: null, detail: message });
      return;
    }
    stopWorker();
    stopLoadOverlay();
    clearDemoCardBusy();
    setStatus(`Error: ${message}`);
    renderErrorStatusPanel(message);
    setDropzoneVisible(true);
    setExportHint('Open a file to export');
    toast(`Could not read ${file.name}. ${message}`, 'bad');
    return;
  }
  const qualityMode = selectedExportMode();

  currentDocument = null;
  const w = resetWorker();
  // A worker the browser stopped sends nothing: say so rather than leave the
  // loading card up.
  const workerStopped = (detail: string): void => {
    if (worker !== w) return;
    failLoad(file, `The browser stopped the worker that reads the file${detail}.`);
  };
  w.addEventListener('error', (ev: ErrorEvent) => {
    workerStopped(ev.message ? `: ${ev.message}` : ' (it may have run out of memory)');
  });
  w.addEventListener('messageerror', () => workerStopped(' while it returned the model'));
  w.addEventListener('message', (ev: MessageEvent<unknown>) => {
    const msg = ev.data as
      | { type: 'progress'; step: string }
      | {
          type: 'stage';
          stage: string;
          done: number;
          total: number | null;
          wasmMemoryBytes: number;
        }
      | {
          type: 'summary';
          summary: { version: number; build?: string; guid?: string; class_name_count?: number };
          document: FileMetadata | null;
        }
      | {
          type: 'ready';
          model: IfcModel;
          scene: SceneNode;
          types: string[];
          glb: Uint8Array;
          schedule: Schedule;
          diagnostics: ExportDiagnostics;
          stats: LoadStats;
          phases?: ProjectPhase[];
        }
      | { type: 'info'; index: number; panel: ElementInfoPanel | null }
      | { type: 'wasm'; module: WebAssembly.Module }
      | { type: 'error'; message: string; tooLarge?: TooLarge };
    if (msg.type === 'wasm') {
      // Kept for good: the same wasm serves every file. The workers that read
      // saved parameters, the main thread's exports and the next open start from
      // it, so none of them fetches anything.
      decoderWasm = msg.module;
      return;
    }
    if (msg.type === 'info') {
      // Stale reply for an element the user already clicked past.
      if (msg.index !== pendingInfoIndex) return;
      if (msg.panel) {
        // A drawn piece the export folds into another element's body is that
        // element: select it, as a click on the element itself would.
        const written = msg.panel.entity_index;
        if (written !== undefined && written !== msg.index) {
          selectedIndex = written;
          selectTreeRow(written);
          highlightEntity(written);
        }
        renderElementPanel(msg.panel);
      } else {
        renderScaffoldPanel(msg.index);
      }
      return;
    }
    if (msg.type === 'stage') {
      onDecodeStage(msg.stage, msg.done, msg.total, msg.wasmMemoryBytes);
      return;
    }
    if (msg.type === 'progress') {
      // The steps after the decode count nothing, so the bar sweeps again.
      setLoadFraction(null);
      // The estimate is for before the decoder has said anything; once it has,
      // a step with no count has none to give.
      loadHintEl.textContent = loadStage === null ? loadExpectationText : '';
      setLoadStep(msg.step);
      loadEventsDropped += keepLast(
        loadEvents,
        {
          t: Math.round(performance.now() - loadStartedAt),
          step: msg.step,
          card: { step: msg.step, value: null, hint: loadHintEl.textContent },
        },
        LOAD_RECORDS_KEPT,
      );
      announceLoad(msg.step);
      return;
    }
    if (msg.type === 'summary') {
      // VW1-20 — show the fast metadata the moment the worker has
      // cracked BasicFileInfo, before the full parse finishes.
      const bits = [
        `${file.name}`,
        formatBytes(file.size),
        `Revit ${msg.summary.version}`,
      ];
      if (msg.summary.build) bits.push(msg.summary.build);
      if (msg.summary.class_name_count !== undefined) {
        bits.push(`${msg.summary.class_name_count} classes`);
      }
      fileMetaEl.textContent = bits.join(' · ');
      currentDocument = msg.document ?? null;
      setDropzoneVisible(false);
      return;
    }
    if (msg.type === 'error') {
      if (msg.tooLarge) {
        showTooLarge(file, msg.tooLarge);
        return;
      }
      // A memory stop is `msg.tooLarge` and went above, whatever limits the
      // open ran under; any other failure, a panic included, says only what it
      // is, since nothing says it is memory.
      failLoad(file, msg.message);
      return;
    }
    model = msg.model;
    loadedWasmMemoryBytes = msg.stats.wasmMemoryBytes;
    sceneGraph = msg.scene;
    listedEntities = listedIndices(msg.scene, new Set<number>());
    distinctTypes = msg.types;
    lastGlb = msg.glb;
    lastSchedule = msg.schedule;
    // Diagnostics are saved and shown as JSON, so their maps become objects.
    currentDiagnostics = plainJson(msg.diagnostics);
    renderPhases(msg.phases ?? [], currentDiagnostics?.room_phase_fallback?.phase);
    lastFileStem = file.name.replace(/\.(rvt|rfa|rte|rft)$/i, '');
    renderScene(msg.glb);
    renderTree();
    renderCategories();
    renderScheduleSummary(msg.schedule);
    renderExportQuality(currentDiagnostics);
    renderStatusPanel(currentDiagnostics);
    fileMetaEl.textContent = `${file.name} · ${formatBytes(file.size)} · ${countEntities(msg.scene)} entities`;
    setDropzoneVisible(false);
    exportGlbBtn.disabled = false;
    exportIfcBtn.disabled = false;
    exportSvgBtn.disabled = false;
    setExportHint(null);
    downloadDiagnosticsBtn.disabled = false;
    // The schedules follow the model: an element schedule whenever any
    // building element decoded, a room schedule only when rooms did.
    downloadScheduleBtn.disabled = !scheduleHasType(msg.schedule, null);
    downloadRoomsBtn.disabled = !scheduleHasType(msg.schedule, 'IFCSPACE');
    // What this open cost, for tests/saved-meshes-measure.spec.ts; cleared when
    // the next open starts.
    statusEl.dataset.loadStats = JSON.stringify({
      ...msg.stats,
      loadMs: Math.round(performance.now() - loadStartedAt),
      estimatedPeakBytes: estimatePeakBytes(file.size),
      memoryWarning: loadMemoryWarned,
    });
    saveLoadEvents();
    setStatus(`Loaded · ${msg.types.length} categories · IFC bar ${qualityMode}`);
    stopLoadOverlay();
    clearDemoCardBusy();
    maybeShowScaffoldNote(currentDiagnostics);
    const elapsed = Math.max(1, Math.round((performance.now() - loadStartedAt) / 1000));
    toast(`Loaded ${file.name} · ${msg.types.length} categories · ${elapsed} s`, 'ok');
  });
  w.postMessage(
    {
      type: 'parse',
      bytes,
      mode: qualityMode,
      limits: { ...openLimits, savedMeshes, ...(phase === undefined ? {} : { phase }) },
      wasmUrl,
      wasmModule: decoderWasm ?? undefined,
    },
    [bytes.buffer],
  );
}

/** The card, the status line, the panel and a toast for a file that did not open. */
function failLoad(file: File, detail: string): void {
  stopWorker();
  stopLoadOverlay();
  clearDemoCardBusy();
  saveLoadEvents();
  setStatus(`Error: ${detail}`);
  renderErrorStatusPanel(detail);
  hideScaffoldNote();
  setDropzoneVisible(true);
  setExportHint('Open a file to export');
  toast(`Could not open ${file.name}. ${detail}`, 'bad');
}

/** The worker's own steps, as a stop is blamed on them. */
const WORKER_STEP_PHRASES: Record<string, string> = {
  'Building scene graph': 'building the scene graph',
  'Rendering glTF': 'rendering the glTF',
  'Building schedule': 'building the schedule',
};

/** Where a stop was, as "reading partition streams" or "building the model", or null when unknown. */
function stopPhrase(tooLarge: TooLarge): string | null {
  if (tooLarge.report) {
    const { stage, done, total } = tooLarge.report;
    const label = stageStep(stage, done, total).label;
    return `${label.charAt(0).toLowerCase()}${label.slice(1)}`;
  }
  return tooLarge.step ? (WORKER_STEP_PHRASES[tooLarge.step] ?? null) : null;
}

/**
 * The decode stopped for want of memory (#1195): wasm32 holds at most 4 GiB,
 * and a failed allocation ends the instance with a bare `unreachable`. When
 * the stop was near what a tab can hold, or the file is big enough that the
 * pre-check would have warned, this says plainly that the model is too large
 * for the browser. Otherwise a stop a long way short of the ceiling on a small
 * file may be a damaged file rather than a large one, and says so, since
 * "needs more memory" would be a claim nothing measured.
 */
function showTooLarge(file: File, tooLarge: TooLarge): void {
  const reached = formatMemory(tooLarge.wasmMemoryBytes);
  const size = formatMemory(file.size);
  const ceiling = formatMemory(memoryCeilingBytes);
  const where = stopPhrase(tooLarge);
  const readFailed = tooLarge.wasmMemoryBytes === 0;
  const firm =
    readFailed ||
    tooLarge.wasmMemoryBytes >= memoryCeilingBytes / 2 ||
    estimatePeakBytes(file.size) >= memoryCeilingBytes;
  const title = firm ? 'Too large for the browser' : 'Ran out of memory';
  const stopped = readFailed ? '' : ` at ${reached}`;
  let summary: string;
  let advice: string;
  if (readFailed) {
    summary = `The browser did not have the memory to read ${file.name} (${size}).`;
    advice = LARGER_FILE_ADVICE;
  } else if (firm) {
    summary = `The browser would not give this tab more than ${reached}${where ? `, while ${where},` : ''} and ${file.name} (${size}) needs more.`;
    advice = `A browser tab can hold at most ${ceiling}, and an open has taken at least about ${PEAK_MEMORY_PER_FILE_BYTE} times its file's size. ${LARGER_FILE_ADVICE}`;
  } else {
    summary = `The decoder ran out of memory at ${reached}${where ? `, while ${where}` : ''}, a long way short of the ${ceiling} a tab can hold, on a ${size} file. The file may be damaged, or may have hit a limit other than the browser's memory.`;
    advice =
      'Running rvt-ifc or rvt-gltf on the same file (with --large-model if it is large) shows whether it is damaged or only large.';
  }
  stopWorker();
  stopLoadOverlay();
  clearDemoCardBusy();
  saveLoadEvents();
  statusEl.dataset.loadFailure = firm ? 'too-large' : 'out-of-memory';
  setStatus(`${title}${stopped ? ` · stopped${stopped}` : ''}`);
  renderErrorStatusPanel(
    summary,
    title,
    `Not decoded · the tab ran out of memory${stopped}`,
    tooLarge.detail,
  );
  scaffoldNoteTitleEl.textContent = title;
  scaffoldNoteBodyEl.textContent = `${summary} ${advice}`;
  scaffoldNoteEl.classList.add('is-failure');
  scaffoldNoteEl.classList.remove('hidden');
  setDropzoneVisible(true);
  setExportHint('Open a file to export');
  toast(
    firm
      ? `${file.name} is too large for the browser. rvt-ifc and rvt-gltf with --large-model can read a larger one.`
      : `${file.name}: the decoder ran out of memory, a long way short of the browser's limit.`,
    'bad',
  );
}

function renderEmptyStatusPanel(): void {
  statusPanelEl.innerHTML = '';
  resetStatusRowStagger();
  statusPanelEl.appendChild(statusRow('File', 'idle', 'No file opened'));
  statusPanelEl.appendChild(statusRow('Mode', 'idle', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Schema', 'idle', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Elements', 'idle', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Geometry', 'idle', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Decode', 'idle', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Export', 'idle', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('IFC bar', 'idle', `Selected · ${selectedExportMode()}`));
  statusPanelEl.appendChild(statusRow('Warnings', 'ok', 'No export warnings'));
  diagnosticsJsonEl.textContent = '';
}

function renderLoadingStatusPanel(filename: string): void {
  statusPanelEl.innerHTML = '';
  resetStatusRowStagger();
  statusPanelEl.appendChild(statusRow('File', 'warn', `Reading ${filename}`));
  statusPanelEl.appendChild(statusRow('Mode', 'warn', 'Evaluating file'));
  statusPanelEl.appendChild(statusRow('Schema', 'warn', 'Not parsed yet'));
  statusPanelEl.appendChild(statusRow('Elements', 'warn', 'Not decoded yet'));
  statusPanelEl.appendChild(statusRow('Geometry', 'warn', 'Not decoded yet'));
  statusPanelEl.appendChild(statusRow('Decode', 'warn', 'Not evaluated yet'));
  statusPanelEl.appendChild(statusRow('Export', 'warn', 'Not evaluated yet'));
  statusPanelEl.appendChild(statusRow('IFC bar', 'warn', `Selected · ${selectedExportMode()}`));
  statusPanelEl.appendChild(statusRow('Warnings', 'ok', 'No export warnings'));
}

function renderErrorStatusPanel(
  message: string,
  fileText = 'Could not open file',
  modeText = 'Corrupt or unreadable file · not a readable Revit OLE/CFB container',
  engineMessage?: string,
): void {
  statusPanelEl.innerHTML = '';
  resetStatusRowStagger();
  statusPanelEl.appendChild(statusRow('File', 'bad', fileText));
  statusPanelEl.appendChild(statusRow('Mode', 'bad', modeText));
  statusPanelEl.appendChild(statusRow('Schema', 'warn', 'Not parsed'));
  statusPanelEl.appendChild(statusRow('Elements', 'warn', 'Not decoded'));
  statusPanelEl.appendChild(statusRow('Geometry', 'warn', 'Not decoded'));
  statusPanelEl.appendChild(statusRow('Decode', 'bad', 'Decode confidence unavailable'));
  statusPanelEl.appendChild(statusRow('Export', 'bad', 'Export confidence unavailable'));
  statusPanelEl.appendChild(statusRow('IFC bar', 'warn', `Selected · ${selectedExportMode()}`));
  statusPanelEl.appendChild(statusRow('Warnings', 'bad', message));
  if (engineMessage) statusPanelEl.appendChild(statusRow('Engine message', 'bad', engineMessage));
  diagnosticsJsonEl.textContent = '';
}

function renderScene(glb: Uint8Array): void {
  clearHighlight();
  if (currentModel) {
    scene.remove(currentModel);
    currentModel.traverse((obj) => {
      if ((obj as THREE.Mesh).geometry) (obj as THREE.Mesh).geometry.dispose();
      const mat = (obj as THREE.Mesh).material;
      if (Array.isArray(mat)) mat.forEach((m) => m.dispose());
      else if (mat) (mat as THREE.Material).dispose();
    });
    currentModel = null;
  }
  // TS 5.7+ parameterises Uint8Array over ArrayBufferLike, which
  // isn't assignable to BlobPart directly. Extract the underlying
  // ArrayBuffer — it's a BlobPart unambiguously.
  const blob = new Blob([glb.buffer as ArrayBuffer], { type: 'model/gltf-binary' });
  const url = URL.createObjectURL(blob);
  const loader = new GLTFLoader();
  loader.load(
    url,
    (gltf) => {
      currentModel = gltf.scene;
      // A category hidden before the meshes arrived stays hidden.
      applyCategoryVisibility();
      scene.add(currentModel);
      frameCamera(currentModel);
      URL.revokeObjectURL(url);
      // The schedule renders before the loader resolves, so the
      // per-type highlight affordance only becomes honest here —
      // once there are meshes to tint.
      if (lastSchedule) renderScheduleSummary(lastSchedule);
    },
    undefined,
    (err) => {
      setStatus(`glTF load error: ${(err as Error).message ?? err}`);
      URL.revokeObjectURL(url);
    },
  );
}

function frameCamera(obj: THREE.Object3D): void {
  frameBox(new THREE.Box3().setFromObject(obj));
}

/** The smallest extent a framed box is treated as having, metres. */
const MIN_FRAME_METRES = 0.5;

function frameBox(box: THREE.Box3): void {
  if (box.isEmpty()) return;
  const size = box.getSize(new THREE.Vector3());
  const center = box.getCenter(new THREE.Vector3());
  const maxDim = Math.max(size.x, size.y, size.z, MIN_FRAME_METRES);
  const fov = camera.fov * (Math.PI / 180);
  const dist = Math.abs(maxDim / Math.sin(fov / 2)) * 0.8;
  // Keep the approach direction when zooming in on a part of the model,
  // so the view does not swing round; the whole model gets the default.
  const approach = camera.position.clone().sub(controls.target);
  const direction =
    currentModel && approach.lengthSq() > 0 && !box.equals(new THREE.Box3().setFromObject(currentModel))
      ? approach.normalize()
      : new THREE.Vector3(1, 0.8, 1).normalize();
  const toPos = center.clone().add(direction.multiplyScalar(dist));
  // Clip planes jump immediately: interpolating them causes visible
  // z-fighting mid-move and carries no information. The far plane
  // keeps the rest of the model in view around a small element.
  const model = currentModel ? new THREE.Box3().setFromObject(currentModel) : box;
  const modelDim = model.isEmpty() ? maxDim : model.getSize(new THREE.Vector3()).length();
  camera.near = maxDim / 100;
  camera.far = Math.max(dist * 10, dist + modelDim * 2);
  camera.updateProjectionMatrix();

  if (prefersReducedMotion()) {
    cameraTween = null;
    controls.enabled = true;
    camera.position.copy(toPos);
    controls.target.copy(center);
    return;
  }

  controls.enabled = false;
  cameraTween = {
    fromPos: camera.position.clone(),
    toPos,
    fromTarget: controls.target.clone(),
    toTarget: center,
    startedAt: performance.now(),
    durationMs: 420,
  };
}

/**
 * IFC entity names as IFC spells them. The model carries the STEP file's
 * upper-case spelling (IFCBUILDINGSTOREY), which is how types are matched;
 * this is only how they are shown.
 */
const IFC_ENTITY_NAMES = [
  'IfcAirTerminal',
  'IfcAlarm',
  'IfcBeam',
  'IfcBuilding',
  'IfcBuildingElementProxy',
  'IfcBuildingStorey',
  'IfcCableCarrierFitting',
  'IfcCableCarrierSegment',
  'IfcColumn',
  'IfcCovering',
  'IfcCurtainWall',
  'IfcDoor',
  'IfcDuctFitting',
  'IfcDuctSegment',
  'IfcElectricAppliance',
  'IfcFlowController',
  'IfcFlowTerminal',
  'IfcFooting',
  'IfcFurnishingElement',
  'IfcFurniture',
  'IfcGrid',
  'IfcLightFixture',
  'IfcMember',
  'IfcOpeningElement',
  'IfcOpeningStandardCase',
  'IfcPipeFitting',
  'IfcPipeSegment',
  'IfcPlate',
  'IfcProject',
  'IfcRailing',
  'IfcRamp',
  'IfcRampFlight',
  'IfcReinforcingBar',
  'IfcRoof',
  'IfcSanitaryTerminal',
  'IfcShadingDevice',
  'IfcSite',
  'IfcSlab',
  'IfcSpace',
  'IfcStair',
  'IfcStairFlight',
  'IfcSystemFurnitureElement',
  'IfcTransportElement',
  'IfcWall',
  'IfcWallStandardCase',
  'IfcWindow',
];
const IFC_ENTITY_BY_STEP_NAME = new Map(IFC_ENTITY_NAMES.map((name) => [name.toUpperCase(), name]));

/** `IFCBUILDINGSTOREY` as `IfcBuildingStorey`; an unlisted `IFCFOO` as `IfcFoo`. */
function ifcTypeLabel(stepName: string): string {
  const known = IFC_ENTITY_BY_STEP_NAME.get(stepName.toUpperCase());
  if (known) return known;
  if (/^IFC[A-Z0-9]+$/.test(stepName)) {
    return `Ifc${stepName.charAt(3)}${stepName.slice(4).toLowerCase()}`;
  }
  return stepName;
}

// ---------- Panels ----------
/**
 * A scene tree row: the project, a storey, one category on a storey, or
 * an element. Built from the worker's scene graph by `buildTreeItems`.
 */
interface TreeItem {
  kind: 'project' | 'storey' | 'category' | 'element';
  name: string;
  ifcType: string;
  entityIndex: number | null;
  storeyIndex: number | null;
  /** The storey a category row belongs to, for its panel. */
  storeyName?: string;
  /** Elements under the row, parts included; 1 for a bare element. */
  count: number;
  children: TreeItem[];
}

/** A Revit element name split for its tree row. */
interface ElementNameParts {
  /** The row's text: the type, or the whole name when it has no type. */
  label: string;
  /** The family, when the name carries one that differs from the label. */
  family: string | null;
  /** The ElementId, with Revit's `:2` part suffix when there is one. */
  id: string | null;
}

/**
 * Split `Family:Type:ElementId` (RE-38, RE-63) and the `Class-ElementId`
 * fallback so a row can lead with the type. Revit names a second part
 * of one element `Family:Type:ElementId:2`, and some types are named
 * `-`; a type equal to its family is shown once.
 */
function splitElementName(name: string): ElementNameParts {
  // A stair's run, landing or support is named after the stair (RE-65),
  // and sits under it in the tree, so it reads as its number alone.
  const stairPart = /^.+:Stair:\d+ ((?:Run|Landing|Stringer) \d+)(?::\d+)?$/.exec(name);
  if (stairPart) return { label: stairPart[1]!, family: null, id: null };
  const parts = name.split(':');
  const digits = (s: string | undefined) => s !== undefined && /^\d+$/.test(s);
  if (parts.length >= 4 && digits(parts[parts.length - 1]) && digits(parts[parts.length - 2])) {
    const typed = splitElementName(parts.slice(0, -1).join(':'));
    if (typed.id) return { ...typed, id: `${typed.id}:${parts[parts.length - 1]}` };
  }
  if (parts.length >= 3 && digits(parts[parts.length - 1])) {
    const family = parts[0]!;
    const type = parts.slice(1, -1).join(':');
    const id = parts[parts.length - 1]!;
    if (type === '' || type === '-' || type === family) return { label: family, family: null, id };
    return { label: type, family, id };
  }
  const fallback = /^(.+)-(\d+)$/.exec(name);
  if (fallback) return { label: fallback[1]!, family: null, id: fallback[2]! };
  return { label: name, family: null, id: null };
}

const nameCollator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

/** Rows of one category read in family, type, then ElementId order. */
function compareElementNames(a: string, b: string): number {
  const x = splitElementName(a);
  const y = splitElementName(b);
  return (
    nameCollator.compare(x.family ?? x.label, y.family ?? y.label) ||
    nameCollator.compare(x.label, y.label) ||
    nameCollator.compare(x.id ?? '', y.id ?? '') ||
    nameCollator.compare(a, b)
  );
}

/**
 * The scene graph as the tree shows it, the way Revit's own browsers
 * group a model: each storey's elements by category, sorted by family
 * and type. An aggregate's parts (a curtain wall's panels and mullions,
 * a stair's flights) sit under their whole, as in Revit's IFC export.
 * Doors and windows are listed in their own category rather than under
 * their host wall; the element panel names the host.
 */
function buildTreeItems(root: SceneNode): TreeItem {
  const elementsOf = (node: SceneNode): SceneNode[] => {
    const out: SceneNode[] = [];
    const walk = (n: SceneNode) => {
      if (n.entity_index !== null) out.push(n);
      n.children.forEach(walk);
    };
    node.children.forEach(walk);
    return out;
  };
  const perStorey = root.children.map(elementsOf);
  const inTree = new Map<number, SceneNode>();
  for (const n of perStorey.flat()) inTree.set(n.entity_index!, n);

  const wholeOf = new Map<number, number>();
  for (const e of model?.entities ?? []) {
    if (e.kind !== 'Aggregate' || e.whole === undefined) continue;
    for (const part of e.parts ?? []) {
      // A part of several wholes takes the first, as the export does.
      if (part !== e.whole && inTree.has(e.whole) && !wholeOf.has(part)) {
        wholeOf.set(part, e.whole);
      }
    }
  }
  // A part whose chain of wholes loops back to it stays a row of its own.
  for (const start of [...wholeOf.keys()]) {
    const seen = new Set<number>([start]);
    for (let at = wholeOf.get(start); at !== undefined; at = wholeOf.get(at)) {
      if (seen.has(at)) {
        wholeOf.delete(start);
        break;
      }
      seen.add(at);
    }
  }
  const partsOf = new Map<number, SceneNode[]>();
  for (const [part, whole] of wholeOf) {
    const node = inTree.get(part);
    if (!node) continue;
    const list = partsOf.get(whole) ?? [];
    list.push(node);
    partsOf.set(whole, list);
  }

  const byName = (a: SceneNode, b: SceneNode) =>
    nameCollator.compare(ifcTypeLabel(a.ifc_type), ifcTypeLabel(b.ifc_type)) ||
    compareElementNames(a.name, b.name);
  const elementItem = (n: SceneNode): TreeItem => {
    const children = (partsOf.get(n.entity_index!) ?? []).sort(byName).map(elementItem);
    return {
      kind: 'element',
      name: n.name,
      ifcType: n.ifc_type,
      entityIndex: n.entity_index,
      storeyIndex: null,
      count: 1 + children.reduce((sum, c) => sum + c.count, 0),
      children,
    };
  };

  const storeys: TreeItem[] = root.children.map((storey, i) => {
    const byType = new Map<string, SceneNode[]>();
    for (const n of perStorey[i]!) {
      if (wholeOf.has(n.entity_index!)) continue;
      const list = byType.get(n.ifc_type) ?? [];
      list.push(n);
      byType.set(n.ifc_type, list);
    }
    const categories: TreeItem[] = [...byType.entries()]
      .sort(([a], [b]) => nameCollator.compare(ifcTypeLabel(a), ifcTypeLabel(b)))
      .map(([ifcType, nodes]) => {
        const children = nodes.sort(byName).map(elementItem);
        return {
          kind: 'category',
          name: `${ifcTypeLabel(ifcType)} on ${storey.name}`,
          ifcType,
          entityIndex: null,
          storeyIndex: storey.storey_index ?? null,
          storeyName: storey.name === 'Unassigned' ? undefined : storey.name,
          count: children.reduce((sum, c) => sum + c.count, 0),
          children,
        };
      });
    return {
      kind: 'storey',
      name: storey.name,
      ifcType: storey.ifc_type,
      entityIndex: null,
      storeyIndex: storey.storey_index ?? null,
      count: categories.reduce((sum, c) => sum + c.count, 0),
      children: categories,
    };
  });
  return {
    kind: 'project',
    name: root.name,
    ifcType: root.ifc_type,
    entityIndex: null,
    storeyIndex: null,
    count: storeys.reduce((sum, c) => sum + c.count, 0),
    children: storeys,
  };
}

// ---------- Tree rows are built in blocks (#1887) ----------

/**
 * The most rows a group builds at once: a storey's categories, a category's
 * elements, an element's parts. A file that puts a million elements in one
 * category should cost the page the rows the reader opens, not a million, so a
 * group builds its children in blocks of this many, and a "Show more" row at
 * its end builds the next. `?treeRowsShown=` lowers it, for the browser tests.
 */
const TREE_ROWS_SHOWN = Math.max(
  1,
  Math.min(500, wholeBytesParam(new URLSearchParams(window.location.search), 'treeRowsShown') ?? 500),
);
/**
 * The rows the tree builds up front, every block of every group in order,
 * before it leaves the rest to be built as a group is opened (its first block)
 * or its "Show more" row is used. The largest model Measure holds, the BIM4LCA
 * office, exports 91,723 entities, so every model measured so far is built
 * whole, as it was before this limit; a file that lists more has the rest built
 * as the reader opens it. `?treeRowsEager=` lowers it, for the browser tests.
 */
const TREE_ROWS_EAGER = Math.min(
  150_000,
  wholeBytesParam(new URLSearchParams(window.location.search), 'treeRowsEager') ?? 150_000,
);
/** The most matches the filter lists, which builds their rows; the status line says how many there are. */
const TREE_FILTER_LISTED = TREE_ROWS_SHOWN * 4;

/** One group of tree rows, which builds its children a block at a time. */
interface TreeGroup {
  item: TreeItem;
  /** The group's `.tree-children` element. */
  el: HTMLElement;
  /** The `aria-level` of its rows. */
  level: number;
  /** The rows of each block of TREE_ROWS_SHOWN children, once the block is built. */
  blocks: Array<HTMLElement[] | undefined>;
  /** How many of the children have a row. */
  built: number;
  /** The "Show more" row, while a child has none. */
  more: HTMLElement | null;
}

/** The group each item with children owns, by item and by its `.tree-children` element. */
let treeGroups = new Map<TreeItem, TreeGroup>();
const treeGroupOfElement = new WeakMap<HTMLElement, TreeGroup>();
/** The row (the `.tree-item` wrapper) built for an item. */
let treeWraps = new Map<TreeItem, HTMLElement>();
/** Where an item stands among its parent's children, for every item whether or not it has a row. */
let treePlaces = new Map<TreeItem, { parent: TreeItem; index: number }>();
let treeItemByEntity = new Map<number, TreeItem>();
let treeItemByStorey = new Map<number, TreeItem>();
let treeRowsBuilt = 0;
/** Elements and the text the filter matches them by, over the whole tree and not the rows built. */
let treeFilterIndex: Array<{ item: TreeItem; text: string }> = [];

/** Records where every item stands, and the elements the filter matches, in the order the tree lists them. */
function indexTreeItems(root: TreeItem): void {
  treeFilterIndex = [];
  const stack = [root];
  while (stack.length > 0) {
    const item = stack.pop()!;
    if (item.kind === 'element') {
      treeFilterIndex.push({ item, text: `${item.name} ${ifcTypeLabel(item.ifcType)}`.toLowerCase() });
    }
    if (item.entityIndex !== null && !treeItemByEntity.has(item.entityIndex)) {
      treeItemByEntity.set(item.entityIndex, item);
    }
    if (item.kind === 'storey' && item.storeyIndex != null && !treeItemByStorey.has(item.storeyIndex)) {
      treeItemByStorey.set(item.storeyIndex, item);
    }
    for (let i = item.children.length - 1; i >= 0; i--) {
      const child = item.children[i]!;
      treePlaces.set(child, { parent: item, index: i });
      stack.push(child);
    }
  }
}

function renderTree(): void {
  if (!sceneGraph) return;
  treeEl.innerHTML = '';
  treeGroups = new Map();
  treeWraps = new Map();
  treePlaces = new Map();
  treeItemByEntity = new Map();
  treeItemByStorey = new Map();
  treeRowsBuilt = 0;
  const root = buildTreeItems(sceneGraph);
  indexTreeItems(root);
  treeEl.appendChild(buildTreeRow(root, 1));
  const first = treeEl.querySelector<HTMLElement>('.tree-node');
  if (first) first.tabIndex = 0;
  treeFilterEl.disabled = false;
  if (treeFilterEl.value.trim()) applyTreeFilter(treeFilterEl.value);
}

/**
 * Builds block `b` of `group` (the children from `b * TREE_ROWS_SHOWN`), in its
 * place among the blocks built already: the filter and a click in the 3-D view
 * build the block an element is in, which need not be the next.
 */
function buildTreeBlock(group: TreeGroup, b: number): void {
  if (group.blocks[b]) return;
  const children = group.item.children;
  const start = b * TREE_ROWS_SHOWN;
  if (start >= children.length) return;
  // The row the new ones go before: the first of the next block built, or the "Show more" row, or none.
  let before: Node | null = group.more;
  for (let k = b + 1; k < group.blocks.length; k++) {
    const later = group.blocks[k];
    if (later) {
      before = later[0]!;
      break;
    }
  }
  const rows: HTMLElement[] = [];
  for (let i = start; i < Math.min(children.length, start + TREE_ROWS_SHOWN); i++) {
    const wrap = buildTreeRow(children[i]!, group.level);
    group.el.insertBefore(wrap, before);
    rows.push(wrap);
  }
  group.blocks[b] = rows;
  group.built += rows.length;
}

/**
 * Builds a group's blocks in order for as long as the page has rows to spend
 * on them up front (TREE_ROWS_EAGER); the first block is always built.
 */
function fillTreeGroup(group: TreeGroup): void {
  const blocks = Math.ceil(group.item.children.length / TREE_ROWS_SHOWN);
  for (let b = 0; b < blocks && (b === 0 || treeRowsBuilt < TREE_ROWS_EAGER); b++) {
    buildTreeBlock(group, b);
  }
  updateTreeMore(group);
}

/** The first block of `group` with no rows yet. */
function nextTreeBlock(group: TreeGroup): number {
  let b = 0;
  while (group.blocks[b]) b++;
  return b;
}

/**
 * The "Show more" row at the end of a group, there while a child has no row:
 * it says how many the next block adds and how many have none.
 */
function updateTreeMore(group: TreeGroup): void {
  const total = group.item.children.length;
  const remaining = total - group.built;
  if (remaining <= 0) {
    group.more?.remove();
    group.more = null;
    return;
  }
  if (!group.more) {
    const wrap = document.createElement('div');
    wrap.className = 'tree-item';
    const row = document.createElement('div');
    row.className = 'tree-node tree-more';
    row.setAttribute('role', 'treeitem');
    row.setAttribute('aria-level', String(group.level));
    row.tabIndex = -1;
    const caret = document.createElement('span');
    caret.className = 'tree-caret tree-caret-none';
    caret.setAttribute('aria-hidden', 'true');
    row.appendChild(caret);
    row.addEventListener('click', (ev) => {
      ev.stopPropagation();
      showMoreTreeRows(group, row);
    });
    row.addEventListener('keydown', (ev) => {
      if (ev.key === 'Enter' || ev.key === ' ') {
        ev.preventDefault();
        showMoreTreeRows(group, row);
        return;
      }
      moveTreeFocus(ev, row);
    });
    wrap.appendChild(row);
    group.el.appendChild(wrap);
    group.more = wrap;
  }
  const row = group.more.firstElementChild as HTMLElement;
  row.querySelector('.tree-text')?.remove();
  const adds = Math.min(TREE_ROWS_SHOWN, total - nextTreeBlock(group) * TREE_ROWS_SHOWN);
  treeRowText(
    row,
    `Show ${adds.toLocaleString('en-US')} more`,
    `${remaining.toLocaleString('en-US')} not listed`,
  );
}

/** The "Show more" row's action: builds the group's next block. */
function showMoreTreeRows(group: TreeGroup, from: HTMLElement): void {
  const b = nextTreeBlock(group);
  buildTreeBlock(group, b);
  updateTreeMore(group);
  // The focus stays on the row, which now says what is left; with none left it goes to the first new row.
  const added = group.blocks[b]?.[0]?.querySelector<HTMLElement>('.tree-node');
  focusTreeRow(group.more ? from : (added ?? from));
}

/**
 * Builds the rows from the project down to `item`'s own, each in the block of
 * its group that holds it, and returns the row. A reader's click on the item in
 * the 3-D view, a jump from the element panel and the filter all need a row the
 * tree may not have built yet.
 */
function ensureTreeRow(item: TreeItem): HTMLElement | null {
  const path: Array<{ parent: TreeItem; index: number }> = [];
  for (let at = treePlaces.get(item); at; at = treePlaces.get(at.parent)) path.push(at);
  for (let k = path.length - 1; k >= 0; k--) {
    const { parent, index } = path[k]!;
    const group = treeGroups.get(parent);
    if (!group) return null;
    buildTreeBlock(group, Math.floor(index / TREE_ROWS_SHOWN));
    updateTreeMore(group);
  }
  return (treeWraps.get(item)?.firstElementChild as HTMLElement | undefined) ?? null;
}

/**
 * Show only the elements whose name or IFC type contains `query`, with the
 * storeys and categories that hold them opened. An element that matches
 * keeps its parts. An empty query restores the tree as it opens.
 */
function applyTreeFilter(query: string): void {
  const needle = query.trim().toLowerCase();
  if (!needle) {
    treeEl.querySelectorAll<HTMLElement>('.tree-item').forEach((wrap) => {
      wrap.hidden = false;
    });
    treeEl.querySelectorAll<HTMLElement>('.tree-node[aria-expanded]').forEach((row) => {
      const open = row.classList.contains('tree-project') || row.classList.contains('tree-storey');
      setTreeRowExpanded(row, open);
    });
    const selected = treeEl.querySelector<HTMLElement>('.tree-node.selected');
    if (selected) revealTreeRow(selected);
    treeFilterStatusEl.textContent = '';
    return;
  }
  // The matches are found over the whole tree, and the first TREE_FILTER_LISTED
  // are listed: their rows are built, and their groups opened, before anything
  // is hidden, so a row built for a match is hidden with the rest.
  const matched = treeFilterIndex.filter((entry) => entry.text.includes(needle));
  const listed = matched.slice(0, TREE_FILTER_LISTED);
  for (const entry of listed) {
    ensureTreeRow(entry.item);
    for (let place = treePlaces.get(entry.item); place; place = treePlaces.get(place.parent)) {
      const owner = treeWraps.get(place.parent)?.firstElementChild as HTMLElement | undefined;
      if (owner) setTreeRowExpanded(owner, true);
    }
  }
  treeEl.querySelectorAll<HTMLElement>('.tree-item').forEach((wrap) => {
    wrap.hidden = true;
  });
  for (const entry of listed) {
    const wrap = treeWraps.get(entry.item);
    if (!wrap) continue;
    wrap.hidden = false;
    wrap.querySelectorAll<HTMLElement>('.tree-item').forEach((inner) => {
      inner.hidden = false;
    });
    for (let place = treePlaces.get(entry.item); place; place = treePlaces.get(place.parent)) {
      const holder = treeWraps.get(place.parent);
      if (holder) holder.hidden = false;
    }
  }
  const matches = matched.length;
  treeFilterStatusEl.textContent =
    matches === 0
      ? 'No element matches'
      : matches === 1
        ? '1 element matches'
        : matches > listed.length
          ? `${matches} elements match; the first ${listed.length} are listed`
          : `${matches} elements match`;
}

let treeFilterTimer: number | undefined;
treeFilterEl.addEventListener('input', () => {
  window.clearTimeout(treeFilterTimer);
  treeFilterTimer = window.setTimeout(() => applyTreeFilter(treeFilterEl.value), 120);
});
treeFilterEl.addEventListener('keydown', (ev) => {
  if (ev.key === 'Escape' && treeFilterEl.value) {
    // Clear the filter only; a second Escape clears the selection.
    ev.stopPropagation();
    treeFilterEl.value = '';
    applyTreeFilter('');
    return;
  }
  if (ev.key === 'ArrowDown' || ev.key === 'Enter') {
    const first = visibleTreeRows().find((row) => row.classList.contains('tree-element'));
    if (first) {
      ev.preventDefault();
      focusTreeRow(first);
    }
  }
});

function resetTreeFilter(): void {
  window.clearTimeout(treeFilterTimer);
  treeFilterEl.value = '';
  treeFilterEl.disabled = true;
  treeFilterStatusEl.textContent = '';
  treeFilterIndex = [];
}

/** A storey row's second line: its elevation and what it holds. */
function storeyNodeDetail(node: TreeItem): string {
  const elev = storeyElevationLabel(node);
  const count = node.count === 1 ? '1 element' : `${node.count} elements`;
  return elev ? `${elev} · ${count}` : count;
}

function storeyElevationLabel(node: TreeItem): string | null {
  if (node.kind !== 'storey') return null;
  if (node.name === 'Unassigned') return 'no Level ElementId bind';
  const storeys = model?.building_storeys ?? [];
  const idx = node.storeyIndex;
  if (idx == null || idx < 0 || idx >= storeys.length) return null;
  const elev = storeys[idx]?.elevation_feet;
  if (typeof elev !== 'number') return null;
  // Name-only 2024 recoveries put every storey at 0.0. When any storey
  // has a non-zero elevation (ArcWall trailers), treat 0.0 as surveyed
  // ground rather than unresolved.
  const anySurveyed = storeys.some(
    (s) => typeof s.elevation_feet === 'number' && Math.abs(s.elevation_feet) >= 1e-9,
  );
  if (Math.abs(elev) < 1e-9 && !anySurveyed) return 'elev unresolved/0';
  return `elev ${elev.toFixed(3)} ft`;
}

/**
 * The text of a row: a label, a quieter second line, and a count on the
 * right. The sidebar is narrow, so the lines wrap rather than cut a
 * Revit type name short.
 */
function treeRowText(row: HTMLElement, label: string, detail?: string, count?: number): void {
  const text = document.createElement('span');
  text.className = 'tree-text';
  const main = document.createElement('span');
  main.className = 'tree-label';
  main.textContent = label;
  text.appendChild(main);
  // Spaces between the spans keep the row's text readable to assistive
  // technology and in copied text; the flex layout does not render them.
  if (detail) {
    text.appendChild(document.createTextNode(' '));
    const sub = document.createElement('span');
    sub.className = 'tree-detail';
    sub.textContent = detail;
    text.appendChild(sub);
  }
  row.appendChild(text);
  if (count !== undefined) {
    row.appendChild(document.createTextNode(' '));
    const n = document.createElement('span');
    n.className = 'tree-count';
    n.textContent = String(count);
    row.appendChild(n);
  }
}

/**
 * Let a long IFC class name (`IfcBuildingElementProxy`) wrap between its
 * words rather than inside one; the text itself is unchanged.
 */
function breakAtCamelHumps(el: HTMLElement): void {
  const words = (el.textContent ?? '').split(/(?=[A-Z][a-z])/);
  el.textContent = '';
  words.forEach((word, i) => {
    if (i > 0) el.appendChild(document.createElement('wbr'));
    el.appendChild(document.createTextNode(word));
  });
}

/** A project named by its file path reads as the file's name. */
function projectLabel(name: string): string {
  return name.split(/[\\/]/).filter(Boolean).pop() ?? name;
}

/** Rows a keyboard user can reach: those neither collapsed nor filtered out. */
function visibleTreeRows(): HTMLElement[] {
  return Array.from(treeEl.querySelectorAll<HTMLElement>('.tree-node[role="treeitem"]')).filter(
    (row) => !row.closest('[hidden]'),
  );
}

/** Move the tree's single tab stop to `row` and focus it. */
function focusTreeRow(row: HTMLElement): void {
  treeEl.querySelectorAll<HTMLElement>('.tree-node[tabindex="0"]').forEach((el) => {
    if (el !== row) el.tabIndex = -1;
  });
  row.tabIndex = 0;
  row.focus();
}

function setTreeRowExpanded(row: HTMLElement, expanded: boolean): void {
  const group = row.nextElementSibling as HTMLElement | null;
  if (!group?.classList.contains('tree-children')) return;
  row.setAttribute('aria-expanded', String(expanded));
  group.hidden = !expanded;
  // A group the page did not build up front builds its first block when it is first opened.
  const owned = treeGroupOfElement.get(group);
  if (expanded && owned && !owned.blocks[0]) {
    buildTreeBlock(owned, 0);
    updateTreeMore(owned);
  }
}

/** Expand every collapsed row above `row`, so a selection is never hidden. */
function revealTreeRow(row: HTMLElement): void {
  // A selection the filter hides clears the filter rather than stay unseen.
  if (row.closest('.tree-item[hidden]') && treeFilterEl.value) {
    treeFilterEl.value = '';
    applyTreeFilter('');
  }
  for (
    let group = row.parentElement?.closest<HTMLElement>('.tree-children');
    group;
    group = group.parentElement?.closest<HTMLElement>('.tree-children')
  ) {
    const owner = group.previousElementSibling as HTMLElement | null;
    if (owner) setTreeRowExpanded(owner, true);
  }
}

function buildTreeRow(item: TreeItem, level: number): HTMLElement {
  const wrap = document.createElement('div');
  wrap.className = 'tree-item';
  treeWraps.set(item, wrap);
  treeRowsBuilt += 1;
  const row = document.createElement('div');
  row.className = `tree-node tree-${item.kind}`;
  if (item.kind === 'storey') {
    if (item.name === 'Unassigned') row.classList.add('tree-storey-unassigned');
    if (item.count === 0) row.classList.add('tree-storey-empty');
  }
  row.setAttribute('role', 'treeitem');
  row.setAttribute('aria-level', String(level));
  row.tabIndex = -1;
  if (item.entityIndex !== null) {
    row.dataset.entityIndex = String(item.entityIndex);
  }
  if (item.kind === 'element' || item.kind === 'category') {
    row.dataset.ifcType = item.ifcType;
  }
  // Storey nodes are synthetic — they carry no entity index, so the
  // panel's storey jump addresses them by storey index instead.
  if (item.kind === 'storey' && item.storeyIndex != null) {
    row.dataset.storeyIndex = String(item.storeyIndex);
  }

  const expandable = item.children.length > 0;
  const caret = document.createElement('span');
  caret.className = expandable ? 'tree-caret' : 'tree-caret tree-caret-none';
  caret.setAttribute('aria-hidden', 'true');
  row.appendChild(caret);
  if (item.kind === 'project') {
    const storeys = item.children.length;
    treeRowText(
      row,
      projectLabel(item.name),
      `IfcProject · ${storeys} storey${storeys === 1 ? '' : 's'}`,
    );
    row.title = item.name;
  } else if (item.kind === 'storey') {
    treeRowText(row, item.name, storeyNodeDetail(item));
    row.title = `${item.name} · ${ifcTypeLabel(item.ifcType)}`;
  } else if (item.kind === 'category') {
    // The count is the rows the category opens to; parts are inside them.
    treeRowText(row, ifcTypeLabel(item.ifcType), undefined, item.children.length);
    breakAtCamelHumps(row.querySelector('.tree-label')!);
    row.title = item.name;
  } else {
    const name = splitElementName(item.name);
    const detail = [name.family, name.id].filter(Boolean).join(' · ');
    treeRowText(row, name.label, detail || undefined);
    row.title = `${item.name} · ${ifcTypeLabel(item.ifcType)}`;
  }

  const activate = (ev: Event) => {
    ev.stopPropagation();
    clearScheduleHighlightState();
    document
      .querySelectorAll('.tree-node.selected')
      .forEach((el) => el.classList.remove('selected'));
    row.classList.add('selected');
    focusTreeRow(row);
    if (item.entityIndex !== null) {
      highlightEntity(item.entityIndex);
      showElementInfo(item.entityIndex);
      selectedIndex = item.entityIndex;
      return;
    }
    // Project, storey and category rows are synthetic — they have no
    // entity to inspect. Describe the row itself rather than leaving
    // whatever element was selected before sitting there as if it
    // applied. A category lights up its elements.
    if (item.kind === 'category') {
      const indices = entityIndicesUnder(item);
      highlightWhere((data) => data.entityIndex !== undefined && indices.has(data.entityIndex));
    } else {
      clearHighlight();
    }
    showContainerInfo(item);
    selectedIndex = null;
  };
  row.addEventListener('click', (ev) => {
    // A category opens as it is selected; other rows open from the caret.
    if (item.kind === 'category') {
      setTreeRowExpanded(row, row.getAttribute('aria-expanded') !== 'true');
      activate(ev);
      return;
    }
    if (expandable && ev.target === caret) {
      ev.stopPropagation();
      setTreeRowExpanded(row, row.getAttribute('aria-expanded') !== 'true');
      focusTreeRow(row);
      return;
    }
    activate(ev);
  });
  // Double-click brings the row into view: an element, or everything in
  // a category, on a storey or in the project.
  row.addEventListener('dblclick', (ev) => {
    if (ev.target === caret) return;
    activate(ev);
    frameEntities(entityIndicesUnder(item), item.name);
  });
  row.addEventListener('keydown', (ev) => {
    if (ev.key === 'Enter' || ev.key === ' ') {
      ev.preventDefault();
      activate(ev);
      return;
    }
    const expanded = row.getAttribute('aria-expanded') === 'true';
    if (ev.key === 'ArrowRight') {
      ev.preventDefault();
      if (!expandable) return;
      if (!expanded) setTreeRowExpanded(row, true);
      else {
        const child = row.nextElementSibling?.querySelector<HTMLElement>('.tree-node');
        if (child) focusTreeRow(child);
      }
      return;
    }
    if (ev.key === 'ArrowLeft') {
      ev.preventDefault();
      if (expandable && expanded) {
        setTreeRowExpanded(row, false);
        return;
      }
      const owner = row.parentElement?.closest('.tree-children')?.previousElementSibling;
      if (owner instanceof HTMLElement) focusTreeRow(owner);
      return;
    }
    moveTreeFocus(ev, row);
  });
  wrap.appendChild(row);
  if (expandable) {
    const ch = document.createElement('div');
    ch.className = 'tree-children';
    ch.setAttribute('role', 'group');
    wrap.appendChild(ch);
    const group: TreeGroup = { item, el: ch, level: level + 1, blocks: [], built: 0, more: null };
    treeGroups.set(item, group);
    treeGroupOfElement.set(ch, group);
    // The project and its storeys open; categories and an element's
    // parts wait for a click, so a storey reads as its categories.
    const open = item.kind === 'project' || item.kind === 'storey';
    setTreeRowExpanded(row, open);
    // Every group builds its rows up front while the page has the rows to
    // spend on them; past that a group waits to be opened.
    if (open || treeRowsBuilt < TREE_ROWS_EAGER) fillTreeGroup(group);
  }
  return wrap;
}

/** Home, End and the up and down arrows move the focus along the rows a keyboard user can reach. */
function moveTreeFocus(ev: KeyboardEvent, row: HTMLElement): void {
  if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(ev.key)) return;
  ev.preventDefault();
  const items = visibleTreeRows();
  const idx = items.indexOf(row);
  if (idx < 0) return;
  const next =
    ev.key === 'Home'
      ? items[0]
      : ev.key === 'End'
        ? items[items.length - 1]
        : ev.key === 'ArrowDown'
          ? items[Math.min(items.length - 1, idx + 1)]
          : items[Math.max(0, idx - 1)];
  if (next) focusTreeRow(next);
}

function renderCategories(): void {
  categoriesEl.innerHTML = '';
  // The project and its storeys draw nothing, so they have no toggle.
  // The elements the tree lists, which are the ones the IFC export writes: a
  // piece folded into another element's body is drawn but not counted.
  const counts = new Map<string, number>();
  const countListed = (node: SceneNode) => {
    if (node.entity_index !== null) {
      counts.set(node.ifc_type, (counts.get(node.ifc_type) ?? 0) + 1);
    }
    node.children.forEach(countListed);
  };
  if (sceneGraph) countListed(sceneGraph);
  for (const t of distinctTypes) {
    if (t === 'IFCPROJECT' || t === 'IFCBUILDINGSTOREY') continue;
    const row = document.createElement('label');
    row.className = 'category-toggle';
    const cb = document.createElement('input');
    cb.type = 'checkbox';
    cb.checked = !hiddenTypes.has(t);
    cb.addEventListener('change', () => {
      if (cb.checked) hiddenTypes.delete(t);
      else hiddenTypes.add(t);
      applyCategoryVisibility();
    });
    row.appendChild(cb);
    row.append(` ${ifcTypeLabel(t)}`);
    const count = counts.get(t);
    if (count !== undefined) {
      row.append(' ');
      const n = document.createElement('span');
      n.className = 'tree-count';
      n.textContent = String(count);
      row.appendChild(n);
    }
    categoriesEl.appendChild(row);
  }
  if (!categoriesEl.firstChild) {
    const empty = document.createElement('div');
    empty.className = 'category-empty';
    empty.textContent = 'No drawn elements to show or hide.';
    categoriesEl.appendChild(empty);
  }
}

function applyCategoryVisibility(): void {
  if (!currentModel) return;
  currentModel.traverse((obj) => {
    const u = obj.userData as { ifcType?: string };
    if (u.ifcType) obj.visible = !hiddenTypes.has(u.ifcType);
  });
}

/** Index whose panel payload we are waiting on from the worker. */
let pendingInfoIndex: number | null = null;

function requestElementPanel(idx: number): void {
  pendingInfoIndex = idx;
  worker?.postMessage({ type: 'info', index: idx });
}

// ---------- Element info panel ----------

/** One `label / value` line in the panel's key gutter. */
function infoRow(label: string, value: string, numeric = false): HTMLElement {
  const row = document.createElement('div');
  row.className = 'info-row';
  const k = document.createElement('div');
  k.className = 'k';
  k.textContent = label;
  const v = document.createElement('div');
  v.className = numeric ? 'v num' : 'v';
  v.textContent = value;
  row.appendChild(k);
  row.appendChild(v);
  return row;
}

/**
 * A band of the panel. Groups carry their kind on the left rule:
 * inert data reads grey, anything you can navigate to reads blue,
 * a decode gap reads amber — the same rule the status panel and
 * the scaffold note already use.
 */
function infoGroup(kind: string, title?: string): HTMLElement {
  const box = document.createElement('div');
  box.className = `info-group info-group-${kind}`;
  box.dataset.group = kind;
  if (title) {
    const heading = document.createElement('div');
    heading.className = 'info-group-title';
    heading.textContent = title;
    box.appendChild(heading);
  }
  return box;
}

/** A row you can activate to move the selection somewhere else. */
function jumpButton(label: string, ariaLabel: string, onActivate: () => void): HTMLButtonElement {
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = 'info-relation';
  btn.setAttribute('aria-label', ariaLabel);
  btn.textContent = label;
  btn.addEventListener('click', onActivate);
  return btn;
}

function relationHeading(text: string): HTMLElement {
  const heading = document.createElement('div');
  heading.className = 'info-relations-title';
  heading.textContent = text;
  return heading;
}

function relationRow(rel: RelatedElement): HTMLElement {
  const btn = jumpButton(
    `${rel.name} · ${ifcTypeLabel(rel.ifc_type)}`,
    `Select ${rel.name} (${ifcTypeLabel(rel.ifc_type)})`,
    () => selectEntity(rel.entity_index),
  );
  btn.dataset.entityIndex = String(rel.entity_index);
  return btn;
}

/**
 * Host relationships (M4-04) computed by `element_info_panel` in
 * Rust: the wall a door or window sits in, the openings a wall
 * carries, and the walls a wall butt-joins at its ends (RE-70, RE-73). Each
 * row re-selects its element — mouse or keyboard.
 */
function relationsBox(panel: ElementInfoPanel): HTMLElement | null {
  const host = panel.host ?? null;
  const hosted = panel.hosted ?? [];
  const joins = panel.joins ?? [];
  if (!host && hosted.length === 0 && joins.length === 0) return null;
  const box = document.createElement('div');
  box.id = 'info-relations';
  box.className = 'info-relations';
  if (host) {
    box.appendChild(relationHeading('Hosted by'));
    box.appendChild(relationRow(host));
  }
  if (hosted.length > 0) {
    box.appendChild(
      relationHeading(`Hosts ${hosted.length} opening${hosted.length === 1 ? '' : 's'}`),
    );
    for (const rel of hosted) box.appendChild(relationRow(rel));
  }
  for (const join of joins) {
    box.appendChild(
      relationHeading(`${join.end}: ${join.runs_through ? 'runs through' : 'stops at'}`),
    );
    box.appendChild(relationRow(join.wall));
  }
  return box;
}

/** Storey band — the level in words, and a jump to it in the tree. */
function storeyGroup(storey: PanelStorey): HTMLElement {
  const box = infoGroup('storey', 'Storey');
  const btn = jumpButton(
    `${storey.name} · ${storey.elevation_label}`,
    `Select storey ${storey.name} in the scene tree`,
    () => selectStorey(storey.index),
  );
  btn.dataset.storeyIndex = String(storey.index);
  box.appendChild(btn);
  return box;
}

/**
 * Material band — the material in words, plus how many elements
 * share it and a toggle that lights all of them in the 3-D view.
 * The affordance only appears when the scene has tintable meshes.
 */
/**
 * An element's layers, one row each. Where the scene can tint, a layer's
 * row is a toggle that lights up every element using its material,
 * directly or as a layer.
 */
function layersGroup(layers: PanelLayers): HTMLElement {
  const box = infoGroup(
    'layers',
    layers.order ? `Layers · ${layers.name} · ${layers.order}` : `Layers · ${layers.name}`,
  );
  const tintable = sceneSupportsHighlight();
  for (const layer of layers.rows) {
    const material = layer.material;
    if (!material || !tintable) {
      box.appendChild(infoRow(layer.label, layer.value, true));
      continue;
    }
    const shared = `${material.element_count} element${material.element_count === 1 ? '' : 's'}`;
    const btn = document.createElement('button');
    btn.type = 'button';
    btn.className = 'info-row info-layer';
    btn.setAttribute('aria-label', `Highlight the ${shared} using ${material.name} in the 3-D view`);
    btn.setAttribute('aria-pressed', 'false');
    btn.dataset.materialIndex = String(material.index);
    const k = document.createElement('span');
    k.className = 'k';
    k.textContent = layer.label;
    const v = document.createElement('span');
    v.className = 'v num';
    v.textContent = layer.value;
    btn.append(k, v);
    btn.addEventListener('click', () => {
      for (const other of box.querySelectorAll('.info-layer')) {
        if (other !== btn) other.setAttribute('aria-pressed', 'false');
      }
      if (activeMaterialHighlight === material.index) {
        clearHighlight();
        btn.setAttribute('aria-pressed', 'false');
        return;
      }
      const lit = highlightByMaterial(material.index);
      btn.setAttribute('aria-pressed', lit > 0 ? 'true' : 'false');
    });
    box.appendChild(btn);
  }
  box.appendChild(infoRow('Total', layers.total, true));
  return box;
}

function materialGroup(material: PanelMaterial): HTMLElement {
  const box = infoGroup('material', 'Material');
  const shared = `${material.element_count} element${material.element_count === 1 ? '' : 's'}`;
  if (!sceneSupportsHighlight()) {
    box.appendChild(infoRow(material.name, shared));
    return box;
  }
  const btn = jumpButton(
    `${material.name} · ${shared}`,
    `Highlight the ${shared} using ${material.name} in the 3-D view`,
    () => {
      if (activeMaterialHighlight === material.index) {
        clearHighlight();
        btn.setAttribute('aria-pressed', 'false');
        return;
      }
      const lit = highlightByMaterial(material.index);
      btn.setAttribute('aria-pressed', lit > 0 ? 'true' : 'false');
    },
  );
  btn.dataset.materialIndex = String(material.index);
  btn.setAttribute('aria-pressed', 'false');
  box.appendChild(btn);
  return box;
}

/**
 * The element's property set as a titled group with one row per
 * property. Long sets collapse behind a native `<details>` so the
 * bands below them stay reachable without a scroll marathon.
 */
const PROPERTY_COLLAPSE_THRESHOLD = 8;

function propertyGroup(group: PanelPropertyGroup): HTMLElement | null {
  if (group.properties.length === 0) return null;
  const rows = document.createElement('div');
  rows.className = 'info-properties';
  for (const p of group.properties) {
    const row = infoRow(p.name, p.value, p.numeric);
    row.dataset.propertyKind = p.kind;
    rows.appendChild(row);
  }
  const count = group.properties.length;
  if (count <= PROPERTY_COLLAPSE_THRESHOLD) {
    const box = infoGroup('properties', group.name);
    box.appendChild(rows);
    return box;
  }
  const box = infoGroup('properties');
  const details = document.createElement('details');
  details.className = 'info-details';
  details.id = 'info-properties-details';
  const summary = document.createElement('summary');
  summary.className = 'info-group-title';
  summary.textContent = `${group.name} · ${count} properties`;
  details.appendChild(summary);
  details.appendChild(rows);
  box.appendChild(details);
  return box;
}

/**
 * The one honest line about what is absent. A field is only
 * called out when the export diagnostics name it as a known
 * decode gap; everything else absent is simply omitted.
 */
const GAP_LABELS: Record<string, string> = {
  storey: 'level binding',
  material: 'material',
  properties: 'IFC property sets',
  placement: 'placement',
  extents: 'geometry extents',
};

function knownGaps(missing: string[], diagnostics: ExportDiagnostics | null): string[] {
  if (!diagnostics) return [];
  const warnings = (diagnostics.warnings ?? []).join(' ');
  const unsupported = diagnostics.unsupported_features ?? [];
  const partialGeometry = unsupported.includes('partial_element_geometry');
  const confirmed: Record<string, boolean> = {
    storey: warnings.includes('unsupported_geometry_missing_level'),
    material: (diagnostics.exported?.material_count ?? 0) === 0,
    properties:
      unsupported.includes('revit_element_parameters_to_ifc_property_sets') ||
      (diagnostics.decoded?.parameter_value_count ?? 0) === 0,
    placement: partialGeometry,
    extents: partialGeometry || warnings.includes('unsupported_geometry_missing_dimensions'),
  };
  return missing.filter((m) => confirmed[m]).map((m) => GAP_LABELS[m] ?? m);
}

function gapNote(missing: string[]): HTMLElement | null {
  const gaps = knownGaps(missing, currentDiagnostics);
  if (gaps.length === 0) return null;
  const box = infoGroup('gap');
  box.id = 'info-gaps';
  const line = document.createElement('div');
  line.className = 'info-gap-note';
  line.textContent = `Not recovered: ${gaps.join(', ')}`;
  box.appendChild(line);
  return box;
}

/** The panel's "Zoom to element" button, when the element is drawn. */
function zoomButton(): HTMLButtonElement | null {
  const idx = selectedIndex;
  if (idx === null || !currentModel) return null;
  let drawn = false;
  currentModel.traverse((obj) => {
    if ((obj.userData as { entityIndex?: number }).entityIndex === idx) drawn = true;
  });
  if (!drawn) return null;
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = 'side-btn';
  btn.id = 'zoom-to-element';
  btn.textContent = 'Zoom to element';
  btn.title = 'Bring this element into view (F, or double-click it)';
  btn.addEventListener('click', () => frameEntities(new Set([idx]), entityLabel(idx)));
  return btn;
}

/**
 * `value` with each `Map` in it made a plain object. serde-wasm-bindgen
 * hands a Rust map over as a `Map`, which `JSON.stringify` writes as `{}`
 * and `Object.values` reads as empty.
 */
function plainJson<T>(value: T): T {
  if (value instanceof Map) {
    return Object.fromEntries(
      [...value].map(([key, item]) => [String(key), plainJson(item)]),
    ) as T;
  }
  if (Array.isArray(value)) return value.map((item) => plainJson(item)) as T;
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value).map(([key, item]) => [key, plainJson(item)]),
    ) as T;
  }
  return value;
}

/** The GlobalId Revit's own export gives entity `idx`, when rebuilt (RE-48). */
function revitGlobalId(idx: number): string | undefined {
  const elements = model?.global_ids?.elements;
  if (elements instanceof Map) return elements.get(idx);
  return elements?.[String(idx)];
}

// ---------- Saved parameters (#35, RE-207) ----------
/**
 * An element's saved parameters are read when it is selected, by a worker of
 * its own that ends with the answer (parameters-worker.ts, the `savedParameters`
 * binding): not by the worker that opened the file, whose reads would hold up
 * the panel behind them and whose wasm memory only grows, and not for every
 * element at the open, which was a pass over the file for each. The file is
 * read again from the `File` this page already holds, so nothing of it stays
 * resident beyond what was held before; a read costs memory only while it runs.
 * The answers are kept per element in a small map until the file is opened
 * again, so selecting an element a second time reads nothing.
 *
 * Selecting an element makes no request: the worker is bundled into this page
 * and starts from a blob URL, and it starts from the compiled wasm module the
 * worker that opened the file handed over (`decoderWasm`), so nothing is
 * fetched, and a page taken offline after it loaded still reads.
 */
const PARAMETERS_READ_DELAY_MS = 150;
/** How long a worker may take to say it has started before it is taken to have died. */
const PARAMETERS_STARTUP_MS = 20_000;
/**
 * The wasm memory a page may hold, the open's and any the main thread's exports
 * made, for a selection to be read on its own. A read has taken 0.74 to 1.04
 * times the open's wasm memory (rising with the file: Einhoven 23 MiB, Core
 * Interior 326 MiB, the BIM4LCA office 910 MiB), and nothing above the office's
 * has been measured, so above 1 GiB a read is only made when asked: a renderer
 * that runs out of memory takes the tab and the open model with it, without an
 * error to show.
 */
const PARAMETERS_MEASURED_WASM_BYTES = 2 ** 30;
/**
 * The most elements whose answers are kept, and the most characters of JSON
 * they come to between them (#1887); the least recently selected goes first.
 * The answer just read always stays, so a selection is not read again for being
 * larger than the rest. An answer measured on the BIM4LCA office is about 2,300
 * characters, so 64 of them are about 150,000 and the budget, 8 Mi, is fifty
 * times that: it bounds what an element record that lists a great many
 * parameters can make the page hold, and no real selection reaches it. The
 * address lowers either limit, for the browser tests (`?parametersRemembered=`,
 * `?parametersRememberedCharacters=`) and never raises it: a link cannot make
 * the page hold more than it would have.
 */
const PARAMETERS_REMEMBERED_MOST = 64;
const PARAMETERS_REMEMBERED_CHARACTERS_MOST = 8 * 2 ** 20;
function parametersLimit(name: string, most: number): number {
  const asked = wholeBytesParam(new URLSearchParams(window.location.search), name);
  return Math.min(most, Math.max(1, asked ?? most));
}
const PARAMETERS_REMEMBERED = parametersLimit('parametersRemembered', PARAMETERS_REMEMBERED_MOST);
const PARAMETERS_REMEMBERED_CHARACTERS = parametersLimit(
  'parametersRememberedCharacters',
  PARAMETERS_REMEMBERED_CHARACTERS_MOST,
);
/** An element's answer, and the characters of its JSON, which is what the map is counted in. */
interface RememberedParameters {
  reading: SavedParametersReading;
  characters: number;
}
const savedParametersCache = new Map<number, RememberedParameters>();
let savedParametersCacheCharacters = 0;
let parametersWorker: Worker | null = null;
let parametersTimer: number | undefined;
let parametersStartup: number | undefined;
/** The ElementId a read is running or waiting for, when one is. */
let parametersTarget: number | null = null;
/** Ends the lock request that tells the page its worker has gone. */
let parametersWatch: AbortController | null = null;
let parametersReads = 0;
/** The decoder's compiled wasm, which the worker of the first open handed over: the page's for good. */
let decoderWasm: WebAssembly.Module | null = null;
/** Whether a read of this file has run out of memory: it is read only when asked from then on. */
let parametersRanOutOfMemory = false;
/** The wasm linear memory the open left (worker.ts `stats`), which a read in a second instance comes on top of. */
let loadedWasmMemoryBytes = 0;
const infoLiveEl = $('info-live');

/** Says something to a screen reader, without moving what it is reading. */
function announceParameters(text: string): void {
  infoLiveEl.textContent = '';
  window.requestAnimationFrame(() => {
    infoLiveEl.textContent = text;
  });
}

/** Ends a read in progress, or one waiting to start. */
function stopParametersRead(): void {
  window.clearTimeout(parametersTimer);
  window.clearTimeout(parametersStartup);
  parametersTimer = undefined;
  parametersStartup = undefined;
  parametersWatch?.abort();
  parametersWatch = null;
  parametersWorker?.terminate();
  parametersWorker = null;
  parametersTarget = null;
}

/** What a new file starts from. */
function resetSavedParameters(): void {
  stopParametersRead();
  savedParametersCache.clear();
  savedParametersCacheCharacters = 0;
  loadedWasmMemoryBytes = 0;
  parametersRanOutOfMemory = false;
}

/** The answer kept for an element, which counts as its most recent use. */
function recallParameters(id: number): SavedParametersReading | undefined {
  const kept = savedParametersCache.get(id);
  if (!kept) return undefined;
  savedParametersCache.delete(id);
  savedParametersCache.set(id, kept);
  return kept.reading;
}

/**
 * Keeps an element's answer, and lets the least recently used ones go past
 * either limit, the one just kept excepted.
 */
function rememberParameters(id: number, reading: SavedParametersReading): void {
  const before = savedParametersCache.get(id);
  if (before) savedParametersCacheCharacters -= before.characters;
  const characters = JSON.stringify(reading).length;
  savedParametersCache.delete(id);
  savedParametersCache.set(id, { reading, characters });
  savedParametersCacheCharacters += characters;
  while (
    savedParametersCache.size > PARAMETERS_REMEMBERED ||
    (savedParametersCache.size > 1 && savedParametersCacheCharacters > PARAMETERS_REMEMBERED_CHARACTERS)
  ) {
    const [oldest, gone] = savedParametersCache.entries().next().value as [number, RememberedParameters];
    savedParametersCache.delete(oldest);
    savedParametersCacheCharacters -= gone.characters;
  }
}

/** The wasm memory this page holds: the open's, and the main thread's where its exports made an instance. */
function residentWasmBytes(): number {
  return loadedWasmMemoryBytes + (mainThreadWasmMemory?.buffer.byteLength ?? 0);
}

/**
 * The most wasm memory the page may hold for a selection to be read on its own:
 * what a read has been measured beside (1 GiB), and a quarter of the ceiling the
 * page judges an open by where `?memoryCeilingBytes=` lowers that.
 */
function parametersAutoReadLimit(): number {
  return Math.min(PARAMETERS_MEASURED_WASM_BYTES, memoryCeilingBytes / 4);
}

/** Whether a selection reads on its own: not past what has been measured, and not after a read ran out of memory. */
function parametersReadAutomatically(): boolean {
  return !parametersRanOutOfMemory && residentWasmBytes() <= parametersAutoReadLimit();
}

type ParametersState =
  | 'loading'
  | 'ready'
  | 'empty'
  | 'unsupported'
  | 'error'
  | 'on-request'
  | 'no-id';

/** Clears what the group showed and says what it shows now. */
function parametersBody(box: HTMLElement, state: ParametersState): HTMLElement {
  box.dataset.state = state;
  if (state === 'loading') box.setAttribute('aria-busy', 'true');
  else box.removeAttribute('aria-busy');
  box.querySelector('.info-parameters-body')?.remove();
  const body = document.createElement('div');
  body.className = 'info-parameters-body';
  // Focus can be put here when a button that was in the group is gone.
  body.tabIndex = -1;
  box.appendChild(body);
  return body;
}

function parametersMessage(box: HTMLElement, state: ParametersState, text: string): HTMLElement {
  const body = parametersBody(box, state);
  const line = document.createElement('div');
  line.className = 'info-parameters-state';
  line.textContent = text;
  body.appendChild(line);
  return body;
}

function parametersButton(id: string, label: string, onActivate: () => void): HTMLButtonElement {
  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = 'side-btn';
  btn.id = id;
  btn.textContent = label;
  btn.addEventListener('click', onActivate);
  return btn;
}

/**
 * A button of the group was pressed and is gone: focus goes to the group's
 * message, which says the read has begun, and from there to its result.
 */
function readFromButton(box: HTMLElement, id: number): void {
  box.dataset.focusResult = '1';
  const body = parametersMessage(box, 'loading', 'Reading the saved parameters…');
  announceParameters('Reading the saved parameters…');
  body.focus();
  readSavedParameters(id);
}

/** Puts focus on the result of a read the user asked for with a button. */
function focusParametersResult(box: HTMLElement): void {
  if (box.dataset.focusResult === undefined) return;
  delete box.dataset.focusResult;
  box.querySelector<HTMLElement>('.info-parameters-body')?.focus();
}

/** One parameter: its name, or its id where it has none, where it is defined, and its value as saved. */
function parameterRow(p: SavedParameter): HTMLElement {
  const { label, why } = parameterLabel(p);
  const row = document.createElement('div');
  row.className = 'info-row';
  const k = document.createElement('div');
  k.className = 'k';
  const name = document.createElement('span');
  name.className = 'info-parameter-name';
  name.textContent = label;
  const kind = document.createElement('span');
  kind.className = 'info-parameter-kind';
  kind.textContent = parameterKindText(p);
  k.append(name, kind);
  const v = document.createElement('div');
  v.className = parameterIsNumeric(p) ? 'v num' : 'v';
  v.textContent = parameterValueText(p);
  row.append(k, v);
  // The entry as the reader gave it, for tests/parameters-row.spec.ts, which
  // holds it to what rvt-elements prints for the same element.
  row.dataset.parameter = JSON.stringify(p);
  row.dataset.parameterId = String(p.parameter_id);
  row.dataset.level = p.level;
  row.dataset.kind = p.kind;
  // Everything the tooltip says is also in the text: it is only a nearer reading.
  row.title = [
    why ?? parameterKindText(p),
    p.level === 'type' ? `read from type record ${p.source_element_id} (${p.source_class})` : null,
  ]
    .filter((part): part is string => part !== null)
    .join(' · ');
  return row;
}

/** The parameters of one level: the named first, then the nameless in a fold, each reason for having no name together. */
function parameterSection(
  level: 'instance' | 'type',
  entries: SavedParameter[],
): HTMLElement | null {
  const mine = entries.filter((p) => p.level === level);
  if (mine.length === 0) return null;
  const section = document.createElement('div');
  section.className = 'info-parameters-section';
  section.dataset.level = level;
  const title = document.createElement('div');
  title.className = 'info-parameters-subtitle';
  title.textContent = `${level === 'instance' ? 'Instance' : 'Type'} parameters · ${mine.length}`;
  section.appendChild(title);
  const named = mine.filter((p) => parameterLabel(p).why === null);
  const unnamed = mine.filter((p) => parameterLabel(p).why !== null);
  for (const p of named) section.appendChild(parameterRow(p));
  if (unnamed.length > 0) {
    const details = document.createElement('details');
    details.className = 'info-details info-parameters-unnamed';
    // Nothing but nameless ones: the fold would hide the whole list.
    details.open = named.length === 0;
    const summary = document.createElement('summary');
    summary.className = 'info-parameters-summary';
    summary.textContent = `${unnamed.length} without a name`;
    details.appendChild(summary);
    const byReason = new Map<string, SavedParameter[]>();
    for (const p of unnamed) {
      const why = parameterLabel(p).why!;
      byReason.set(why, [...(byReason.get(why) ?? []), p]);
    }
    for (const [why, group] of byReason) {
      const reason = document.createElement('div');
      reason.className = 'info-parameters-state info-parameters-reason';
      reason.textContent = `${group.length}: ${why}`;
      details.appendChild(reason);
      for (const p of group) details.appendChild(parameterRow(p));
    }
    section.appendChild(details);
  }
  return section;
}

/** What the read could not list for the element, in words. */
function parametersNotListed(reading: SavedParametersResult): string | null {
  const notes: string[] = [];
  const slots = reading.unresolved_family_slots;
  if (slots > 0) {
    notes.push(`${slots} family parameter slot${slots === 1 ? '' : 's'} no definition in the file resolves`);
  }
  const defaults = reading.instance_parameters_without_value;
  if (defaults > 0) {
    notes.push(
      `${defaults} instance parameter${defaults === 1 ? '' : 's'} this element holds no value for (its type holds the family's default, which is not the element's)`,
    );
  }
  return notes.length > 0 ? `Not listed: ${notes.join('; ')}.` : null;
}

/** A line under the group's body, in the body's own voice. */
function parametersNote(body: HTMLElement, text: string): void {
  const line = document.createElement('div');
  line.className = 'info-parameters-state';
  line.textContent = text;
  body.appendChild(line);
}

/** The note a release read only experimentally carries (RE-281), which the support matrix states. */
function releaseNote(reading: SavedParametersResult): string | null {
  if (!reading.release_experimental) return null;
  const [measuredFirst] = reading.releases_measured;
  return `This is a Revit ${reading.release} file. Saved parameters are read from files of Revit ${reading.releases_read[0]} to ${measuredFirst - 1} experimentally: no Revit export of those releases saves a parameter to check them against.`;
}

/** An element's saved parameters in the group, or why there are none. */
function showSavedParameters(box: HTMLElement, id: number, reading: SavedParametersReading): void {
  box.dataset.readMs = String(Math.round(reading.readMs));
  box.dataset.wasmMemoryBytes = String(reading.wasmMemoryBytes);
  // What the page keeps, for tests/parameters-row.spec.ts and the office's report.
  box.dataset.cacheElements = String(savedParametersCache.size);
  box.dataset.cacheCharacters = String(savedParametersCacheCharacters);
  if (!reading.release_read) {
    const text = `This is a Revit ${reading.release} file. rvt-rs reads saved parameters from files of ${releasesText(reading)}, so none are listed.`;
    parametersMessage(box, 'unsupported', text);
    announceParameters(text);
    return;
  }
  const entries = reading.elements[String(id)];
  const note = releaseNote(reading);
  if (entries === undefined || entries.length === 0) {
    const text =
      entries === undefined
        ? 'The file holds no record of this element that rvt-rs reads, so it has no saved parameters to list.'
        : "This element's record holds no saved parameters.";
    const body = parametersMessage(box, 'empty', text);
    if (note) parametersNote(body, note);
    announceParameters(text);
    return;
  }
  const body = parametersBody(box, 'ready');
  for (const level of ['instance', 'type'] as const) {
    const section = parameterSection(level, entries);
    if (section) body.appendChild(section);
  }
  const notListed = parametersNotListed(reading);
  if (notListed) parametersNote(body, notListed);
  if (note) parametersNote(body, note);
  announceParameters(
    `${entries.length} saved parameter${entries.length === 1 ? '' : 's'} read for this element.`,
  );
}

/** What a read's end was: its answer, the engine's own failure, or a worker that ended with no word. */
type ParametersOutcome =
  | { type: 'result'; result: SavedParametersResult; readMs: number; wasmMemoryBytes: number }
  | { type: 'error'; message: string; outOfMemory: boolean; stopped?: boolean };

/** A read that did not finish, in words, with a way to try it again. */
function showParametersError(
  box: HTMLElement,
  id: number,
  outcome: Extract<ParametersOutcome, { type: 'error' }>,
): void {
  const text = outcome.stopped
    ? 'The worker that reads them stopped without answering. A browser ends a worker that runs out of memory this way, so this file is read only when asked from now on, and a read may close this tab.'
    : outcome.outOfMemory
      ? `The tab ran out of memory reading the file a second time (${outcome.message}). This file is read only when asked from now on, and a read may close this tab.`
      : `The saved parameters could not be read: ${outcome.message}`;
  const body = parametersMessage(box, 'error', text);
  body.appendChild(
    parametersButton('retry-saved-parameters', 'Try again', () => readFromButton(box, id)),
  );
  announceParameters(text);
}

/** A read's end: its answer kept, and shown if the panel is still the element's. */
function finishRead(id: number, outcome: ParametersOutcome): void {
  parametersTarget = null;
  if (outcome.type === 'result') {
    rememberParameters(id, {
      ...outcome.result,
      readMs: outcome.readMs,
      wasmMemoryBytes: outcome.wasmMemoryBytes,
    });
  } else if (outcome.outOfMemory) {
    parametersRanOutOfMemory = true;
  }
  // The panel may have moved on while the worker read.
  const box = infoEl.querySelector<HTMLElement>('#info-parameters');
  if (!box || box.dataset.elementId !== String(id)) return;
  const reading = outcome.type === 'result' ? recallParameters(id) : undefined;
  if (outcome.type === 'result' && reading) showSavedParameters(box, id, reading);
  else if (outcome.type === 'error') showParametersError(box, id, outcome);
  focusParametersResult(box);
}

/**
 * Reads the saved parameters of the element `id` of the open file in a new
 * worker, which ends with its answer. Another read, or a new file, ends this one.
 * A worker the browser ends without a word is found by the lock it held, or by
 * not having started.
 */
function readSavedParameters(id: number): void {
  stopParametersRead();
  parametersTarget = id;
  const file = currentFile;
  const module = decoderWasm;
  if (!file || !module) {
    finishRead(id, {
      type: 'error',
      message: file ? 'the open did not hand over the decoder' : 'the file is no longer open',
      outOfMemory: false,
    });
    return;
  }
  const limits: OpenLimitsOptions = {};
  if (currentOpenLimits?.largeModel !== undefined) limits.largeModel = currentOpenLimits.largeModel;
  if (currentOpenLimits?.maxStreamBytes !== undefined) {
    limits.maxStreamBytes = currentOpenLimits.maxStreamBytes;
  }
  if (currentOpenLimits?.maxInflateBytes !== undefined) {
    limits.maxInflateBytes = currentOpenLimits.maxInflateBytes;
  }
  const lock =
    typeof navigator !== 'undefined' && navigator.locks
      ? `rvt-parameters-${++parametersReads}-${Math.random().toString(36).slice(2)}`
      : null;
  const w = new ParametersWorker({ name: 'saved-parameters' });
  parametersWorker = w;
  const settle = (outcome: ParametersOutcome): void => {
    if (parametersWorker !== w) return;
    stopParametersRead();
    finishRead(id, outcome);
  };
  const stopped = (): void =>
    settle({
      type: 'error',
      message: 'the worker stopped without answering',
      outOfMemory: true,
      stopped: true,
    });
  parametersStartup = window.setTimeout(stopped, PARAMETERS_STARTUP_MS);
  w.addEventListener('message', (ev: MessageEvent<ParametersOutcome | { type: 'started' }>) => {
    const msg = ev.data;
    if (msg.type !== 'started') {
      settle(msg);
      return;
    }
    window.clearTimeout(parametersStartup);
    parametersStartup = undefined;
    if (lock === null) return;
    // The worker holds the lock until it ends, for whatever reason: being given
    // it is the worker having gone, and ending the read first cancels the wait.
    const watch = new AbortController();
    parametersWatch = watch;
    navigator.locks.request(lock, { signal: watch.signal }, stopped).catch(() => undefined);
  });
  w.addEventListener('error', (ev: ErrorEvent) =>
    settle({
      type: 'error',
      message: ev.message ? ev.message : 'the worker that reads them stopped',
      outOfMemory: false,
    }),
  );
  w.addEventListener('messageerror', () =>
    settle({ type: 'error', message: 'the answer could not be received', outOfMemory: false }),
  );
  w.postMessage({ type: 'read', file, ids: [id], module, lock, limits });
}

/** The group of a selected element: its saved parameters, or the state they are in. */
function savedParametersGroup(panel: ElementInfoPanel): HTMLElement {
  const box = infoGroup('parameters', 'Saved parameters');
  box.id = 'info-parameters';
  const id = elementIdOf(panel.type_guid);
  if (id === null) {
    stopParametersRead();
    const text =
      'This element has no ElementId in the export, so its saved parameters cannot be looked up.';
    parametersMessage(box, 'no-id', text);
    announceParameters(text);
    return box;
  }
  box.dataset.elementId = String(id);
  const cached = recallParameters(id);
  if (cached) {
    // What an earlier selection read; a read for another element is for one nobody is looking at.
    stopParametersRead();
    showSavedParameters(box, id, cached);
    return box;
  }
  if (parametersTarget === id) {
    // This element's read is already running (or waiting out the delay): it fills this group when it ends.
    parametersMessage(box, 'loading', 'Reading the saved parameters…');
    announceParameters('Reading the saved parameters…');
    return box;
  }
  stopParametersRead();
  if (!currentFile) {
    parametersMessage(box, 'error', 'The file is no longer open.');
    return box;
  }
  if (!parametersReadAutomatically()) {
    const text = parametersRanOutOfMemory
      ? 'The last read of this file ran out of memory, so saved parameters are read only when asked. A read may close this tab.'
      : `Reading them opens the file a second time, in a worker of its own. That has taken about as much memory as opening it did, and this page already holds ${formatMemory(residentWasmBytes())} of wasm memory, past the ${formatMemory(parametersAutoReadLimit())} a read has been measured beside. Past that a read can run the tab out of memory, which closes the tab and the open model with it, so clicking may close this tab.`;
    const body = parametersMessage(box, 'on-request', text);
    body.appendChild(
      parametersButton('read-saved-parameters', 'Read saved parameters', () =>
        readFromButton(box, id),
      ),
    );
    announceParameters(text);
    return box;
  }
  parametersMessage(box, 'loading', 'Reading the saved parameters…');
  announceParameters('Reading the saved parameters…');
  // A run through the tree selects element after element: only the one the
  // selection stops on is read.
  parametersTarget = id;
  parametersTimer = window.setTimeout(() => readSavedParameters(id), PARAMETERS_READ_DELAY_MS);
  return box;
}

/** Render the whole panel from the Rust payload. */
function renderElementPanel(panel: ElementInfoPanel): void {
  infoEl.innerHTML = '';
  infoEl.removeAttribute('aria-busy');

  const identity = infoGroup('identity');
  identity.appendChild(infoRow('Name', panel.name));
  identity.appendChild(
    infoRow(
      'Type',
      panel.predefined_type
        ? `${ifcTypeLabel(panel.ifc_type)} · ${panel.predefined_type}`
        : ifcTypeLabel(panel.ifc_type),
    ),
  );
  // `type_guid` carries the Revit ElementId, which the IFC export writes as
  // the element's Tag; the GlobalId is Revit's own (RE-48) when rebuilt.
  if (panel.type_guid) identity.appendChild(infoRow('ElementId', panel.type_guid, true));
  const globalId = selectedIndex !== null ? revitGlobalId(selectedIndex) : undefined;
  if (globalId) identity.appendChild(infoRow('GlobalId', globalId));
  infoEl.appendChild(identity);
  const zoom = zoomButton();
  if (zoom) infoEl.appendChild(zoom);

  if (panel.storey) infoEl.appendChild(storeyGroup(panel.storey));
  if (panel.material) infoEl.appendChild(materialGroup(panel.material));
  if (panel.layers && panel.layers.rows.length > 0) infoEl.appendChild(layersGroup(panel.layers));

  const placement = panel.placement_rows ?? [];
  if (placement.length > 0) {
    const box = infoGroup('placement', 'Placement');
    for (const row of placement) box.appendChild(infoRow(row.label, row.value, true));
    infoEl.appendChild(box);
  }

  const extents = panel.extent_rows ?? [];
  if (extents.length > 0) {
    const box = infoGroup('extents', 'Extents');
    for (const row of extents) {
      box.appendChild(infoRow(row.label, row.value, row.label !== 'Profile'));
    }
    infoEl.appendChild(box);
  }

  if (panel.property_group) {
    const box = propertyGroup(panel.property_group);
    if (box) infoEl.appendChild(box);
  }
  for (const group of panel.further_property_groups ?? []) {
    const box = propertyGroup(group);
    if (box) infoEl.appendChild(box);
  }

  infoEl.appendChild(savedParametersGroup(panel));

  const relations = relationsBox(panel);
  if (relations) infoEl.appendChild(relations);

  const gaps = gapNote(panel.missing ?? []);
  if (gaps) infoEl.appendChild(gaps);
}

/**
 * Fallback for an index the typed exporter could not describe —
 * scaffold and partial exports reach the scene graph without a
 * populated `entities[]` row. Says so rather than showing blanks.
 */
function renderScaffoldPanel(idx: number): void {
  const node = findSceneNodeByIndex(sceneGraph, idx);
  stopParametersRead();
  infoEl.innerHTML = '';
  infoEl.removeAttribute('aria-busy');
  if (!node) {
    infoEl.textContent = 'That element is no longer in the scene.';
    return;
  }
  const identity = infoGroup('identity');
  identity.appendChild(infoRow('Name', node.name));
  identity.appendChild(infoRow('Type', ifcTypeLabel(node.ifc_type)));
  infoEl.appendChild(identity);
  const box = infoGroup('gap');
  box.id = 'info-gaps';
  const line = document.createElement('div');
  line.className = 'info-gap-note';
  line.textContent = 'Partial decode — no typed element fields were recovered for this node.';
  box.appendChild(line);
  infoEl.appendChild(box);
}

/**
 * Selection entry point for the panel. The payload comes back
 * from the worker, so show the identity we already hold rather
 * than blanking the panel for the round-trip.
 */
function showElementInfo(idx: number): void {
  requestElementPanel(idx);
  const e = model?.entities?.[idx];
  // The panel is replaced by the identity alone until the worker answers, so a
  // read running for another element is for one nobody is looking at. The
  // element a read is running for keeps it: selecting it again must not
  // start it over.
  if (elementIdOf(e?.type_guid) !== parametersTarget) stopParametersRead();
  const node = e ? null : findSceneNodeByIndex(sceneGraph, idx);
  const name = e?.name ?? node?.name;
  const ifcType = e?.ifc_type ?? node?.ifc_type;
  if (name === undefined || ifcType === undefined) return;
  infoEl.innerHTML = '';
  infoEl.setAttribute('aria-busy', 'true');
  const identity = infoGroup('identity');
  identity.appendChild(infoRow('Name', name));
  identity.appendChild(infoRow('Type', ifcTypeLabel(ifcType)));
  infoEl.appendChild(identity);
}

/**
 * Panel for a synthetic container node — the project root or a
 * storey. These carry no entity, so the panel reports what the
 * node itself is and how much it holds.
 */
function showContainerInfo(node: TreeItem): void {
  pendingInfoIndex = null;
  stopParametersRead();
  infoEl.innerHTML = '';
  infoEl.removeAttribute('aria-busy');
  const identity = infoGroup('identity');
  const elements = `${node.count} element${node.count === 1 ? '' : 's'}`;
  if (node.kind === 'category') {
    const rows = node.children.length;
    const parts = node.count - rows;
    identity.appendChild(infoRow('Category', ifcTypeLabel(node.ifcType)));
    identity.appendChild(infoRow('Storey', node.storeyName ?? 'Unassigned'));
    identity.appendChild(infoRow('Contains', `${rows} element${rows === 1 ? '' : 's'}`, true));
    if (parts > 0) {
      identity.appendChild(infoRow('Their parts', String(parts), true));
    }
    infoEl.appendChild(identity);
    return;
  }
  identity.appendChild(infoRow('Name', node.name));
  identity.appendChild(infoRow('Type', ifcTypeLabel(node.ifcType)));
  const kids = node.children.length;
  if (node.kind === 'storey') {
    identity.appendChild(infoRow('Contains', elements, true));
    const storeys = model?.building_storeys ?? [];
    const idx = node.storeyIndex;
    const elev = idx != null ? storeys[idx]?.elevation_feet : undefined;
    if (typeof elev === 'number') {
      identity.appendChild(infoRow('Elevation', `${elev.toFixed(3)} ft`, true));
    }
  } else {
    identity.appendChild(infoRow('Contains', `${kids} storey${kids === 1 ? '' : 's'}`, true));
  }
  infoEl.appendChild(identity);
}

/** Select a storey node in the tree — the panel's storey jump. */
function selectStorey(storeyIndex: number): void {
  const storey = treeItemByStorey.get(storeyIndex);
  if (storey) ensureTreeRow(storey);
  const row = treeEl.querySelector<HTMLElement>(`.tree-node[data-storey-index="${storeyIndex}"]`);
  if (!row) return;
  document.querySelectorAll('.tree-node.selected').forEach((el) => el.classList.remove('selected'));
  revealTreeRow(row);
  row.classList.add('selected');
  row.scrollIntoView({
    block: 'nearest',
    behavior: prefersReducedMotion() ? 'auto' : 'smooth',
  });
  focusTreeRow(row);
  clearHighlight();
}

function findSceneNodeByIndex(node: SceneNode | null, idx: number): SceneNode | null {
  if (!node) return null;
  if (node.entity_index === idx) return node;
  for (const child of node.children) {
    const hit = findSceneNodeByIndex(child, idx);
    if (hit) return hit;
  }
  return null;
}

/**
 * The schedule as a per-type breakdown rather than a bare
 * total. Each row carries its count, and — when the scene has
 * tintable meshes — a toggle that lights the whole type at once.
 */
function renderScheduleSummary(schedule: unknown): void {
  const s = schedule as Schedule | null;
  // Both children are in the markup so the live region survives a
  // re-render — replacing it would silence the announcement.
  const total = scheduleTotalEl;
  const list = scheduleGroupsEl;
  list.innerHTML = '';
  const rowCount = s?.rows?.length ?? 0;
  if (!s || rowCount === 0) {
    total.textContent = 'No scheduled elements in this export.';
    return;
  }
  const groups = s.groups ?? [];
  total.textContent =
    groups.length > 0
      ? `${rowCount} scheduled elements · ${groups.length} type${groups.length === 1 ? '' : 's'}`
      : `${rowCount} scheduled elements`;

  const canHighlight = sceneSupportsHighlight();
  for (const group of groups) {
    list.appendChild(scheduleGroupRow(group, canHighlight));
  }
}

function scheduleGroupRow(group: ScheduleTypeGroup, canHighlight: boolean): HTMLElement {
  const countLabel = String(group.count);
  const storeyNote =
    group.storeys.length > 0
      ? ` across ${group.storeys.length} storey${group.storeys.length === 1 ? '' : 's'}`
      : '';

  if (!canHighlight) {
    const row = document.createElement('div');
    row.className = 'schedule-row';
    row.dataset.ifcType = group.ifc_type;
    row.appendChild(scheduleCell('schedule-type', ifcTypeLabel(group.ifc_type)));
    row.appendChild(scheduleCell('schedule-count', countLabel));
    return row;
  }

  const btn = document.createElement('button');
  btn.type = 'button';
  btn.className = 'schedule-row schedule-action';
  btn.dataset.ifcType = group.ifc_type;
  btn.setAttribute('aria-pressed', 'false');
  btn.setAttribute(
    'aria-label',
    `Highlight ${group.count} ${ifcTypeLabel(group.ifc_type)} elements${storeyNote} in the 3-D view`,
  );
  btn.appendChild(scheduleCell('schedule-type', ifcTypeLabel(group.ifc_type)));
  btn.appendChild(scheduleCell('schedule-count', countLabel));
  const verb = scheduleCell('schedule-verb', 'Highlight');
  btn.appendChild(verb);
  btn.addEventListener('click', () => {
    const wasActive = activeTypeHighlight === group.ifc_type;
    clearScheduleHighlightState();
    if (wasActive) return;
    const lit = highlightIfcType(group.ifc_type);
    if (lit === 0) {
      toast(`No ${ifcTypeLabel(group.ifc_type)} geometry to highlight.`, 'info');
      return;
    }
    btn.setAttribute('aria-pressed', 'true');
    verb.textContent = 'Clear';
    setStatus(`Highlighted ${lit} ${ifcTypeLabel(group.ifc_type)} mesh${lit === 1 ? '' : 'es'}`);
  });
  return btn;
}

function scheduleCell(className: string, text: string): HTMLElement {
  const cell = document.createElement('span');
  cell.className = className;
  cell.textContent = text;
  return cell;
}

/** Return every schedule row to its resting state. */
function clearScheduleHighlightState(): void {
  clearHighlight();
  scheduleEl.querySelectorAll<HTMLElement>('.schedule-action').forEach((el) => {
    el.setAttribute('aria-pressed', 'false');
    const verb = el.querySelector('.schedule-verb');
    if (verb) verb.textContent = 'Highlight';
  });
}

function renderExportQuality(diagnostics: ExportDiagnostics): void {
  const level = diagnostics.confidence?.level ?? 'unknown';
  const label = exportQualityLabel(level);
  const score = diagnostics.confidence?.score;
  const suffix = typeof score === 'number' ? ` · ${Math.round(score * 100)}%` : '';
  const nextText = `${label}${suffix}`;
  const changed = exportQualityEl.textContent !== nextText;
  exportQualityEl.textContent = nextText;
  exportQualityEl.className = `quality-pill ${exportQualityClass(level)}`;
  if (changed && !prefersReducedMotion()) {
    // className was just rewritten, so re-apply on the next frame to
    // restart the animation rather than inherit a stale one.
    requestAnimationFrame(() => exportQualityEl.classList.add('is-updated'));
    window.setTimeout(() => exportQualityEl.classList.remove('is-updated'), 400);
  }

  const elements = diagnostics.exported?.building_elements ?? 0;
  const geometry = diagnostics.exported?.building_elements_with_geometry ?? 0;
  const warnings = diagnostics.confidence?.warning_count ?? diagnostics.warnings?.length ?? 0;
  const bar = selectedExportMode();
  const barCheck = validateExportBar(bar, diagnostics);
  exportIfcBtn.title = `Download as IFC4 STEP · ${label} · bar ${bar}${barCheck.ok ? '' : ' (will warn)'} · ${elements} elements · ${geometry} with geometry · ${warnings} warnings`;
}

function renderStatusPanel(diagnostics: ExportDiagnostics): void {
  statusPanelEl.innerHTML = '';
  resetStatusRowStagger();
  diagnosticsJsonEl.textContent = JSON.stringify(diagnostics, null, 2);

  const input = diagnostics.input ?? {};
  const decoded = diagnostics.decoded ?? {};
  const exported = diagnostics.exported ?? {};
  const confidence = diagnostics.confidence ?? {};
  const warnings = diagnostics.warnings ?? [];
  const unsupported = diagnostics.unsupported_features ?? [];
  const validatedElements = decoded.production_walker_elements ?? 0;
  const diagnosticCandidates = decoded.diagnostic_proxy_candidates ?? 0;
  const geometryCount = exported.building_elements_with_geometry ?? 0;
  const qualityLevel = confidence.level ?? 'unknown';
  const scorePct =
    typeof confidence.score === 'number' ? Math.round(confidence.score * 100) : null;
  const exportModeLabel = diagnosticsModeLabel(diagnostics.mode);
  const bar = selectedExportMode();
  const barCheck = validateExportBar(bar, diagnostics);

  statusPanelEl.appendChild(
    statusRow(
      'File',
      input.stream_count ? 'ok' : 'warn',
      input.revit_version
        ? `Opened Revit ${input.revit_version} · ${input.stream_count ?? 0} streams`
        : `Opened · ${input.stream_count ?? 0} streams`,
    ),
  );
  appendDocumentRows(currentDocument);
  const failureMode = classifyFailureMode(diagnostics);
  statusPanelEl.appendChild(
    statusRow('Mode', failureMode.kind, `${failureMode.title} · ${failureMode.summary}`),
  );
  if (largeModelRetryOffered(diagnostics)) statusPanelEl.appendChild(largeModelRetryRow());
  statusPanelEl.appendChild(
    statusRow(
      'Schema',
      input.has_formats_latest && input.has_global_latest ? 'ok' : 'warn',
      input.has_formats_latest && input.has_global_latest
        ? 'Schema and model streams found'
        : 'Required schema/model stream missing',
    ),
  );
  const formatsIntegrity = diagnostics.formats_latest_integrity;
  if (formatsIntegrity) {
    const status = formatsIntegrity.integrity_status ?? 'unknown';
    const kind =
      status === 'ok' ? 'ok' : status === 'uncertain' || status === 'incomplete' ? 'warn' : 'warn';
    const code = formatsIntegrity.diagnostic_code
      ? ` · ${formatsIntegrity.diagnostic_code}`
      : '';
    const pages = formatsIntegrity.page_boundary_detected
      ? 'multipage boundary detected'
      : 'single-page';
    const strip = formatsIntegrity.checksum_tail_stripping ?? 'disabled';
    statusPanelEl.appendChild(
      statusRow(
        'Formats integrity',
        kind,
        `${status.charAt(0).toUpperCase()}${status.slice(1)} · ${pages} · strip ${strip}${code}`,
      ),
    );
  }
  statusPanelEl.appendChild(
    statusRow(
      'Elements',
      validatedElements > 0 ? 'ok' : 'warn',
      validatedElements > 0
        ? `${validatedElements} validated elements decoded`
        : diagnosticCandidates > 0
          ? `No validated elements · ${diagnosticCandidates} diagnostic candidates`
          : 'No validated elements decoded',
    ),
  );
  const prodClasses = decoded.production_class_counts ?? {};
  const classBits = [
    'Level',
    'Floor',
    'BuildingPad',
    'Room',
    'Material',
    'ArcWall',
    'Wall',
    'Door',
    'Window',
    'Column',
  ]
    .map((name) => {
      const n = prodClasses[name];
      return typeof n === 'number' && n > 0 ? `${name} ${n}` : null;
    })
    .filter(Boolean);
  if (classBits.length > 0) {
    statusPanelEl.appendChild(
      statusRow('Typed MVP', 'ok', classBits.join(' · ')),
    );
  }
  const storeyCount = exported.storey_count ?? 0;
  const materialCount = exported.material_count ?? 0;
  const storeyNames =
    exported.storey_names ??
    (model?.building_storeys ?? []).map((s) => s.name).filter(Boolean);
  const materialSample =
    exported.material_names_sample ??
    (model?.materials ?? []).map((m) => m.name).filter(Boolean).slice(0, 12);
  if (storeyCount > 0 || materialCount > 0) {
    const storeySummary =
      storeyNames.length > 0
        ? `${storeyCount} storeys: ${storeyNames.join(', ')}`
        : `${storeyCount} storeys`;
    statusPanelEl.appendChild(
      statusRow('Spatial', storeyCount > 0 ? 'ok' : 'warn', storeySummary),
    );
  }
    if (materialCount > 0) {
      const named =
        materialSample.length > 0
          ? materialSample.join(', ') +
            (materialCount > materialSample.length
              ? ` · +${materialCount - materialSample.length} more`
              : '')
          : `${materialCount} display names`;
      const layered = exported.layered_element_count ?? 0;
      statusPanelEl.appendChild(
        statusRow(
          'Materials',
          'ok',
          layered > 0
            ? `${materialCount} · ${named} · layers and colours read for ${layered} ${layered === 1 ? 'element' : 'elements'}`
            : `${materialCount} · ${named} (names only; no compound layers)`,
        ),
      );
    }
    // #409: how much of the model is drawn from the element's own data and
    // how much is a stand-in box.
    const bodyTotal = Object.values(exported.body_sources ?? {}).reduce(
      (sum, n) => sum + n,
      0,
    );
    const boxBodies = exported.bounding_box_bodies ?? 0;
    const savedBodies = exported.body_sources?.saved_graphics_mesh ?? 0;
    const savedNote = savedBodies > 0 ? ` · ${savedBodies} from the meshes Revit saved` : '';
    if (bodyTotal > 0) {
      statusPanelEl.appendChild(
        statusRow(
          'Shapes',
          boxBodies > 0 ? 'warn' : 'ok',
          boxBodies > 0
            ? `${bodyTotal - boxBodies} of ${bodyTotal} drawn from the element's own data; ${boxBodies} are the element's bounding box, placed and sized with its shape not decoded${savedNote}`
            : `all ${bodyTotal} drawn from the element's own data${savedNote}`,
        ),
      );
    }
    const parameterValueCount =
      diagnostics?.decoded?.parameter_value_count ?? 0;
    statusPanelEl.appendChild(
      statusRow(
        'Parameters',
        parameterValueCount > 0 ? 'ok' : 'warn',
        parameterValueCount > 0
          ? `${parameterValueCount.toLocaleString('en-US')} ${parameterValueCount === 1 ? 'value' : 'values'} written in the IFC property sets · select an element to read its saved parameters`
          : 'No parameter values written in the IFC property sets · select an element to read its saved parameters',
      ),
    );
    statusPanelEl.appendChild(
      statusRow(
        'Geometry',
        geometryCount > 0 ? 'ok' : 'warn',
      geometryCount > 0
        ? `${geometryCount} elements have geometry`
        : 'No real-file element geometry decoded',
    ),
  );
  statusPanelEl.appendChild(
    statusRow('Decode', decodeConfidenceKind(confidence), decodeConfidenceSummary(confidence)),
  );
  const meanConf = decoded.mean_element_confidence;
  const belowMin = decoded.elements_below_min_confidence ?? 0;
  const minConf = decoded.min_element_confidence ?? 0.55;
  if (typeof meanConf === 'number' || belowMin > 0 || validatedElements > 0) {
    const meanPct =
      typeof meanConf === 'number' ? `${Math.round(meanConf * 100)}% mean` : 'Mean n/a';
    statusPanelEl.appendChild(
      statusRow(
        'Provenance',
        belowMin > 0 ? 'warn' : 'ok',
        `${meanPct} · hide < ${Math.round(minConf * 100)}% · ${belowMin} below floor (M3-07)`,
      ),
    );
  }
  statusPanelEl.appendChild(
    statusRow(
      'Export',
      qualityLevel === 'geometry' ? 'ok' : 'warn',
      [
        exportQualityLabel(qualityLevel),
        scorePct !== null ? `${scorePct}%` : null,
        `sidecar ${exportModeLabel}`,
        confidence.has_typed_elements ? 'typed' : 'scaffold/typed unset',
        confidence.has_geometry ? 'geometry' : 'no geometry',
      ]
        .filter(Boolean)
        .join(' · '),
    ),
  );
  statusPanelEl.appendChild(
    statusRow(
      'IFC bar',
      barCheck.ok ? (bar === 'scaffold' ? 'warn' : 'ok') : 'warn',
      barCheck.ok
        ? `Selected ${bar} · diagnostics satisfy this bar`
        : `Selected ${bar} · ${barCheck.reason}`,
    ),
  );
  statusPanelEl.appendChild(
    statusRow(
      'Warnings',
      warnings.length === 0 && unsupported.length === 0 ? 'ok' : 'warn',
      warningSummary(warnings, unsupported),
    ),
  );
}

type StatusKind = 'ok' | 'warn' | 'bad' | 'idle';

interface FailureModeStatus {
  kind: StatusKind;
  title: string;
  summary: string;
}

/** Streams a resource limit kept from being read whole (#1075): one warning each. */
function unreadStreamCount(diagnostics: ExportDiagnostics): number {
  const warnings = diagnostics.warnings ?? [];
  return warnings.filter((warning) => warning.startsWith(UNREAD_STREAM_WARNING)).length;
}

/** True while raising the limits is still untried: the file was not opened under the large-model ones. */
function largeModelRetryOffered(diagnostics: ExportDiagnostics): boolean {
  const untried = currentFile !== null && !currentOpenLimits?.largeModel;
  return untried && unreadStreamCount(diagnostics) > 0;
}

/**
 * The retry offered under the Mode row when streams were not read: it reopens
 * the same file with the limits `rvt-ifc --large-model` uses, and says what the
 * browser cannot lift.
 */
function largeModelRetryRow(): HTMLElement {
  const row = document.createElement('div');
  row.className = 'status-action';
  row.style.setProperty('--row-i', String(Math.min(statusRowIndex, 8)));
  statusRowIndex += 1;
  const note = document.createElement('p');
  note.id = 'large-model-note';
  note.textContent =
    'Retrying reads those streams with the limits rvt-ifc --large-model uses: streams up to 2 GiB. A browser tab can use at most 4 GiB of memory, so a very large model can still fail to open.';
  const button = document.createElement('button');
  button.type = 'button';
  button.id = 'retry-large-model';
  button.className = 'side-btn';
  button.textContent = 'Retry with large-model limits';
  button.setAttribute('aria-describedby', note.id);
  button.addEventListener('click', () => {
    if (currentFile) void loadBytes(currentFile, LARGE_MODEL_LIMITS);
  });
  row.appendChild(note);
  row.appendChild(button);
  return row;
}

function classifyFailureMode(diagnostics: ExportDiagnostics): FailureModeStatus {
  const input = diagnostics.input ?? {};
  const decoded = diagnostics.decoded ?? {};
  const exported = diagnostics.exported ?? {};
  const confidence = diagnostics.confidence ?? {};
  const warnings = diagnostics.warnings ?? [];
  const unsupported = diagnostics.unsupported_features ?? [];
  const revitVersion = input.revit_version;
  const buildingElements = exported.building_elements ?? 0;
  const geometryElements = exported.building_elements_with_geometry ?? 0;
  const diagnosticCandidates = decoded.diagnostic_proxy_candidates ?? 0;
  const level = confidence.level ?? 'unknown';

  // #1075: a stream over the reader's resource limits was not read, so the
  // model is missing what it holds, however many elements came through.
  const unreadStreams = unreadStreamCount(diagnostics);
  if (unreadStreams > 0) {
    const streams = `${unreadStreams} stream${unreadStreams === 1 ? '' : 's'}`;
    return {
      kind: 'bad',
      title: 'Part of the file not read',
      summary: largeModelRetryOffered(diagnostics)
        ? `${streams} over the browser's resource limits, so the model is incomplete`
        : `${streams} still over the large-model limits, so the model is incomplete; rvt-ifc --large-model on a desktop inflates up to 8 GiB`,
    };
  }
  // The releases rvt-rs reads, 2008 to 2022, 2026 and 2027 experimentally, as rvt-inspect has it.
  if (typeof revitVersion === 'number' && (revitVersion < 2008 || revitVersion > 2027)) {
    return {
      kind: 'warn',
      title: 'Unsupported Revit version',
      summary: 'outside the 2008 to 2027 releases rvt-rs reads',
    };
  }
  if (level === 'unknown') {
    return {
      kind: 'bad',
      title: 'Parser bug, please report',
      summary: 'diagnostics did not include an export readiness level',
    };
  }
  if (input.has_formats_latest === false || input.has_global_latest === false) {
    return {
      kind: 'warn',
      title: 'Partial decode',
      summary: 'required schema/model streams were not decoded completely',
    };
  }
  if (buildingElements === 0 && diagnosticCandidates > 0) {
    return {
      kind: 'warn',
      title: 'Supported file, unsupported model layout',
      summary: 'only diagnostic candidates were found',
    };
  }
  if (level === 'scaffold' || buildingElements === 0) {
    return {
      kind: 'warn',
      title: 'Scaffold-only export',
      summary: 'no validated building elements were decoded',
    };
  }
  const unexported = confidence.unexported_element_records ?? 0;
  if (unexported > 0) {
    return {
      kind: 'warn',
      title: 'Element records not exported',
      summary: `${unexported} element records belong to ElementIds the file does not declare. On the one model compared with Revit's export they were elements deleted since its last Save As (#1838); if none was deleted, the model is likely incomplete (RE-30)`,
    };
  }
  if (input.release_support?.status === 'experimental') {
    return {
      kind: 'warn',
      title: 'Experimental Revit release',
      summary: `Revit ${revitVersion} is read experimentally: too few files of it are measured against Revit's own export to rely on its elements`,
    };
  }
  if (unsupported.length > 0 || warnings.length > 0 || geometryElements === 0) {
    return {
      kind: 'warn',
      title: 'Partial decode',
      summary: 'warnings, unsupported features, or missing geometry remain',
    };
  }
  return {
    kind: 'ok',
    title: 'Supported profile',
    summary: 'decoded output meets the current export profile',
  };
}

/** Row counter for the entrance stagger; reset whenever a panel rebuilds. */
let statusRowIndex = 0;
function resetStatusRowStagger(): void {
  statusRowIndex = 0;
}

/**
 * Document identity from the file's BasicFileInfo text block and Atom
 * entry (Rust `FileMetadata`, via the wasm `fileMetadata` binding).
 * Everything here is read verbatim from the file; nothing is inferred.
 */
interface FileMetadata {
  revit_version: number;
  build?: string | null;
  title?: string | null;
  last_saved?: string | null;
  worksharing?: string | null;
  workshared?: boolean | null;
  username?: string | null;
  central_model_path?: string | null;
  last_save_path?: string | null;
  document_guid?: string | null;
  document_increments?: number | null;
  locale?: string | null;
  single_user_cloud_model?: boolean | null;
  properties: { key: string; value: string }[];
}

/** Identity of the file currently open, or null before / without one. */
let currentDocument: FileMetadata | null = null;

/** `2023-09-07T12:50:35Z` → `2023-09-07 12:50 UTC`; anything else verbatim. */
function formatSavedAt(iso: string): string {
  const m = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/.exec(iso);
  return m ? `${m[1]} ${m[2]} UTC` : iso;
}

/** "Saved" and "Worksharing" rows of the File status panel. */
function appendDocumentRows(doc: FileMetadata | null): void {
  if (!doc) return;
  const saved: string[] = [];
  if (doc.last_saved) saved.push(formatSavedAt(doc.last_saved));
  if (doc.username) saved.push(`by ${doc.username}`);
  if (typeof doc.document_increments === 'number') {
    saved.push(`save counter ${doc.document_increments}`);
  }
  if (saved.length > 0) statusPanelEl.appendChild(statusRow('Saved', 'ok', saved.join(' · ')));
  if (doc.worksharing) {
    const parts = [doc.worksharing];
    if (doc.central_model_path) parts.push(`central: ${doc.central_model_path}`);
    if (doc.single_user_cloud_model) parts.push('single-user cloud model');
    statusPanelEl.appendChild(statusRow('Worksharing', 'ok', parts.join(' · ')));
  }
}

function statusRow(label: string, kind: StatusKind, value: string): HTMLElement {
  const row = document.createElement('div');
  row.className = 'status-row';
  // Capped so a long panel does not turn into a slow cascade.
  row.style.setProperty('--row-i', String(Math.min(statusRowIndex, 8)));
  statusRowIndex += 1;
  const dot = document.createElement('span');
  dot.className = `status-dot ${kind}`;
  const labelEl = document.createElement('div');
  labelEl.className = 'status-label';
  labelEl.textContent = label;
  const valueEl = document.createElement('div');
  valueEl.className = 'status-value';
  valueEl.textContent = value;
  row.appendChild(dot);
  row.appendChild(labelEl);
  row.appendChild(valueEl);
  return row;
}

function warningSummary(warnings: string[], unsupported: string[]): string {
  if (warnings.length > 0) {
    const suffix = warnings.length > 1 ? ` · ${warnings.length - 1} more` : '';
    return `${warnings[0]}${suffix}`;
  }
  if (unsupported.length > 0) {
    const suffix = unsupported.length > 1 ? ` · ${unsupported.length - 1} more` : '';
    return `${unsupported[0]}${suffix}`;
  }
  return 'No export warnings';
}

function diagnosticsModeLabel(mode: string | undefined): string {
  switch (mode) {
    case 'default':
      return 'default';
    case 'diagnostic_proxies':
      return 'diagnostic proxies';
    case 'placeholder':
      return 'placeholder';
    default:
      return mode ?? 'unknown';
  }
}

function decodeConfidenceKind(
  confidence: NonNullable<ExportDiagnostics['confidence']>,
): StatusKind {
  const unexported = confidence.unexported_element_records ?? 0;
  if (confidence.has_typed_elements && confidence.has_geometry && unexported === 0) return 'ok';
  if (confidence.has_typed_elements || confidence.has_project_metadata) return 'warn';
  return 'warn';
}

function decodeConfidenceSummary(
  confidence: NonNullable<ExportDiagnostics['confidence']>,
): string {
  const scorePct =
    typeof confidence.score === 'number' ? Math.round(confidence.score * 100) : null;
  const bits = [
    scorePct !== null ? `${scorePct}% coverage` : null,
    confidence.has_project_metadata ? 'project metadata' : 'no project metadata',
    confidence.has_typed_elements ? 'typed elements' : 'no typed elements',
    confidence.has_geometry ? 'geometry recovered' : 'no element geometry',
    confidence.has_diagnostic_proxies ? 'diagnostic proxies present' : null,
    confidence.unexported_element_records
      ? `${confidence.unexported_element_records} element records not exported`
      : null,
  ].filter(Boolean);
  if (!confidence.has_typed_elements && scorePct !== null && scorePct <= 30) {
    bits.push('scaffold ~25% expected for synthetics');
  }
  return bits.join(' · ');
}

/** Mirror ExportQualityMode::validate for UI messaging (Lane Seven). */
function validateExportBar(
  mode: string,
  diagnostics: ExportDiagnostics,
): { ok: boolean; reason: string } {
  const confidence = diagnostics.confidence ?? {};
  const exported = diagnostics.exported ?? {};
  const warnings = diagnostics.warnings ?? [];
  const unsupported = diagnostics.unsupported_features ?? [];
  const failures: string[] = [];

  const needsTyped = mode === 'typed-no-geometry' || mode === 'geometry' || mode === 'strict';
  const needsGeometry = mode === 'geometry' || mode === 'strict';

  if (needsTyped && !confidence.has_typed_elements) {
    failures.push('no validated typed IFC elements');
  }
  if (needsGeometry && !confidence.has_geometry) {
    failures.push('no recovered element geometry');
  }
  if (mode === 'strict') {
    if (!confidence.has_project_metadata) failures.push('no project metadata');
    if ((exported.unit_assignment_count ?? 0) === 0) failures.push('no unit assignment');
    if ((exported.storey_count ?? 0) === 0) failures.push('no storeys');
    if (unsupported.length > 0) failures.push('unsupported features remain');
    if (warnings.length > 0) failures.push(`${warnings.length} warning(s) remain`);
  }

  if (failures.length === 0) {
    return { ok: true, reason: '' };
  }
  return { ok: false, reason: failures.join('; ') };
}

function exportQualityLabel(level: string): string {
  switch (level) {
    case 'scaffold':
      return 'Scaffold';
    case 'typed_no_geometry':
      return 'Typed';
    case 'geometry':
      return 'Geometry';
    case 'diagnostic_partial':
      return 'Diagnostic';
    case 'proxy_only':
      return 'Proxy';
    default:
      return 'Unknown';
  }
}

function exportQualityClass(level: string): string {
  switch (level) {
    case 'geometry':
      return 'geometry';
    case 'typed_no_geometry':
      return 'typed';
    case 'diagnostic_partial':
    case 'proxy_only':
      return 'diagnostic';
    case 'scaffold':
    default:
      return 'scaffold';
  }
}

/** The entity indices of the nodes under and including `node`. */
function listedIndices(node: SceneNode, out: Set<number>): Set<number> {
  if (node.entity_index !== null) out.add(node.entity_index);
  node.children.forEach((child) => listedIndices(child, out));
  return out;
}

function countEntities(node: SceneNode): number {
  let n = node.entity_index !== null ? 1 : 0;
  for (const c of node.children) n += countEntities(c);
  return n;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

// ---------- Drag & drop / file picker ----------
pickBtn.addEventListener('click', () => fileInput.click());
fileInput.addEventListener('change', () => {
  const f = fileInput.files?.[0];
  if (f) void loadBytes(f);
});

// Keyboard: F brings the selection into view, or the whole model when
// nothing is selected. Not while typing in a field or choosing an option.
document.addEventListener('keydown', (ev) => {
  if ((ev.key !== 'f' && ev.key !== 'F') || ev.metaKey || ev.ctrlKey || ev.altKey) return;
  const target = ev.target as HTMLElement | null;
  if (target?.closest('input, select, textarea, [contenteditable="true"]')) return;
  if (!currentModel) return;
  ev.preventDefault();
  if (selectedIndex !== null) {
    frameEntities(new Set([selectedIndex]), entityLabel(selectedIndex));
  } else {
    frameCamera(currentModel);
    setStatus('Zoomed to the whole model');
  }
});

// Keyboard: / moves to the scene tree filter, as in most file browsers.
document.addEventListener('keydown', (ev) => {
  if (ev.key !== '/' || ev.metaKey || ev.ctrlKey || ev.altKey) return;
  const target = ev.target as HTMLElement | null;
  if (target?.closest('input, select, textarea, [contenteditable="true"]')) return;
  if (treeFilterEl.disabled) return;
  ev.preventDefault();
  treeFilterEl.focus();
  treeFilterEl.select();
});

// Keyboard: Escape clears tree selection / returns focus toward file open.
document.addEventListener('keydown', (ev) => {
  if (ev.key !== 'Escape') return;
  // The scaffold explainer is ambient, not modal — Escape clears it
  // alongside the tree selection rather than swallowing the keypress.
  hideScaffoldNote();
  const selected = treeEl.querySelector('.tree-node.selected');
  if (selected) {
    selected.classList.remove('selected');
    clearHighlight();
    pendingInfoIndex = null;
    selectedIndex = null;
    infoEl.textContent =
      'Select an element in the 3-D view or scene tree (Enter / Space on a tree row). Double-click it or press F to zoom to it.';
  }
  if (!dropzone.classList.contains('hidden')) {
    pickBtn.focus();
  }
});

['dragenter', 'dragover'].forEach((type) =>
  document.body.addEventListener(type, (ev) => {
    ev.preventDefault();
    dropzone.classList.add('drag-over');
  }),
);
['dragleave', 'drop'].forEach((type) =>
  document.body.addEventListener(type, (ev) => {
    ev.preventDefault();
    dropzone.classList.remove('drag-over');
  }),
);
document.body.addEventListener('drop', (ev) => {
  ev.preventDefault();
  const f = ev.dataTransfer?.files[0];
  if (!f) return;
  if (!/\.(rvt|rfa|rte|rft)$/i.test(f.name)) {
    setStatus(`Ignored: ${f.name} — not a Revit file`);
    toast(`${f.name} is not a Revit file. Drop a .rvt, .rfa, .rte or .rft.`, 'bad');
    return;
  }
  void loadBytes(f);
});

// ---------- Export buttons (VW1-16 / VW1-17 / VW1-11 surfaced) ----------

/**
 * The wasm module for main-thread exports (IFC STEP, plan SVG). Parsing
 * runs in the worker, which initialises its own instance; a
 * `--target web` module has to be initialised on this thread too before
 * any export is called, or every call dies with "Cannot read properties
 * of undefined (reading '__wbindgen_add_to_stack_pointer')" — which is
 * what Export IFC and Export plan SVG did from their introduction until
 * this was added. A failed initialisation is not cached, so the next
 * click retries.
 */
type WasmModule = typeof wasmGlue;
let mainThreadWasmReady: Promise<WasmModule> | null = null;
/** The main thread's instance's memory once an export has made one: it counts toward what a read may be made beside. */
let mainThreadWasmMemory: WebAssembly.Memory | null = null;
function mainThreadWasm(): Promise<WasmModule> {
  if (!mainThreadWasmReady) {
    // The glue is part of the page (a static import) and the module is the one
    // the open compiled, so an export fetches nothing, however late it is
    // clicked. `default` takes a module and instantiates it asynchronously,
    // which the main thread allows at any size; `initSync` would not.
    mainThreadWasmReady = (async () => {
      if (!decoderWasm) throw new Error('the open did not hand over the decoder');
      mainThreadWasmMemory = (await wasmGlue.default({ module_or_path: decoderWasm })).memory;
      return wasmGlue;
    })().catch((err: unknown) => {
      mainThreadWasmReady = null;
      throw err;
    });
  }
  return mainThreadWasmReady;
}

function download(filename: string, blob: Blob): void {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  // Revoke on next tick so the download actually starts first.
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

exportGlbBtn.addEventListener('click', () => {
  if (!lastGlb) return;
  const blob = new Blob([lastGlb.buffer as ArrayBuffer], {
    type: 'model/gltf-binary',
  });
  download(`${lastFileStem}.glb`, blob);
  setStatus(`Exported ${lastFileStem}.glb`);
  toast(`Exported ${lastFileStem}.glb`, 'ok');
});

exportIfcBtn.addEventListener('click', () => {
  if (!model) return;
  // The IFC STEP writer is synchronous + fast; no worker hop needed
  // for the sample-family-sized models we've seen. If this ever blocks
  // the main thread on big projects, move it into worker.ts.
  void (async () => {
    const quality = currentDiagnostics
      ? exportQualityLabel(currentDiagnostics.confidence?.level ?? 'unknown').toLowerCase()
      : 'unknown';
    const bar = selectedExportMode();
    if (currentDiagnostics) {
      const check = validateExportBar(bar, currentDiagnostics);
      if (!check.ok) {
        setStatus(
          `IFC bar ${bar} not satisfied (${check.reason}) — exporting scaffold STEP anyway; download diagnostics for triage`,
        );
      }
    }
    setStatus(`Rendering IFC STEP · ${quality} · bar ${bar}`);
    try {
      const { modelToIfcStep } = await mainThreadWasm();
      const text = modelToIfcStep(model as unknown as object);
      const blob = new Blob([text], { type: 'application/x-step' });
      download(`${lastFileStem}.ifc`, blob);
      setStatus(`Exported ${lastFileStem}.ifc`);
      toast(`Exported ${lastFileStem}.ifc · ${quality}`, 'ok');
    } catch (err) {
      const message = (err as Error).message ?? String(err);
      setStatus(`IFC export failed: ${message}`);
      toast(`IFC export failed. ${message}`, 'bad');
    }
  })();
});

// The doors and windows are part of the model the worker builds, so the file
// is read again with the option rather than redrawn.
savedMeshesEl.addEventListener('change', () => {
  if (currentFile) void loadBytes(currentFile, currentOpenLimits);
});

// The phase decides which elements and rooms the model has, so the file is read
// again with it, as for the saved meshes.
exportPhaseEl.addEventListener('change', () => {
  currentPhase = exportPhaseEl.value;
  if (currentFile) void loadBytes(currentFile, currentOpenLimits);
});

exportModeEl.addEventListener('change', () => {
  if (currentDiagnostics) {
    renderExportQuality(currentDiagnostics);
    renderStatusPanel(currentDiagnostics);
    const check = validateExportBar(selectedExportMode(), currentDiagnostics);
    setStatus(
      check.ok
        ? `IFC bar ${selectedExportMode()} · diagnostics satisfy this bar`
        : `IFC bar ${selectedExportMode()} · ${check.reason}`,
    );
  } else {
    renderEmptyStatusPanel();
    setStatus(`IFC bar ${selectedExportMode()} · open a file to evaluate`);
  }
});

exportSvgBtn.addEventListener('click', () => {
  if (!model) return;
  void (async () => {
    setStatus('Rendering plan SVG…');
    try {
      const { renderPlanSvg } = await mainThreadWasm();
      const svg = renderPlanSvg(model as unknown as object, null);
      const blob = new Blob([svg], { type: 'image/svg+xml' });
      download(`${lastFileStem}.svg`, blob);
      setStatus(`Exported ${lastFileStem}.svg`);
      toast(`Exported ${lastFileStem}.svg`, 'ok');
    } catch (err) {
      const message = (err as Error).message ?? String(err);
      setStatus(`Plan export failed: ${message}`);
      toast(`Plan export failed. ${message}`, 'bad');
    }
  })();
});

/**
 * The diagnostics with every name they take from the project replaced, as
 * `ExportDiagnostics::anonymize` does for `rvt-ifc --anonymize`: the file
 * and project names, levels (`Level 1`...), the material sample
 * (`Material 1`...) and skipped elements' sample names (`element-<id>`),
 * and the "Export to IFC As" values a warning lists. Counts and ElementIds
 * stay, so the download can be posted publicly.
 */
function anonymizedDiagnostics(diagnostics: ExportDiagnostics): ExportDiagnostics {
  const copy = plainJson(diagnostics);
  const redacted = '<redacted>';
  const input = copy.input;
  if (input?.original_path) {
    const fileName = input.original_path.split(/[\\/]/).pop() ?? input.original_path;
    const dot = fileName.lastIndexOf('.');
    input.original_path = dot > 0 ? `${redacted}${fileName.slice(dot)}` : redacted;
  }
  if (input?.project_name) input.project_name = redacted;
  const exported = copy.exported;
  if (exported) {
    exported.storey_names = (exported.storey_names ?? []).map((_, index) => `Level ${index + 1}`);
    exported.material_names_sample = (exported.material_names_sample ?? []).map(
      (_, index) => `Material ${index + 1}`,
    );
  }
  if (copy.room_phase_fallback?.phase !== undefined) copy.room_phase_fallback.phase = redacted;
  for (const item of copy.skipped ?? []) {
    if (item.phase !== undefined) item.phase = redacted;
    item.sample_names = (item.sample_names ?? []).map((name) => {
      const id = /(\d+)$/.exec(name)?.[1];
      return id ? `element-${id}` : redacted;
    });
  }
  // The "Export to IFC As" values the export does not apply are the
  // project's own text: keep their count, drop the list (#792). No other
  // warning carries project text.
  const unapplied = "so each keeps its category's entity (#792):";
  copy.warnings = (copy.warnings ?? []).map((warning) => {
    const at = warning.indexOf(unapplied);
    return at >= 0 ? `${warning.slice(0, at + unapplied.length - 1)}.` : warning;
  });
  return copy;
}

downloadDiagnosticsBtn.addEventListener('click', () => {
  if (!currentDiagnostics) return;
  const json = JSON.stringify(anonymizedDiagnostics(currentDiagnostics), null, 2);
  const blob = new Blob([json], { type: 'application/json' });
  download('rvt-diagnostics.json', blob);
  setStatus('Exported rvt-diagnostics.json, project names replaced');
  toast('Downloaded rvt-diagnostics.json, project names replaced so it can be posted', 'ok');
});

/** Whether the schedule lists a building element of `ifcType` (any type for null). */
function scheduleHasType(schedule: Schedule, ifcType: string | null): boolean {
  return (schedule.groups ?? []).some(
    (group) => group.count > 0 && (ifcType === null || group.ifc_type === ifcType),
  );
}

/**
 * CSV schedules (`rvt::ifc::schedule_csv` via the wasm `scheduleCsv`
 * binding). Generated in the tab from the already-decoded model — no
 * upload. Written with a UTF-8 byte-order mark: most people open these in
 * Excel, which needs it to read non-ASCII room names.
 */
function downloadSchedule(kind: 'elements' | 'rooms'): void {
  if (!model) return;
  const label = kind === 'elements' ? 'element schedule' : 'room schedule';
  void (async () => {
    setStatus(`Building ${label}…`);
    try {
      const { scheduleCsv } = await mainThreadWasm();
      const csv = scheduleCsv(model as unknown as object, kind, false, true);
      const rows = Math.max(0, csv.split('\r\n').filter((line) => line.length > 0).length - 1);
      download(`${lastFileStem}.${kind}.csv`, new Blob([csv], { type: 'text/csv' }));
      setStatus(`Exported ${lastFileStem}.${kind}.csv · ${rows} rows`);
      toast(`Exported ${label} · ${rows} rows`, 'ok');
    } catch (err) {
      const message = (err as Error).message ?? String(err);
      setStatus(`Could not build the ${label}: ${message}`);
      toast(`Could not build the ${label}. ${message}`, 'bad');
    }
  })();
}

downloadScheduleBtn.addEventListener('click', () => downloadSchedule('elements'));
downloadRoomsBtn.addEventListener('click', () => downloadSchedule('rooms'));

// ---------- Demo gallery (VW1-22 / M6-03) ----------
interface DemoEntry {
  id: string;
  name: string;
  file: string;
  format: string;
  description: string;
  loadable?: boolean;
  available?: boolean;
  expected_quality?: string;
  expected_quality_note?: string;
  license?: string;
  provenance?: string;
  revit_version?: number | null;
  element_count_hint?: number;
  thumbnail?: string;
  tags?: string[];
}

interface DemoCatalog {
  attribution?: string;
  license?: string;
  privacy_note?: string;
  demos: DemoEntry[];
}

function demoAssetUrl(relPath: string): string {
  const cleaned = relPath.replace(/^\.\//, '').replace(/^\//, '');
  return new URL(cleaned, new URL('./', window.location.href)).toString();
}

/**
 * The card that started the in-flight load. Held at module scope because
 * the busy state has to survive until the worker reports back, which is
 * long after loadBytes() resolves.
 */
let busyDemoCard: HTMLElement | null = null;

function setDemoCardBusy(card: HTMLElement): void {
  clearDemoCardBusy();
  busyDemoCard = card;
  card.setAttribute('aria-busy', 'true');
  demoListEl.querySelectorAll<HTMLButtonElement>('button.demo-card').forEach((other) => {
    if (other !== card && !other.disabled) {
      other.disabled = true;
      other.setAttribute('data-demo-requeued', 'true');
    }
  });
}

function clearDemoCardBusy(): void {
  if (busyDemoCard) {
    busyDemoCard.removeAttribute('aria-busy');
    busyDemoCard = null;
  }
  demoListEl
    .querySelectorAll<HTMLButtonElement>('button.demo-card[data-demo-requeued]')
    .forEach((other) => {
      other.disabled = false;
      other.removeAttribute('data-demo-requeued');
    });
}

async function loadDemoFile(demo: DemoEntry, card?: HTMLElement): Promise<void> {
  if (!demo.loadable || !demo.available) {
    setStatus(`Demo ${demo.id} is reference-only — use download link in gallery`);
    return;
  }
  if (card) setDemoCardBusy(card);
  setStatus(`Loading demo ${demo.name}…`);
  setStatusBusy(true);
  try {
    const response = await fetch(demoAssetUrl(demo.file));
    if (!response.ok) {
      throw new Error(`HTTP ${response.status} for ${demo.file}`);
    }
    const buffer = await response.arrayBuffer();
    const fileName = pathBasename(demo.file);
    const file = new File([buffer], fileName, {
      type: 'application/octet-stream',
    });
    await loadBytes(file);
  } catch (err) {
    const message = (err as Error).message ?? String(err);
    stopLoadOverlay();
    clearDemoCardBusy();
    setStatus(`Demo load failed: ${message}`);
    toast(`Could not open ${demo.name}. ${message}`, 'bad');
  }
}

function pathBasename(p: string): string {
  const parts = p.split('/');
  return parts[parts.length - 1] || p;
}

function qualityPillClass(label: string | undefined): string {
  const normalized = (label ?? '').toLowerCase();
  if (normalized === 'geometry') return 'geometry';
  if (normalized === 'scaffold' || normalized === 'diagnostic') return '';
  return 'reference';
}

function renderDemoGallery(catalog: DemoCatalog): void {
  demoListEl.innerHTML = '';
  const attributionBits = [
    catalog.attribution,
    catalog.license,
    'Same-origin static assets only — Activate loads bytes in-tab.',
  ].filter(Boolean);
  demoAttributionEl.textContent = attributionBits.join(' ');

  for (const demo of catalog.demos) {
    const canOpen = Boolean(demo.loadable && demo.available);
    const canDownload = Boolean(!demo.loadable && demo.available && demo.format === 'ifc');

    if (canDownload) {
      const link = document.createElement('a');
      link.href = demoAssetUrl(demo.file);
      link.download = pathBasename(demo.file);
      link.className = 'demo-card';
      link.setAttribute('role', 'listitem');
      link.setAttribute('data-demo-id', demo.id);
      link.setAttribute('aria-label', `Download reference ${demo.name}`);
      fillDemoCard(link, demo, false);
      demoListEl.appendChild(link);
      continue;
    }

    const card = document.createElement('button');
    card.type = 'button';
    card.className = 'demo-card';
    card.setAttribute('role', 'listitem');
    card.setAttribute('data-demo-id', demo.id);
    card.setAttribute(
      'aria-label',
      canOpen ? `Open demo ${demo.name}` : `Demo unavailable: ${demo.name}`,
    );
    if (!canOpen) {
      card.disabled = true;
    } else {
      card.addEventListener('click', () => {
        void loadDemoFile(demo, card);
      });
    }
    fillDemoCard(card, demo, canOpen);
    demoListEl.appendChild(card);
  }
}

function isRealProject(demo: DemoEntry): boolean {
  return (demo.tags ?? []).includes('real-project');
}

function fillDemoCard(card: HTMLElement, demo: DemoEntry, loadable: boolean): void {
  const real = isRealProject(demo);
  if (real) card.setAttribute('data-demo-real', 'true');
  if (demo.thumbnail) {
    const img = document.createElement('img');
    img.src = demoAssetUrl(demo.thumbnail);
    img.alt = '';
    img.width = 56;
    img.height = 36;
    card.appendChild(img);
  }
  const body = document.createElement('div');
  body.className = 'demo-body';
  const title = document.createElement('div');
  title.className = 'demo-title';
  title.textContent = demo.name;
  if (real) {
    // Distinguishes an actual Revit project from a synthetic fixture at a
    // glance — the two decode to very different results.
    const badge = document.createElement('span');
    badge.className = 'demo-badge';
    badge.textContent = 'Real project';
    title.appendChild(badge);
  }
  // One line of facts and the licence; the provenance and notes are the
  // card's tooltip.
  const meta = document.createElement('div');
  meta.className = 'demo-meta';
  const bits = [
    demo.format.toUpperCase(),
    demo.revit_version ? `Revit ${demo.revit_version}` : null,
    demo.element_count_hint ? `~${demo.element_count_hint} elements` : null,
    demo.license ?? null,
  ];
  if (!demo.available) {
    bits.push('not bundled in this build');
  } else if (!loadable && demo.format !== 'ifc') {
    bits.push('unavailable in this build');
  } else if (!loadable) {
    bits.push('Reference download, not opened by the RVT parser');
  }
  meta.textContent = bits.filter(Boolean).join(' · ');
  card.title = [demo.provenance, demo.expected_quality_note]
    .filter(Boolean)
    .join('\n');
  const quality = document.createElement('span');
  quality.className = `demo-quality ${qualityPillClass(demo.expected_quality)}`;
  quality.textContent = `Expected: ${demo.expected_quality ?? 'Unknown'}`;
  body.appendChild(title);
  body.appendChild(meta);
  card.appendChild(body);
  card.appendChild(quality);
}

async function initDemoGallery(): Promise<void> {
  try {
    const response = await fetch(demoAssetUrl('demos/catalog.json'));
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const catalog = (await response.json()) as DemoCatalog;
    renderDemoGallery(catalog);
  } catch (err) {
    demoAttributionEl.textContent =
      'Demo catalog unavailable in this build. Drop a local .rvt / .rfa instead.';
    setStatus(`Demo gallery: ${(err as Error).message ?? err}`);
  }
}

/**
 * Shows or hides the empty state. The profile rows sit beside the support
 * grid while the canvas is empty and move to the right panel once a file is
 * open, so they stay reachable.
 */
function setDropzoneVisible(visible: boolean): void {
  dropzone.classList.toggle('hidden', !visible);
  if (visible) {
    supportSlotEl.appendChild(supportMatrixEl);
    supportHomeEl.hidden = true;
  } else {
    supportHomeEl.appendChild(supportMatrixEl);
    supportHomeEl.hidden = false;
  }
}

/** Why the export buttons are disabled, or null once they are not. */
function setExportHint(text: string | null): void {
  exportHintEl.hidden = text === null;
  exportHintEl.textContent = text ?? '';
}

type SupportState = 'typed' | 'verified' | 'experimental' | 'not-measured';

const SUPPORT_STATE_LABEL: Record<SupportState, string> = {
  typed: 'Typed elements',
  verified: 'Verified metadata and schema',
  experimental: 'Experimental export',
  'not-measured': 'Not measured',
};

/**
 * What the viewer reads of each Revit release's projects and family files,
 * drawn from docs/support-matrix.json: its releases table names the rows
 * that rate each release, and those rows' statuses decide the cells, so the
 * grid cannot claim more than the matrix (tests/support_matrix.rs pins this
 * import).
 */
function renderSupportGrid(): void {
  const statusOf = new Map<string, string>(
    supportMatrix.capabilities.map((row): [string, string] => [row.id, row.status]),
  );
  const projectState = (id: string): SupportState | null => {
    const status = statusOf.get(id);
    if (status === 'verified' || status === 'partial') return 'typed';
    if (status === 'experimental') return 'experimental';
    return null;
  };
  const familyState = (ids: readonly string[]): SupportState =>
    ids.length > 0 && ids.every((id) => statusOf.get(id) === 'verified')
      ? 'verified'
      : 'not-measured';

  const head = supportGridEl.createTHead().insertRow();
  const corner = document.createElement('th');
  corner.scope = 'col';
  corner.textContent = 'Revit release';
  head.appendChild(corner);
  const body = supportGridEl.createTBody();
  const projects = body.insertRow();
  const families = body.insertRow();
  for (const [row, label] of [
    [projects, 'Projects'],
    [families, 'Family files'],
  ] as const) {
    const th = document.createElement('th');
    th.scope = 'row';
    th.textContent = label;
    row.appendChild(th);
  }
  const shown = new Set<SupportState>();
  for (const release of supportMatrix.releases) {
    const year = document.createElement('th');
    year.scope = 'col';
    year.textContent = String(release.release);
    head.appendChild(year);
    const cells: [HTMLTableRowElement, SupportState | null, string][] = [
      [projects, projectState(release.project_capability), 'projects'],
      [families, familyState(release.family_capabilities), 'family files'],
    ];
    for (const [row, state, what] of cells) {
      const cell = row.insertCell();
      if (!state) continue;
      shown.add(state);
      cell.className = `support-cell ${state}`;
      cell.title = `Revit ${release.release} ${what}: ${SUPPORT_STATE_LABEL[state]}`;
      const text = document.createElement('span');
      text.className = 'visually-hidden';
      text.textContent = SUPPORT_STATE_LABEL[state];
      cell.appendChild(text);
    }
  }
  for (const state of ['typed', 'verified', 'experimental', 'not-measured'] as const) {
    if (!shown.has(state)) continue;
    const item = document.createElement('li');
    const swatch = document.createElement('span');
    swatch.className = `support-cell ${state}`;
    swatch.setAttribute('aria-hidden', 'true');
    item.append(swatch, SUPPORT_STATE_LABEL[state]);
    supportLegendEl.appendChild(item);
  }
  if (supportMatrix.releases.some((release) => release.release < 2015)) {
    const outside = document.createElement('li');
    outside.className = 'support-legend-note';
    outside.textContent =
      'Revit 2014 and earlier: outside the 1.0 range (2015 to 2027), measured only on a contributor\'s local files.';
    supportLegendEl.appendChild(outside);
  }
  const note = document.createElement('li');
  note.className = 'support-legend-note';
  note.textContent = 'Other files export a valid scaffold.';
  supportLegendEl.appendChild(note);
}

renderSupportGrid();
setDropzoneVisible(true);
setExportHint('Open a file to export');
renderEmptyStatusPanel();
void initDemoGallery();
setStatus('Ready · drop a file or open a demo');
