#!/usr/bin/env python3
"""How long Revit's beam is beside the record box and the column-cut bounds (#1839, RE-122).

Reads `probes/examples/probe_1839_beam_column_ends.rs`'s output and Revit's IFC export. Each
beam of the probe is paired by `Tag` with Revit's element, which is meshed in its own placement's
frame (as `box_bodies_vs_ifc.py` does); its length is the longest side of that mesh's box. It is
set beside

    record   the record box's length along its longer plan axis (the line's ends),
    trimmed  the bounds RE-122's rule gives when each end that exactly one named column holds is
             cut back to the column's near face (`beam_column_trims`), where it gives any.

and the beams are counted by how Revit's length stands to the record's (the same within 1 mm,
shorter, longer), by whether the rule gives bounds, and by how many named columns hold each end.
For a beam Revit makes shorter than its record box and that the rule cuts: exact (the trimmed
length is Revit's within 1 mm), cut too far, or cut too little. For a beam the rule cuts and that
Revit leaves at the record: cut where Revit does not. The beams Revit shortens that the rule gives
no bounds are listed by what they name. Beams whose box is square in plan (no longer axis) are
counted apart.

The same is asked of each kind of element a beam's list can name (`column`, `wall`, `beam`), whose
bounds the probe prints as `<kind>_low` and `<kind>_high`: the near face of the one element of that
kind that holds the end, where it lies more than 1 mm inside the box. The beams Revit makes shorter
are counted by whether they name an element of the kind, whether one holds an end, whether the
kind gives them bounds and, where it does, whether the cut length is Revit's within 1 mm. The beams
Revit leaves at the record or makes longer are counted by whether the kind cuts them, which it must
not.

Every summary line starts `beam ends:` and holds counts beside fixed words; `--list` adds one line
per beam, which names an ElementId.

Usage:

    python3 tools/re/beam_ends_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell.
"""

import collections
import math
import multiprocessing
import statistics
import sys

import ifcopenshell
import ifcopenshell.geom

TOLERANCE_FEET = 0.0033  # 1 mm
METRES_FEET = 1 / 0.3048
KINDS = ("column", "wall", "beam")


def faced_length(fields, kind, low, high):
    """The beam's length with each end that exactly one element of `kind` holds cut back to that
    element's near face, or None where the kind gives it no bounds."""
    cut_low, cut_high = fields.get(f"{kind}_low", "-"), fields.get(f"{kind}_high", "-")
    if cut_low == "-" and cut_high == "-":
        return None
    start = low if cut_low == "-" else float(cut_low)
    end = high if cut_high == "-" else float(cut_high)
    return end - start if end > start else None


def cut_by(fields, kind):
    """One `--list` cell: the length `kind` cuts the beam to, and how many it names and holds."""
    length = faced_length(fields, kind, float(fields["low"]), float(fields["high"]))
    shown = "-" if length is None else f"{length:.3f}"
    named = fields.get(f"{kind}_named", "0")
    held = fields.get(f"{kind}_held", "0/0")
    return f"{kind} {shown} (names {named} holds {held})"


def verdict_of(length, theirs):
    if abs(length - theirs) <= TOLERANCE_FEET:
        return "exact"
    return "cut too far" if length < theirs else "cut too little"


def read_probe(path):
    beams = {}
    for line in open(path, encoding="utf-8", errors="replace"):
        if not line.startswith("beam "):
            continue
        fields = dict(part.split("=", 1) for part in line.split()[1:] if "=" in part)
        beams[fields["tag"]] = fields
    return beams


def lengths(model, tags):
    """Tag -> the longest side of the box of Revit's body, in the file's mesh unit, in its own frame."""
    elements = [
        e
        for e in model.by_type("IfcElement")
        if e.Tag in tags and not e.is_a("IfcOpeningElement") and e.Representation
    ]
    out = {}
    if not elements:
        return out
    iterator = ifcopenshell.geom.iterator(
        ifcopenshell.geom.settings(), model, multiprocessing.cpu_count(), include=elements
    )
    if not iterator.initialize():
        return out
    while True:
        shape = iterator.get()
        verts = shape.geometry.verts
        points = [(verts[i], verts[i + 1], verts[i + 2]) for i in range(0, len(verts), 3)]
        if points:
            extent = max(max(p[k] for p in points) - min(p[k] for p in points) for k in range(3))
            tag = model.by_id(shape.id).Tag
            out[tag] = max(out.get(tag, 0.0), extent)
        if not iterator.next():
            break
    return out


def feet_per_unit(raw, records):
    """The factor from the mesh unit to feet: the one of metres, millimetres and feet under which the
    median of record length over Revit's is nearest 1."""
    best = None
    for factor in (METRES_FEET, METRES_FEET / 1000, 1.0):
        ratios = [record / (length * factor) for tag, length in raw.items() if (record := records.get(tag)) and length > 0]
        if not ratios:
            continue
        distance = abs(math.log(statistics.median(ratios)))
        if best is None or distance < best[0]:
            best = (distance, factor)
    return best[1] if best else METRES_FEET


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    beams = read_probe(args[0])
    record_lengths = {
        tag: float(f["high"]) - float(f["low"]) for tag, f in beams.items() if f["axis"] != "-"
    }
    raw = lengths(ifcopenshell.open(args[1]), set(beams))
    factor = feet_per_unit(raw, record_lengths)
    revit = {tag: length * factor for tag, length in raw.items()}
    square = sum(1 for f in beams.values() if f["axis"] == "-")
    print(f"beam ends: {len(beams)} beams in the probe, {len(revit)} with a body in Revit's export, {square} square in plan (not scored)")

    table = collections.Counter()
    cut = collections.Counter()
    uncut = collections.Counter()
    shortened = collections.Counter()  # (kind, what) -> beams Revit makes shorter
    spared = collections.Counter()  # (kind, stands) -> beams Revit leaves or lengthens that the kind cuts
    shortfalls = []
    rows = []
    for tag, f in beams.items():
        if f["axis"] == "-" or tag not in revit:
            continue
        record = record_lengths[tag]
        theirs = revit[tag]
        stands = "same" if abs(record - theirs) <= TOLERANCE_FEET else "shorter" if theirs < record else "longer"
        trimmed = None
        if f["trim_low"] != "-" and f["trim_high"] != "-":
            trimmed = float(f["trim_high"]) - float(f["trim_low"])
        gives = "rule gives bounds" if trimmed is not None else "rule gives none"
        table[(stands, gives)] += 1
        if stands == "shorter":
            shortfalls.append(record - theirs)
        if trimmed is not None:
            cut[(stands, verdict_of(trimmed, theirs))] += 1
        else:
            uncut[(stands, f"names {f['named']} column(s), holds {f['held_low']}/{f['held_high']}")] += 1
        low, high = float(f["low"]), float(f["high"])
        for kind in KINDS:
            length = faced_length(f, kind, low, high)
            if stands == "shorter":
                held_low, held_high = (int(n) for n in f.get(f"{kind}_held", "0/0").split("/"))
                shortened[(kind, "names")] += int(f.get(f"{kind}_named", 0)) > 0
                shortened[(kind, "holds an end")] += held_low + held_high > 0
                if length is not None:
                    shortened[(kind, "gives bounds")] += 1
                    shortened[(kind, verdict_of(length, theirs))] += 1
            elif length is not None:
                spared[(kind, stands)] += 1
        rows.append((tag, f, record, theirs, trimmed, stands))

    print("beam ends: Revit's length against the record box's, and whether the rule gives bounds:")
    for (stands, gives), n in sorted(table.items()):
        print(f"beam ends:   Revit {stands:<8} {gives:<18} {n}")
    print("beam ends: the beams the rule cuts, against Revit's length (Revit's length against the record box's, verdict):")
    for (stands, verdict), n in sorted(cut.items()):
        print(f"beam ends:   Revit {stands:<8} {verdict:<14} {n}")
    print("beam ends: the beams the rule gives no bounds, by what they name (columns named, held at the low/high end):")
    for (stands, named), n in sorted(uncut.items(), key=lambda item: (item[0][0], -item[1])):
        print(f"beam ends:   Revit {stands:<8} {named:<34} {n}")
    if shortfalls:
        print(f"beam ends: the {len(shortfalls)} beams Revit makes shorter than the record box, by the kind of element their list names:")
        for kind in KINDS:
            print(
                f"beam ends:   {kind:<6} names {shortened[(kind, 'names')]}, holds an end {shortened[(kind, 'holds an end')]},"
                f" gives bounds {shortened[(kind, 'gives bounds')]}"
                f" (exact {shortened[(kind, 'exact')]}, cut too far {shortened[(kind, 'cut too far')]},"
                f" cut too little {shortened[(kind, 'cut too little')]})"
            )
    print("beam ends: the beams Revit leaves at the record box's length or makes longer, cut by a kind of element (cut where Revit does not):")
    for kind in KINDS:
        print(f"beam ends:   {kind:<6} same {spared[(kind, 'same')]}, longer {spared[(kind, 'longer')]}")
    if shortfalls:
        mm = sorted(value * 304.8 for value in shortfalls)
        print(
            f"beam ends: Revit's beam is shorter than the record box by a median {statistics.median(mm):.0f} mm"
            f" (quartiles {mm[len(mm) // 4]:.0f} and {mm[(3 * len(mm)) // 4]:.0f}, longest {mm[-1]:.0f}) on {len(mm)} beams"
        )
    if "--list" in sys.argv:
        for tag, f, record, theirs, trimmed, stands in sorted(rows, key=lambda row: (len(row[0]), row[0])):
            shown = "-" if trimmed is None else f"{trimmed:.3f}"
            faced = " ".join(cut_by(f, kind) for kind in KINDS)
            print(
                f"beam {tag} record {record:.3f} Revit {theirs:.3f} trimmed {shown} ({stands}) named {f['named']}"
                f" held {f['held_low']}/{f['held_high']} cut by {faced}"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
