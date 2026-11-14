#!/usr/bin/env python3
"""What the rules of #1945 write on each release, held to the same project's 2024 save (RE-461).

Eight rules write values on releases their reports never measured against a Revit export. For each
Autodesk sample project, this reads the kept exports of every release (a Measure run dispatched with
`-f samples=true -f keep_ifc=true`, the files at `<dir>/sample-<year>-<project>/model.ifc`), pairs each
element with the 2024 save's by `Tag` (an ElementId, which a project keeps across saves), and prints,
for each release other than 2024, how many elements each rule can touch and how many of them the 2024
save's reading equals:

  hollow       columns drawn as a hollow section (RE-247 section 8's NetVolume)
  sweeps, ramps  wall sweeps and ramps, and how many are typed (RE-240)
  doors        doors that fill no opening (RE-361): their origin to 3 mm (on a release whose reader
               places no such door by its stored transform, the origin is the record box's)
  railings     name, type and `Type Id` (RE-66)
  pipes        name, type and base quantities (RE-370)
  beams        the storey of the beams RE-119's rule binds (`LevelBindSource` structural_framing_top_level)

It does not say a reading is Revit's: the 2024 save is itself held to Revit's export only where one
exists. It says the reading is the same across saves of one project, which is what the same-project
gates hold the 2025 to 2027 saves to for bodies and properties.

    python3 tools/re/release_rules_across_saves.py <kept exports dir> [project ...]

Needs ifcopenshell.
"""

import collections
import glob
import os
import re
import sys

import ifcopenshell
import ifcopenshell.util.element as element_util
import ifcopenshell.util.placement as placement_util

REFERENCE_YEAR = 2024
ORIGIN_TOLERANCE = 0.003  # feet
RE119_SOURCE = "structural_framing_top_level"


def project_and_year(directory):
    match = re.match(r"sample-(\d{4})-(.*)", directory)
    if not match:
        return None
    project = re.sub(r"_?sample_?project", "", match.group(2)).replace("_", "")
    return project, int(match.group(1))


def origin(product):
    try:
        matrix = placement_util.get_local_placement(product.ObjectPlacement)
    except Exception:
        return None
    return (float(matrix[0][3]), float(matrix[1][3]), float(matrix[2][3]))


def storey(product):
    for relation in product.ContainedInStructure or []:
        return relation.RelatingStructure.Name
    return None


def has_voids(product):
    representation = getattr(product, "Representation", None)
    for shape in representation.Representations if representation else []:
        for item in shape.Items:
            if item.is_a("IfcExtrudedAreaSolid") and item.SweptArea.is_a("IfcArbitraryProfileDefWithVoids"):
                return True
    return False


def read(path):
    """Tag -> what the eight rules write, by kind."""
    model = ifcopenshell.open(path)
    filled = {relation.RelatedBuildingElement.id() for relation in model.by_type("IfcRelFillsElement")}
    out = {kind: {} for kind in ("hollow", "sweeps", "ramps", "doors", "railings", "pipes", "beams")}
    for product in model.by_type("IfcProduct"):
        tag = getattr(product, "Tag", None)
        if product.is_a("IfcOpeningElement") or not tag:
            continue
        psets = element_util.get_psets(product)
        record = psets.get("RvtElementRecordGeometry", {})
        if product.is_a("IfcColumn") and has_voids(product):
            out["hollow"][tag] = psets.get("Qto_ColumnBaseQuantities", {}).get("NetVolume")
        if record.get("RevitClass") == "WallSweep":
            out["sweeps"][tag] = element_util.get_type(product) is not None
        if product.is_a("IfcRamp"):
            out["ramps"][tag] = element_util.get_type(product) is not None
        if product.is_a("IfcDoor") and product.id() not in filled:
            out["doors"][tag] = origin(product)
        if product.is_a("IfcRailing"):
            other = psets.get("Other", {})
            kind = element_util.get_type(product)
            out["railings"][tag] = (product.Name, getattr(kind, "Name", None), other.get("Type Id"))
        if product.is_a("IfcPipeSegment"):
            quantities = element_util.get_psets(product, qtos_only=True).get("Qto_PipeSegmentBaseQuantities", {})
            kind = element_util.get_type(product)
            out["pipes"][tag] = (
                product.Name,
                getattr(kind, "Name", None),
                tuple(sorted((k, round(v, 4)) for k, v in quantities.items() if k != "id" and isinstance(v, float))),
            )
        if product.is_a("IfcBeam"):
            out["beams"][tag] = (storey(product), record.get("LevelBindSource"))
    return out


def doors_line(candidate, reference):
    offsets = collections.Counter()
    for tag, point in candidate.items():
        if tag in reference and point and reference[tag]:
            offsets[tuple(round(a - b, 3) for a, b in zip(point, reference[tag]))] += 1
    paired = sum(offsets.values())
    same = sum(count for delta, count in offsets.items() if all(abs(d) <= ORIGIN_TOLERANCE for d in delta))
    return f"doors {len(candidate)} (paired {paired}, origin as 2024's {same})"


def pairs_line(name, candidate, reference):
    paired = [tag for tag in candidate if tag in reference]
    same = sum(1 for tag in paired if candidate[tag] == reference[tag])
    return f"{name} {len(candidate)} (paired {len(paired)}, as 2024's {same})"


def beams_line(candidate, reference):
    bound = [tag for tag, (_, source) in candidate.items() if source == RE119_SOURCE]
    paired = [tag for tag in bound if tag in reference]
    same = sum(1 for tag in paired if candidate[tag][0] == reference[tag][0])
    return f"beams bound by RE-119 {len(bound)} (paired {len(paired)}, same storey as 2024's {same})"


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    root, wanted = sys.argv[1], sys.argv[2:]
    saves = collections.defaultdict(dict)
    for path in sorted(glob.glob(os.path.join(root, "sample-*", "model.ifc"))):
        found = project_and_year(os.path.basename(os.path.dirname(path)))
        if found and (not wanted or found[0] in wanted):
            saves[found[0]][found[1]] = path
    for project, years in sorted(saves.items()):
        if REFERENCE_YEAR not in years:
            print(f"{project}: no {REFERENCE_YEAR} save")
            continue
        reference = read(years[REFERENCE_YEAR])
        for year in sorted(years):
            candidate = reference if year == REFERENCE_YEAR else read(years[year])
            typed_sweeps = sum(1 for typed in candidate["sweeps"].values() if typed)
            typed_ramps = sum(1 for typed in candidate["ramps"].values() if typed)
            print(
                f"{project} {year}: hollow columns {len(candidate['hollow'])}; "
                f"wall sweeps {len(candidate['sweeps'])} (typed {typed_sweeps}); ramps {len(candidate['ramps'])} (typed {typed_ramps}); "
                + "; ".join(
                    [
                        doors_line(candidate["doors"], reference["doors"]),
                        pairs_line("railings", candidate["railings"], reference["railings"]),
                        pairs_line("pipes", candidate["pipes"], reference["pipes"]),
                        beams_line(candidate["beams"], reference["beams"]),
                    ]
                ),
                flush=True,
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
