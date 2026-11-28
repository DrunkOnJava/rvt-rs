#!/usr/bin/env python3
"""#2112: do a door's, window's or lamp's saved parts name a see-through material where Revit's
export draws the element with a see-through surface style?

Reads `probe_2112_render_styles`'s `element` lines (each element's ElementId and the names of
the materials its visible parts' render styles are) and Revit's IFC export of the same file. On
Revit's side an element's styles are the `IfcSurfaceStyle`s its representation's items carry,
mapped items followed; a style is see-through where its rendering or shading has a
transparency above 0. Revit names a surface style after its material, so on rvt-rs's side a
part's material is see-through where Revit's style of that name is, and unknown where Revit's
export has no style of that name.

The rows (#2112, registered before the first run):
1. Of the elements Revit's export draws with both a see-through and an opaque style, rvt-rs's
   parts name both kinds on at least 95 per cent.
2. Of those Revit draws with one kind only, rvt-rs's parts name that kind alone on at least
   95 per cent.
3. Per part, not per element (#2112's own question): of the elements Revit draws with both
   kinds, the see-through area of rvt-rs's parts (the summed triangle area of the parts whose
   material Revit draws see-through) is within 10 per cent of the see-through area of Revit's
   body (its triangles meshed by IfcOpenShell, grouped by the surface style each carries) on at
   least 90 per cent. Area in square feet on both sides.

Prints counts only, lines starting `render styles:`.

Usage:
    python3 tools/re/render_styles_vs_ifc.py probe.txt REVIT.ifc [--list]
"""

import collections
import re
import sys

import ifcopenshell
import ifcopenshell.geom

ELEMENT = re.compile(r"^element tag=(\d+) category=(\w+) parts=\d+ materials=(.*)$")
SQUARE_FEET_PER_SQUARE_METRE = 10.763910416709722


def triangle_areas(shape):
    """Each triangle's area in square metres and the index of the style it carries."""
    verts = shape.geometry.verts
    faces = shape.geometry.faces
    ids = shape.geometry.material_ids
    out = []
    for k in range(0, len(faces), 3):
        a, b, c = (verts[3 * faces[k + j]: 3 * faces[k + j] + 3] for j in range(3))
        u = [b[i] - a[i] for i in range(3)]
        v = [c[i] - a[i] for i in range(3)]
        n = (u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0])
        out.append(((n[0] ** 2 + n[1] ** 2 + n[2] ** 2) ** 0.5 / 2.0, ids[k // 3] if k // 3 < len(ids) else -1))
    return out


def transparency(style):
    """The largest transparency of a surface style's rendering or shading, 0 where none is set."""
    out = 0.0
    for part in style.Styles or []:
        if part.is_a("IfcSurfaceStyleShading"):
            out = max(out, float(getattr(part, "Transparency", None) or 0.0))
    return out


def surface_styles(styled):
    """The surface styles an `IfcStyledItem` names, through style assignments where IFC2X3 puts them."""
    for style in styled.Styles or []:
        if style.is_a("IfcSurfaceStyle"):
            yield style
        elif style.is_a("IfcPresentationStyleAssignment"):
            for inner in style.Styles or []:
                if inner.is_a("IfcSurfaceStyle"):
                    yield inner


def items_of(representation, seen):
    for item in representation.Items or []:
        if item.id() in seen:
            continue
        seen.add(item.id())
        yield item
        if item.is_a("IfcMappedItem"):
            yield from items_of(item.MappingSource.MappedRepresentation, seen)
        if item.is_a("IfcBooleanResult"):
            for operand in (item.FirstOperand, item.SecondOperand):
                if operand is not None and operand.id() not in seen:
                    seen.add(operand.id())
                    yield operand


def main():
    # Measure appends `--list` to every probe scorer's arguments; this one lists nothing more.
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    elements = {}
    areas = collections.defaultdict(dict)
    for line in open(args[0], encoding="utf-8", errors="replace"):
        if line.startswith("area\t"):
            _, tag, name, value = line.rstrip("\n").split("\t")
            areas[tag][name] = float(value)
            continue
        match = ELEMENT.match(line.rstrip("\n"))
        if match:
            elements[match.group(1)] = (match.group(2), match.group(3).split("|") if match.group(3) else [])
    model = ifcopenshell.open(args[1])
    styles_of = collections.defaultdict(list)
    by_name = {}
    for styled in model.by_type("IfcStyledItem"):
        for style in surface_styles(styled):
            if styled.Item is not None:
                styles_of[styled.Item.id()].append(style)
            if style.Name:
                by_name[style.Name] = max(by_name.get(style.Name, 0.0), transparency(style))
    revit = {}
    for product in model.by_type("IfcProduct"):
        tag = getattr(product, "Tag", None)
        if tag not in elements or product.Representation is None:
            continue
        found = []
        seen = set()
        for representation in product.Representation.Representations or []:
            for item in items_of(representation, seen):
                found.extend(styles_of.get(item.id(), []))
        if found:
            revit[tag] = {transparency(style) > 0 for style in found}
    rows = collections.Counter()
    unmatched = collections.Counter()
    for tag, (category, materials) in elements.items():
        if tag not in revit:
            rows[(category, "no styled body in Revit's export")] += 1
            continue
        kinds = revit[tag]
        known = [by_name[name] for name in materials if name in by_name]
        unknown = [name for name in materials if name not in by_name]
        unmatched.update((category, name) for name in unknown)
        ours = {t > 0 for t in known}
        theirs = "both" if kinds == {True, False} else ("see-through" if kinds == {True} else "opaque")
        mine = "both" if ours == {True, False} else ("see-through" if ours == {True} else ("opaque" if ours == {False} else "none known"))
        rows[(category, f"Revit {theirs}, rvt-rs {mine}")] += 1
        if unknown:
            rows[(category, "elements judged without some of their materials (no style of that name in Revit's export)")] += 1
    print(f"render styles: {len(elements)} elements in the probe, {len(revit)} with a styled body in Revit's export")
    for (category, what), n in sorted(rows.items()):
        print(f"render styles:   {category} {what}: {n}")
    for (category, name), n in sorted(unmatched.items(), key=lambda kv: (-kv[1], kv[0]))[:20]:
        print(f"render styles:   unmatched {category} material {name!r} on {n} elements")
    # Row 3: per element Revit draws with both kinds, the see-through share of the area.
    settings = ifcopenshell.geom.settings()
    ratios = collections.defaultdict(list)
    shapes = []
    for product in model.by_type("IfcProduct"):
        tag = getattr(product, "Tag", None)
        if tag not in revit or revit[tag] != {True, False} or tag not in areas:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, product)
        except Exception:
            ratios[elements[tag][0]].append(None)
            continue
        shapes.append(shape)
        clear = sum(area for area, style in triangle_areas(shape)
                    if 0 <= style < len(shape.geometry.materials)
                    and float(getattr(shape.geometry.materials[style], "transparency", 0) or 0) > 0)
        theirs = clear * SQUARE_FEET_PER_SQUARE_METRE
        ours = sum(a for name, a in areas[tag].items() if by_name.get(name, 0.0) > 0)
        ratios[elements[tag][0]].append((ours, theirs))
    for category in sorted(ratios):
        pairs = ratios[category]
        unmeshed = sum(1 for p in pairs if p is None)
        pairs = [p for p in pairs if p is not None]
        close = sum(1 for ours, theirs in pairs if theirs > 0 and abs(ours - theirs) <= 0.10 * theirs)
        print(f"render styles: {category} row 3: {close} of {len(pairs)} within 10 per cent of Revit's see-through area"
              + (f"; {unmeshed} not meshed by IfcOpenShell" if unmeshed else ""))
        for ours, theirs in sorted(pairs, key=lambda p: (p[0] / p[1]) if p[1] else float("inf"))[:3] + sorted(pairs, key=lambda p: (p[0] / p[1]) if p[1] else float("inf"))[-3:]:
            print(f"render styles:   {category} see-through ft2 rvt-rs {ours:.3f} Revit {theirs:.3f}")
    for category in sorted({c for c, _ in rows}):
        both = rows[(category, "Revit both, rvt-rs both")]
        both_all = sum(n for (c, w), n in rows.items() if c == category and w.startswith("Revit both"))
        one_same = rows[(category, "Revit opaque, rvt-rs opaque")] + rows[(category, "Revit see-through, rvt-rs see-through")]
        one_all = sum(n for (c, w), n in rows.items() if c == category and (w.startswith("Revit opaque") or w.startswith("Revit see-through")))
        print(f"render styles: {category} row 1: {both} of {both_all}; row 2: {one_same} of {one_all}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
