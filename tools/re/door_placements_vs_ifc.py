#!/usr/bin/env python3
"""Compare the local axes rvt-rs writes for each door and window with Revit's
own, by Tag, and tabulate them by hand flip and facing flip and by how the
element is bound to its wall (RE-347, #1447).

Research tool for `reports/element-framing/RE-347-door-window-placement-axes.md`.
RE-343 measured Revit's `IfcLocalPlacement` axes against the axes the instance
stores; this measures what rvt-rs's own export writes for the same elements.
For every door and window of the probe `probe_re343_door_flips` whose
ElementId is the Tag of an IfcDoor or IfcWindow in both exports, it takes the
X, Y and Z axes of each file's placement (the chain of `IfcLocalPlacement`s
composed), each moved into Revit's internal frame through its own file's
IfcSite placement (rotation only; rvt-rs writes the site at the identity), and
says how rvt-rs's axes compare with Revit's:

- `same`: all three axes the same;
- `half turn`: X and Y reversed, Z the same (a half turn about the vertical);
- `quarter turn`: X and Y a right angle from Revit's, Z the same;
- `other`: anything else, a tilt included.

The state of an element is `m_flippedX m_flippedY` of its native record
(`10` is hand flipped and not facing flipped, `unread` has no record). An
element is `bound to a plain wall` where the probe read its stored transform
and the wall rvt-rs binds it to has a native record whose curve is read (the
definition of RE-343 Rule 1), `bound to none` where it read the transform and
no such wall, and `no stored transform` where it read none (a Revit 2023 or
older project). `rvt-rs voids a wall` counts the elements rvt-rs's own export
fills a wall opening for. `rule` lines count the elements whose rvt-rs export
carries `RvtElementRecordGeometry.PlacementAxesSource`, the ones placed as
Revit's export places them (the rule of RE-347), and how they compare with
Revit's: all of them should be `same`.

RE-357 (#1465) adds the origin. For every element whose stored transform the
probe read, `d` is Revit's placement origin less the stored origin
(`InstanceTransform.origin`), taken in the stored axes and in Revit's own. The
`origin` lines say how many elements have the stored origin as Revit's; how
many `IfcDoorType` or `IfcWindowType` entities, symbols, and symbols with a flip
state hold one `d`; the signs of `d` by state; and, per axis, how many `d` equal
the corner of the opening each export gives the element (the lowest corner in
Revit's axes about the stored origin where the flips are equal, the negated
highest x and y with the lowest z where they differ). An open model also
prints the offsets by type and the largest distances.

It prints count lines beginning `door flips:` (counts only, never an id or a
name, in a restricted alphabet, so a held-out model's lines can leave the
runner through tools/ci/scrub-holdout-results.sh), then, unless the model is
held out, `row` lines (one JSON object per element) for offline reading.

rvt-rs's IFC is `model.ifc` in the probe output's directory, which Measure
keeps only for a run with `keep_ifc=true`; `--ours` names another file.

Usage:

    python3 tools/re/door_placements_vs_ifc.py <probe output> <revit-export.ifc> [--ours rvt-rs.ifc] [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and NumPy.
"""

import collections
import json
import os
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

import door_flips_vs_ifc as flips
from ifc_common import FEET, mesh_parts, site_frame

STATES = ("00", "01", "10", "11", "unread")
BINDINGS = ("bound to a plain wall", "bound to none", "no stored transform")
KINDS = ("door", "window")
VERDICTS = ("same", "half turn", "quarter turn", "other")
UP = np.array([0.0, 0.0, 1.0])
RECORD_SET = "RvtElementRecordGeometry"
ORIGIN_BANDS = ((0.01, "within 0.01 ft"), (0.5, "within 0.5 ft"), (3.0, "within 3 ft"))
STORED_BANDS = (0.001, 0.01, 0.5)
CORNER_BANDS = (0.001, 0.01, 0.05)
ZERO = 0.001
TYPE_OFFSETS_SHOWN = 6


def tagged(f):
    out = {}
    for cls in ("IfcDoor", "IfcWindow"):
        for element in f.by_type(cls):
            if element.Tag is not None and element.ObjectPlacement is not None:
                out.setdefault(str(element.Tag), element)
    return out


def placement(element, to_internal, f):
    """(X, Y, Z axes in the internal frame, origin in internal feet)."""
    matrix = np.array(ifcopenshell.util.placement.get_local_placement(element.ObjectPlacement), dtype=float)
    axes = [to_internal @ matrix[:3, i] for i in range(3)]
    unit = ifcopenshell.util.unit.calculate_unit_scale(f)
    origin = site_frame(f)(np.array([matrix[:3, 3] * unit]))[0]
    return axes, origin


def direction(v):
    """+X, -X, +Y, -Y, turned (upright, off the axes) or tilted."""
    v = flips.unit(v)
    if abs(v[2]) > 1e-3:
        return "tilted"
    for name, d in (("+X", (1, 0)), ("-X", (-1, 0)), ("+Y", (0, 1)), ("-Y", (0, -1))):
        if v[0] * d[0] + v[1] * d[1] > 1 - flips.ANGLE:
            return name
    return "turned"


def verdict(ours, theirs):
    rel = [flips.relation(a, b) for a, b in zip(ours, theirs)]
    if rel == ["same", "same", "same"]:
        return "same"
    if rel == ["reverse", "reverse", "same"]:
        return "half turn"
    if rel == ["right angle", "right angle", "same"]:
        return "quarter turn"
    return "other"


def state_of(row):
    x, y = row["state"].get("m_flippedX"), row["state"].get("m_flippedY")
    if x is None or y is None:
        return "unread"
    return f"{int(bool(x))}{int(bool(y))}"


def binding_of(row):
    if not row["read"]:
        return "no stored transform"
    return "bound to a plain wall" if row["rvt_x_wall"] in ("+", "-") else "bound to none"


def placement_source(element):
    """The `PlacementAxesSource` rvt-rs wrote on the element (RE-347), or None."""
    value = ifcopenshell.util.element.get_psets(element).get(RECORD_SET, {}).get("PlacementAxesSource")
    return str(value) if value else None


def has_host(element):
    return flips.host_wall(element) is not None


def band(distance):
    for limit, label in ORIGIN_BANDS:
        if distance <= limit:
            return label
    return "farther"


def arguments(argv):
    """(probe output, Revit export, rvt-rs export or None)."""
    positional = []
    ours = None
    rest = iter(argv[1:])
    for a in rest:
        if a == "--ours":
            ours = next(rest, None)
        elif not a.startswith("--"):
            positional.append(a)
    if len(positional) != 2 or ("--ours" in argv and ours is None):
        raise SystemExit(__doc__)
    return positional[0], positional[1], ours


def tally(cell):
    counts = collections.Counter(r["verdict"] for r in cell)
    return "; ".join(f"{v} {counts[v]}" for v in VERDICTS)


def opening_of(element):
    for rel in getattr(element, "FillsVoids", None) or ():
        return rel.RelatingOpeningElement
    return None


def opening_vertices(f, elements):
    """Tag -> the vertices of the opening each element fills, in internal feet
    (world axes of the file's own frame), for the openings IfcOpenShell meshes."""
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    openings = {tag: opening_of(element) for tag, element in elements.items()}
    meshes = mesh_parts(f, settings, [o for o in openings.values() if o is not None and o.Representation is not None])
    frame = site_frame(f)
    return {tag: frame(meshes[o.id()][0]) for tag, o in openings.items() if o is not None and o.id() in meshes}


def box_in(vertices, origin, axes):
    """(lowest, highest) corner of `vertices` in the frame of `axes` about `origin`."""
    if vertices is None or len(vertices) == 0:
        return None
    local = (vertices - origin) @ np.array(axes).T
    return local.min(axis=0), local.max(axis=0)


def sign(v):
    return "+" if v > ZERO else "-" if v < -ZERO else "0"


def signs(vector):
    return "(" + ",".join(sign(v) for v in vector) + ")"


def feet(vector):
    return "(" + ", ".join(f"{v:+.3f}" for v in vector) + ")"


def plain(value):
    """`value` as JSON: arrays and tuples of arrays as rounded lists."""
    if isinstance(value, np.ndarray):
        return [round(float(v), 6) for v in value]
    if isinstance(value, (tuple, list)):
        return [plain(v) for v in value]
    if isinstance(value, float):
        return round(value, 6)
    return value


def within(values, tolerance):
    return sum(1 for v in values if v <= tolerance)


def unequal_flips(r):
    """True where the element's two flips differ (its placement is the stored
    axes reversed, RE-343), False where they are equal, None where unread."""
    return None if r["st"] == "unread" else r["st"] in ("01", "10")


def lowest_corner(box, unequal):
    """Where the origin sits if it is the opening's corner in the stored
    frame, as coordinates in Revit's placement frame: the box's lowest corner
    where the flips are equal, and the negated highest x and y with the
    lowest z where they differ (RE-357)."""
    if box is None or unequal is None:
        return None
    lo, hi = box
    return np.array([-hi[0], -hi[1], lo[2]]) if unequal else np.array(lo)


def origin_fields(r, element, stored, theirs_origin, ours_origin, theirs_axes, theirs_box, ours_box, to_feet):
    """The offset of Revit's origin from the stored origin, in the stored
    axes and in Revit's own, and how far each axis of it is from the corner of
    each export's opening (the corner in the stored frame, about the stored
    origin, as coordinates in Revit's frame)."""
    stored_axes, stored_origin = stored
    d = theirs_origin - stored_origin
    d_revit = np.array([d @ a for a in theirs_axes])
    own = ours_origin - stored_origin
    unequal = unequal_flips(r)

    def gaps(box):
        corner = lowest_corner(box, unequal)
        return np.abs(d_revit - corner) if corner is not None else None

    r.update(
        {
            "d_stored": np.array([d @ a for a in stored_axes]),
            "d_revit": d_revit,
            "d_dist": float(np.linalg.norm(d)),
            "own_revit": np.array([own @ a for a in theirs_axes]),
            "theirs_box": theirs_box,
            "ours_box": ours_box,
            "gaps_theirs": gaps(theirs_box),
            "gaps_ours": gaps(ours_box),
            "type_name": getattr(ifcopenshell.util.element.get_type(element), "Name", None),
            "theirs_width": round(element.OverallWidth * to_feet, 6) if getattr(element, "OverallWidth", None) else None,
            "theirs_height": round(element.OverallHeight * to_feet, 6) if getattr(element, "OverallHeight", None) else None,
        }
    )


def one_offset(group, field):
    return len({tuple(np.round(r[field], 2)) for r in group}) == 1


def constancy(read, label, held_out):
    """How many groups of elements (an IfcDoorType or IfcWindowType entity, a
    symbol, a symbol and its flip state) hold one offset of Revit's origin
    from the stored origin, in the stored axes and in Revit's own (RE-357)."""
    keys = {
        "type entity": lambda r: r["ftype"],
        "symbol": lambda r: r["symbol"],
        "symbol and state": lambda r: (r["symbol"], r["st"]),
    }
    for name, key in keys.items():
        groups = collections.defaultdict(list)
        for r in read:
            groups[key(r)].append(r)
        many = [g for g in groups.values() if len(g) > 1]
        parts = []
        for axes_name, field in (("stored", "d_stored"), ("Revit", "d_revit")):
            parts.append(
                f"one offset in the {axes_name} axes {sum(one_offset(g, field) for g in groups.values())} of {len(groups)}, "
                f"of those with more than one element {sum(one_offset(g, field) for g in many)} of {len(many)}"
            )
        print(f"{label}: by {name}: groups {len(groups)}; " + "; ".join(parts))
    bottoms = collections.defaultdict(set)
    for r in read:
        if r["theirs_box"] is not None:
            bottoms[r["ftype"]].add(round(float(r["theirs_box"][0][2]), 2))
    print(f"{label}: type entities with an opening {len(bottoms)}; holding more than one opening bottom height {sum(1 for v in bottoms.values() if len(v) > 1)}")


def axis_lines(label, name, subset, key):
    """Per axis, how many of `subset` have Revit origin within each band of
    the corner (the gaps are None where the flips are unread)."""
    have = [r for r in subset if r[key] is not None]
    if not subset:
        print(f"{label}: with {name} 0")
        return
    if not have:
        print(f"{label}: with {name} {len(subset)}: flips unread on all")
        return
    for axis, letter in enumerate("xyz"):
        values = [float(r[key][axis]) for r in have]
        print(
            f"{label}: with {name} {len(have)} of {len(subset)}: {letter} of Revit origin is the {letter} of its corner within "
            + ", ".join(f"{t} ft {within(values, t)}" for t in CORNER_BANDS)
        )
    worst = [float(r[key].max()) for r in have]
    print(f"{label}: with {name} {len(have)}: all three axes within " + ", ".join(f"{t} ft {within(worst, t)}" for t in CORNER_BANDS))


def origin_lines(compared, held_out):
    """Counts (and, for an open model, the offsets) of Revit's placement origin
    against the stored origin and against the corner of the opening (RE-357,
    #1465). A held-out model prints counts and signs only."""
    read = [r for r in compared if "d_revit" in r]
    print(
        f"door flips: origin: stored transform read {len(read)} of {len(compared)} compared; d is Revit origin less the stored origin; "
        "the corner of an opening is its lowest corner in Revit axes about the stored origin where the flips are equal, "
        "and the negated highest x and y with the lowest z where they differ"
    )
    for kind in KINDS:
        for binding in BINDINGS[:2]:
            mine = [r for r in read if r["kind"] == kind and r["binding"] == binding]
            if not mine:
                continue
            label = f"door flips: origin {kind}s {binding}"
            dist = [r["d_dist"] for r in mine]
            print(
                f"{label}: checked {len(mine)}; Revit origin is the stored origin within "
                + ", ".join(f"{t} ft {within(dist, t)}" for t in STORED_BANDS)
                + f"; farther than {STORED_BANDS[-1]} ft {len(mine) - within(dist, STORED_BANDS[-1])}"
            )
            theirs = [r for r in mine if r["theirs_box"] is not None]
            ours = [r for r in mine if r["ours_box"] is not None]
            print(f"{label}: Revit export has a meshed opening for {len(theirs)}; rvt-rs export for {len(ours)}")
            constancy(mine, label, held_out)
            axis_lines(label, "Revit opening", theirs, "gaps_theirs")
            axis_lines(label, "rvt-rs opening", ours, "gaps_ours")
            bare = [r for r in mine if r["theirs_box"] is None]
            print(
                f"{label}: with no Revit opening {len(bare)}: Revit origin is the stored origin within "
                + ", ".join(f"{t} ft {within([r['d_dist'] for r in bare], t)}" for t in STORED_BANDS)
            )
            for st in STATES:
                cell = [r for r in mine if r["st"] == st]
                if not cell:
                    continue
                in_stored = collections.Counter(signs(r["d_stored"]) for r in cell)
                in_revit = collections.Counter(signs(r["d_revit"]) for r in cell)
                print(
                    f"{label} state {st}: n={len(cell)}; signs of d in the stored axes "
                    + ", ".join(f"{k} {v}" for k, v in sorted(in_stored.items()))
                    + "; in Revit axes "
                    + ", ".join(f"{k} {v}" for k, v in sorted(in_revit.items()))
                )
            if held_out:
                continue
            for name, key, subset in (("Revit opening", "gaps_theirs", theirs), ("rvt-rs opening", "gaps_ours", ours)):
                have = [r[key] for r in subset if r[key] is not None]
                if have:
                    worst = np.max(np.array(have), axis=0)
                    print(f"{label}: largest distance of Revit origin from the corner of the {name}, per axis x y z: {feet(worst)[1:-1]} ft over {len(have)}")
    if held_out:
        return
    types = collections.defaultdict(list)
    for r in read:
        types[(r["kind"], r["symbol"], r["type_name"], r["binding"])].append(r)
    for (kind, symbol, name, binding), mine in sorted(types.items(), key=lambda kv: (kv[0][0], -len(kv[1]), str(kv[0][2]))):
        offsets = collections.Counter(tuple(np.round(r["d_revit"], 2)) for r in mine)
        shown = sorted(offsets.items(), key=lambda kv: -kv[1])[:TYPE_OFFSETS_SHOWN]
        print(
            f"door flips: origin by type: {kind} {name} (type {symbol}) {binding}: n={len(mine)}, type entities {len({r['ftype'] for r in mine})}; "
            "offset of Revit origin in Revit axes "
            + "; ".join(f"{feet(v)} x {k}" for v, k in shown)
            + (f"; {len(offsets) - len(shown)} more offsets" if len(offsets) > len(shown) else "")
        )
        for st in STATES:
            cell = [r for r in mine if r["st"] == st]
            stored = collections.Counter(tuple(np.round(r["d_stored"], 2)) for r in cell)
            if stored:
                print(
                    f"door flips: origin by type and state: {kind} {name} (type {symbol}) {binding} state {st}: n={len(cell)}; offset in the stored axes "
                    + "; ".join(f"{feet(v)} x {k}" for v, k in sorted(stored.items(), key=lambda kv: -kv[1])[:TYPE_OFFSETS_SHOWN])
                )


def main(argv):
    probe, theirs_path, ours_path = arguments(argv)
    probe_path = os.path.abspath(probe)
    held_out = any(word in probe_path for word in flips.HELD_OUT)
    if ours_path is None:
        ours_path = os.path.join(os.path.dirname(probe_path), "model.ifc")
    if not os.path.isfile(ours_path):
        print("door flips: placements: the rvt-rs IFC is not beside the probe output (dispatch with keep_ifc=true), nothing compared")
        raise SystemExit(1)

    model, _, items, walls = flips.read_probe(probe)
    theirs = ifcopenshell.open(theirs_path)
    ours = ifcopenshell.open(ours_path)
    to_internal_theirs = flips.site_rotation(theirs).T
    to_internal_ours = flips.site_rotation(ours).T
    theirs_by_tag, ours_by_tag = tagged(theirs), tagged(ours)
    names = flips.flag_fields(items)

    print(
        f"door flips: placements: release {model.get('release', 'unknown')} probe doors {model.get('doors', 0)} "
        f"windows {model.get('windows', 0)}; the Revit export has {len(theirs_by_tag)} doors and windows, the rvt-rs export {len(ours_by_tag)}"
    )
    identity = bool(np.allclose(flips.site_rotation(ours), np.eye(3)))
    print(f"door flips: placements: rvt-rs site placement rotation is the identity: {'yes' if identity else 'no'}")

    rows, unmatched = flips.paired_rows(items, theirs_by_tag, walls, names, to_internal_theirs)
    no_ours = 0
    compared = []
    for r in rows:
        tag = str(r["id"])
        ours_element, theirs_element = ours_by_tag.get(tag), theirs_by_tag[tag]
        if ours_element is None:
            no_ours += 1
            continue
        ours_axes, ours_origin = placement(ours_element, to_internal_ours, ours)
        theirs_axes, theirs_origin = placement(theirs_element, to_internal_theirs, theirs)
        stored = np.array(r["rvt_x"]) if r["rvt_x"] is not None else None
        r.update(
            {
                "binding": binding_of(r),
                "st": state_of(r),
                "verdict": verdict(ours_axes, theirs_axes),
                "ours_x": direction(ours_axes[0]),
                "theirs_x": direction(theirs_axes[0]),
                "ours_up": flips.relation(ours_axes[2], UP) == "same",
                "ours_default": flips.relation(ours_axes[0], np.array([1.0, 0.0, 0.0])) == "same"
                and flips.relation(ours_axes[2], UP) == "same",
                "ours_vs_stored": flips.relation(ours_axes[0], stored) if stored is not None else "none",
                "origin": band(float(np.linalg.norm((ours_origin - theirs_origin)[:2]))),
                "voids": has_host(ours_element),
                "rule": placement_source(ours_element) is not None,
                "theirs_voids": has_host(theirs_element),
                "inputs": all(k in r["decoded"] for k in ("m_hand_flipped", "m_facing_flipped", "m_opening_facing_x", "m_opening_facing_y")),
                "ours_class": ours_element.is_a(),
                "theirs_class": theirs_element.is_a(),
                "ours_axes": [[round(float(v), 6) for v in a] for a in ours_axes],
                "theirs_axes": [[round(float(v), 6) for v in a] for a in theirs_axes],
                "ours_origin": ours_origin,
                "theirs_origin": theirs_origin,
                "theirs_axes_vectors": theirs_axes,
            }
        )
        compared.append(r)
    theirs_to_feet = ifcopenshell.util.unit.calculate_unit_scale(theirs) / FEET
    stored_tags = [str(r["id"]) for r in compared if items[str(r["id"])].get("read")]
    theirs_openings = opening_vertices(theirs, {t: theirs_by_tag[t] for t in stored_tags})
    ours_openings = opening_vertices(ours, {t: ours_by_tag[t] for t in stored_tags})
    for r in compared:
        tag = str(r["id"])
        read = items[tag].get("read")
        if not read:
            continue
        stored_axes, stored_origin = [np.array(a) for a in read["axes"]], np.array(read["origin"])
        origin_fields(
            r,
            theirs_by_tag[tag],
            (stored_axes, stored_origin),
            r["theirs_origin"],
            r["ours_origin"],
            r["theirs_axes_vectors"],
            box_in(theirs_openings.get(tag), stored_origin, r["theirs_axes_vectors"]),
            box_in(ours_openings.get(tag), stored_origin, r["theirs_axes_vectors"]),
            theirs_to_feet,
        )
    print(
        f"door flips: placements: probe elements {len(items)}; in the Revit export {len(rows)}; "
        f"not in the Revit export {unmatched}; not in the rvt-rs export {no_ours}; compared {len(compared)}"
    )
    print(f"door flips: placements: IFC class equal to the Revit export {sum(r['ours_class'] == r['theirs_class'] for r in compared)} of {len(compared)}")

    for kind in KINDS:
        of_kind = [r for r in compared if r["kind"] == kind]
        for binding in BINDINGS:
            mine = [r for r in of_kind if r["binding"] == binding]
            print(
                f"door flips: placements {kind}s {binding}: checked {len(mine)}; {tally(mine)}; "
                f"rvt-rs axes are the default axes {sum(r['ours_default'] for r in mine)}; "
                f"decoded record carries both flips and the opening facing {sum(r['inputs'] for r in mine)}; "
                f"rvt-rs voids a wall {sum(r['voids'] for r in mine)}; Revit voids a wall {sum(r['theirs_voids'] for r in mine)}"
            )
            ruled = [r for r in mine if r["rule"]]
            print(
                f"door flips: placements {kind}s {binding} rule: rvt-rs writes Revit axes for {len(ruled)} of {len(mine)}; "
                f"of those {tally(ruled)}; the other {len(mine) - len(ruled)} {tally([r for r in mine if not r['rule']])}"
            )
            for st in STATES:
                cell = [r for r in mine if r["st"] == st]
                if not cell:
                    continue
                stored = collections.Counter(r["ours_vs_stored"] for r in cell)
                print(
                    f"door flips: placements {kind}s {binding} state {st}: n={len(cell)}; {tally(cell)}; "
                    f"default axes {sum(r['ours_default'] for r in cell)}; "
                    f"rvt-rs X against the stored X: " + ", ".join(f"{k} {v}" for k, v in sorted(stored.items()))
                )
        crossing = collections.Counter((r["theirs_x"], r["ours_x"]) for r in of_kind)
        for (theirs_x, ours_x), n in sorted(crossing.items()):
            print(f"door flips: placements {kind}s X direction: Revit {theirs_x}, rvt-rs {ours_x}: n={n}")
        origins = collections.Counter(r["origin"] for r in of_kind)
        print(
            f"door flips: placements {kind}s origin in plan against Revit: "
            + ", ".join(f"{label} {origins[label]}" for _, label in ORIGIN_BANDS)
            + f", farther {origins['farther']}"
        )
        print(f"door flips: placements {kind}s rvt-rs Z axis is up: {sum(r['ours_up'] for r in of_kind)} of {len(of_kind)}")

    print(f"door flips: placements all: checked {len(compared)}; {tally(compared)}")
    ruled = [r for r in compared if r["rule"]]
    wrong = [r for r in ruled if r["verdict"] != "same"]
    print(f"door flips: placements rule: written for {len(ruled)}; differ from Revit {len(wrong)}")
    origin_lines(compared, held_out)

    if not held_out:
        keys = ("id", "kind", "ftype", "binding", "st", "verdict", "rule", "ours_x", "theirs_x", "ours_vs_stored", "origin", "voids", "theirs_voids", "ours_axes", "theirs_axes")
        for r in compared:
            row = {k: r[k] for k in keys}
            for k in ("d_stored", "d_revit", "own_revit", "type_name"):
                if k in r:
                    row[k] = plain(r[k])
            for k in ("theirs_box", "ours_box", "gaps_theirs", "gaps_ours"):
                if r.get(k) is not None:
                    row[k] = plain(r[k])
            if "d_revit" in r:
                row["symbol"] = r["symbol"]
                row["host"] = r["host"]
                row["theirs_width"] = r["theirs_width"]
                row["theirs_height"] = r["theirs_height"]
            print("row " + json.dumps(row, sort_keys=True))


if __name__ == "__main__":
    main(sys.argv)
