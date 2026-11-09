#!/usr/bin/env python3
"""The doors rvt-rs binds to no host wall, against Revit's export: which are
the doors Revit gives no opening, where Revit places their origin, and what
decides the axes it gives them (RE-361, #1494).

Research tool for `reports/element-framing/RE-361-doors-with-no-opening.md`.
For every door and window of `probe_re343_door_flips` whose stored transform
the probe read and whose ElementId is the Tag of an `IfcDoor` or `IfcWindow` in
both exports, it prints count lines beginning `door flips: unwalled`:

- `opening`: how many have an opening in rvt-rs's export (a host it voids) and
  in Revit's, and how many in one only, both ways (a mismatch is a door the
  condition would select wrongly or miss);
- `origin`: for the doors rvt-rs writes no opening for, and for those Revit
  gives none, how many have Revit's placement origin at the stored origin
  (`InstanceTransform.origin`, RE-87) within 0.001 ft and 0.01 ft; how many
  carry `RvtElementRecordGeometry.PlacementOriginSource`, the ones rvt-rs
  places at the stored origin; and how far rvt-rs's own origin is from Revit's
  on the doors that carry it and those that do not;
- `host`: the class Revit's export gives the element the door's stored host
  (`m_hostId`, from the probe's `stored_host`) names, by whether either export
  gives the door an opening;
- `axes`: for the doors rvt-rs writes no opening for, Revit's X axis against
  the stored one by the two flips, the type entities' reversal (whether every
  door of an `IfcDoorType` has Revit's axes reversed from the stored ones or
  none), and where the stored host is a wall the probe read (its native
  record's straight line), Revit's X against that wall's direction by the two
  flips, and how many of three rules agree with Revit's axes: reversed where the
  flips differ (RE-343 Rule 1), reversed where `m_flippedY` is set, and reversed
  where `m_flippedX` is set, each taken against the wall's direction.

It prints counts only, never an id or a name, in a restricted alphabet, so a
held-out model's lines can leave the runner through
tools/ci/scrub-holdout-results.sh (the `door flips:` lines); unless the model
is held out it then prints `row` lines (one JSON object per door) for offline
reading.

rvt-rs's IFC is `model.ifc` in the probe output's directory, which Measure
keeps only for a run with `keep_ifc=true`; `--ours` names another file.

Usage:

    python3 tools/re/door_unwalled_vs_ifc.py <probe output> <revit-export.ifc> [--ours rvt-rs.ifc] [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins) and NumPy.
"""

import collections
import json
import os
import sys

import ifcopenshell
import ifcopenshell.util.element
import numpy as np

import door_flips_vs_ifc as flips
from door_placements_vs_ifc import has_host, placement, state_of, tagged

RECORD_SET = "RvtElementRecordGeometry"
ORIGIN_MARKER = "PlacementOriginSource"
BANDS = (0.001, 0.01)
ALONG = 0.999
KINDS = ("door", "window")


def arguments(argv):
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


def origin_marker(element):
    return ifcopenshell.util.element.get_psets(element).get(RECORD_SET, {}).get(ORIGIN_MARKER) is not None


def within(values, tolerance):
    return sum(1 for v in values if v <= tolerance)


def bands(values):
    return ", ".join(f"{t} ft {within(values, t)}" for t in BANDS)


def plan_direction(wall):
    start, end = np.array(wall["start"], dtype=float), np.array(wall["end"], dtype=float)
    return flips.unit(end - start)


def along(a, direction):
    """same, reverse, across or other: how a plan axis lies against a direction."""
    a = flips.unit(np.array(a, dtype=float)[:2])
    dot = float(a @ direction)
    if dot >= ALONG:
        return "same"
    if dot <= -ALONG:
        return "reverse"
    if abs(dot) <= 1e-3:
        return "across"
    return "other"


def main(argv):
    probe, theirs_path, ours_path = arguments(argv)
    probe_path = os.path.abspath(probe)
    held_out = any(word in probe_path for word in flips.HELD_OUT)
    if ours_path is None:
        ours_path = os.path.join(os.path.dirname(probe_path), "model.ifc")
    if not os.path.isfile(ours_path):
        print("door flips: unwalled: the rvt-rs IFC is not beside the probe output (dispatch with keep_ifc=true), nothing compared")
        raise SystemExit(1)

    model, _, items, walls = flips.read_probe(probe)
    theirs = ifcopenshell.open(theirs_path)
    ours = ifcopenshell.open(ours_path)
    to_internal_theirs = flips.site_rotation(theirs).T
    to_internal_ours = flips.site_rotation(ours).T
    theirs_by_tag, ours_by_tag = tagged(theirs), tagged(ours)
    by_any_tag = {}
    for element in theirs.by_type("IfcElement"):
        if getattr(element, "Tag", None) is not None:
            by_any_tag.setdefault(str(element.Tag), element)
    names = flips.flag_fields(items)
    rows, _ = flips.paired_rows(items, theirs_by_tag, walls, names, to_internal_theirs)

    compared = []
    for r in rows:
        tag = str(r["id"])
        item = items[tag]
        ours_element, theirs_element = ours_by_tag.get(tag), theirs_by_tag[tag]
        if ours_element is None or not item.get("read"):
            continue
        stored_axes = [np.array(a, dtype=float) for a in item["read"]["axes"]]
        stored_origin = np.array(item["read"]["origin"], dtype=float)
        _, ours_origin = placement(ours_element, to_internal_ours, ours)
        theirs_axes, theirs_origin = placement(theirs_element, to_internal_theirs, theirs)
        host_id = item.get("stored_host")
        host = by_any_tag.get(str(host_id)) if host_id is not None else None
        host_class = "none" if host_id is None else (host.is_a() if host is not None else "not in the export")
        wall = walls.get(str(host_id)) if host_id is not None else None
        direction = plan_direction(wall) if wall else None
        state = state_of(r)
        hand, facing = r["state"].get("m_flippedX"), r["state"].get("m_flippedY")
        r.update(
            {
                "tag": tag,
                "ours_voids": has_host(ours_element),
                "theirs_voids": has_host(theirs_element),
                "marked": origin_marker(ours_element),
                "state": state,
                "unequal": None if state == "unread" else state in ("01", "10"),
                "hand": hand,
                "facing": facing,
                "d_theirs": float(np.linalg.norm(theirs_origin - stored_origin)),
                "d_ours": float(np.linalg.norm(ours_origin - stored_origin)),
                "ours_to_theirs": float(np.linalg.norm(ours_origin - theirs_origin)),
                "host_class": host_class,
                "wall": direction,
                "x_to_stored": flips.relation(theirs_axes[0], stored_axes[0]),
                "x_to_wall": along(theirs_axes[0], direction) if direction is not None else None,
                "stored_x_to_wall": along(stored_axes[0], direction) if direction is not None else None,
                "y_to_wall_left": (
                    along(theirs_axes[1], np.array([-direction[1], direction[0]])) if direction is not None else None
                ),
                "theirs_x": [round(float(v), 6) for v in theirs_axes[0]],
                "stored_x": [round(float(v), 6) for v in stored_axes[0]],
            }
        )
        compared.append(r)

    print(f"door flips: unwalled: release {model.get('release', 'unknown')}; elements with a stored transform in both exports {len(compared)}")
    for kind in KINDS:
        mine = [r for r in compared if r["kind"] == kind]
        if not mine:
            continue
        both = sum(1 for r in mine if r["ours_voids"] and r["theirs_voids"])
        neither = sum(1 for r in mine if not r["ours_voids"] and not r["theirs_voids"])
        ours_only = sum(1 for r in mine if r["ours_voids"] and not r["theirs_voids"])
        theirs_only = sum(1 for r in mine if not r["ours_voids"] and r["theirs_voids"])
        print(
            f"door flips: unwalled opening {kind}s: checked {len(mine)}; an opening in both exports {both}; in neither {neither}; "
            f"in rvt-rs only {ours_only}; in Revit only {theirs_only}"
        )
    doors = [r for r in compared if r["kind"] == "door"]
    for label, chosen in (
        ("rvt-rs writes no opening", [r for r in doors if not r["ours_voids"]]),
        ("Revit gives no opening", [r for r in doors if not r["theirs_voids"]]),
        ("an opening in both", [r for r in doors if r["ours_voids"] and r["theirs_voids"]]),
    ):
        print(
            f"door flips: unwalled origin doors where {label}: {len(chosen)}; Revit origin is the stored origin within "
            + bands([r["d_theirs"] for r in chosen])
            + "; carry the placement origin marker "
            + str(sum(r["marked"] for r in chosen))
            + "; rvt-rs origin is Revit origin within "
            + bands([r["ours_to_theirs"] for r in chosen])
            + "; rvt-rs origin is the stored origin within "
            + bands([r["d_ours"] for r in chosen])
        )
    marked = [r for r in doors if r["marked"]]
    wrong = [r for r in marked if r["ours_to_theirs"] > BANDS[0]]
    print(
        f"door flips: unwalled origin rule: written for {len(marked)} of {len([r for r in doors if not r['ours_voids']])} doors with no opening in rvt-rs export; "
        f"differ from Revit origin by more than {BANDS[0]} ft {len(wrong)}; written where Revit gives an opening {len([r for r in marked if r['theirs_voids']])}"
    )
    windows = [r for r in compared if r["kind"] == "window"]
    print(
        f"door flips: unwalled origin windows: {len(windows)}; with no opening in rvt-rs export {len([r for r in windows if not r['ours_voids']])}; "
        f"carry the placement origin marker {sum(r['marked'] for r in windows)}"
    )

    classes = collections.Counter(
        (r["host_class"], "rvt-rs opening" if r["ours_voids"] else "no rvt-rs opening", "Revit opening" if r["theirs_voids"] else "no Revit opening")
        for r in doors
    )
    for (host_class, ours_label, theirs_label), n in sorted(classes.items()):
        print(f"door flips: unwalled host doors whose stored host is {host_class}: {ours_label}, {theirs_label}: n={n}")

    hostless = [r for r in doors if not r["ours_voids"]]
    by_flips = collections.Counter((r["state"], r["x_to_stored"]) for r in hostless)
    for (state, relation), n in sorted(by_flips.items()):
        print(f"door flips: unwalled axes doors with no rvt-rs opening state {state}: Revit X against the stored X is {relation}: n={n}")
    wrong_rule1 = sum(1 for r in hostless if r["unequal"] is not None and (r["x_to_stored"] == "reverse") != r["unequal"])
    print(f"door flips: unwalled axes doors with no rvt-rs opening {len(hostless)}: Revit X is the stored X reversed where the flips differ: wrong {wrong_rule1}")
    types = collections.defaultdict(list)
    for r in hostless:
        types[r["ftype"]].append(r)
    uniform = sum(1 for g in types.values() if len({x["x_to_stored"] for x in g}) == 1)
    mixed = sum(1 for g in types.values() if len({x["unequal"] for x in g}) > 1)
    creator = collections.Counter()
    for group in types.values():
        group.sort(key=lambda x: x["id"])
        reversed_ = group[0]["x_to_stored"] == "reverse"
        creator["a door whose flips differ exactly where the type is reversed"] += any(x["unequal"] == reversed_ for x in group)
        creator["the lowest ElementId door"] += group[0]["unequal"] == reversed_
        creator["the highest ElementId door"] += group[-1]["unequal"] == reversed_
    print(
        f"door flips: unwalled axes type entities {len(types)}: every door with its axes reversed from the stored ones, or none, in {uniform}; "
        f"holding doors whose flips differ and doors whose flips do not {mixed}; "
        + "; ".join(f"{k} {v}" for k, v in sorted(creator.items()))
    )

    walled = [r for r in hostless if r["wall"] is not None]
    print(f"door flips: unwalled axes doors with no rvt-rs opening whose stored host is a wall the probe read: {len(walled)} of {len(hostless)}")
    table = collections.Counter(
        (
            "hand" if r["hand"] else "no hand",
            "facing" if r["facing"] else "no facing",
            "stored X " + r["stored_x_to_wall"],
            "Revit X " + r["x_to_wall"],
            "Revit Y against the wall left " + r["y_to_wall_left"],
        )
        for r in walled
    )
    for key, n in sorted(table.items()):
        print("door flips: unwalled axes wall table: " + ", ".join(key) + f": n={n}")
    tested = [r for r in walled if r["x_to_wall"] in ("same", "reverse") and r["facing"] is not None and r["hand"] is not None]
    for label, flip in (
        ("reversed where m_flippedY is set", lambda r: bool(r["facing"])),
        ("reversed where m_flippedX is set", lambda r: bool(r["hand"])),
        ("reversed where the flips differ", lambda r: bool(r["hand"]) != bool(r["facing"])),
        ("never reversed", lambda r: False),
    ):
        agree = sum(1 for r in tested if (r["x_to_wall"] == "reverse") == flip(r))
        print(f"door flips: unwalled axes rule: Revit X is the wall direction {label}: agrees {agree} of {len(tested)} doors along the wall")
    print(f"door flips: unwalled axes: doors whose stored host is a wall and whose Revit X is not along it: {len(walled) - len(tested)}")

    if not held_out:
        for r in compared:
            keys = ("tag", "kind", "ftype", "state", "ours_voids", "theirs_voids", "marked", "d_theirs", "d_ours", "ours_to_theirs", "host_class", "x_to_stored", "x_to_wall", "stored_x_to_wall", "y_to_wall_left", "theirs_x", "stored_x")
            row = {k: (round(r[k], 6) if isinstance(r[k], float) else r[k]) for k in keys}
            row["stored_host"] = items[r["tag"]].get("stored_host")
            row["wall"] = [round(float(v), 6) for v in r["wall"]] if r["wall"] is not None else None
            print("row " + json.dumps(row, sort_keys=True))


if __name__ == "__main__":
    main(sys.argv)
