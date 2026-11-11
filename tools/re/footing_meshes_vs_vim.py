#!/usr/bin/env python3
"""Score the saved meshes rvt-rs draws for an element class against the VIM export of the model.

Research tool for `reports/element-framing/RE-422-ruled-surface-roofs.md` (#1683).
Autodesk's Snowdon Towers Sample Structural has no Revit IFC4 export, so the footings that take a saved mesh
have no scorer in Measure. The VIM export of the model (vimaec/vim-hackathon, the 2027 edition of the sample;
its content is Autodesk's, so it stays local) holds the same elements' geometry as Revit's exporter wrote it.
For each element of the class (its Tag is the ElementId) whose `BodySource` is `saved_graphics_mesh` in the IFC:
the height of its mesh and the volume of its mesh, against the VIM's meshes for that element.

Usage:

    python3 tools/re/footing_meshes_vs_vim.py <rvt-rs.ifc> <model.vim> [--base <other.ifc>] [--class IfcFooting] [--list N]

With `--base`, the elements are split into those that were drawn from a saved mesh in the other export too and
those that were not (the ones a change newly draws). The VIM is a BFAST container; the reader below is the
minimum needed. ElementIds repeat across the VIM's six documents (the Architectural document holds an element
with the id of a footing), so only the document whose title contains `--document` (default `Structural`)
counts. The VIM's meshes are coarser than rvt-rs's at the fine chord bar, so a cylinder's volume reads a little
higher in rvt-rs's (+0.55% for a 32-sided prism); heights and bottoms are exact.

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and NumPy. Prints counts and percentages
only, for the model's own element ids; no content of the VIM is written out.
"""

import argparse
import struct
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element as el
import numpy as np

FEET = 0.3048


def bfast(buf, base=0):
    _magic, _start, _end, n = struct.unpack_from("<QQQQ", buf, base)
    ranges = [struct.unpack_from("<QQ", buf, base + 32 + 16 * i) for i in range(n)]
    names = buf[base + ranges[0][0] : base + ranges[0][1]].split(b"\0")
    return {names[i - 1].decode(): (base + ranges[i][0], base + ranges[i][1]) for i in range(1, n)}


def load_vim(path, document):
    data = open(path, "rb").read()
    top = bfast(data)
    entities = bfast(data, top["entities"][0])

    def column(table, col, dtype):
        a, b = bfast(data, entities[table][0])[col]
        return np.frombuffer(data, dtype=dtype, count=(b - a) // np.dtype(dtype).itemsize, offset=a)

    a, b = top["strings"]
    strings = data[a:b].split(b"\0")
    titles = [strings[i].decode() for i in column("Vim.BimDocument", "string:Title", "<i4")]
    wanted = [i for i, t in enumerate(titles) if document in t]
    if len(wanted) != 1:
        sys.exit(f"the VIM has {len(wanted)} documents titled like {document!r}: {titles}")

    geometry = bfast(data, top["geometry"][0])

    def attr(name, dtype):
        a, b = geometry[name]
        return np.frombuffer(data, dtype=dtype, count=(b - a) // np.dtype(dtype).itemsize, offset=a)

    return dict(
        ids=column("Vim.Element", "long:Id", "<i8"),
        element_doc=column("Vim.Element", "index:Vim.BimDocument:BimDocument", "<i4"),
        document=wanted[0],
        node_element=column("Vim.Node", "index:Vim.Element:Element", "<i4"),
        pos=attr("g3d:vertex:position:0:float32:3", "<f4").reshape(-1, 3),
        idx=attr("g3d:corner:index:0:int32:1", "<i4"),
        inst_tf=attr("g3d:instance:transform:0:float32:16", "<f4").reshape(-1, 16),
        inst_mesh=attr("g3d:instance:mesh:0:int32:1", "<i4"),
        mesh_sm=attr("g3d:mesh:submeshoffset:0:int32:1", "<i4"),
        sm_idx=attr("g3d:submesh:indexoffset:0:int32:1", "<i4"),
    )


def mesh_triangles(v, mesh):
    first = v["mesh_sm"][mesh]
    last = v["mesh_sm"][mesh + 1] if mesh + 1 < len(v["mesh_sm"]) else len(v["sm_idx"])
    start = v["sm_idx"][first]
    end = v["sm_idx"][last] if last < len(v["sm_idx"]) else len(v["idx"])
    return v["idx"][start:end].reshape(-1, 3)


def vim_triangles(v, element_id):
    """The element's triangles in the VIM's frame, in metres, as an (n, 3, 3) array, or None if it has none."""
    rows = np.where((v["ids"] == element_id) & (v["element_doc"] == v["document"]))[0]
    out = []
    for node in np.where(np.isin(v["node_element"], rows))[0]:
        mesh = v["inst_mesh"][node]
        if mesh < 0:
            continue
        triangles = mesh_triangles(v, mesh)
        if len(triangles) == 0:
            continue
        points = v["pos"][triangles.reshape(-1)].astype(np.float64)
        transform = v["inst_tf"][node].astype(np.float64).reshape(4, 4)
        world = np.hstack([points, np.ones((len(points), 1))]) @ transform
        out.append(world[:, :3].reshape(-1, 3, 3) * FEET)
    return np.concatenate(out) if out else None


def volume(triangles):
    a, b, c = triangles[:, 0], triangles[:, 1], triangles[:, 2]
    return abs(float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6.0))


def body_source(product):
    for props in el.get_psets(product).values():
        if "BodySource" in props:
            return str(props["BodySource"])
    return ""


def ifc_triangles(product, settings):
    shape = ifcopenshell.geom.create_shape(settings, product)
    verts = np.array(shape.geometry.verts).reshape(-1, 3)
    return verts[np.array(shape.geometry.faces).reshape(-1, 3)]


def report(label, tags, products, vim, settings, listed):
    rows, missing = [], 0
    for tag in tags:
        theirs = vim_triangles(vim, int(tag))
        if theirs is None:
            missing += 1
            continue
        ours = ifc_triangles(products[tag], settings)
        height = float(ours[..., 2].max() - ours[..., 2].min()) - float(theirs[..., 2].max() - theirs[..., 2].min())
        bottom = float(ours[..., 2].min() - theirs[..., 2].min())
        ratio = volume(ours) / volume(theirs) - 1 if volume(theirs) else float("nan")
        rows.append((tag, height, ratio, bottom))
    print(f"{label}: {len(tags)} elements, scored {len(rows)}, not in the VIM {missing}")
    if not rows:
        return
    heights = np.array([r[1] for r in rows])
    ratios = np.array([r[2] for r in rows])
    bottoms = np.array([r[3] for r in rows])
    print(
        f"  height, rvt-rs minus the VIM's: within 0.001 m {int((np.abs(heights) <= 1e-3).sum())} of {len(rows)}, "
        f"largest {np.abs(heights).max():.5f} m"
    )
    print(
        f"  lowest point, rvt-rs minus the VIM's: within 0.001 m {int((np.abs(bottoms) <= 1e-3).sum())} of {len(rows)}, "
        f"largest {np.abs(bottoms).max():.5f} m"
    )
    print(
        f"  volume, rvt-rs over the VIM's: within 0.1% {int((np.abs(ratios) <= 1e-3).sum())}, "
        f"within 1% {int((np.abs(ratios) <= 1e-2).sum())} of {len(rows)}, largest {np.nanmax(np.abs(ratios)) * 100:.4f}%"
    )
    for tag, height, ratio, _bottom in sorted(rows, key=lambda r: -abs(r[1]))[:listed]:
        print(f"    tag {tag}: height {height:+.5f} m, volume {ratio * 100:+.4f}%")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("ifc")
    parser.add_argument("vim")
    parser.add_argument("--base", help="another export of the model, to tell the elements newly drawn from a mesh")
    parser.add_argument("--class", dest="cls", default="IfcFooting")
    parser.add_argument("--document", default="Structural")
    parser.add_argument("--list", type=int, default=0, help="print the N elements whose heights differ most")
    args = parser.parse_args()

    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    model = ifcopenshell.open(args.ifc)
    products = {p.Tag: p for p in model.by_type(args.cls) if p.Tag}
    drawn = [t for t in sorted(products) if body_source(products[t]) == "saved_graphics_mesh"]
    vim = load_vim(args.vim, args.document)
    if args.base:
        base_model = ifcopenshell.open(args.base)  # kept: its products read the file
        base = {p.Tag: p for p in base_model.by_type(args.cls) if p.Tag}
        newly = [t for t in drawn if t in base and body_source(base[t]) != "saved_graphics_mesh"]
        before = [t for t in drawn if t in base and body_source(base[t]) == "saved_graphics_mesh"]
        report(f"{args.cls}, newly drawn from a saved mesh", newly, products, vim, settings, args.list)
        report(f"{args.cls}, drawn from a saved mesh in both", before, products, vim, settings, args.list)
    else:
        report(f"{args.cls}, drawn from a saved mesh", drawn, products, vim, settings, args.list)


if __name__ == "__main__":
    main()
