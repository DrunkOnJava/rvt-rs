#!/usr/bin/env python3
"""Score the material layer sets in an rvt-rs IFC against Revit's own IFC4
export of the same file.

Research tool for `reports/element-framing/RE-58-material-layer-sets.md`.
For every element rvt-rs gives an `IfcMaterialLayerSetUsage`, it:

- compares the set's name with the `Family:Type` part of Revit's name for
  the element (RE-61);
- compares the layer materials' names with the names of the layers Revit's
  export gives the same element (its constituent or layer set), in order;
- places each layer where the usage puts it: along the usage's axis of the
  element's placement, from `OffsetFromReferenceLine`, stacking in
  `DirectionSense` by each layer's thickness;
- compares that span with the span of the faces Revit's body styles with
  the same material, for layers whose material occurs once in the element;
- compares the whole set's span with the span of Revit's whole body along
  the same axis;
- where Revit's export writes its own `IfcMaterialLayerSetUsage` (its
  Coordination View 2.0 does, its Reference View does not), places Revit's
  layers the same way and compares them with rvt-rs's, layer by layer, and
  with Revit's own body.

rvt-rs writes Revit internal coordinates in metres with an identity site;
Revit's export is in shared coordinates, moved to internal ones by its
`IfcSite` placement, heights too.

Usage:

    python3 tools/re/layer_sets_vs_ifc.py [-v] <rvt-rs.ifc> <revit-export.ifc>

`-v` prints each layer set whose names or spans differ.

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit

FEET = 0.3048
VERBOSE = False


def names_of(element):
    material = ifcopenshell.util.element.get_material(element, should_skip_usage=True)
    if material is None:
        return None
    if material.is_a("IfcMaterialConstituentSet"):
        return [k.Material.Name for k in material.MaterialConstituents]
    if material.is_a("IfcMaterialLayerSet"):
        return [layer.Material.Name if layer.Material else None for layer in material.MaterialLayers]
    if material.is_a("IfcMaterial"):
        return [material.Name]
    return None


def main():
    global VERBOSE
    if "-v" in sys.argv:
        sys.argv.remove("-v")
        VERBOSE = True
    if len(sys.argv) != 3:
        raise SystemExit(__doc__)
    ours = ifcopenshell.open(sys.argv[1])
    theirs = ifcopenshell.open(sys.argv[2])
    ours_unit = ifcopenshell.util.unit.calculate_unit_scale(ours)
    site = theirs.by_type("IfcSite")[0]
    sm = ifcopenshell.util.placement.get_local_placement(site.ObjectPlacement)
    t_unit = ifcopenshell.util.unit.calculate_unit_scale(theirs)
    ox, oy, oz = sm[0][3] * t_unit, sm[1][3] * t_unit, sm[2][3] * t_unit
    c, s = sm[0][0], sm[1][0]

    def internal(x, y, z):
        dx, dy = x - ox, y - oy
        return ((c * dx + s * dy) / FEET, (-s * dx + c * dy) / FEET, (z - oz) / FEET)

    style = {}
    for st in theirs.by_type("IfcSurfaceStyle"):
        for item in st.Styles or []:
            style[f"{item.is_a()}-{item.id()}"] = st.Name
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    by_tag = {}
    for element in theirs.by_type("IfcElement"):
        # An opening carries the Tag of the element that cuts it.
        if element.Tag and not element.is_a("IfcOpeningElement"):
            by_tag.setdefault(element.Tag, element)
    stats = collections.Counter()
    errors = []
    whole_errors = []
    usage_errors = []
    revit_self_errors = []
    for element in ours.by_type("IfcElement"):
        usage = ifcopenshell.util.element.get_material(element)
        if usage is None or not usage.is_a("IfcMaterialLayerSetUsage"):
            continue
        stats["layer sets written"] += 1
        layers = usage.ForLayerSet.MaterialLayers
        our_names = [layer.Material.Name if layer.Material else None for layer in layers]
        revit = by_tag.get(element.Tag)
        if revit is None:
            stats["not in Revit's export"] += 1
            continue
        revit_names = names_of(revit)
        family_type = ":".join((revit.Name or "").split(":")[:2])
        if family_type.count(":") == 1:
            same = usage.ForLayerSet.LayerSetName == family_type
            stats["set name = Revit's Family:Type" if same else "set name differs from Revit's Family:Type"] += 1
        if revit_names and len(revit_names) == len(our_names):
            for a, b in zip(our_names, revit_names):
                if a is None:
                    stats["layer names: none written"] += 1
                elif a == b:
                    stats["layer names: Revit's, in Revit's order"] += 1
                else:
                    stats["layer names: differ from Revit's"] += 1
                    if VERBOSE:
                        print(f"    name {element.Tag}: {our_names} vs {revit_names}")
        else:
            stats["layer sets whose count differs from Revit's"] += 1
            if VERBOSE:
                print(f"    count {element.Tag}: {our_names} vs {revit_names}")
        # Place the layers.
        m = ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement)
        axis_index = {"AXIS1": 0, "AXIS2": 1, "AXIS3": 2}[usage.LayerSetDirection]
        axis = [m[0][axis_index], m[1][axis_index], m[2][axis_index]]
        origin = [m[0][3] * ours_unit / FEET, m[1][3] * ours_unit / FEET, m[2][3] * ours_unit / FEET]
        sense = 1.0 if usage.DirectionSense == "POSITIVE" else -1.0
        at = usage.OffsetFromReferenceLine * ours_unit / FEET
        base = sum(o * a for o, a in zip(origin, axis))
        spans = []
        for layer in layers:
            t = layer.LayerThickness * ours_unit / FEET
            lo, hi = sorted((base + at, base + at + sense * t))
            spans.append((layer.Material.Name if layer.Material else None, lo, hi))
            at += sense * t
        revit_usage_at = None
        revit_usage = ifcopenshell.util.element.get_material(revit)
        if revit_usage is not None and revit_usage.is_a("IfcMaterialLayerSetUsage"):
            rm = ifcopenshell.util.placement.get_local_placement(revit.ObjectPlacement)
            r_index = {"AXIS1": 0, "AXIS2": 1, "AXIS3": 2}[revit_usage.LayerSetDirection]
            vx, vy, vz = rm[0][r_index], rm[1][r_index], rm[2][r_index]
            r_axis = (c * vx + s * vy, -s * vx + c * vy, vz)
            r_origin = internal(rm[0][3] * t_unit, rm[1][3] * t_unit, rm[2][3] * t_unit)
            along = sum(a * b for a, b in zip(r_axis, axis))
            r_base = sum(o * a for o, a in zip(r_origin, axis))
            r_sense = 1.0 if revit_usage.DirectionSense == "POSITIVE" else -1.0
            r_at = revit_usage.OffsetFromReferenceLine * t_unit / FEET
            revit_layers = []
            for layer in revit_usage.ForLayerSet.MaterialLayers:
                t = layer.LayerThickness * t_unit / FEET
                ends = sorted((r_base + along * r_at, r_base + along * (r_at + r_sense * t)))
                revit_layers.append((layer.Material.Name if layer.Material else None, *ends))
                r_at += r_sense * t
            if abs(abs(along) - 1.0) > 1e-6 or len(revit_layers) != len(spans):
                stats["Revit's usage: another axis or layer count"] += 1
            else:
                off = 0.0
                for (n1, lo1, hi1), (n2, lo2, hi2) in zip(spans, revit_layers):
                    off = max(off, abs(lo1 - lo2), abs(hi1 - hi2))
                    if n1 is None:
                        stats["Revit's usage: layer names not written"] += 1
                    elif n1 == n2:
                        stats["Revit's usage: layer names equal"] += 1
                    else:
                        stats["Revit's usage: layer names differ"] += 1
                usage_errors.append((off, element.Tag, element.is_a()))
                revit_usage_at = (
                    min(lo for _, lo, _ in revit_layers),
                    max(hi for _, _, hi in revit_layers),
                )
                if VERBOSE and off > 0.001:
                    ours_span = [(n, round(lo, 4), round(hi, 4)) for n, lo, hi in spans]
                    theirs_span = [(n, round(lo, 4), round(hi, 4)) for n, lo, hi in revit_layers]
                    print(
                        f"    usage {element.Tag}: ours {ours_span} Revit's {theirs_span}"
                        f" ({revit_usage.LayerSetDirection} {revit_usage.DirectionSense}"
                        f" {revit_usage.OffsetFromReferenceLine})"
                    )
        try:
            shape = ifcopenshell.geom.create_shape(settings, revit)
        except Exception:
            stats["IfcOpenShell cannot mesh Revit's element"] += 1
            continue
        g = shape.geometry
        verts, faces, ids = g.verts, g.faces, g.material_ids
        mats = [style.get(x.name, x.name) for x in g.materials]
        revit_span = collections.defaultdict(lambda: [float("inf"), float("-inf")])
        for tri in range(len(faces) // 3):
            mi = ids[tri] if tri < len(ids) else -1
            if not 0 <= mi < len(mats):
                continue
            for k in range(3):
                v = faces[3 * tri + k]
                p = internal(verts[3 * v] * 1.0, verts[3 * v + 1] * 1.0, verts[3 * v + 2] * 1.0)
                d = sum(pp * a for pp, a in zip(p, axis))
                span = revit_span[mats[mi]]
                span[0], span[1] = min(span[0], d), max(span[1], d)
        whole = [float("inf"), float("-inf")]
        for lo, hi in revit_span.values():
            whole[0], whole[1] = min(whole[0], lo), max(whole[1], hi)
        if whole[0] <= whole[1]:
            lo = min(lo for _, lo, _ in spans)
            hi = max(hi for _, _, hi in spans)
            off = max(abs(whole[0] - lo), abs(whole[1] - hi))
            whole_errors.append((off, element.Tag, element.is_a()))
            if revit_usage_at is not None:
                off = max(abs(whole[0] - revit_usage_at[0]), abs(whole[1] - revit_usage_at[1]))
                revit_self_errors.append((off, element.Tag, element.is_a()))
        counts = collections.Counter(n for n, _, _ in spans)
        worst, compared = 0.0, 0
        for name, lo, hi in spans:
            if not name or counts[name] > 1 or name not in revit_span or len(revit_span) < 2:
                continue
            rlo, rhi = revit_span[name]
            worst = max(worst, abs(rlo - lo), abs(rhi - hi))
            compared += 1
        if compared:
            stats["positions compared"] += 1
            errors.append((worst, element.Tag, element.is_a()))
            if VERBOSE and worst > 0.001:
                ours_span = [(n, round(lo, 4), round(hi, 4)) for n, lo, hi in spans]
                theirs_span = {n: (round(a, 4), round(b, 4)) for n, (a, b) in revit_span.items()}
                print(f"    span {element.Tag}: ours {ours_span} Revit's {theirs_span}")
        else:
            stats["positions not comparable"] += 1
    for what, count in sorted(stats.items()):
        if count:
            print(f"  {count:>5}  {what}")
    for label, found in (
        ("layer spans", errors),
        ("whole layer sets", whole_errors),
        ("layers placed by Revit's own usage", usage_errors),
        ("Revit's own usage against Revit's own body: sets", revit_self_errors),
    ):
        es = sorted(e for e, _, _ in found)
        if es:
            for t in (0.001, 0.01, 0.1):
                print(f"  {label} within {t} ft of Revit's: {sum(1 for e in es if e <= t)} of {len(es)}")
            worst = [(round(float(e), 4), tag, kind) for e, tag, kind in sorted(found)[-5:]]
            print(f"  worst {worst}")


if __name__ == "__main__":
    main()
