#!/usr/bin/env python3
"""Which rule picks the piece whose area Revit writes as a ceiling's GrossArea (#1209).

Reads `probes/examples/probe_1209_ceiling_piece_lines.rs`'s output (each
ceiling whose sketch is several loops: every piece's plan area, with and
without its holes, and the piece four rules pick: the one holding the first
and the last curve of the Sketch's stored list, and the one holding the
lowest and the highest sketch line ElementId) and Revit's own IFC export. For
each ceiling Revit writes as an `IfcCovering` with a
`Qto_CoveringBaseQuantities.GrossArea`, it says which pieces' areas are that
value and whether each rule's piece is one of them. A rule is Revit's only
where it holds on every ceiling.

Prints counts and, for each ceiling a rule misses, its ElementId and the
pieces (a research lister on Autodesk and BIM4LCA files, not on held-out
ones):

    ceiling pieces: N scored; GrossArea is one piece's area (net N, gross N)
    rule first_listed: N of M; last_listed ...; lowest_id ...; highest_id ...

Usage:

    python3 tools/re/ceiling_piece_lines_vs_ifc.py <probe output> <revit-export.ifc>
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.unit

SQFT_TO_SQM = 0.09290304
# Two areas are one where they differ by less than this, relative (Revit's GrossArea is written to a few digits).
AREA_RELATIVE = 1e-3
RULES = ("first_listed", "last_listed", "lowest_id", "highest_id")


def gross_area(product):
    for rel in getattr(product, "IsDefinedBy", []) or []:
        if not rel.is_a("IfcRelDefinesByProperties"):
            continue
        qset = rel.RelatingPropertyDefinition
        if qset.is_a("IfcElementQuantity") and qset.Name == "Qto_CoveringBaseQuantities":
            for q in qset.Quantities:
                if q.is_a("IfcQuantityArea") and q.Name == "GrossArea":
                    return q.AreaValue
    return None


def main(argv):
    argv = [arg for arg in argv if not arg.startswith("--")]
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ceilings = {}
    with open(argv[1], encoding="utf-8") as probe:
        for line in probe:
            if line.startswith("json "):
                row = json.loads(line[5:])
                ceilings[str(row["ceiling"])] = row
            elif line.startswith("summary "):
                print("probe " + line.strip())
    revit = ifcopenshell.open(argv[2])
    area_unit = ifcopenshell.util.unit.calculate_unit_scale(revit, "AREAUNIT")
    tally = collections.Counter()
    picked = collections.Counter()
    misses = []
    for covering in revit.by_type("IfcCovering"):
        row = ceilings.get(covering.Tag)
        if row is None:
            continue
        value = gross_area(covering)
        if value is None:
            tally["no GrossArea"] += 1
            continue
        value *= area_unit
        tally["scored"] += 1
        hits = {}
        for kind in ("areas_sqft", "gross_sqft"):
            hits[kind] = [
                i for i, a in enumerate(row[kind]) if abs(a * SQFT_TO_SQM - value) <= AREA_RELATIVE * value
            ]
            if hits[kind]:
                tally[f"one piece's {kind}"] += 1
        revit_pieces = set(hits["areas_sqft"]) | set(hits["gross_sqft"])
        for rule in RULES:
            if row[rule] is None:
                picked[f"{rule} none"] += 1
            elif row[rule] in revit_pieces:
                picked[rule] += 1
        missed = [rule for rule in RULES if row[rule] not in revit_pieces]
        if missed:
            misses.append(
                f"  {covering.Tag}: Revit {value:.3f} m2 = piece {sorted(revit_pieces)}; "
                + ", ".join(f"{rule} {row[rule]}" for rule in RULES)
                + "; pieces m2 "
                + " ".join(f"{a * SQFT_TO_SQM:.3f}" for a in row["areas_sqft"])
            )
    scored = tally["scored"]
    net = tally["one piece's areas_sqft"]
    gross = tally["one piece's gross_sqft"]
    print(
        f"ceiling pieces: {scored} scored; GrossArea is one piece's area: net {net}, "
        f"gross {gross}; no GrossArea {tally['no GrossArea']}"
    )
    print(
        "rule "
        + "; ".join(f"{rule}: {picked[rule]} of {scored} (no piece {picked[rule + ' none']})" for rule in RULES)
    )
    if misses:
        print("ceilings a rule misses:")
        print("\n".join(misses))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
