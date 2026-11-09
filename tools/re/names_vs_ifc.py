#!/usr/bin/env python3
"""Compare the Name and ObjectType of each element in an rvt-rs IFC with
Revit's own IFC export of the same file.

Research tool for `reports/element-framing/RE-63-system-family-element-names.md`,
`RE-64-panel-walls-slab-edges-ramps.md` and `RE-65-stair-names.md`.

Elements are matched by `Tag`, the Revit ElementId. An `IfcOpeningElement`
carries the Tag of the element that cuts it, so openings are left out. An
element counts as a system-family element when Revit's Name starts with a
system family (Basic Wall, Curtain Wall, Floor, Pad, Slab Edge, Ramp,
Compound Ceiling, Basic Roof, Railing, ...), and as a family instance otherwise.

Usage:

    python3 tools/re/names_vs_ifc.py [-v] [--any-copy] <rvt-rs.ifc> <revit-export.ifc>

`-v` prints the system-family elements whose Name differs. Revit writes a
multistory stair once per storey with one `Tag`, naming the later copies
with a `:2` suffix; `--any-copy` counts a name equal when it equals any of
the elements sharing its `Tag`, where the default compares with the last.
Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell

SYSTEM_FAMILIES = {
    "Basic Wall",
    "Curtain Wall",
    "Stacked Wall",
    "Floor",
    "Foundation Slab",
    "Pad",
    "Slab Edge",
    "Ramp",
    "Compound Ceiling",
    "Basic Ceiling",
    "Basic Roof",
    "Sloped Glazing",
    "Railing",
    "Assembled Stair",
    "Cast-In-Place Stair",
    "Precast Stair",
    "Model Text",
    "Wall Sweep",
}


def tag(entity):
    try:
        return entity.Tag
    except AttributeError:
        return None


def spacing_free(text):
    return "".join(ch for ch in text.casefold() if ch.isalnum())


def object_type_pattern(ours, theirs):
    """How two differing ObjectType values relate, in fixed words (#1472).

    The words say nothing of the values, so a held-out model's census can leave the runner.
    """
    if not ours:
        return "rvt-rs writes none"
    if not theirs:
        return "Revit writes none"
    if ours.casefold() == theirs.casefold():
        return "case only"
    if spacing_free(ours) == spacing_free(theirs):
        return "spacing or punctuation only"
    ours_family, _, ours_type = ours.partition(":")
    theirs_family, _, theirs_type = theirs.partition(":")
    if ":" not in theirs and theirs == ours_type:
        return "Revit's is the type part of rvt-rs's"
    if ":" not in ours and ours == theirs_type:
        return "rvt-rs's is the type part of Revit's"
    if theirs.startswith(ours):
        return "Revit's extends rvt-rs's"
    if ours.startswith(theirs):
        return "rvt-rs's extends Revit's"
    if ":" in ours and ":" in theirs:
        if ours_family == theirs_family:
            return "same family, other type"
        if ours_type == theirs_type:
            return "other family, same type"
    return "other"


def main():
    args = [a for a in sys.argv[1:] if a not in ("-v", "--any-copy")]
    verbose = "-v" in sys.argv[1:]
    any_copy = "--any-copy" in sys.argv[1:]
    if len(args) != 2:
        raise SystemExit(__doc__)
    theirs = {}
    copies = collections.defaultdict(list)
    # Both files stay bound while their instances are in use: IfcOpenShell
    # 0.9 frees a file its instances outlive (#773).
    revit_file, ours_file = ifcopenshell.open(args[1]), ifcopenshell.open(args[0])
    for e in revit_file.by_type("IfcElement"):
        if tag(e) and not e.is_a("IfcOpeningElement"):
            theirs[tag(e)] = e
            copies[tag(e)].append(e)
    counts = collections.Counter()
    patterns = collections.Counter()
    for element in ours_file.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement"):
            continue
        revit = theirs.get(tag(element))
        if revit is None:
            continue
        family = (revit.Name or "").split(":")[0]
        kind = "system family" if family in SYSTEM_FAMILIES else "family instance"
        candidates = copies[tag(element)] if any_copy else [revit]
        name = (
            "Name equal"
            if any(element.Name == c.Name for c in candidates)
            else "Name differs"
        )
        object_type = (
            "ObjectType equal"
            if any(
                getattr(element, "ObjectType", None) == getattr(c, "ObjectType", None)
                for c in candidates
            )
            else "ObjectType differs"
        )
        counts[(kind, element.is_a(), name, object_type)] += 1
        if object_type == "ObjectType differs":
            pattern = object_type_pattern(
                getattr(element, "ObjectType", None), getattr(revit, "ObjectType", None)
            )
            patterns[(kind, element.is_a(), "ObjectType pattern: " + pattern)] += 1
        if verbose and kind == "system family" and name == "Name differs":
            print(f"    {tag(element)} {element.is_a()}: {element.Name} vs Revit's {revit.Name}")
    for key, count in sorted(counts.items(), key=lambda kv: -kv[1]):
        print(f"  {count:>5}  {' | '.join(key)}")
    for key, count in sorted(patterns.items(), key=lambda kv: (-kv[1], kv[0])):
        print(f"  {count:>5}  {' | '.join(key)}")


if __name__ == "__main__":
    main()
