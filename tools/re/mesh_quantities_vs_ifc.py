#!/usr/bin/env python3
"""Whether the volume and the surface area of a body rvt-rs draws from a saved
mesh are the quantities Revit's export writes (#1046, W7).

Revit writes `NetVolume`, `GrossVolume`, `NetSurfaceArea` and
`GrossSurfaceArea` from the element's solid or its computed volume parameter
(revit-ifc's `PropertyUtil`, CLEANROOM.md source 5), and rvt-rs draws an
element whose body would be its record's bounding box from the mesh Revit
saved for it, which is that solid tessellated (#1191). This takes the
`IfcTriangulatedFaceSet` bodies of the objects rvt-rs writes, works their
volume and surface area out with IfcOpenShell, and for each object both files
hold under one key counts, by entity and quantity, how many of Revit's values
the candidate equals (a relative 1e-5, or 1e-6 where that is larger, as
`qtos_vs_ifc.py` scores), how many it does not, and how many of those rvt-rs
writes already. Each line names the quantity's candidate (the mesh's volume
or area); a quantity the candidate is not is not listed. It scores nothing and
changes nothing.

It prints, per entity and quantity:

  <entity> <set>.<quantity>: Revit writes N on an object drawn from a mesh;
      candidate equal E, different D; rvt-rs writes it on W (equal to Revit's
      on V); none written, candidate equal on K of M

then up to 8 differences of each. Usage:

    python3 tools/re/mesh_quantities_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

Needs IfcOpenShell and numpy (the versions tools/ci/requirements-ifc.txt pins).
"""

import collections
import multiprocessing
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import numpy as np

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qtos_vs_ifc  # noqa: E402
from psets_vs_ifc import property_key_of  # noqa: E402

CANDIDATE = {
    "NetVolume": "volume",
    "GrossVolume": "volume",
    "NetSurfaceArea": "area",
    "GrossSurfaceArea": "area",
}


def mesh_bodies(f):
    """The products whose Body is an IfcTriangulatedFaceSet, by key."""
    out = {}
    for product in f.by_type("IfcProduct"):
        rep = product.Representation
        if rep is None:
            continue
        items = [
            item
            for representation in rep.Representations or ()
            if representation.RepresentationIdentifier == "Body"
            for item in representation.Items or ()
        ]
        if items and all(item.is_a("IfcTriangulatedFaceSet") for item in items):
            key = property_key_of(product)
            if key:
                out[key] = product
    return out


def measure(f, products):
    """key -> {"volume": cubic metres, "area": square metres} of each product's body."""
    settings = ifcopenshell.geom.settings()
    by_id = {product.id(): key for key, product in products.items()}
    out = {}
    iterator = ifcopenshell.geom.iterator(settings, f, multiprocessing.cpu_count(), include=list(products.values()))
    if not iterator.initialize():
        return out
    while True:
        shape = iterator.get()
        key = by_id.get(shape.id)
        verts = np.asarray(shape.geometry.verts, dtype=float).reshape(-1, 3)
        faces = np.asarray(shape.geometry.faces, dtype=int).reshape(-1, 3)
        if key is not None and len(faces):
            tri = verts[faces]
            centre = tri.reshape(-1, 3).mean(axis=0)
            a, b, c = tri[:, 0] - centre, tri[:, 1] - centre, tri[:, 2] - centre
            volume = abs(float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum())) / 6
            area = float(np.linalg.norm(np.cross(b - a, c - a), axis=1).sum() / 2)
            out[key] = {"volume": volume, "area": area}
        if not iterator.next():
            break
    return out


def main(argv):
    args = [a for a in argv[1:] if not a.startswith("--")]
    if len(args) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    ours_path, theirs_path = args
    ours_file = ifcopenshell.open(ours_path)
    ours, _, our_entities, _, _, _ = qtos_vs_ifc.quantities(ours_path)
    theirs, _, their_entities, _, _, _ = qtos_vs_ifc.quantities(theirs_path, our_entities)
    bodies = mesh_bodies(ours_file)
    print(f"objects rvt-rs draws from a mesh: {len(bodies)}")
    values = measure(ours_file, bodies)
    rows = collections.defaultdict(lambda: collections.Counter())
    examples = collections.defaultdict(list)
    for (key, set_name, name), (kind, revit_value) in sorted(theirs.items()):
        quantity = CANDIDATE.get(name)
        if quantity is None or key not in values or revit_value is None:
            continue
        candidate = values[key][quantity]
        entity = our_entities.get(key, "?")
        row = rows[(entity, set_name, name)]
        row["revit"] += 1
        mine = ours.get((key, set_name, name))
        equal = qtos_vs_ifc.same((kind, revit_value), (kind, candidate))
        row["candidate equal" if equal else "candidate different"] += 1
        if mine is None:
            row["none written"] += 1
            if equal:
                row["none written, candidate equal"] += 1
        else:
            row["written"] += 1
            if qtos_vs_ifc.same((kind, revit_value), mine):
                row["written equal"] += 1
        if not equal and len(examples[(entity, set_name, name)]) < 8:
            examples[(entity, set_name, name)].append((key, revit_value, candidate, None if mine is None else mine[1]))
    for (entity, set_name, name), row in sorted(rows.items()):
        print(
            f"{entity} {set_name}.{name}: Revit writes {row['revit']} on an object drawn from a mesh; "
            f"candidate equal {row['candidate equal']}, different {row['candidate different']}; "
            f"rvt-rs writes it on {row['written']} (equal to Revit's on {row['written equal']}); "
            f"none written, candidate equal on {row['none written, candidate equal']} of {row['none written']}"
        )
    for (entity, set_name, name), items in sorted(examples.items()):
        for key, revit_value, candidate, mine in items:
            written = "none" if mine is None else f"{mine:.6g}"
            print(f"  different {entity} {set_name}.{name} {key}: Revit {revit_value:.6g}, mesh {candidate:.6g}, rvt-rs {written}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
