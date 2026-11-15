#!/usr/bin/env python3
"""Whether a door's or window's placement axes in Revit's export follow the
flips of the instance that created its IfcDoorType or IfcWindowType (#1494).

Research tool. RE-347's rule takes Revit's placement X as the stored X,
reversed where the instance's own hand and facing flips differ; it holds on
every door and window bound to a plain wall (1,061 of 1,061) and is not applied
to the doors bound to none, 49 of which it would invert, uniformly by
IfcDoorType. revit-ifc (CLEANROOM.md source 5) builds a door type's shared
geometry once, from the first instance it exports with that type key, under a
transform made from that instance's flips (`FamilyInstanceExporter`,
`DoorWindowInfo`), and places every later instance of the type at its own
transform times the inverse of that one (`typeInfo.StyleTransform`). The type
key holds `FlippedSymbol`, which only a door with a real wall host can set.

So the hypothesis: within one Revit type entity, every instance's X is the
stored X reversed exactly where the creating instance's flips differ. This
scorer reads, for every door and window of `probe_re343_door_flips` whose stored
transform it read and whose Revit X is the stored X or its reverse, whether Revit
reverses it (`p`), and tests four predictors of `p` by binding:

- `own`: the instance's own flips differ (RE-347's rule);
- `uniform`: one value of `p` for every instance of the type entity (counted
  per type entity, not a predictor of any one instance);
- `first in file`: the flips of the instance of the type that comes first in
  Revit's file (the lowest STEP id), taken as the one exported first;
- `least id`: the flips of the instance of the type with the least ElementId.

It prints count lines beginning `door creator:` (numbers beside fixed words),
then, unless the model is held out, `row` lines (one JSON object per element).

Usage:

    python3 tools/re/door_type_creator_vs_ifc.py <probe output> <revit-export.ifc>
"""

import collections
import json
import os
import sys

import ifcopenshell

import door_flips_vs_ifc as flips
from door_placements_vs_ifc import binding_of, state_of, tagged

BINDINGS = ("bound to a plain wall", "bound to none")


def main(argv):
    # Measure adds `--list`, which this scorer has no use for.
    positional = [a for a in argv[1:] if not a.startswith("--")]
    if len(positional) != 2:
        sys.exit(__doc__)
    probe, theirs_path = positional
    held_out = any(word in os.path.abspath(probe) for word in flips.HELD_OUT)
    _, _, items, walls = flips.read_probe(probe)
    theirs = ifcopenshell.open(theirs_path)
    to_internal = flips.site_rotation(theirs).T
    by_tag = tagged(theirs)
    names = flips.flag_fields(items)
    rows, _ = flips.paired_rows(items, by_tag, walls, names, to_internal)

    kept = []
    for r in rows:
        st = state_of(r)
        if not r["read"] or st == "unread" or r["x_rel"] not in ("same", "reverse"):
            continue
        r["binding"] = binding_of(r)
        r["st"] = st
        r["p"] = r["x_rel"] == "reverse"
        r["parity"] = st in ("01", "10")
        r["order"] = by_tag[str(r["id"])].id()
        kept.append(r)

    types = collections.defaultdict(list)
    for r in kept:
        if r["ftype"] is not None:
            types[(r["kind"], r["ftype"])].append(r)
    for members in types.values():
        first = min(members, key=lambda r: r["order"])
        least = min(members, key=lambda r: r["id"])
        values = {r["p"] for r in members}
        for r in members:
            r["first"] = first["parity"]
            r["least"] = least["parity"]
            r["uniform"] = len(values) == 1
            r["type_size"] = len(members)
            r["type_bindings"] = sorted({m["binding"] for m in members})

    print(
        f"door creator: elements with a stored X Revit keeps or reverses {len(kept)}; "
        f"of the probe's {len(items)}; in type entities {sum(1 for r in kept if 'first' in r)}"
    )
    for kind in ("door", "window"):
        for binding in BINDINGS:
            mine = [r for r in kept if r["kind"] == kind and r["binding"] == binding and "first" in r]
            if not mine:
                print(f"door creator: {kind}s {binding}: 0")
                continue
            own = sum(r["p"] == r["parity"] for r in mine)
            first = sum(r["p"] == r["first"] for r in mine)
            least = sum(r["p"] == r["least"] for r in mine)
            uniform = sum(r["uniform"] for r in mine)
            print(
                f"door creator: {kind}s {binding}: n={len(mine)}; reversed {sum(r['p'] for r in mine)}; "
                f"own flips predict {own}; first in file predicts {first}; least id predicts {least}; "
                f"in a type entity with one value {uniform}"
            )
        of_kind = [(k, v) for k, v in types.items() if k[0] == kind]
        mixed = [members for _, members in of_kind if len({r["p"] for r in members}) > 1]
        shared = [members for _, members in of_kind if len({r["binding"] for r in members}) > 1]
        print(
            f"door creator: {kind} type entities {len(of_kind)}; with one value {len(of_kind) - len(mixed)}; "
            f"mixed {len(mixed)}; holding both bindings {len(shared)}"
        )
        for binding in BINDINGS:
            sole = [m for _, m in of_kind if {r["binding"] for r in m} == {binding}]
            print(
                f"door creator: {kind} type entities whose instances are all {binding} {len(sole)}; "
                f"with one value {sum(1 for m in sole if len({r['p'] for r in m}) == 1)}"
            )

    if not held_out:
        keys = ("id", "kind", "ftype", "symbol", "binding", "st", "p", "parity", "order", "first", "least", "uniform", "type_size", "type_bindings")
        for r in kept:
            print("row " + json.dumps({k: r.get(k) for k in keys}, sort_keys=True))


if __name__ == "__main__":
    main(sys.argv)
