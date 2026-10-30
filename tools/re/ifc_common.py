"""Helpers the tools/re scorers share, one copy of each so they cannot drift."""

import re

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
