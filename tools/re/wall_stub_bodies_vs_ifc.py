#!/usr/bin/env python3
"""List the horizontal faces of the walls that embed a curtain wall, in rvt-rs's
IFC and in Revit's, so that the end stubs Revit keeps or cuts away can be read
off (#1385, RE-335).

For each `IfcWall` with a `Tag` in `HOSTS` (the walls of the BIM4LCA office, as
built and at building permit, that name a `WallEmbedWallGStep`), it meshes the
Body in world coordinates with IfcOpenShell and prints, for rvt-rs's export and
for Revit's:

    <file> tag:<Tag> volume <ft3> box <z low> <z high> <x min> <x max> <y min> <y max>
    <file> tag:<Tag> face <up|down> z <z> area <ft2> box <x min> <x max> <y min> <y max>

one `face` line for each level at which horizontal faces of one direction lie
(rounded to 0.001 ft), with their total area and box. A wall that Revit cuts to
the strip over its curtain wall has one `down` level at the strip's lowest
point; a wall that keeps its end stubs has a `down` level at its base.

Usage:

    python3 tools/re/wall_stub_bodies_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>

The columns the hosts have cut partners with (`PARTNERS`) are listed the same way,
and so are the walls of #1067 a floor covers all but a small piece of
(`STUB_WALLS`, RE-320 section 4).

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import sys

import ifcopenshell
import ifcopenshell.geom

FOOT = 0.3048

HOSTS = {
    1429758, 2041385, 2041479, 2098938, 2103576, 2104286, 2105250, 2177158, 2177212,
    2178914, 2178961, 2180467, 2180468, 2181428, 2181501, 2293116, 2294996, 2491432,
    2495664, 2496491, 2500627, 2724158, 2964905, 3449245, 3450274, 3458395, 3458466,
    3458467, 3458528, 3458559, 1378125, 1416796, 1341937, 1348324,
}

# The office walls whose plan a floor or roof covers all but a small piece of
# (RE-320 section 4, #1067): 3004978's stub, inside the wall it abuts, Revit
# cuts with the rest of the wall; the others' pieces Revit keeps at the wall's
# full height. Listed so their bodies can be read beside their saved meshes.
STUB_WALLS = {
    3004978, 1457415, 1892033, 3123954, 1488456, 1348323, 3039425, 1630966,
    1429758, 2984235, 2984610, 2992096, 1478932, 1493966,
}

# The elements those walls have a CutPairGStep or a CleanPairGStep with, on the
# columns: their volume against the prism of their box tells which of the pair
# the other cuts.
PARTNERS = {
    1927726, 1927727, 3461614, 3461615, 1926699, 1926727, 1926731, 1928007, 1928008,
    1929253, 1929254, 2294995, 1343915,
}


def body_mesh(settings, product):
    """(vertices in feet, triangles) of the Body of `product`, or None."""
    body = next((r for r in product.Representation.Representations if r.RepresentationIdentifier == "Body"), None)
    if body is None:
        return None
    try:
        geometry = ifcopenshell.geom.create_shape(settings, product, body).geometry
    except Exception:
        return None
    verts = [v / FOOT for v in geometry.verts]
    faces = list(geometry.faces)
    if not verts or not faces:
        return None
    return verts, faces


def report(name, path):
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    f = ifcopenshell.open(path)
    for wall in list(f.by_type("IfcWall")) + list(f.by_type("IfcColumn")):
        try:
            tag = int(wall.Tag)
        except (TypeError, ValueError):
            continue
        if tag not in HOSTS | PARTNERS | STUB_WALLS or wall.Representation is None:
            continue
        mesh = body_mesh(settings, wall)
        if mesh is None:
            print(f"{name} tag:{tag} no body")
            continue
        verts, faces = mesh
        point = lambda i: (verts[3 * i], verts[3 * i + 1], verts[3 * i + 2])
        volume = 0.0
        levels = {}
        zs, xs, ys = verts[2::3], verts[0::3], verts[1::3]
        for k in range(0, len(faces), 3):
            a, b, c = point(faces[k]), point(faces[k + 1]), point(faces[k + 2])
            volume += (
                a[0] * (b[1] * c[2] - b[2] * c[1])
                - a[1] * (b[0] * c[2] - b[2] * c[0])
                + a[2] * (b[0] * c[1] - b[1] * c[0])
            ) / 6.0
            ux, uy, uz = b[0] - a[0], b[1] - a[1], b[2] - a[2]
            vx, vy, vz = c[0] - a[0], c[1] - a[1], c[2] - a[2]
            nx, ny, nz = uy * vz - uz * vy, uz * vx - ux * vz, ux * vy - uy * vx
            norm = (nx * nx + ny * ny + nz * nz) ** 0.5
            if norm < 1e-12 or abs(nz) / norm < 0.999:
                continue
            key = ("up" if nz > 0 else "down", round(a[2], 3))
            entry = levels.setdefault(key, [0.0, 1e18, -1e18, 1e18, -1e18])
            entry[0] += norm / 2.0
            for p in (a, b, c):
                entry[1] = min(entry[1], p[0])
                entry[2] = max(entry[2], p[0])
                entry[3] = min(entry[3], p[1])
                entry[4] = max(entry[4], p[1])
        print(
            f"{name} tag:{tag} volume {abs(volume):.4f} box "
            f"{min(zs):.4f} {max(zs):.4f} {min(xs):.4f} {max(xs):.4f} {min(ys):.4f} {max(ys):.4f}"
        )
        for (direction, z), (area, x0, x1, y0, y1) in sorted(levels.items(), key=lambda kv: (kv[0][1], kv[0][0])):
            print(f"{name} tag:{tag} face {direction} z {z:.3f} area {area:.4f} box {x0:.4f} {x1:.4f} {y0:.4f} {y1:.4f}")


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    report("rvt-rs", argv[1])
    report("revit", argv[2])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
