"""Helpers the tools/re scorers share, one copy of each so they cannot drift."""

import concurrent.futures
import concurrent.futures.process
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
    """Shared metres (IfcOpenShell world) -> Revit internal feet."""
    site = f.by_type("IfcSite")[0]
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


def _mesh_one(part_id):
    try:
        shape = ifcopenshell.geom.create_shape(_MESHING["settings"], _MESHING["model"].by_id(part_id))
    except RuntimeError:
        return None
    return (
        np.array(shape.geometry.verts, dtype=float).reshape(-1, 3),
        np.array(shape.geometry.faces, dtype=int).reshape(-1, 3),
    )


def mesh_parts(model, settings, parts):
    """Instance id -> (vertices (N, 3), faces (M, 3)) for each of `parts` that
    IfcOpenShell meshes, one `create_shape` per part spread over the CPUs. The
    triangles are those `create_shape` gives, in the same order (the
    iterator's are the same, but it took twice the CPU time per element in
    a measurement on the BIM4LCA office, and no less wall time with 12 threads
    than with 4); a part it cannot mesh is left out."""
    ids = list(dict.fromkeys(part.id() for part in parts))
    cpus = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count() or 1
    workers = min(cpus, len(ids))
    _MESHING.update(model=model, settings=settings)
    meshes = None
    if workers > 1:
        # A forked worker flushes the buffered output it inherits when it ends.
        sys.stdout.flush()
        sys.stderr.flush()
        try:
            with concurrent.futures.ProcessPoolExecutor(workers, mp_context=multiprocessing.get_context("fork")) as pool:
                meshes = list(pool.map(_mesh_one, ids, chunksize=1))
        except concurrent.futures.process.BrokenProcessPool:
            meshes = None
    if meshes is None:
        meshes = [_mesh_one(i) for i in ids]
    _MESHING.clear()
    return {i: mesh for i, mesh in zip(ids, meshes) if mesh is not None}
