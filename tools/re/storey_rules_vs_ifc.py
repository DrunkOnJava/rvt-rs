#!/usr/bin/env python3
"""Which of the Levels an element's record names is the storey Revit's export
puts it in? (#1842, #1864; RE-59, RE-175.)

Reads `probes/examples/probe_1842_storey_rules.rs`'s output and Revit's IFC
export. For every placed instance the probe lists (those whose record names two or more
Levels, and the structural framing, columns and foundations, floors and stairs
whatever it names), matched
to Revit's export by `Tag` (the Revit ElementId) and kept where Revit contains
it in a storey, it sets Revit's storey beside:

- `ours`: the storey rvt-rs's export puts it in;
- the storey of the Level each rule picks from the Levels the record names, in
  order of elevation, with `lo` the lowest, `hi` the highest (unique), `hi*`
  the highest where none of the named Levels lies within the box, `bc` RE-59's
  base constraint (two Levels: the higher if it lies at or below the record
  box's base, else the lower), `<base` the highest Level at or below the base
  (else the lowest), `=top` the one Level at the box's top, `<top` the highest
  at or below the top, `>top` the lowest at or above it, `only` the one Level a
  record names, and, among every building-story Level of the model, `top*` the
  one at the box's top (RE-119) and `base*` the highest at or below the base. A
  Level whose Building Story setting is off is the storey of the Level below it.

Elements are grouped by the IFC entity rvt-rs writes, the number of Levels named
(none, 1, 2, or 3 and more) and where they lie against the box (at or below the base /
within / at or above the top), and each cell is the number of elements the rule
puts in Revit's storey.

Usage:

    python3 tools/re/storey_rules_vs_ifc.py <probe.txt> <revit.ifc> [--list]

`--list` also lists the elements whose storey in rvt-rs's export is not Revit's.
Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import sys

import ifcopenshell

import storeys_vs_ifc

EPS = 1e-3
LISTED = 60


def keyed(text):
    return dict(part.split("=", 1) for part in text.split() if "=" in part)


def read_probe(path):
    levels, storeys, rows = {}, {}, []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.rstrip("\n")
        if line.startswith(("level ", "storey ")) and " name=" in line:
            head, name = line.split(" name=", 1)
            fields = keyed(head)
            if line.startswith("level "):
                levels[fields["id"]] = (float(fields["elev"]), fields["story"], name)
            else:
                storeys[int(fields["index"])] = (float(fields["elev"]), name)
        elif line.startswith("named "):
            rows.append(keyed(line))
    return levels, storeys, rows


def storey_name(level, levels):
    """The storey a Level is: itself, or where its Building Story setting is off
    the Level below it that is a building story."""
    elevation, story, name = levels[level]
    if story != "no":
        return name
    below = [
        (other[0], other[2])
        for other in levels.values()
        if other[1] != "no" and other[0] < elevation
    ]
    return max(below)[1] if below else None


# Each rule takes the named Levels as (id, elevation) in order of elevation (an
# empty list for a record that names none), and the record box's lowest and
# highest z, and gives one of them or none. The last two choose among every
# building-story Level of the model, `ALL_LEVELS`.
ALL_LEVELS = []


def lo(named, base, top):
    return named[0] if named else None


def hi(named, base, top):
    if len(named) < 2:
        return None
    return named[-1] if named[-1][1] - named[-2][1] > 1e-6 else None


def hi_outside(named, base, top):
    """`hi`, unless a named Level lies strictly within the box."""
    if any(base + EPS < elevation < top - EPS for _, elevation in named):
        return None
    return hi(named, base, top)


def base_constraint(named, base, top):
    if len(named) != 2 or named[0][1] == named[1][1]:
        return None
    return named[1] if named[1][1] <= base + EPS else named[0]


def below_base(named, base, top):
    if not named:
        return None
    below = [level for level in named if level[1] <= base + EPS]
    return below[-1] if below else named[0]


def at_top(named, base, top):
    at = [level for level in named if abs(level[1] - top) <= EPS]
    return at[0] if len(at) == 1 else None


def below_top(named, base, top):
    below = [level for level in named if level[1] <= top + EPS]
    return below[-1] if below else None


def above_top(named, base, top):
    above = [level for level in named if level[1] >= top - EPS]
    return above[0] if above else None


def only(named, base, top):
    return named[0] if len(named) == 1 else None


def model_top(named, base, top):
    """RE-119: the one Level of the model at the box's top."""
    at = [level for level in ALL_LEVELS if abs(level[1] - top) <= EPS]
    return at[0] if len(at) == 1 else None


def model_below_base(named, base, top):
    """The highest Level of the model at or below the box's base."""
    below = [level for level in ALL_LEVELS if level[1] <= base + EPS]
    return below[-1] if below else None


RULES = (
    ("lo", lo),
    ("hi", hi),
    ("hi*", hi_outside),
    ("bc", base_constraint),
    ("<base", below_base),
    ("=top", at_top),
    ("<top", below_top),
    (">top", above_top),
    ("only", only),
    ("top*", model_top),
    ("base*", model_below_base),
)


def pattern(named, base, top):
    cells = [0, 0, 0]
    for _, elevation in named:
        cells[0 if elevation <= base else 2 if elevation >= top else 1] += 1
    return "/".join(map(str, cells))


def main():
    args = [a for a in sys.argv[1:] if a != "--list"]
    if len(args) != 2:
        print(__doc__)
        return 2
    levels, storeys, rows = read_probe(args[0])
    ALL_LEVELS[:] = sorted(
        ((level, elevation) for level, (elevation, story, _) in levels.items() if story != "no"),
        key=lambda item: item[1],
    )
    theirs = storeys_vs_ifc.storeys(ifcopenshell.open(args[1]))
    print(f"probe: {len(levels)} levels, {len(storeys)} storeys, {len(rows)} elements listed")

    cells = collections.defaultdict(collections.Counter)
    wrong = []
    skipped = collections.Counter()
    for row in rows:
        revit = theirs.get(row["tag"])
        if revit is None:
            skipped["not in Revit's export"] += 1
            continue
        if not storeys_vs_ifc.on_storey(revit):
            skipped["Revit contains it in " + revit] += 1
            continue
        ids = [] if row["levels"] == "-" else row["levels"].split(",")
        if any(level not in levels for level in ids):
            skipped["a Level without an elevation"] += 1
            continue
        named = sorted(((level, levels[level][0]) for level in ids), key=lambda item: item[1])
        base, top = float(row["base"]), float(row["top"])
        count = str(len(named)) if len(named) < 3 else "3+"
        key = (row["class"], count, pattern(named, base, top))
        cell = cells[key]
        cell["n"] += 1
        ours = None if row["ours"] == "-" else storeys[int(row["ours"])][1]
        cell["ours"] += ours == revit
        if ours != revit:
            wrong.append((row, ours, revit, named, base, top))
        for name, rule in RULES:
            picked = rule(named, base, top)
            cell[name] += picked is not None and storey_name(picked[0], levels) == revit
            cell[name + "?"] += picked is not None
    for what, n in skipped.most_common():
        print(f"  left out: {n} ({what})")

    names = [name for name, _ in RULES]
    print("elements by entity, Levels named and their place against the box (at or below base / within / at or above top):")
    print("  right = the rule's storey is Revit's; ? = the rule gives a Level")
    head = f"  {'entity':<22} {'n':<3} {'below/in/above':<15} {'N':>5} {'ours':>5} " + " ".join(f"{n:>9}" for n in names)
    print(head)
    totals = collections.defaultdict(collections.Counter)
    for key, cell in sorted(cells.items(), key=lambda item: (item[0][0], item[0][1], -item[1]["n"])):
        entity, count, place = key
        line = f"  {entity:<22} {count:<3} {place:<15} {cell['n']:>5} {cell['ours']:>5} "
        line += " ".join(f"{cell[name]:>4}/{cell[name + '?']:<4}" for name in names)
        print(line)
        total = totals[(entity, count)]
        for name, value in cell.items():
            total[name] += value
    print("totals by entity and Levels named:")
    for (entity, count), total in sorted(totals.items()):
        line = f"  {entity:<22} {count:<3} {'':<15} {total['n']:>5} {total['ours']:>5} "
        line += " ".join(f"{total[name]:>4}/{total[name + '?']:<4}" for name in names)
        print(line)

    if "--list" in sys.argv:
        for row, ours, revit, named, base, top in wrong[:LISTED * 5]:
            levels_text = ", ".join(f"{levels[level][2]}@{elevation:.2f}" for level, elevation in named)
            print(
                f"ours differs {row['tag']} {row['class']} ({row['cat']}): Levels {levels_text};"
                f" base {base:.2f} top {top:.2f}; ours {ours}, Revit's {revit}"
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
