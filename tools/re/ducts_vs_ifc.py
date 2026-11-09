#!/usr/bin/env python3
"""Each duct's own record, and each connector's system, against Revit's export (RE-371).

Reads `probe_re371_ducts --native`'s lines and Revit's IFC, and says for each
duct of the export that the probe gives, by ElementId (`Tag`), whether the
record gives Revit's:

- type name (the second part of `Family:Type:ElementId`);
- `Length` (`Dimensions` or a `Qto_`/`Pset_` set, where written);
- section (`Dimensions` `Diameter`, or `Width` and `Height`, where written);
- body: the world box of the record's section swept along its line, a round
  type's as its circle (where the type's name holds no `Rect`), against the
  world box of Revit's body, within 0.01 ft;
- `Constraints` values (where written, as in an export with Revit's own
  property sets): Start, End and Middle Elevation, the Lower and Upper End
  Centerline, Bottom and Top Elevation, `Slope`, and the justification, by
  RE-371's rules, for a duct whose record is justified at its centre.

For each port whose connector the probe gives, by its name
(`<In|Out>Port_<ElementId>_<index>`), it also says whether RE-371's
`SystemType` (the connector's `m_eSystemType`: 1 and 2 `VENTILATION`, 3
`EXHAUST`, 7 `HEATING`; none for any other) is Revit's, where Revit writes one
(IFC4), and whether every port of an Exhaust Air connector is
`SOURCEANDSINK`, as RE-371 writes it.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/ducts_vs_ifc.py <probe output> <revit.ifc>
"""

import argparse
import collections
import math
import re

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.element as element
import ifcopenshell.util.unit as unit

FT = 0.3048
LENGTH_FT = 1e-3
BOX_FT = 0.01
ELEVATION_FT = 1e-4
PORT_NAME = re.compile(r"^(?:In|Out)?Port_(\d+)_(\d+)$")
SYSTEM_TYPES = {1: "VENTILATION", 2: "VENTILATION", 3: "EXHAUST", 7: "HEATING"}


def read_probe(path):
    ducts, ports = {}, {}
    with open(path, encoding="utf-8") as lines:
        for line in lines:
            words = line.split()
            if words[0] == "duct":
                values = dict(zip(words[5::2], words[6::2]))
                duct = {"type": words[4].replace("|", " ")}
                for key in ("start", "end", "normal"):
                    duct[key] = (
                        None if values[key] == "-" else [float(v) for v in values[key].split(",")]
                    )
                for key in ("width", "height"):
                    duct[key] = None if values[key] == "-" else float(values[key])
                offsets = values["offsets"].split(",")
                duct["offsets"] = None if "-" in offsets else [float(v) for v in offsets]
                duct["justify"] = values["justify"]
                ducts[words[1]] = duct
            elif words[0] == "port":
                values = dict(zip(words[4::2], words[5::2]))
                system = values.get("system", "-")
                ports[(words[1], words[2])] = (words[3], None if system == "-" else int(system))
    return ducts, ports


def swept_box(duct, round_section):
    start, end = duct["start"], duct["end"]
    length = math.dist(start, end)
    unit_vector = [(end[i] - start[i]) / length for i in range(3)]
    width, height = duct["width"], duct["height"]
    if round_section:
        across = [width / 2 * math.sqrt(max(0.0, 1 - u * u)) for u in unit_vector]
    else:
        n = duct["normal"]
        x = [
            n[1] * unit_vector[2] - n[2] * unit_vector[1],
            n[2] * unit_vector[0] - n[0] * unit_vector[2],
            n[0] * unit_vector[1] - n[1] * unit_vector[0],
        ]
        across = [width / 2 * abs(x[i]) + height / 2 * abs(n[i]) for i in range(3)]
    return [min(start[i], end[i]) - across[i] for i in range(3)] + [
        max(start[i], end[i]) + across[i] for i in range(3)
    ]


def revit_box(shape):
    vertices = shape.geometry.verts
    points = [vertices[i : i + 3] for i in range(0, len(vertices), 3)]
    return [min(p[i] for p in points) / FT for i in range(3)] + [
        max(p[i] for p in points) / FT for i in range(3)
    ]


def constraint_values(duct):
    """RE-371's Constraints values for a duct, feet, or None."""
    if duct["offsets"] is None or duct["justify"] != "0,0" or duct["start"] is None:
        return None
    start, end = duct["start"], duct["end"]
    first, second = duct["offsets"]
    length = math.dist(start, end)
    run = math.hypot(end[0] - start[0], end[1] - start[1])
    half = duct["height"] / 2 * run / length
    lower, upper = (first, second) if start[2] <= end[2] else (second, first)
    values = {
        "Start Middle Elevation": first,
        "End Middle Elevation": second,
        "Middle Elevation": first,
        "Lower End Centerline Elevation": lower,
        "Lower End Bottom Elevation": lower - half,
        "Lower End Top Elevation": lower + half,
        "Upper End Centerline Elevation": upper,
        "Upper End Bottom Elevation": upper - half,
        "Upper End Top Elevation": upper + half,
        "Horizontal Justification": "Center",
        "Vertical Justification": "Middle",
    }
    if run > 1e-6:
        values["Slope"] = abs(end[2] - start[2]) / run
    return values


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("probe")
    parser.add_argument("ifc")
    args = parser.parse_args()
    ducts, ports = read_probe(args.probe)
    model = ifcopenshell.open(args.ifc)
    scale = unit.calculate_unit_scale(model) / FT
    entity = "IfcFlowSegment" if model.schema == "IFC2X3" else "IfcDuctSegment"
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    scores = collections.defaultdict(lambda: [0, 0])
    misses = []
    compared = 0

    def score(tag, name, ok):
        scores[name][0] += ok
        scores[name][1] += 1
        if not ok:
            misses.append((tag, name))

    for segment in model.by_type(entity):
        duct = ducts.get(segment.Tag)
        if duct is None:
            continue
        compared += 1
        sets = element.get_psets(segment)
        values = {}
        for properties in sets.values():
            values.update(properties)
        parts = (segment.Name or "").split(":")
        if len(parts) == 3:
            score(segment.Tag, "type name", parts[1] == duct["type"])
        if duct["start"] is not None and "Length" in values:
            length = math.dist(duct["start"], duct["end"])
            score(segment.Tag, "Length", abs(values["Length"] * scale - length) <= LENGTH_FT)
        dimensions = sets.get("Dimensions", {})
        round_section = "Diameter" in dimensions
        if round_section:
            score(
                segment.Tag,
                "section",
                abs(dimensions["Diameter"] * scale - duct["width"]) <= LENGTH_FT,
            )
        elif "Width" in dimensions:
            score(
                segment.Tag,
                "section",
                abs(dimensions["Width"] * scale - duct["width"]) <= LENGTH_FT
                and abs(dimensions["Height"] * scale - duct["height"]) <= LENGTH_FT,
            )
        if duct["start"] is not None and duct["normal"] is not None and segment.Representation:
            try:
                shape = ifcopenshell.geom.create_shape(settings, segment)
            except RuntimeError:
                shape = None
            if shape is not None:
                ours = swept_box(duct, round_section)
                theirs = revit_box(shape)
                score(
                    segment.Tag,
                    "body box",
                    all(abs(a - b) <= BOX_FT for a, b in zip(ours, theirs)),
                )
        constraints = sets.get("Constraints", {})
        wanted = constraint_values(duct)
        if constraints and wanted:
            for name, want in wanted.items():
                if name not in constraints:
                    continue
                got = constraints[name]
                ok = got == want if isinstance(want, str) else abs(got * scale - want) <= (
                    ELEVATION_FT if name != "Slope" else 1e-4
                )
                score(segment.Tag, f"Constraints {name}", ok)
            if "Slope" not in wanted:
                score(segment.Tag, "Constraints no Slope on a vertical duct", "Slope" not in constraints)
    print(f"{args.ifc}: {compared} of the export's ducts are in the probe output")
    port_scores = collections.Counter()
    for port in model.by_type("IfcDistributionPort"):
        match = PORT_NAME.match(port.Name or "")
        if not match or match.groups() not in ports:
            continue
        _, system = ports[match.groups()]
        revit_system = getattr(port, "SystemType", None) if model.schema != "IFC2X3" else None
        want = SYSTEM_TYPES.get(system)
        if revit_system is not None:
            if want is None:
                port_scores[f"SystemType not written (m_eSystemType {system}, Revit {revit_system})"] += 1
            else:
                score(port.Name, "port SystemType", revit_system == want)
        if system == 3:
            score(port.Name, "Exhaust Air port SOURCEANDSINK", port.FlowDirection == "SOURCEANDSINK")
    for name, (ok, total) in sorted(scores.items()):
        print(f"  {name}: {ok} of {total} as Revit's")
    for name, count in sorted(port_scores.items()):
        print(f"  {name}: {count}")
    for tag, name in misses[:20]:
        print(f"  miss {tag} {name}")


if __name__ == "__main__":
    main()
