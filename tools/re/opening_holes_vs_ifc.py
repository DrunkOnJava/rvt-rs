#!/usr/bin/env python3
"""Whether Revit's own body of a host is cut where rvt-rs writes an unfilled
opening (#1987).

For each unfilled opening rvt-rs writes, the points of a grid inside both the
opening's body and rvt-rs's host body with no opening taken from it (the
material the opening removes) are tested against Revit's body of the same host,
also with no opening taken from it: where Revit drew the hole into its body,
none of those points lies inside it; where it did not, all do. The share inside
Revit's body is printed for the openings Revit's export writes too and for those
it does not, by Revit's host body type, so the two can be compared.

Usage:

    python3 tools/re/opening_holes_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and NumPy.
"""

import collections
import os
import sys

import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import ifcopenshell  # noqa: E402
import ifcopenshell.geom  # noqa: E402
from ifc_common import site_frame  # noqa: E402

# Grid points per opening, at most (its box split into cells of equal size).
GRID_POINTS = 4000
# The fewest points in both bodies for an opening to be scored.
FEWEST_POINTS = 20
RAY = np.array([1.0, 0.0123, 0.0071])
RAY = RAY / np.linalg.norm(RAY)


def unfilled(model):
    out = {}
    for rel in model.by_type("IfcRelVoidsElement"):
        opening = rel.RelatedOpeningElement
        host = rel.RelatingBuildingElement
        if opening.HasFillings or not opening.Tag or not host.Tag:
            continue
        out[(opening.Tag, host.Tag)] = (opening, host)
    return out


def body_type(element):
    shape = element.Representation
    if shape is None:
        return "none"
    for rep in shape.Representations:
        if rep.RepresentationIdentifier == "Body":
            kind = rep.RepresentationType or "unset"
            if kind == "MappedRepresentation" and rep.Items:
                kind = "mapped " + (rep.Items[0].MappingSource.MappedRepresentation.RepresentationType or "unset")
            return kind
    return "no body"


class Mesher:
    def __init__(self, model):
        self.model = model
        self.to_feet = site_frame(model)
        self.settings = ifcopenshell.geom.settings()
        self.settings.set("use-world-coords", True)
        self.settings.set("disable-opening-subtractions", True)
        self.cache = {}

    def triangles(self, element):
        key = element.id()
        if key not in self.cache:
            try:
                shape = ifcopenshell.geom.create_shape(self.settings, element)
                geometry = shape.geometry
                verts = np.array(geometry.verts, dtype=float).reshape(-1, 3)
                faces = np.array(geometry.faces, dtype=int).reshape(-1, 3)
                self.cache[key] = self.to_feet(verts)[faces] if len(faces) else None
            except Exception:  # noqa: BLE001 - a body IfcOpenShell cannot mesh is reported as such
                self.cache[key] = None
        return self.cache[key]


def inside(points, tris):
    """Parity of crossings of a ray from each point with the triangles."""
    if tris is None or not len(points):
        return np.zeros(len(points), dtype=bool)
    v0, e1, e2 = tris[:, 0], tris[:, 1] - tris[:, 0], tris[:, 2] - tris[:, 0]
    h = np.cross(RAY, e2)
    a = np.einsum("ij,ij->i", e1, h)
    keep = np.abs(a) > 1e-12
    v0, e1, e2, h, a = v0[keep], e1[keep], e2[keep], h[keep], a[keep]
    f = 1.0 / a
    counts = np.zeros(len(points), dtype=int)
    for start in range(0, len(points), 256):
        p = points[start : start + 256]
        s = p[:, None, :] - v0[None, :, :]
        u = f[None, :] * np.einsum("pij,ij->pi", s, h)
        q = np.cross(s, e1[None, :, :])
        v = f[None, :] * np.einsum("j,pij->pi", RAY, q)
        t = f[None, :] * np.einsum("ij,pij->pi", e2, q)
        hit = (u >= 0) & (u <= 1) & (v >= 0) & (u + v <= 1) & (t > 1e-9)
        counts[start : start + 256] = hit.sum(axis=1)
    return counts % 2 == 1


def grid(tris):
    low, high = tris.reshape(-1, 3).min(axis=0), tris.reshape(-1, 3).max(axis=0)
    span = np.maximum(high - low, 1e-6)
    step = (np.prod(span) / GRID_POINTS) ** (1 / 3)
    axes = [np.arange(low[i] + step / 2, high[i], step) for i in range(3)]
    return np.stack(np.meshgrid(*axes, indexing="ij"), axis=-1).reshape(-1, 3)


def main():
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours = ifcopenshell.open(sys.argv[1])
    revit = ifcopenshell.open(sys.argv[2])
    ours_mesh, revit_mesh = Mesher(ours), Mesher(revit)
    revit_open = unfilled(revit)
    revit_by_tag = {}
    for element in revit.by_type("IfcElement"):
        if element.Tag and not element.is_a("IfcOpeningElement"):
            revit_by_tag.setdefault(element.Tag, element)
    rows = []
    for (tag, host_tag), (opening, host) in sorted(unfilled(ours).items()):
        revit_host = revit_by_tag.get(host_tag)
        kind = "both" if (tag, host_tag) in revit_open else "extra"
        if revit_host is None:
            rows.append((kind, "host not in Revit's export", tag, host_tag, None, 0))
            continue
        body = body_type(revit_host)
        cut = ours_mesh.triangles(opening)
        whole = ours_mesh.triangles(host)
        theirs = revit_mesh.triangles(revit_host)
        if cut is None or whole is None or theirs is None:
            rows.append((kind, body, tag, host_tag, None, 0))
            continue
        points = grid(cut)
        points = points[inside(points, cut)]
        points = points[inside(points, whole)]
        if len(points) < FEWEST_POINTS:
            rows.append((kind, body, tag, host_tag, None, len(points)))
            continue
        kept = float(inside(points, theirs).mean())
        rows.append((kind, body, tag, host_tag, kept, len(points)))
    for kind, body, tag, host_tag, kept, n in rows:
        if kind == "extra":
            shown = "not scored" if kept is None else f"{kept:.3f}"
            print(f"  extra {tag} in {host_tag}: Revit's host body {body}, share of the hole's points inside it {shown} ({n} points)")
    print("share of the hole's points inside Revit's host body, by opening and Revit's host body:")
    bands = collections.Counter()
    for kind, body, _tag, _host, kept, _n in rows:
        band = "not scored" if kept is None else ("cut (< 0.05)" if kept < 0.05 else ("whole (> 0.95)" if kept > 0.95 else "part"))
        bands[(kind, body, band)] += 1
    for (kind, body, band), count in sorted(bands.items()):
        print(f"  {count:>4}  {kind:<5} | {body} | {band}")
    for kind, body, tag, host_tag, kept, n in rows:
        if kind == "both" and kept is not None and kept > 0.05:
            print(f"  both {tag} in {host_tag}: Revit's host body {body}, share inside {kept:.3f} ({n} points)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
