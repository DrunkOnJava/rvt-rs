#!/usr/bin/env python3
"""Where the Length rvt-rs wrote on a column or a member is longer than the mesh it drew (RE-444).

For each rvt-rs IFC export given, take every `IfcColumn` and `IfcMember` whose `Body` is a face set (a body
drawn from the mesh Revit saved) and that holds a `Qto_*BaseQuantities.Length`, and set the Length against the
extent of the mesh:

    a column   the vertical range of its mesh (the element's own frame has Z up);
    a member   the extent along the principal axis of its mesh, for an elongated mesh only (the principal extent
               at least three times the second: a stub's principal axis is its section's).

A rigid placement keeps lengths, so the face set's own coordinates are used. Revit's `Length` is the extent of
its body, which the saved mesh is (RE-418, RE-424), and a record box can overrun the solid and cannot fall
short of it, so the elements counted are the ones whose written Length is longer than the mesh by more than
2 mm (`longer`), shorter by more than 2 mm (`shorter`), or within 2 mm (`within`); the mullions and columns that
are `longer` are named with both lengths.

Every summary line starts `length vs mesh extent:` and holds counts beside fixed words; the lines that name an
ElementId start with `  longer` and are dropped from a held-out scrub.

Usage:

    python3 tools/re/length_vs_mesh_extent.py <rvt-rs.ifc> [<rvt-rs.ifc> ...]

Needs IfcOpenShell and numpy.
"""

import collections
import sys

import ifcopenshell
import numpy as np

TOLERANCE = 0.002  # metres: a written Length within this of the mesh's extent agrees with it
ELONGATED = 3.0  # a member's mesh is read along its principal axis only if that extent is this many times the next


def body_points(element):
    """The corner coordinates of the element's `Body` face set, or None where it is not drawn as one."""
    representation = element.Representation
    if representation is None:
        return None
    for body in representation.Representations:
        if body.RepresentationIdentifier != "Body":
            continue
        for item in body.Items:
            if item.is_a("IfcTriangulatedFaceSet") or item.is_a("IfcPolygonalFaceSet"):
                return np.array(item.Coordinates.CoordList, dtype=float)
    return None


def written_length(element):
    """The element's `Length` quantity, or None."""
    for relation in element.IsDefinedBy or []:
        definition = getattr(relation, "RelatingPropertyDefinition", None)
        if definition is not None and definition.is_a("IfcElementQuantity"):
            for quantity in definition.Quantities:
                if quantity.Name == "Length" and hasattr(quantity, "LengthValue"):
                    return quantity.LengthValue
    return None


def mesh_extent(ifc_class, points):
    """The extent of the mesh along the element's axis, or None where it is not read."""
    if ifc_class == "IfcColumn":
        return float(points[:, 2].max() - points[:, 2].min())
    centred = points - points.mean(axis=0)
    _, _, axes = np.linalg.svd(centred, full_matrices=False)
    ranges = sorted(
        (float((centred @ axes[i]).max() - (centred @ axes[i]).min()) for i in range(3)),
        reverse=True,
    )
    return ranges[0] if ranges[0] >= ELONGATED * ranges[1] else None


def score(path):
    model = ifcopenshell.open(path)
    print(f"length vs mesh extent: {path}")
    for ifc_class in ("IfcColumn", "IfcMember"):
        tally = collections.Counter()
        longer = []
        for element in model.by_type(ifc_class):
            length = written_length(element)
            points = body_points(element)
            if length is None or points is None or len(points) < 4:
                tally["not scored (no Length or no face-set body)"] += 1
                continue
            extent = mesh_extent(ifc_class, points)
            if extent is None:
                tally["not scored (member not elongated)"] += 1
                continue
            difference = length - extent
            if difference > TOLERANCE:
                tally["longer"] += 1
                longer.append((str(element.Tag), length, extent))
            elif difference < -TOLERANCE:
                tally["shorter"] += 1
            else:
                tally["within"] += 1
        counts = ", ".join(f"{name} {count}" for name, count in sorted(tally.items()))
        print(f"length vs mesh extent: {ifc_class}: {counts or 'none'}")
        for tag, length, extent in sorted(longer):
            print(f"  longer {ifc_class} {tag}: Length {length:.6f} extent {extent:.6f} ({length - extent:+.6f})")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    for ifc_path in sys.argv[1:]:
        score(ifc_path)
