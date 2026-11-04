"""Helpers the tools/re scorers share, one copy of each so they cannot drift."""

import concurrent.futures
import concurrent.futures.process
import hashlib
import multiprocessing
import os
import re
import sys
import time

import ifcopenshell.geom
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

FEET = 0.3048
MESH_ERRORS = set()


def site_frame(f):
    """Shared metres (IfcOpenShell world) -> Revit internal feet; a file with
    no IfcSite is taken as placed at the origin. The site's placement is in the
    file's length unit (millimetres in the held-out 2026 house's and Gdynia's
    exports), so its origin is scaled to metres first: read as metres, it put
    Revit's side of every comparison 1000 times its offset from rvt-rs's."""
    sites = f.by_type("IfcSite")
    if not sites:
        return lambda v: v / FEET
    site = sites[0]
    mat = ifcopenshell.util.placement.get_local_placement(site.ObjectPlacement)
    unit = ifcopenshell.util.unit.calculate_unit_scale(f)
    ox, oy, oz = mat[0][3] * unit, mat[1][3] * unit, mat[2][3] * unit
    c, s = mat[0][0], mat[1][0]

    def points(v):
        dx, dy = v[:, 0] - ox, v[:, 1] - oy
        return np.stack(
            [(c * dx + s * dy) / FEET, (-s * dx + c * dy) / FEET, (v[:, 2] - oz) / FEET], axis=1
        )

    return points


def report_mesh_error(error):
    """Print each distinct reason IfcOpenShell gives for not meshing an element, once (#773)."""
    text = (str(error).strip().splitlines() or [""])[0]
    message = f"{type(error).__name__}: {re.sub(r'#[0-9]+', '#N', text)[:200]}"
    if message not in MESH_ERRORS:
        MESH_ERRORS.add(message)
        print(f"IfcOpenShell cannot mesh: {message}")


_MESHING = {}
# Instance id -> the first line of IfcOpenShell's reason, for each part
# `mesh_parts` could not mesh (a part kept in the cache keeps its reason too).
# Nothing prints them: a scorer's output does not change when a part fails.
MESH_FAILURES = {}
# Fewer parts than this are meshed in this process: a forked worker parses
# again the parts of the file it meshes, which a handful do not repay.
POOL_FROM = 16


def _in_worker():
    _MESHING["parent"] = os.getppid()


def _mesh_one(part_id):
    """(vertices, faces) of the part, or the first line of why IfcOpenShell would not mesh it."""
    # An OOM-killed parent leaves its workers with the queued parts: stop at the next one.
    if "parent" in _MESHING and os.getppid() != _MESHING["parent"]:
        os._exit(1)
    try:
        shape = ifcopenshell.geom.create_shape(_MESHING["settings"], _MESHING["model"].by_id(part_id))
    except RuntimeError as error:
        return (str(error).strip().splitlines() or [""])[0] or type(error).__name__
    return (
        np.array(shape.geometry.verts, dtype=float).reshape(-1, 3),
        np.array(shape.geometry.faces, dtype=int).reshape(-1, 3),
    )


def _settings_key(settings):
    items = []
    for name in sorted(settings.setting_names()):
        try:
            items.append(f"{name}={settings.get(name)!r}")
        except RuntimeError:
            items.append(f"{name}=unset")
    return ";".join(items)


def _mesh_cache(path, settings):
    root = os.environ.get("RVT_MESH_CACHE")
    if not root or not path:
        return None
    stat = os.stat(path)
    key = f"{os.path.realpath(path)} {stat.st_size} {stat.st_mtime_ns} {_settings_key(settings)}"
    return os.path.join(root, hashlib.sha256(key.encode()).hexdigest()[:16])


def _read_meshes(path):
    """What `_write_meshes` kept: instance id -> (vertices, faces), or the reason IfcOpenShell gave for a part it could not mesh."""
    out = {}
    with np.load(path, allow_pickle=False) as kept:
        ids, reasons, nverts, nfaces = kept["ids"], kept["reasons"], kept["nverts"], kept["nfaces"]
        verts, faces = kept["verts"], kept["faces"]
    vstart = np.concatenate([[0], np.cumsum(nverts)])
    fstart = np.concatenate([[0], np.cumsum(nfaces)])
    for k, part_id in enumerate(ids):
        out[int(part_id)] = str(reasons[k]) or (verts[vstart[k] : vstart[k + 1]], faces[fstart[k] : fstart[k + 1]])
    return out


def _write_meshes(path, meshes):
    ids = list(meshes)
    parts = [(np.empty((0, 3)), np.empty((0, 3), dtype=int)) if isinstance(meshes[i], str) else meshes[i] for i in ids]
    scratch = f"{path}.tmp"
    with open(scratch, "wb") as f:
        np.savez(
            f,
            ids=np.array(ids, dtype=np.int64),
            reasons=np.array([meshes[i] if isinstance(meshes[i], str) else "" for i in ids]),
            nverts=np.array([len(v) for v, _ in parts], dtype=np.int64),
            nfaces=np.array([len(t) for _, t in parts], dtype=np.int64),
            verts=np.concatenate([v for v, _ in parts]),
            faces=np.concatenate([t for _, t in parts]),
        )
    os.replace(scratch, path)


def mesh_parts(model, settings, parts, path=None):
    """Instance id -> (vertices (N, 3), faces (M, 3)) for each of `parts` that
    IfcOpenShell meshes, one `create_shape` per part spread over the CPUs. The
    triangles are those `create_shape` gives, in the same order (the
    iterator's are the same, but it took twice the CPU time per element in
    a measurement on the BIM4LCA office, and no less wall time with 12 threads
    than with 4); a part it cannot mesh is left out, with its reason in
    `MESH_FAILURES`.

    With `RVT_MESH_CACHE` set to a directory and the model's `path`, what is
    meshed is kept there for the next scorer to read, which Measure sets for
    the scorers it runs one after another on one pair of files and deletes
    afterwards. Each call keeps its own shard, named by process and time, and
    the cache is keyed by the file's path, size and modification time and by
    every setting."""
    ids = list(dict.fromkeys(part.id() for part in parts))
    cache = _mesh_cache(path, settings)
    known = {}
    if cache and os.path.isdir(cache):
        for name in sorted(os.listdir(cache)):
            if name.endswith(".npz"):
                known.update(_read_meshes(os.path.join(cache, name)))
    todo = [i for i in ids if i not in known]
    cpus = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count() or 1
    workers = min(int(os.environ.get("RVT_MESH_WORKERS") or cpus), len(todo))
    if len(todo) < POOL_FROM:
        workers = 1
    _MESHING.update(model=model, settings=settings)
    meshes = None
    if workers > 1:
        # A forked worker flushes the buffered output it inherits when it ends.
        sys.stdout.flush()
        sys.stderr.flush()
        try:
            with concurrent.futures.ProcessPoolExecutor(
                workers, mp_context=multiprocessing.get_context("fork"), initializer=_in_worker
            ) as pool:
                meshes = list(pool.map(_mesh_one, todo, chunksize=1))
        except concurrent.futures.process.BrokenProcessPool:
            print("mesh_parts: a worker died, so the parts are meshed one after another", file=sys.stderr)
            meshes = None
    if meshes is None:
        meshes = [_mesh_one(i) for i in todo]
    _MESHING.clear()
    fresh = dict(zip(todo, meshes))
    if cache and fresh:
        os.makedirs(cache, exist_ok=True)
        _write_meshes(os.path.join(cache, f"{os.getpid()}-{time.time_ns()}.npz"), fresh)
    known.update(fresh)
    MESH_FAILURES.update({i: known[i] for i in ids if isinstance(known[i], str)})
    return {i: known[i] for i in ids if not isinstance(known[i], str)}
