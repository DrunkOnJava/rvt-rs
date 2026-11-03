"""Helpers the tools/re scorers share, one copy of each so they cannot drift."""

import concurrent.futures
import concurrent.futures.process
import hashlib
import multiprocessing
import os
import re
import sys

import ifcopenshell.geom
import ifcopenshell.util.placement
import numpy as np

FEET = 0.3048
MESH_ERRORS = set()


def site_frame(f):
    """Shared metres (IfcOpenShell world) -> Revit internal feet; a file with
    no IfcSite is taken as placed at the origin."""
    sites = f.by_type("IfcSite")
    if not sites:
        return lambda v: v / FEET
    site = sites[0]
    mat = ifcopenshell.util.placement.get_local_placement(site.ObjectPlacement)
    ox, oy, oz = mat[0][3], mat[1][3], mat[2][3]
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
# Fewer parts than this are meshed in this process: a forked worker parses
# again the parts of the file it meshes, which a handful do not repay.
POOL_FROM = 16


def _mesh_one(part_id):
    try:
        shape = ifcopenshell.geom.create_shape(_MESHING["settings"], _MESHING["model"].by_id(part_id))
    except RuntimeError:
        return None
    return (
        np.array(shape.geometry.verts, dtype=float).reshape(-1, 3),
        np.array(shape.geometry.faces, dtype=int).reshape(-1, 3),
    )


def _mesh_cache(path, settings):
    root = os.environ.get("RVT_MESH_CACHE")
    if not root or not path:
        return None
    stat = os.stat(path)
    key = f"{os.path.realpath(path)} {stat.st_size} {stat.st_mtime_ns} {settings.get('use-world-coords')}"
    return os.path.join(root, hashlib.sha256(key.encode()).hexdigest()[:16])


def _read_meshes(path):
    """What `_write_meshes` kept: instance id -> (vertices, faces), or None for a part IfcOpenShell could not mesh."""
    out = {}
    with np.load(path, allow_pickle=False) as kept:
        ids, meshed, nverts, nfaces = kept["ids"], kept["meshed"], kept["nverts"], kept["nfaces"]
        verts, faces = kept["verts"], kept["faces"]
    vstart = np.concatenate([[0], np.cumsum(nverts)])
    fstart = np.concatenate([[0], np.cumsum(nfaces)])
    for k, part_id in enumerate(ids):
        out[int(part_id)] = (verts[vstart[k] : vstart[k + 1]], faces[fstart[k] : fstart[k + 1]]) if meshed[k] else None
    return out


def _write_meshes(path, meshes):
    ids = list(meshes)
    parts = [meshes[i] if meshes[i] is not None else (np.empty((0, 3)), np.empty((0, 3), dtype=int)) for i in ids]
    scratch = f"{path}.tmp"
    with open(scratch, "wb") as f:
        np.savez(
            f,
            ids=np.array(ids, dtype=np.int64),
            meshed=np.array([meshes[i] is not None for i in ids], dtype=bool),
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
    than with 4); a part it cannot mesh is left out.

    With `RVT_MESH_CACHE` set to a directory and the model's `path`, what is
    meshed is kept there for the next scorer to read, which Measure sets for
    the scorers it runs one after another on one pair of files and deletes
    afterwards."""
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
            with concurrent.futures.ProcessPoolExecutor(workers, mp_context=multiprocessing.get_context("fork")) as pool:
                meshes = list(pool.map(_mesh_one, todo, chunksize=1))
        except concurrent.futures.process.BrokenProcessPool:
            meshes = None
    if meshes is None:
        meshes = [_mesh_one(i) for i in todo]
    _MESHING.clear()
    fresh = dict(zip(todo, meshes))
    if cache and fresh:
        os.makedirs(cache, exist_ok=True)
        _write_meshes(os.path.join(cache, f"{os.getpid()}.npz"), fresh)
    known.update(fresh)
    return {i: known[i] for i in ids if known[i] is not None}
