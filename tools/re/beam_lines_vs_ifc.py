#!/usr/bin/env python3
"""Per beam, Revit's Length, Span, Slope and Roll against the location line rvt-rs reads from its element data (#1697).

Reads the output of `probes/examples/probe_1697_beam_lines.rs`, one `beam <ElementId> ...` line per
structural-framing element, and Revit's IFC export of the same model. Pairs each `IfcBeam` by `Tag` with the
probe's beam of that ElementId. For each beam with a line it takes

    Length, Span  the line's length (the probe's feet, in the export's length unit);
    Slope         the line's angle to the horizontal, atan2(|dz|, horizontal run);
    Roll          the Cross-Section Rotation the probe read, in the export's angle unit,

and says whether `Qto_BeamBaseQuantities.Length`, `Pset_BeamCommon.Span`, `Slope` and `Roll` of Revit's beam
equal them (a relative 1e-6 or 1e-6 ft for lengths, 1e-6 of a turn for angles). The beams are counted by the
group the exporter puts them in: a line attached (its body is a box along the line, and the properties of
`add_beam_length` apply), a line read but not attached, a column-cut beam, and no line in the data; and by whether
Revit's export holds each of the four values. `--list` names the beams where a value differs, or where Revit holds
a value the line does not give.

Every summary line starts `beam lines:` and holds counts beside fixed words, which the held-out scrub keeps; the
lines `--list` adds name an ElementId and are dropped there. Revit's export in IFC2X3, which has no `Qto_` sets,
is not scored.

Usage:

    python3 tools/re/beam_lines_vs_ifc.py <probe output> <revit-export.ifc> [--list]

Needs IfcOpenShell.
"""

import collections
import math
import re
import sys

import ifcopenshell
import ifcopenshell.util.element
import ifcopenshell.util.unit

FEET_METRES = 0.3048
LINE = re.compile(
    r"^beam (\d+) line 1 attached (\d) cut (\d) rotation (\S+) inside (\S+) closes (\S+) "
    r"start (\S+) (\S+) (\S+) end (\S+) (\S+) (\S+) length (\S+)$"
)
NO_LINE = re.compile(r"^beam (\d+) line 0 attached (\d) cut (\d)$")


def read_probe(path):
    beams = {}
    for line in open(path):
        line = line.strip()
        m = LINE.match(line)
        if m:
            tag, attached, cut, rotation, inside, closes, *rest = m.groups()
            values = [float(v) for v in rest]
            start, end, length = values[0:3], values[3:6], values[6]
            beams[tag] = {
                "line": True,
                "attached": attached == "1",
                "cut": cut == "1",
                "rotation": None if rotation == "none" else float(rotation),
                "inside": inside,
                "closes": closes,
                "start": start,
                "end": end,
                "length": length,
            }
            continue
        m = NO_LINE.match(line)
        if m:
            tag, attached, cut = m.groups()
            beams[tag] = {"line": False, "attached": attached == "1", "cut": cut == "1"}
    return beams


def angle_unit(model):
    """Radians per unit of the export's plane angle unit."""
    try:
        return ifcopenshell.util.unit.calculate_unit_scale(model, "PLANEANGLEUNIT")
    except Exception:  # noqa: BLE001
        return math.pi / 180.0


def length_unit(model):
    """Metres per unit of the export's length unit."""
    return ifcopenshell.util.unit.calculate_unit_scale(model)


def equal_length(a, b, unit_feet):
    return abs(a - b) <= max(1e-6 * max(abs(a), abs(b)), 1e-6 * unit_feet)


def group_of(beam):
    if not beam["line"]:
        return "cut by a column, no line" if beam["cut"] else "no line in its data"
    if beam["cut"]:
        return "cut by a column"
    if beam["attached"]:
        return "line attached"
    return "line read, not attached"


def main():
    probe = read_probe(sys.argv[1])
    listing = "--list" in sys.argv
    model = ifcopenshell.open(sys.argv[2])
    if model.schema == "IFC2X3":
        print("Revit's export is IFC2X3, which has no Qto_ sets: not scored")
        return
    metre = length_unit(model)
    radian = angle_unit(model)
    revit = {}
    for beam in model.by_type("IfcBeam"):
        if beam.is_a() != "IfcBeam" or not beam.Tag:
            continue
        psets = ifcopenshell.util.element.get_psets(beam)
        common = psets.get("Pset_BeamCommon", {})
        qto = psets.get("Qto_BeamBaseQuantities", {})
        revit[str(beam.Tag)] = {
            "Length": qto.get("Length"),
            "Span": common.get("Span"),
            "Slope": common.get("Slope"),
            "Roll": common.get("Roll"),
        }
    paired = [t for t in revit if t in probe]
    print(
        f"beam lines: beams in Revit's export {len(revit)}, in the probe {len(probe)}, paired by ElementId {len(paired)}; "
        f"length unit {metre:g} m, angle unit {radian:g} rad"
    )
    unit_feet = FEET_METRES / metre
    table = collections.defaultdict(collections.Counter)
    rows = []
    for tag in paired:
        beam, theirs = probe[tag], revit[tag]
        group = group_of(beam)
        t = table[group]
        t["beams"] += 1
        for name in ("Length", "Span", "Slope", "Roll"):
            t[f"Revit holds {name}"] += theirs[name] is not None
        if not beam["line"]:
            continue
        line_length = beam["length"] * unit_feet
        dx = [beam["end"][k] - beam["start"][k] for k in range(3)]
        run = math.hypot(dx[0], dx[1])
        slope = math.atan2(abs(dx[2]), run) / radian
        roll = None if beam["rotation"] is None else beam["rotation"] / radian
        verdict = {}
        for name, mine in (("Length", line_length), ("Span", line_length), ("Slope", slope), ("Roll", roll)):
            theirs_value = theirs[name]
            if theirs_value is None:
                verdict[name] = "Revit holds none"
            elif mine is None:
                verdict[name] = "not read"
            elif name in ("Length", "Span"):
                verdict[name] = "equal" if equal_length(mine, theirs_value, unit_feet) else "different"
            else:
                verdict[name] = "equal" if abs(mine - theirs_value) <= 1e-6 * (2 * math.pi / radian) else "different"
            t[f"{name} {verdict[name]}"] += 1
        if listing and ("different" in verdict.values() or "not read" in verdict.values()):
            rows.append((group, tag, theirs, line_length, slope, roll, verdict))
    for group in sorted(table):
        t = table[group]
        print(f"beam lines: {group}: {t['beams']} beams")
        for name in ("Length", "Span", "Slope", "Roll"):
            parts = ", ".join(
                f"{label} {t[f'{name} {label}']}"
                for label in ("equal", "different", "not read", "Revit holds none")
                if t[f"{name} {label}"]
            )
            print(f"beam lines: {group} {name}: Revit holds {t[f'Revit holds {name}']}; {parts or 'no beam with a line'}")
    for group, tag, theirs, line_length, slope, roll, verdict in rows[:40]:
        print(
            f"  {group} {tag}: Revit Length {theirs['Length']} Span {theirs['Span']} Slope {theirs['Slope']} "
            f"Roll {theirs['Roll']}; line Length {line_length:.6f} Slope {slope:.6f} Roll {roll}; {verdict}"
        )


if __name__ == "__main__":
    main()
