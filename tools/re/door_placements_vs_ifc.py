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
import ifcopenshell.util.element
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

import door_flips_vs_ifc as flips
from ifc_common import site_frame

STATES = ("00", "01", "10", "11", "unread")
BINDINGS = ("bound to a plain wall", "bound to none", "no stored transform")
KINDS = ("door", "window")
VERDICTS = ("same", "half turn", "quarter turn", "other")
UP = np.array([0.0, 0.0, 1.0])
RECORD_SET = "RvtElementRecordGeometry"
ORIGIN_BANDS = ((0.01, "within 0.01 ft"), (0.5, "within 0.5 ft"), (3.0, "within 3 ft"))


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
            }
        )
        compared.append(r)
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

    if not held_out:
        keys = ("id", "kind", "binding", "st", "verdict", "rule", "ours_x", "theirs_x", "ours_vs_stored", "origin", "voids", "theirs_voids", "ours_axes", "theirs_axes")
        for r in compared:
            print("row " + json.dumps({k: r[k] for k in keys}, sort_keys=True))


if __name__ == "__main__":
    main(sys.argv)
