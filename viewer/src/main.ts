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
const statusPanelEl = $('status-panel');
const diagnosticsJsonEl = $('diagnostics-json');
const downloadDiagnosticsBtn = $('download-diagnostics') as HTMLButtonElement;
const exportGlbBtn = $('export-glb') as HTMLButtonElement;
const exportIfcBtn = $('export-ifc') as HTMLButtonElement;
const exportSvgBtn = $('export-svg') as HTMLButtonElement;
const exportQualityEl = $('export-quality');
const exportModeEl = $('export-mode') as HTMLSelectElement;
const demoListEl = $('demo-list');
const demoAttributionEl = $('demo-attribution');

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
  project_name?: string;
  description?: string;
  building_storeys?: Array<{ name: string; elevation_feet?: number }>;
  materials?: Array<{ name: string; color_packed?: number; transparency?: number }>;
  entities?: Array<{ name: string; ifc_type: string; guid?: string }>;
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
let lastFileStem = 'model';
let currentDiagnostics: ExportDiagnostics | null = null;
const hiddenTypes = new Set<string>();

function selectedExportMode(): string {
  return exportModeEl.value || 'scaffold';
}

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
  const qualityMode = selectedExportMode();

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
    setStatus(`loaded · ${msg.types.length} categories · IFC bar ${qualityMode}`);
  });
  w.postMessage({ type: 'parse', bytes, mode: qualityMode }, [bytes.buffer]);
}

function renderEmptyStatusPanel(): void {
  statusPanelEl.innerHTML = '';
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
    document
      .querySelectorAll('.tree-node.selected')
      .forEach((el) => el.classList.remove('selected'));
    row.classList.add('selected');
    if (node.entity_index !== null) showElementInfo(node.entity_index);
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

function showElementInfo(idx: number): void {
  if (!model) return;
  const e = model.entities?.[idx];
  if (e) {
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
    return;
  }

  // Scaffold / partial exports may expose a scene-graph index without a
  // populated entities[] row. Fall back to the tree node so inspect still
  // surfaces something honest for the MVP workflow shell.
  const node = findSceneNodeByIndex(sceneGraph, idx);
  if (!node) {
    infoEl.textContent = 'not found';
    return;
  }
  infoEl.innerHTML = '';
  for (const [k, v] of Object.entries({
    name: node.name,
    ifc_type: node.ifc_type,
    entity_index: node.entity_index,
    note: 'Partial decode — typed element fields were not recovered for this node.',
  })) {
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

function findSceneNodeByIndex(node: SceneNode | null, idx: number): SceneNode | null {
  if (!node) return null;
  if (node.entity_index === idx) return node;
  for (const child of node.children) {
    const hit = findSceneNodeByIndex(child, idx);
    if (hit) return hit;
  }
  return null;
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
  const bar = selectedExportMode();
  const barCheck = validateExportBar(bar, diagnostics);
  exportIfcBtn.title = `Download as IFC4 STEP · ${label} · bar ${bar}${barCheck.ok ? '' : ' (will warn)'} · ${elements} elements · ${geometry} with geometry · ${warnings} warnings`;
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
  const classBits = ['Level', 'Floor', 'Room', 'Material', 'ArcWall', 'ArcWallRectOpening', 'Wall', 'Door', 'Window']
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
  if (confidence.has_typed_elements && confidence.has_geometry) return 'ok';
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
  const selected = treeEl.querySelector('.tree-node.selected');
  if (selected) {
    selected.classList.remove('selected');
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

async function loadDemoFile(demo: DemoEntry): Promise<void> {
  if (!demo.loadable || !demo.available) {
    setStatus(`demo ${demo.id} is reference-only — use download link in gallery`);
    return;
  }
  setStatus(`loading demo ${demo.name}…`);
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
    setStatus(`demo load failed: ${(err as Error).message ?? err}`);
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
        void loadDemoFile(demo);
      });
    }
    fillDemoCard(card, demo, canOpen);
    demoListEl.appendChild(card);
  }
}

function fillDemoCard(card: HTMLElement, demo: DemoEntry, loadable: boolean): void {
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
