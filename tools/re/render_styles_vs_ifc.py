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

Prints counts only, lines starting `render styles:`.

Usage:
    python3 tools/re/render_styles_vs_ifc.py probe.txt REVIT.ifc [--list]
"""

import collections
import re
import sys

import ifcopenshell

ELEMENT = re.compile(r"^element tag=(\d+) category=(\w+) parts=\d+ materials=(.*)$")


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
    for line in open(args[0], encoding="utf-8", errors="replace"):
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
    for tag, (category, materials) in elements.items():
        if tag not in revit:
            rows[(category, "no styled body in Revit's export")] += 1
            continue
        kinds = revit[tag]
        known = [by_name[name] for name in materials if name in by_name]
        unknown = len(materials) - len(known)
        ours = {t > 0 for t in known}
        theirs = "both" if kinds == {True, False} else ("see-through" if kinds == {True} else "opaque")
        mine = "both" if ours == {True, False} else ("see-through" if ours == {True} else ("opaque" if ours == {False} else "none known"))
        rows[(category, f"Revit {theirs}, rvt-rs {mine}")] += 1
        if unknown:
            rows[(category, "parts naming a material Revit's export has no style for")] += 1
    print(f"render styles: {len(elements)} elements in the probe, {len(revit)} with a styled body in Revit's export")
    for (category, what), n in sorted(rows.items()):
        print(f"render styles:   {category} {what}: {n}")
    for category in sorted({c for c, _ in rows}):
        both = rows[(category, "Revit both, rvt-rs both")]
        both_all = sum(n for (c, w), n in rows.items() if c == category and w.startswith("Revit both"))
        one_same = rows[(category, "Revit opaque, rvt-rs opaque")] + rows[(category, "Revit see-through, rvt-rs see-through")]
        one_all = sum(n for (c, w), n in rows.items() if c == category and (w.startswith("Revit opaque") or w.startswith("Revit see-through")))
        print(f"render styles: {category} row 1: {both} of {both_all}; row 2: {one_same} of {one_all}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
