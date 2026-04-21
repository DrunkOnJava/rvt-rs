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

let model: IfcModel | null = null;
let sceneGraph: SceneNode | null = null;
let distinctTypes: string[] = [];
const hiddenTypes = new Set<string>();

// ---------- Load flow ----------
async function loadBytes(file: File): Promise<void> {
  setStatus(`reading ${formatBytes(file.size)}…`);
  const bytes = new Uint8Array(await file.arrayBuffer());

  const w = resetWorker();
  w.addEventListener('message', (ev: MessageEvent<unknown>) => {
    const msg = ev.data as
      | { type: 'progress'; step: string }
      | {
          type: 'summary';
          summary: { version: number; build?: string; guid?: string; class_name_count?: number };
        }
      | { type: 'ready'; model: IfcModel; scene: SceneNode; types: string[]; glb: Uint8Array; schedule: unknown }
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
      dropzone.classList.remove('hidden');
      return;
    }
    model = msg.model;
    sceneGraph = msg.scene;
    distinctTypes = msg.types;
    renderScene(msg.glb);
    renderTree();
    renderCategories();
    renderScheduleSummary(msg.schedule);
    fileMetaEl.textContent = `${file.name} · ${formatBytes(file.size)} · ${countEntities(msg.scene)} entities`;
    dropzone.classList.add('hidden');
    setStatus(`loaded · ${msg.types.length} categories`);
  });
  w.postMessage({ type: 'parse', bytes }, [bytes.buffer]);
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

setStatus('ready · drop a file to begin');
