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
const statusEl = $('status');
const fileMetaEl = $('file-meta');
const treeEl = $('tree');
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
const demoListEl = $('demo-list');
const demoAttributionEl = $('demo-attribution');
const loadOverlayEl = $('load-overlay');
const loadStepEl = $('load-step');
const loadElapsedEl = $('load-elapsed');
const loadHintEl = $('load-hint');
const scaffoldNoteEl = $('scaffold-note');
const scaffoldNoteBodyEl = $('scaffold-note-body');
const showRealProjectsBtn = $('show-real-projects') as HTMLButtonElement;
const dismissScaffoldNoteBtn = $('dismiss-scaffold-note') as HTMLButtonElement;
const toastRegionEl = $('toast-region');

// ---------- Motion ----------
const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
function prefersReducedMotion(): boolean {
  return reducedMotionQuery.matches;
}

// ---------- Three.js scene ----------
const scene = new THREE.Scene();
scene.background = new THREE.Color(0x0b0e13);
const camera = new THREE.PerspectiveCamera(45, 1, 0.1, 2000);
camera.position.set(60, 40, 60);
const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
viewport.appendChild(renderer.domElement);
const controls = new OrbitControls(camera, renderer.domElement);
controls.enableDamping = true;
controls.target.set(0, 0, 0);

const hemi = new THREE.HemisphereLight(0xd7dce3, 0x0b0e13, 0.8);
scene.add(hemi);
const dir = new THREE.DirectionalLight(0xffffff, 0.7);
dir.position.set(50, 80, 50);
scene.add(dir);
const grid = new THREE.GridHelper(100, 20, 0x1d2430, 0x11161d);
scene.add(grid);
const axes = new THREE.AxesHelper(10);
scene.add(axes);

let currentModel: THREE.Group | null = null;

function resize(): void {
  const w = viewport.clientWidth;
  const h = viewport.clientHeight;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}
window.addEventListener('resize', resize);
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

renderer.domElement.addEventListener('pointerdown', (ev) => {
  if (!currentModel) return;
  const rect = renderer.domElement.getBoundingClientRect();
  pointer.x = ((ev.clientX - rect.left) / rect.width) * 2 - 1;
  pointer.y = -((ev.clientY - rect.top) / rect.height) * 2 + 1;
  raycaster.setFromCamera(pointer, camera);
  const hits = raycaster.intersectObject(currentModel, true);
  if (hits.length === 0) return;
  const hit = hits[0]!;
  const userData = hit.object.userData as { entityIndex?: number };
  if (userData.entityIndex === undefined) return;
  selectEntity(userData.entityIndex);
});

// ---------- Selection highlight (M4-04) ----------
const highlightMaterial = new THREE.MeshStandardMaterial({
  color: 0x6bb7ff,
  emissive: 0x14304d,
  side: THREE.DoubleSide,
});
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

/** Tint every mesh carrying `entityIndex` so the scene agrees with the tree. */
function highlightEntity(idx: number): void {
  highlightWhere((data) => data.entityIndex === idx);
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
 * Entities associated with a material. The model is already in
 * memory on this thread, so the lookup needs no worker round-trip.
 */
function entityIndicesWithMaterial(materialIndex: number): Set<number> {
  const out = new Set<number>();
  const entities = model?.entities ?? [];
  for (let i = 0; i < entities.length; i += 1) {
    if (entities[i]?.material_index === materialIndex) out.add(i);
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
function selectEntity(idx: number): void {
  clearScheduleHighlightState();
  selectTreeRow(idx);
  highlightEntity(idx);
  showElementInfo(idx);
}

function selectTreeRow(idx: number): void {
  document
    .querySelectorAll('.tree-node.selected')
    .forEach((el) => el.classList.remove('selected'));
  const row = treeEl.querySelector<HTMLElement>(`.tree-node[data-entity-index="${idx}"]`);
  if (!row) return;
  row.classList.add('selected');
  row.scrollIntoView({
    block: 'nearest',
    behavior: prefersReducedMotion() ? 'auto' : 'smooth',
  });
  row.focus();
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
 * Observed decode throughput across the staged demos: since the
 * partition streams are inflated once per file (#266) the 33.7 MB
 * project decodes in about 3 s in the browser, measured on the deployed
 * site. Only ever used to set an expectation — the decoder cannot report
 * real progress, so the bar stays indeterminate rather than faking a
 * percentage.
 */
const DECODE_BYTES_PER_SECOND = 11e6;

function loadExpectation(bytes: number): string {
  const size = formatBytes(bytes);
  if (bytes < 256 * 1024) return `${size}, usually instant`;
  const raw = bytes / DECODE_BYTES_PER_SECOND;
  const seconds = raw < 10 ? Math.max(1, Math.round(raw)) : Math.round(raw / 5) * 5;
  return `${size}, about ${seconds} s`;
}

let loadStartedAt = 0;
let loadTimer: number | null = null;

function startLoadOverlay(file: File): void {
  loadStartedAt = performance.now();
  loadStepEl.textContent = `Opening ${file.name}`;
  loadHintEl.textContent = loadExpectation(file.size);
  loadElapsedEl.textContent = '0 s elapsed';
  loadOverlayEl.classList.remove('hidden');
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
  setStatusBusy(false);
}

// ---------- Scaffold-only empty state ----------
function hideScaffoldNote(): void {
  scaffoldNoteEl.classList.add('hidden');
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
  dropzone.classList.remove('hidden');
  const card = demoListEl.querySelector<HTMLElement>('[data-demo-real="true"]');
  if (!card) {
    setStatus('real-project demos are not staged in this build');
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
function resetWorker(): Worker_ {
  if (worker) worker.terminate();
  const w = new Worker(new URL('./worker.ts', import.meta.url), {
    type: 'module',
  }) as Worker_;
  worker = w;
  return w;
}

// ---------- Model / scene-graph state ----------
interface IfcModel {
  project_name?: string;
  description?: string;
  building_storeys?: Array<{ name: string; elevation_feet?: number }>;
  materials?: Array<{ name: string; color_packed?: number; transparency?: number }>;
  entities?: Array<{
    name: string;
    ifc_type: string;
    guid?: string;
    material_index?: number | null;
  }>;
}
interface RelatedElement {
  entity_index: number;
  name: string;
  ifc_type: string;
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
interface ElementInfoPanel {
  name: string;
  ifc_type: string;
  type_guid?: string | null;
  predefined_type?: string | null;
  storey?: PanelStorey | null;
  placement_rows?: PanelRow[];
  extent_rows?: PanelRow[];
  material?: PanelMaterial | null;
  property_group?: PanelPropertyGroup | null;
  host?: RelatedElement | null;
  hosted?: RelatedElement[];
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
    project_name?: string;
    stream_count?: number;
    has_basic_file_info?: boolean;
    has_part_atom?: boolean;
    has_formats_latest?: boolean;
    has_global_latest?: boolean;
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
    /** RE-30 records found but not exported; non-zero means the model is incomplete. */
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
  };
  unsupported_features?: string[];
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

// ---------- Load flow ----------
async function loadBytes(file: File): Promise<void> {
  setStatus(`reading ${formatBytes(file.size)}…`);
  startLoadOverlay(file);
  hideScaffoldNote();
  model = null;
  sceneGraph = null;
  distinctTypes = [];
  lastGlb = null;
  lastSchedule = null;
  currentDiagnostics = null;
  exportGlbBtn.disabled = true;
  exportIfcBtn.disabled = true;
  exportSvgBtn.disabled = true;
  downloadDiagnosticsBtn.disabled = true;
  downloadScheduleBtn.disabled = true;
  downloadRoomsBtn.disabled = true;
  exportQualityEl.textContent = 'quality: pending';
  exportQualityEl.className = 'quality-pill';
  diagnosticsJsonEl.textContent = '';
  // A new file invalidates the panel and the schedule; leaving the
  // previous file's numbers up would read as if they still applied.
  pendingInfoIndex = null;
  infoEl.removeAttribute('aria-busy');
  infoEl.textContent =
    'Select an element in the 3-D view or scene tree (Enter / Space on a tree row).';
  scheduleTotalEl.textContent = 'Reading the file…';
  scheduleGroupsEl.innerHTML = '';
  clearHighlight();
  renderLoadingStatusPanel(file.name);
  let bytes: Uint8Array;
  try {
    bytes = new Uint8Array(await file.arrayBuffer());
  } catch (err) {
    const message = (err as Error).message ?? String(err);
    stopLoadOverlay();
    clearDemoCardBusy();
    setStatus(`error: ${message}`);
    renderErrorStatusPanel(message);
    dropzone.classList.remove('hidden');
    toast(`Could not read ${file.name}. ${message}`, 'bad');
    return;
  }
  const qualityMode = selectedExportMode();

  currentDocument = null;
  const w = resetWorker();
  w.addEventListener('message', (ev: MessageEvent<unknown>) => {
    const msg = ev.data as
      | { type: 'progress'; step: string }
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
        }
      | { type: 'info'; index: number; panel: ElementInfoPanel | null }
      | { type: 'error'; message: string };
    if (msg.type === 'info') {
      // Stale reply for an element the user already clicked past.
      if (msg.index !== pendingInfoIndex) return;
      if (msg.panel) renderElementPanel(msg.panel);
      else renderScaffoldPanel(msg.index);
      return;
    }
    if (msg.type === 'progress') {
      setStatus(msg.step);
      setLoadStep(msg.step);
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
      dropzone.classList.add('hidden');
      return;
    }
    if (msg.type === 'error') {
      stopLoadOverlay();
      clearDemoCardBusy();
      setStatus(`error: ${msg.message}`);
      renderErrorStatusPanel(msg.message);
      hideScaffoldNote();
      dropzone.classList.remove('hidden');
      toast(`Could not open ${file.name}. ${msg.message}`, 'bad');
      return;
    }
    model = msg.model;
    sceneGraph = msg.scene;
    distinctTypes = msg.types;
    lastGlb = msg.glb;
    lastSchedule = msg.schedule;
    currentDiagnostics = msg.diagnostics;
    lastFileStem = file.name.replace(/\.(rvt|rfa|rte|rft)$/i, '');
    renderScene(msg.glb);
    renderTree();
    renderCategories();
    renderScheduleSummary(msg.schedule);
    renderExportQuality(msg.diagnostics);
    renderStatusPanel(msg.diagnostics);
    fileMetaEl.textContent = `${file.name} · ${formatBytes(file.size)} · ${countEntities(msg.scene)} entities`;
    dropzone.classList.add('hidden');
    exportGlbBtn.disabled = false;
    exportIfcBtn.disabled = false;
    exportSvgBtn.disabled = false;
    downloadDiagnosticsBtn.disabled = false;
    // The schedules follow the model: an element schedule whenever any
    // building element decoded, a room schedule only when rooms did.
    downloadScheduleBtn.disabled = !scheduleHasType(msg.schedule, null);
    downloadRoomsBtn.disabled = !scheduleHasType(msg.schedule, 'IFCSPACE');
    setStatus(`loaded · ${msg.types.length} categories · IFC bar ${qualityMode}`);
    stopLoadOverlay();
    clearDemoCardBusy();
    maybeShowScaffoldNote(msg.diagnostics);
    const elapsed = Math.max(1, Math.round((performance.now() - loadStartedAt) / 1000));
    toast(`Loaded ${file.name} · ${msg.types.length} categories · ${elapsed} s`, 'ok');
  });
  w.postMessage({ type: 'parse', bytes, mode: qualityMode }, [bytes.buffer]);
}

function renderEmptyStatusPanel(): void {
  statusPanelEl.innerHTML = '';
  resetStatusRowStagger();
  statusPanelEl.appendChild(statusRow('File', 'warn', 'No file opened'));
  statusPanelEl.appendChild(statusRow('Mode', 'warn', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Schema', 'warn', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Elements', 'warn', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Geometry', 'warn', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Decode', 'warn', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Export', 'warn', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('IFC bar', 'warn', `Selected · ${selectedExportMode()}`));
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

function renderErrorStatusPanel(message: string): void {
  statusPanelEl.innerHTML = '';
  resetStatusRowStagger();
  statusPanelEl.appendChild(statusRow('File', 'bad', 'Could not open file'));
  statusPanelEl.appendChild(
    statusRow(
      'Mode',
      'bad',
      'Corrupt or unreadable file · not a readable Revit OLE/CFB container',
    ),
  );
  statusPanelEl.appendChild(statusRow('Schema', 'warn', 'Not parsed'));
  statusPanelEl.appendChild(statusRow('Elements', 'warn', 'Not decoded'));
  statusPanelEl.appendChild(statusRow('Geometry', 'warn', 'Not decoded'));
  statusPanelEl.appendChild(statusRow('Decode', 'bad', 'Decode confidence unavailable'));
  statusPanelEl.appendChild(statusRow('Export', 'bad', 'Export confidence unavailable'));
  statusPanelEl.appendChild(statusRow('IFC bar', 'warn', `Selected · ${selectedExportMode()}`));
  statusPanelEl.appendChild(statusRow('Warnings', 'bad', message));
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
      setStatus(`gltf load error: ${(err as Error).message ?? err}`);
      URL.revokeObjectURL(url);
    },
  );
}

function frameCamera(obj: THREE.Object3D): void {
  const box = new THREE.Box3().setFromObject(obj);
  if (box.isEmpty()) return;
  const size = box.getSize(new THREE.Vector3());
  const center = box.getCenter(new THREE.Vector3());
  const maxDim = Math.max(size.x, size.y, size.z);
  const fov = camera.fov * (Math.PI / 180);
  const dist = Math.abs(maxDim / Math.sin(fov / 2)) * 0.8;
  const toPos = center
    .clone()
    .add(new THREE.Vector3(1, 0.8, 1).normalize().multiplyScalar(dist));
  // Clip planes jump immediately: interpolating them causes visible
  // z-fighting mid-move and carries no information.
  camera.near = maxDim / 100;
  camera.far = dist * 10;
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

// ---------- Panels ----------
function renderTree(): void {
  if (!sceneGraph) return;
  treeEl.innerHTML = '';
  treeEl.appendChild(buildTreeNode(sceneGraph));
}

function storeyNodeLabel(node: SceneNode): string {
  const kidCount = node.children.length;
  const elev = storeyElevationLabel(node);
  const bits = [node.name, node.ifc_type];
  if (elev) bits.push(elev);
  bits.push(kidCount === 1 ? '1 element' : `${kidCount} elements`);
  return bits.join(' · ');
}

function storeyElevationLabel(node: SceneNode): string | null {
  if (node.ifc_type !== 'IFCBUILDINGSTOREY') return null;
  if (node.name === 'Unassigned') return 'no Level ElementId bind';
  const storeys = model?.building_storeys ?? [];
  const idx = node.storey_index;
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

function buildTreeNode(node: SceneNode): HTMLElement {
  const wrap = document.createElement('div');
  const row = document.createElement('div');
  row.className = 'tree-node';
  if (node.ifc_type === 'IFCBUILDINGSTOREY') {
    row.classList.add('tree-storey');
    if (node.name === 'Unassigned') row.classList.add('tree-storey-unassigned');
    if (node.children.length === 0) row.classList.add('tree-storey-empty');
  } else if (node.ifc_type === 'IFCPROJECT') {
    row.classList.add('tree-project');
  }
  row.setAttribute('role', 'treeitem');
  row.tabIndex = 0;
  if (node.entity_index !== null) {
    row.dataset.entityIndex = String(node.entity_index);
  }
  // Storey nodes are synthetic — they carry no entity index, so the
  // panel's storey jump addresses them by storey index instead.
  if (node.ifc_type === 'IFCBUILDINGSTOREY' && node.storey_index != null) {
    row.dataset.storeyIndex = String(node.storey_index);
  }
  row.textContent =
    node.ifc_type === 'IFCBUILDINGSTOREY'
      ? storeyNodeLabel(node)
      : node.ifc_type === 'IFCPROJECT'
        ? `${node.name} · IFCPROJECT · ${node.children.length} storey${
            node.children.length === 1 ? '' : 's'
          }`
        : `${node.name} · ${node.ifc_type}`;
  const activate = (ev: Event) => {
    ev.stopPropagation();
    clearScheduleHighlightState();
    document
      .querySelectorAll('.tree-node.selected')
      .forEach((el) => el.classList.remove('selected'));
    row.classList.add('selected');
    if (node.entity_index !== null) {
      highlightEntity(node.entity_index);
      showElementInfo(node.entity_index);
      return;
    }
    // Project and storey rows are synthetic — they have no entity to
    // inspect. Describe the node itself rather than leaving whatever
    // element was selected before sitting there as if it applied.
    clearHighlight();
    showContainerInfo(node);
  };
  row.addEventListener('click', activate);
  row.addEventListener('keydown', (ev) => {
    if (ev.key === 'Enter' || ev.key === ' ') {
      ev.preventDefault();
      activate(ev);
      return;
    }
    if (ev.key !== 'ArrowDown' && ev.key !== 'ArrowUp') return;
    ev.preventDefault();
    const items = Array.from(
      treeEl.querySelectorAll<HTMLElement>('.tree-node[role="treeitem"]'),
    );
    const idx = items.indexOf(row);
    if (idx < 0) return;
    const next =
      ev.key === 'ArrowDown'
        ? items[Math.min(items.length - 1, idx + 1)]
        : items[Math.max(0, idx - 1)];
    next?.focus();
  });
  wrap.appendChild(row);
  if (node.children.length > 0) {
    const ch = document.createElement('div');
    ch.className = 'tree-children';
    ch.setAttribute('role', 'group');
    for (const c of node.children) ch.appendChild(buildTreeNode(c));
    wrap.appendChild(ch);
  }
  return wrap;
}

function renderCategories(): void {
  categoriesEl.innerHTML = '';
  for (const t of distinctTypes) {
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
    row.append(` ${t}`);
    categoriesEl.appendChild(row);
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
    `${rel.name} · ${rel.ifc_type}`,
    `Select ${rel.name} (${rel.ifc_type})`,
    () => selectEntity(rel.entity_index),
  );
  btn.dataset.entityIndex = String(rel.entity_index);
  return btn;
}

/**
 * Host relationships (M4-04) computed by `element_info_panel` in
 * Rust: the wall a door or window sits in, and the openings a wall
 * carries. Each row re-selects its element — mouse or keyboard.
 */
function relationsBox(panel: ElementInfoPanel): HTMLElement | null {
  const host = panel.host ?? null;
  const hosted = panel.hosted ?? [];
  if (!host && hosted.length === 0) return null;
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
  properties: 'Revit parameters',
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

/** Render the whole panel from the Rust payload. */
function renderElementPanel(panel: ElementInfoPanel): void {
  infoEl.innerHTML = '';
  infoEl.removeAttribute('aria-busy');

  const identity = infoGroup('identity');
  identity.appendChild(infoRow('Name', panel.name));
  identity.appendChild(
    infoRow(
      'Type',
      panel.predefined_type ? `${panel.ifc_type} · ${panel.predefined_type}` : panel.ifc_type,
    ),
  );
  if (panel.type_guid) identity.appendChild(infoRow('GUID', panel.type_guid));
  infoEl.appendChild(identity);

  if (panel.storey) infoEl.appendChild(storeyGroup(panel.storey));
  if (panel.material) infoEl.appendChild(materialGroup(panel.material));

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
  infoEl.innerHTML = '';
  infoEl.removeAttribute('aria-busy');
  if (!node) {
    infoEl.textContent = 'That element is no longer in the scene.';
    return;
  }
  const identity = infoGroup('identity');
  identity.appendChild(infoRow('Name', node.name));
  identity.appendChild(infoRow('Type', node.ifc_type));
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
  const node = e ? null : findSceneNodeByIndex(sceneGraph, idx);
  const name = e?.name ?? node?.name;
  const ifcType = e?.ifc_type ?? node?.ifc_type;
  if (name === undefined || ifcType === undefined) return;
  infoEl.innerHTML = '';
  infoEl.setAttribute('aria-busy', 'true');
  const identity = infoGroup('identity');
  identity.appendChild(infoRow('Name', name));
  identity.appendChild(infoRow('Type', ifcType));
  infoEl.appendChild(identity);
}

/**
 * Panel for a synthetic container node — the project root or a
 * storey. These carry no entity, so the panel reports what the
 * node itself is and how much it holds.
 */
function showContainerInfo(node: SceneNode): void {
  pendingInfoIndex = null;
  infoEl.innerHTML = '';
  infoEl.removeAttribute('aria-busy');
  const identity = infoGroup('identity');
  identity.appendChild(infoRow('Name', node.name));
  identity.appendChild(infoRow('Type', node.ifc_type));
  const kids = node.children.length;
  if (node.ifc_type === 'IFCBUILDINGSTOREY') {
    identity.appendChild(
      infoRow('Contains', `${kids} element${kids === 1 ? '' : 's'}`, true),
    );
    const storeys = model?.building_storeys ?? [];
    const idx = node.storey_index;
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
  const row = treeEl.querySelector<HTMLElement>(`.tree-node[data-storey-index="${storeyIndex}"]`);
  if (!row) return;
  document.querySelectorAll('.tree-node.selected').forEach((el) => el.classList.remove('selected'));
  row.classList.add('selected');
  row.scrollIntoView({
    block: 'nearest',
    behavior: prefersReducedMotion() ? 'auto' : 'smooth',
  });
  row.focus();
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
    row.appendChild(scheduleCell('schedule-type', group.ifc_type));
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
    `Highlight ${group.count} ${group.ifc_type} elements${storeyNote} in the 3-D view`,
  );
  btn.appendChild(scheduleCell('schedule-type', group.ifc_type));
  btn.appendChild(scheduleCell('schedule-count', countLabel));
  const verb = scheduleCell('schedule-verb', 'Highlight');
  btn.appendChild(verb);
  btn.addEventListener('click', () => {
    const wasActive = activeTypeHighlight === group.ifc_type;
    clearScheduleHighlightState();
    if (wasActive) return;
    const lit = highlightIfcType(group.ifc_type);
    if (lit === 0) {
      toast(`No ${group.ifc_type} geometry to highlight.`, 'info');
      return;
    }
    btn.setAttribute('aria-pressed', 'true');
    verb.textContent = 'Clear';
    setStatus(`highlighted ${lit} ${group.ifc_type} mesh${lit === 1 ? '' : 'es'}`);
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
        `${status} · ${pages} · strip ${strip}${code}`,
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
    'ArcWallRectOpening',
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
      statusPanelEl.appendChild(
        statusRow(
          'Materials',
          'ok',
          `${materialCount} · ${named} (names only; no compound layers)`,
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
          ? `${parameterValueCount} AProperty* value(s) recovered`
          : 'none recovered (AProperty* host joins pending)',
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
      typeof meanConf === 'number' ? `${Math.round(meanConf * 100)}% mean` : 'mean n/a';
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

type StatusKind = 'ok' | 'warn' | 'bad';

interface FailureModeStatus {
  kind: StatusKind;
  title: string;
  summary: string;
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

  if (typeof revitVersion === 'number' && (revitVersion < 2016 || revitVersion > 2026)) {
    return {
      kind: 'warn',
      title: 'Unsupported Revit version',
      summary: 'outside the verified support range',
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
      title: 'Incomplete model',
      summary: `${unexported} wall, door, window, column, floor or room records could not be exported (RE-30)`,
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
    infoEl.textContent =
      'Select an element in the 3-D view or scene tree (Enter / Space on a tree row).';
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
    setStatus(`ignored: ${f.name} — not a Revit file`);
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
type WasmModule = typeof import('../pkg/rvt.js');
let mainThreadWasmReady: Promise<WasmModule> | null = null;
function mainThreadWasm(): Promise<WasmModule> {
  if (!mainThreadWasmReady) {
    mainThreadWasmReady = import('../pkg/rvt.js')
      .then(async (mod) => {
        await mod.default();
        return mod;
      })
      .catch((err: unknown) => {
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
  setStatus(`exported ${lastFileStem}.glb`);
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
    setStatus(`rendering IFC STEP · ${quality} · bar ${bar}`);
    try {
      const { modelToIfcStep } = await mainThreadWasm();
      const text = modelToIfcStep(model as unknown as object);
      const blob = new Blob([text], { type: 'application/x-step' });
      download(`${lastFileStem}.ifc`, blob);
      setStatus(`exported ${lastFileStem}.ifc`);
      toast(`Exported ${lastFileStem}.ifc · ${quality}`, 'ok');
    } catch (err) {
      const message = (err as Error).message ?? String(err);
      setStatus(`IFC export failed: ${message}`);
      toast(`IFC export failed. ${message}`, 'bad');
    }
  })();
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
    setStatus('rendering plan SVG…');
    try {
      const { renderPlanSvg } = await mainThreadWasm();
      const svg = renderPlanSvg(model as unknown as object, null);
      const blob = new Blob([svg], { type: 'image/svg+xml' });
      download(`${lastFileStem}.svg`, blob);
      setStatus(`exported ${lastFileStem}.svg`);
      toast(`Exported ${lastFileStem}.svg`, 'ok');
    } catch (err) {
      const message = (err as Error).message ?? String(err);
      setStatus(`plan export failed: ${message}`);
      toast(`Plan export failed. ${message}`, 'bad');
    }
  })();
});

downloadDiagnosticsBtn.addEventListener('click', () => {
  if (!currentDiagnostics) return;
  const json = JSON.stringify(currentDiagnostics, null, 2);
  const blob = new Blob([json], { type: 'application/json' });
  download(`${lastFileStem}.diagnostics.json`, blob);
  setStatus(`exported ${lastFileStem}.diagnostics.json`);
  toast(`Downloaded ${lastFileStem}.diagnostics.json`, 'ok');
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
    setStatus(`building ${label}…`);
    try {
      const { scheduleCsv } = await mainThreadWasm();
      const csv = scheduleCsv(model as unknown as object, kind, false, true);
      const rows = Math.max(0, csv.split('\r\n').filter((line) => line.length > 0).length - 1);
      download(`${lastFileStem}.${kind}.csv`, new Blob([csv], { type: 'text/csv' }));
      setStatus(`exported ${lastFileStem}.${kind}.csv · ${rows} rows`);
      toast(`Exported ${label} · ${rows} rows`, 'ok');
    } catch (err) {
      const message = (err as Error).message ?? String(err);
      setStatus(`${label} export failed: ${message}`);
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
    setStatus(`demo ${demo.id} is reference-only — use download link in gallery`);
    return;
  }
  if (card) setDemoCardBusy(card);
  setStatus(`loading demo ${demo.name}…`);
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
    setStatus(`demo load failed: ${message}`);
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
    img.width = 72;
    img.height = 40;
    card.appendChild(img);
  }
  const body = document.createElement('div');
  const title = document.createElement('div');
  title.className = 'demo-title';
  title.textContent = demo.name;
  if (real) {
    // Distinguishes an actual Revit project from a synthetic fixture at a
    // glance — the two decode to very different results.
    const badge = document.createElement('span');
    badge.className = 'demo-badge';
    badge.textContent = 'real project';
    title.appendChild(badge);
  }
  const meta = document.createElement('div');
  meta.className = 'demo-meta';
  const bits = [
    demo.format.toUpperCase(),
    demo.revit_version ? `Revit ${demo.revit_version}` : null,
    demo.element_count_hint ? `~${demo.element_count_hint} elements` : null,
    demo.license ?? null,
  ].filter(Boolean);
  meta.textContent = bits.join(' · ');
  if (demo.provenance) {
    meta.textContent += `\n${demo.provenance}`;
  }
  if (demo.expected_quality_note) {
    meta.textContent += `\n${demo.expected_quality_note}`;
  }
  if (!demo.available) {
    meta.textContent += '\nNot bundled in this build (corpus optional).';
  } else if (!loadable && demo.format !== 'ifc') {
    meta.textContent += '\nUnavailable in this build.';
  } else if (!loadable) {
    meta.textContent += '\nReference download — not opened by the RVT parser.';
  }
  const quality = document.createElement('span');
  quality.className = `demo-quality ${qualityPillClass(demo.expected_quality)}`;
  quality.textContent = `expected: ${demo.expected_quality ?? 'Unknown'}`;
  body.appendChild(title);
  body.appendChild(meta);
  body.appendChild(quality);
  card.appendChild(body);
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
    setStatus(`demo gallery: ${(err as Error).message ?? err}`);
  }
}

renderEmptyStatusPanel();
void initDemoGallery();
setStatus('ready · drop a file or open a demo');
