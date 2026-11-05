#!/usr/bin/env python3
"""Check that every IfcSpace of an rvt-rs export says what it is (#1437).

`RvtElementRecordGeometry.SpaceKind` is `room`, `area`, `mep_space` or
`family_instance`, taken from the Revit class of the record the space was read
from. `SpaceOutlineSource` says only where an outline came from, and an
importer must not filter on its values. This reads the export and counts, for
the file:

- the `IfcSpace`s, and how many hold each kind;
- the spaces with no `SpaceKind`, and those whose value is not one of the four;
- the spaces whose kind and outline source disagree. `area` holds exactly where
  the outline source is `area_boundary`, `area_boundary_joined`,
  `area_plan_topology`, `area_plan_topology_unchecked` or `none`, and no other
  kind may hold one of those:
  an `area` without such a source and a space of another kind with one are
  each counted. An export with no geometry (`typed_no_geometry`) has no
  `SpaceOutlineSource` on any space; the agreement is then not checked, and
  the spaces it was not checked for are counted on a line of their own.

The file passes (exit 0) when no space lacks a kind, none has another value
and none disagrees; a file with no `IfcSpace` passes with a count of zero
spaces, because there is nothing in it to check. Any other result exits 1.
The lines starting `space kinds:` are made of fixed words and numbers, so
the Measure workflow keeps them for the models it keeps only counts of
(tools/ci/scrub-holdout-results.sh): `line` takes words written in this file and
integers and raises on anything else. A space that fails is listed under them
with its GlobalId, indented, and those lines are not kept.

A run that checks nothing must not pass. `--total` reads the results of a whole
Measure run, `<dir>/<model>/space_kinds_vs_ifc.txt` or, for a model kept as
counts only, the `space kinds:` lines of `<dir>/<model>/counts.txt`, and
sums them. It exits 1 when a model that was exported has no result, when any
model failed, when the kinds do not add up to the spaces, or when the run
saw no `IfcSpace` at all. It prints how many of each kind the run saw.

Usage:

    python3 tools/re/space_kinds_vs_ifc.py <rvt-rs.ifc>
    python3 tools/re/space_kinds_vs_ifc.py --total <measure results directory>

The first needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins);
`--total` needs only Python.
"""

import argparse
import collections
import os
import re
import sys

SET = "RvtElementRecordGeometry"
KIND_PROPERTY = "SpaceKind"
SOURCE_PROPERTY = "SpaceOutlineSource"
KINDS = ("room", "area", "mep_space", "family_instance")
AREA_SOURCES = (
    "area_boundary",
    "area_boundary_joined",
    "area_plan_topology",
    # #1428 (Griffin, 2026-11-05, "Mark 2025 to 2027 in the file"): the Areas of
    # Revit 2025 to 2027. A reader that never writes it, as main's, cannot fail on it.
    "area_plan_topology_unchecked",
    "none",
)
LIST_LIMIT = 25
PREFIX = "space kinds: "

# The words of each count line, in the order they are printed. `--total` reads
# the same words back, so the two cannot drift.
SPACES = "IfcSpace"
NO_KIND = "no SpaceKind"
OTHER_KIND = "SpaceKind of another value"
AREA_SOURCE = "SpaceOutlineSource of an Area"
AREA_WITHOUT_SOURCE = "area without an Area outline source"
OTHER_WITH_SOURCE = "other kind with an Area outline source"
NOT_CHECKED = "spaces whose outline source agreement was not checked"
COUNTED = (
    SPACES,
    *KINDS,
    NO_KIND,
    OTHER_KIND,
    AREA_SOURCE,
    AREA_WITHOUT_SOURCE,
    OTHER_WITH_SOURCE,
    NOT_CHECKED,
)

WORDS = re.compile(r"[A-Za-z0-9 _:.,;()=%/+-]+")


def line(words, *numbers):
    """Print `space kinds: <words> <number>`: fixed words and integers only, so a
    name, an id or a message cannot leave the runner for a model kept as counts."""
    if words not in COUNTED and words not in ("PASS", "FAIL"):
        raise ValueError(f"not a word of this scorer: {words!r}")
    if not WORDS.fullmatch(words):
        raise ValueError(f"not fixed words: {words!r}")
    for value in numbers:
        if isinstance(value, bool) or not isinstance(value, int):
            raise TypeError(f"a count line holds integers, not {value!r}")
    print(PREFIX + " ".join([words, *map(str, numbers)]), flush=True)


def record_set(space):
    """The properties of the space's own `RvtElementRecordGeometry` set, by name:
    read from the space's `IsDefinedBy` alone, as `mcp_answers_vs_ifc.py` reads a
    room's, so a set of its type is never taken for its own."""
    out = {}
    for rel in space.IsDefinedBy or ():
        if not rel.is_a("IfcRelDefinesByProperties"):
            continue
        definition = rel.RelatingPropertyDefinition
        if definition is None or not definition.is_a("IfcPropertySet") or definition.Name != SET:
            continue
        for prop in definition.HasProperties or ():
            if prop.is_a("IfcPropertySingleValue") and prop.NominalValue is not None:
                out[prop.Name] = prop.NominalValue.wrappedValue
    return out


def measure(path):
    """The counts of one export, and up to LIST_LIMIT failing spaces."""
    import ifcopenshell

    counts = collections.Counter()
    failing = []
    found = []
    # The file is held for the whole loop: an entity of a file that has been
    # freed reads as "Instance #N not found" or crashes the interpreter, which
    # is what `ifcopenshell.open(path).by_type(...)` did on run 42016234086.
    ifc = ifcopenshell.open(path)
    for space in ifc.by_type("IfcSpace"):
        props = record_set(space)
        found.append((space.GlobalId, props.get(KIND_PROPERTY), props.get(SOURCE_PROPERTY)))
    counts[SPACES] = len(found)
    # An export with no geometry writes no SpaceOutlineSource on any space: the
    # agreement of kind and source is then not applicable. In a file where some
    # space has one, a space without one is a disagreement, not an exemption.
    checked = any(source is not None for _, _, source in found)
    for gid, kind, source in found:
        is_area_source = source in AREA_SOURCES
        counts[AREA_SOURCE] += is_area_source
        problem = None
        if kind is None:
            counts[NO_KIND] += 1
            problem = "no SpaceKind"
        elif kind not in KINDS:
            counts[OTHER_KIND] += 1
            problem = "SpaceKind of another value"
        else:
            counts[kind] += 1
            if not checked:
                counts[NOT_CHECKED] += 1
            elif kind == "area" and not is_area_source:
                counts[AREA_WITHOUT_SOURCE] += 1
                problem = "area without an Area outline source"
            elif kind != "area" and is_area_source:
                counts[OTHER_WITH_SOURCE] += 1
                problem = "other kind with an Area outline source"
        if problem:
            failing.append((gid, problem, kind, source))
    return counts, failing


def failed(counts):
    return any(counts[name] for name in (NO_KIND, OTHER_KIND, AREA_WITHOUT_SOURCE, OTHER_WITH_SOURCE))


def check_file(path):
    try:
        counts, failing = measure(path)
    except Exception as error:  # an export that cannot be read is a failure, not a pass
        print(f"  could not read {os.path.basename(path)}: {type(error).__name__}: {error}")
        print(PREFIX + "FAIL")
        return 1
    for name in COUNTED:
        line(name, counts[name])
    bad = failed(counts)
    line("FAIL" if bad else "PASS")
    for gid, problem, kind, source in sorted(failing, key=lambda item: item[0])[:LIST_LIMIT]:
        print(f"  {gid}: {problem}; SpaceKind {kind!r}, SpaceOutlineSource {source!r}")
    if len(failing) > LIST_LIMIT:
        print(f"  ... {len(failing) - LIST_LIMIT} more")
    return 1 if bad else 0


LINE = re.compile(r"^" + re.escape(PREFIX) + r"(?P<words>.+?) (?P<n>[0-9]+)$")


def read_result(path):
    """(counts, verdict) of a result file: its `space kinds:` lines. The verdict
    is `PASS`, `FAIL` or None where the file holds none."""
    counts = collections.Counter()
    seen = set()
    verdict = None
    with open(path, encoding="utf-8", errors="replace") as handle:
        for text in handle:
            text = text.rstrip("\n")
            if text in (PREFIX + "PASS", PREFIX + "FAIL"):
                verdict = text[len(PREFIX) :]
                continue
            found = LINE.match(text)
            if found and found["words"] in COUNTED:
                counts[found["words"]] += int(found["n"])
                seen.add(found["words"])
    return counts, verdict, seen


def check_total(results):
    models = 0
    with_spaces = 0
    problems = []
    total = collections.Counter()
    for entry in sorted(os.listdir(results)):
        directory = os.path.join(results, entry)
        # A model or a sample that was exported has the hash of its IFC.
        if not os.path.isfile(os.path.join(directory, "ifc.sha256")):
            continue
        models += 1
        own = os.path.join(directory, "space_kinds_vs_ifc.txt")
        kept = os.path.join(directory, "counts.txt")
        path = own if os.path.isfile(own) else kept if os.path.isfile(kept) else None
        counts, verdict, seen = read_result(path) if path else (collections.Counter(), None, set())
        if verdict is None or set(COUNTED) - seen:
            problems.append("a model has no complete result")
            continue
        if verdict != "PASS":
            problems.append("a model failed")
        if sum(counts[kind] for kind in KINDS) + counts[NO_KIND] + counts[OTHER_KIND] != counts[SPACES]:
            problems.append("a model's kinds do not add up to its spaces")
        with_spaces += counts[SPACES] > 0
        total.update(counts)
    print(f"space kinds across the run: {models} models and samples checked, {with_spaces} with IfcSpaces")
    for name in COUNTED:
        print(f"space kinds across the run: {name} {total[name]}")
    if models == 0:
        problems.append("no model was checked")
    if total[SPACES] == 0:
        problems.append("the run saw no IfcSpace")
    for problem in sorted(set(problems)):
        print(f"space kinds across the run: FAIL, {problem}")
    if problems:
        print("space kinds across the run: FAIL")
        return 1
    print("space kinds across the run: PASS")
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("path", help="an rvt-rs IFC export, or with --total a Measure results directory")
    parser.add_argument("--total", action="store_true", help="sum the results of every model of a run")
    args = parser.parse_args()
    return check_total(args.path) if args.total else check_file(args.path)


if __name__ == "__main__":
    sys.exit(main())
