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
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';

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
const statusPanelEl = $('status-panel');
const diagnosticsJsonEl = $('diagnostics-json');
const downloadDiagnosticsBtn = $('download-diagnostics') as HTMLButtonElement;
const exportGlbBtn = $('export-glb') as HTMLButtonElement;
const exportIfcBtn = $('export-ifc') as HTMLButtonElement;
const exportSvgBtn = $('export-svg') as HTMLButtonElement;
const exportQualityEl = $('export-quality');

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

function tick(): void {
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
  showElementInfo(userData.entityIndex);
});

// ---------- Status ----------
function setStatus(text: string): void {
  statusEl.textContent = text;
}

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
  project?: { name?: string; description?: string };
  storeys?: Array<{ name: string; elevation?: number }>;
  entities?: Array<{ name: string; ifc_type: string; guid?: string }>;
}
interface SceneNode {
  name: string;
  ifc_type: string;
  entity_index: number | null;
  children: SceneNode[];
}
interface ExportDiagnostics {
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
  };
  confidence?: {
    level?: string;
    score?: number;
    has_typed_elements?: boolean;
    has_geometry?: boolean;
    warning_count?: number;
  };
  exported?: {
    building_elements?: number;
    building_elements_with_geometry?: number;
  };
  unsupported_features?: string[];
  warnings?: string[];
}

let model: IfcModel | null = null;
let sceneGraph: SceneNode | null = null;
let distinctTypes: string[] = [];
let lastGlb: Uint8Array | null = null;
let lastFileStem = 'model';
let currentDiagnostics: ExportDiagnostics | null = null;
const hiddenTypes = new Set<string>();

// ---------- Load flow ----------
async function loadBytes(file: File): Promise<void> {
  setStatus(`reading ${formatBytes(file.size)}…`);
  model = null;
  sceneGraph = null;
  distinctTypes = [];
  lastGlb = null;
  currentDiagnostics = null;
  exportGlbBtn.disabled = true;
  exportIfcBtn.disabled = true;
  exportSvgBtn.disabled = true;
  downloadDiagnosticsBtn.disabled = true;
  exportQualityEl.textContent = 'quality: pending';
  exportQualityEl.className = 'quality-pill';
  diagnosticsJsonEl.textContent = '';
  renderLoadingStatusPanel(file.name);
  const bytes = new Uint8Array(await file.arrayBuffer());

  const w = resetWorker();
  w.addEventListener('message', (ev: MessageEvent<unknown>) => {
    const msg = ev.data as
      | { type: 'progress'; step: string }
      | {
          type: 'summary';
          summary: { version: number; build?: string; guid?: string; class_name_count?: number };
        }
      | {
          type: 'ready';
          model: IfcModel;
          scene: SceneNode;
          types: string[];
          glb: Uint8Array;
          schedule: unknown;
          diagnostics: ExportDiagnostics;
        }
      | { type: 'error'; message: string };
    if (msg.type === 'progress') {
      setStatus(msg.step);
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
      dropzone.classList.add('hidden');
      return;
    }
    if (msg.type === 'error') {
      setStatus(`error: ${msg.message}`);
      renderErrorStatusPanel(msg.message);
      dropzone.classList.remove('hidden');
      return;
    }
    model = msg.model;
    sceneGraph = msg.scene;
    distinctTypes = msg.types;
    lastGlb = msg.glb;
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
    setStatus(`loaded · ${msg.types.length} categories`);
  });
  w.postMessage({ type: 'parse', bytes }, [bytes.buffer]);
}

function renderEmptyStatusPanel(): void {
  statusPanelEl.innerHTML = '';
  statusPanelEl.appendChild(statusRow('File', 'warn', 'No file opened'));
  statusPanelEl.appendChild(statusRow('Schema', 'warn', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Elements', 'warn', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Geometry', 'warn', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('IFC', 'warn', 'Waiting for file'));
  statusPanelEl.appendChild(statusRow('Warnings', 'ok', 'No export warnings'));
  diagnosticsJsonEl.textContent = '';
}

function renderLoadingStatusPanel(filename: string): void {
  statusPanelEl.innerHTML = '';
  statusPanelEl.appendChild(statusRow('File', 'warn', `Reading ${filename}`));
  statusPanelEl.appendChild(statusRow('Schema', 'warn', 'Not parsed yet'));
  statusPanelEl.appendChild(statusRow('Elements', 'warn', 'Not decoded yet'));
  statusPanelEl.appendChild(statusRow('Geometry', 'warn', 'Not decoded yet'));
  statusPanelEl.appendChild(statusRow('IFC', 'warn', 'Not evaluated yet'));
  statusPanelEl.appendChild(statusRow('Warnings', 'ok', 'No export warnings'));
}

function renderErrorStatusPanel(message: string): void {
  statusPanelEl.innerHTML = '';
  statusPanelEl.appendChild(statusRow('File', 'bad', 'Could not open file'));
  statusPanelEl.appendChild(statusRow('Schema', 'warn', 'Not parsed'));
  statusPanelEl.appendChild(statusRow('Elements', 'warn', 'Not decoded'));
  statusPanelEl.appendChild(statusRow('Geometry', 'warn', 'Not decoded'));
  statusPanelEl.appendChild(statusRow('IFC', 'warn', 'Not available'));
  statusPanelEl.appendChild(statusRow('Warnings', 'bad', message));
  diagnosticsJsonEl.textContent = '';
}

function renderScene(glb: Uint8Array): void {
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
  camera.position.copy(center).add(new THREE.Vector3(1, 0.8, 1).normalize().multiplyScalar(dist));
  controls.target.copy(center);
  camera.near = maxDim / 100;
  camera.far = dist * 10;
  camera.updateProjectionMatrix();
}

// ---------- Panels ----------
function renderTree(): void {
  if (!sceneGraph) return;
  treeEl.innerHTML = '';
  treeEl.appendChild(buildTreeNode(sceneGraph));
}
function buildTreeNode(node: SceneNode): HTMLElement {
  const wrap = document.createElement('div');
  const row = document.createElement('div');
  row.className = 'tree-node';
  row.textContent = `${node.name} · ${node.ifc_type}`;
  row.addEventListener('click', (ev) => {
    ev.stopPropagation();
    if (node.entity_index !== null) showElementInfo(node.entity_index);
  });
  wrap.appendChild(row);
  if (node.children.length > 0) {
    const ch = document.createElement('div');
    ch.className = 'tree-children';
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

function showElementInfo(idx: number): void {
  if (!model || !model.entities) return;
  const e = model.entities[idx];
  if (!e) {
    infoEl.textContent = 'not found';
    return;
  }
  infoEl.innerHTML = '';
  for (const [k, v] of Object.entries(e)) {
    const row = document.createElement('div');
    row.className = 'info-row';
    const kE = document.createElement('div');
    kE.className = 'k';
    kE.textContent = k;
    const vE = document.createElement('div');
    vE.className = 'v';
    vE.textContent = v === null || v === undefined ? '—' : JSON.stringify(v);
    row.appendChild(kE);
    row.appendChild(vE);
    infoEl.appendChild(row);
  }
}

function renderScheduleSummary(schedule: unknown): void {
  const s = schedule as { rows?: unknown[] } | null;
  if (!s || !s.rows) {
    scheduleEl.textContent = '(empty)';
    return;
  }
  scheduleEl.textContent = `${s.rows.length} scheduled elements`;
}

function renderExportQuality(diagnostics: ExportDiagnostics): void {
  const level = diagnostics.confidence?.level ?? 'unknown';
  const label = exportQualityLabel(level);
  const score = diagnostics.confidence?.score;
  const suffix = typeof score === 'number' ? ` · ${Math.round(score * 100)}%` : '';
  exportQualityEl.textContent = `${label}${suffix}`;
  exportQualityEl.className = `quality-pill ${exportQualityClass(level)}`;

  const elements = diagnostics.exported?.building_elements ?? 0;
  const geometry = diagnostics.exported?.building_elements_with_geometry ?? 0;
  const warnings = diagnostics.confidence?.warning_count ?? diagnostics.warnings?.length ?? 0;
  exportIfcBtn.title = `Download as IFC4 STEP · ${label} · ${elements} elements · ${geometry} with geometry · ${warnings} warnings`;
}

function renderStatusPanel(diagnostics: ExportDiagnostics): void {
  statusPanelEl.innerHTML = '';
  diagnosticsJsonEl.textContent = JSON.stringify(diagnostics, null, 2);

  const input = diagnostics.input ?? {};
  const decoded = diagnostics.decoded ?? {};
  const exported = diagnostics.exported ?? {};
  const confidence = diagnostics.confidence ?? {};
  const warnings = diagnostics.warnings ?? [];
  const unsupported = diagnostics.unsupported_features ?? [];
  const validatedElements =
    (decoded.production_walker_elements ?? 0) + (decoded.arcwall_records ?? 0);
  const diagnosticCandidates = decoded.diagnostic_proxy_candidates ?? 0;
  const geometryCount = exported.building_elements_with_geometry ?? 0;
  const qualityLevel = confidence.level ?? 'unknown';

  statusPanelEl.appendChild(
    statusRow(
      'File',
      input.stream_count ? 'ok' : 'warn',
      input.revit_version
        ? `Opened Revit ${input.revit_version} · ${input.stream_count ?? 0} streams`
        : `Opened · ${input.stream_count ?? 0} streams`,
    ),
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
    statusRow(
      'IFC',
      qualityLevel === 'geometry' ? 'ok' : 'warn',
      `${exportQualityLabel(qualityLevel)}${typeof confidence.score === 'number' ? ` · ${Math.round(confidence.score * 100)}%` : ''}`,
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

function statusRow(label: string, kind: StatusKind, value: string): HTMLElement {
  const row = document.createElement('div');
  row.className = 'status-row';
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
  const parts: string[] = [];
  if (warnings.length > 0) {
    parts.push(`${warnings.length} warning${warnings.length === 1 ? '' : 's'}`);
  }
  if (unsupported.length > 0) {
    parts.push(`${unsupported.length} unsupported feature${unsupported.length === 1 ? '' : 's'}`);
  }
  if (parts.length === 0) return 'No export warnings';
  return parts.join(' · ');
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
    return;
  }
  void loadBytes(f);
});

// ---------- Export buttons (VW1-16 / VW1-17 / VW1-11 surfaced) ----------

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
    setStatus(`rendering IFC STEP · ${quality}`);
    try {
      const { modelToIfcStep } = await import('../pkg/rvt.js');
      const text = modelToIfcStep(model as unknown as object);
      const blob = new Blob([text], { type: 'application/x-step' });
      download(`${lastFileStem}.ifc`, blob);
      setStatus(`exported ${lastFileStem}.ifc`);
    } catch (err) {
      setStatus(`IFC export failed: ${(err as Error).message ?? err}`);
    }
  })();
});

exportSvgBtn.addEventListener('click', () => {
  if (!model) return;
  void (async () => {
    setStatus('rendering plan SVG…');
    try {
      const { renderPlanSvg } = await import('../pkg/rvt.js');
      const svg = renderPlanSvg(model as unknown as object, null);
      const blob = new Blob([svg], { type: 'image/svg+xml' });
      download(`${lastFileStem}.svg`, blob);
      setStatus(`exported ${lastFileStem}.svg`);
    } catch (err) {
      setStatus(`plan export failed: ${(err as Error).message ?? err}`);
    }
  })();
});

downloadDiagnosticsBtn.addEventListener('click', () => {
  if (!currentDiagnostics) return;
  const json = JSON.stringify(currentDiagnostics, null, 2);
  const blob = new Blob([json], { type: 'application/json' });
  download(`${lastFileStem}.diagnostics.json`, blob);
  setStatus(`exported ${lastFileStem}.diagnostics.json`);
});

renderEmptyStatusPanel();
setStatus('ready · drop a file to begin');
