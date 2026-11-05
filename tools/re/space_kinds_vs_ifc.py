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
  the outline source is one of `AREA_SOURCES`, and no other kind may hold one:
  an `area` whose source is not one of them, and a space of another kind with
  one, are each counted;
- the spaces with an outline source that this scorer does not know, of any
  kind: a value not in `AREA_SOURCES` or `ROOM_SOURCES`, which is a failure, so
  that a release that writes a new value has it listed here, with the kind it
  belongs to, before it passes. These two lists are the one place the values are
  kept.

An export with no geometry (`typed_no_geometry`) has no `SpaceOutlineSource` on
any space. The agreement is then not checked, and the spaces it was not checked
for are counted on a line of their own (`--total` allows that only where the
export's own diagnostics say it has no geometry).

The file passes (exit 0) when no space lacks a kind, none has another value,
none has an outline source this scorer does not know and none disagrees; a file
with no `IfcSpace` passes with a count of zero spaces, because there is nothing
in it to check. Any other result exits 1. The lines starting `space kinds:` are
made of fixed words and numbers, so the Measure workflow keeps them for the
models it keeps only counts of (tools/ci/scrub-holdout-results.sh): `line` takes
words written in this file and integers and raises on anything else. A space
that fails is listed under them with its GlobalId, indented, and those lines are
not kept.

A run that checks nothing must not pass. `--total` reads the results of a whole
Measure run, `<dir>/<model>/space_kinds_vs_ifc.txt` or, for a model kept as
counts only, the `space kinds:` lines of `<dir>/<model>/counts.txt`, and sums
them. It judges each model on its counts, not on its verdict word, and exits 1
when:

- a model that was exported has no complete result;
- a model has any space without a kind, with another kind, with an outline
  source this scorer does not know, or in disagreement, whatever its verdict says,
  or the kinds do not add up to its spaces;
- a model's spaces were not checked for agreement (no `SpaceOutlineSource`) and
  its `diagnostics.json` does not say `confidence.level` is `typed_no_geometry`;
- a model's spaces are not the number the export itself counted
  (`diagnostics.json` `exported.by_ifc_type.IFCSPACE`, or for a model kept as
  counts only the MCP server's `rooms` count): so a result that saw no space in
  a model that has them fails, and a run of models with none, such as a filtered
  one, does not;
- the run saw no `IfcSpace` and some model had no count of its own to check
  against, or no model was checked.

It prints how many of each kind the run saw.

Usage:

    python3 tools/re/space_kinds_vs_ifc.py <rvt-rs.ifc>
    python3 tools/re/space_kinds_vs_ifc.py --total <measure results directory>

The first needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins);
`--total` needs only Python.
"""

import argparse
import collections
import json
import os
import re
import sys

SET = "RvtElementRecordGeometry"
KIND_PROPERTY = "SpaceKind"
SOURCE_PROPERTY = "SpaceOutlineSource"
KINDS = ("room", "area", "mep_space", "family_instance")
# The values of `SpaceOutlineSource` of an Area. `area_plan_topology_unchecked`
# is #1428's value for the Areas of Revit 2025 to 2027 (Griffin, 2026-11-05,
# "Mark 2025 to 2027 in the file"): a reader that never writes it, as main's,
# cannot fail on it.
AREA_SOURCES = (
    "area_boundary",
    "area_boundary_joined",
    "area_plan_topology",
    "area_plan_topology_unchecked",
    "none",
)
# The values of the others: rooms, Revit Spaces and family instances.
ROOM_SOURCES = (
    "stored_solid",
    "stored_solid_faces",
    "stored_solid_approximate",
    "record_box",
    "saved_mesh",
    "family_body",
)
LIST_LIMIT = 25
PREFIX = "space kinds: "

# The words of each count line, in the order they are printed. `--total` reads
# the same words back, so the two cannot drift.
SPACES = "IfcSpace"
NO_KIND = "no SpaceKind"
OTHER_KIND = "SpaceKind of another value"
UNKNOWN_SOURCE = "SpaceOutlineSource this scorer does not know"
AREA_SOURCE = "SpaceOutlineSource of an Area"
AREA_WITHOUT_SOURCE = "area without an Area outline source"
OTHER_WITH_SOURCE = "other kind with an Area outline source"
NOT_CHECKED = "spaces whose outline source agreement was not checked"
COUNTED = (
    SPACES,
    *KINDS,
    NO_KIND,
    OTHER_KIND,
    UNKNOWN_SOURCE,
    AREA_SOURCE,
    AREA_WITHOUT_SOURCE,
    OTHER_WITH_SOURCE,
    NOT_CHECKED,
)
# What fails a model, whatever its verdict word says.
FAILURES = (NO_KIND, OTHER_KIND, UNKNOWN_SOURCE, AREA_WITHOUT_SOURCE, OTHER_WITH_SOURCE)

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


def judge(kind, source, checked, counts):
    """Count one space and return what is wrong with it, or None."""
    is_area_source = source in AREA_SOURCES
    counts[AREA_SOURCE] += is_area_source
    if kind is None:
        counts[NO_KIND] += 1
        return "no SpaceKind"
    if kind not in KINDS:
        counts[OTHER_KIND] += 1
        return "SpaceKind of another value"
    counts[kind] += 1
    if not checked:
        counts[NOT_CHECKED] += 1
        return None
    if source is not None and not is_area_source and source not in ROOM_SOURCES:
        counts[UNKNOWN_SOURCE] += 1
        return "an Area value this scorer does not know" if kind == "area" else "an outline source this scorer does not know"
    if kind == "area" and not is_area_source:
        counts[AREA_WITHOUT_SOURCE] += 1
        return "area without an Area outline source"
    if kind != "area" and is_area_source:
        counts[OTHER_WITH_SOURCE] += 1
        return "other kind with an Area outline source"
    return None


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
        problem = judge(kind, source, checked, counts)
        if problem:
            failing.append((gid, problem, kind, source))
    return counts, failing


def failed(counts):
    return any(counts[name] for name in FAILURES)


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
MCP_ROOMS = re.compile(r"^mcp answers: rooms: (?P<n>[0-9]+)$")


def read_result(path):
    """(counts, verdict, the words seen, the MCP server's count of spaces) of a
    result file: its `space kinds:` lines, and for a model kept as counts only
    the `mcp answers: rooms:` line. The verdict is `PASS`, `FAIL` or None where
    the file holds none."""
    counts = collections.Counter()
    seen = set()
    verdict = None
    mcp_rooms = None
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
                continue
            found = MCP_ROOMS.match(text)
            if found:
                mcp_rooms = int(found["n"])
    return counts, verdict, seen, mcp_rooms


def read_diagnostics(directory):
    """(the export's own count of `IfcSpace`s, whether it says it has no geometry)
    from `diagnostics.json`, or (None, False) where there is none to read."""
    try:
        with open(os.path.join(directory, "diagnostics.json"), encoding="utf-8") as handle:
            diagnostics = json.load(handle)
        spaces = diagnostics["exported"]["by_ifc_type"].get("IFCSPACE", 0)
        no_geometry = diagnostics.get("confidence", {}).get("level") == "typed_no_geometry"
        return spaces, no_geometry
    except (OSError, ValueError, KeyError, AttributeError, TypeError):
        return None, False


def check_total(results):
    models = 0
    with_spaces = 0
    unchecked_counts = 0
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
        counts, verdict, seen, mcp_rooms = read_result(path) if path else (collections.Counter(), None, set(), None)
        if verdict is None or set(COUNTED) - seen:
            problems.append("a model has no complete result")
            continue
        # The model is judged on its counts: a verdict word that says PASS over a failure count is a failure.
        if verdict != "PASS" or failed(counts):
            problems.append("a model failed")
        if sum(counts[kind] for kind in KINDS) + counts[NO_KIND] + counts[OTHER_KIND] != counts[SPACES]:
            problems.append("a model's kinds do not add up to its spaces")
        exported, no_geometry = read_diagnostics(directory)
        if exported is None:
            exported = mcp_rooms
        if counts[NOT_CHECKED] and not no_geometry:
            problems.append("spaces of an export that has geometry have no SpaceOutlineSource to check")
        if exported is None:
            unchecked_counts += 1
        elif exported != counts[SPACES]:
            problems.append("a model's spaces are not the number the export itself counted")
        with_spaces += counts[SPACES] > 0
        total.update(counts)
    print(f"space kinds across the run: {models} models and samples checked, {with_spaces} with IfcSpaces")
    for name in COUNTED:
        print(f"space kinds across the run: {name} {total[name]}")
    if models == 0:
        problems.append("no model was checked")
    if total[SPACES] == 0 and unchecked_counts:
        problems.append("the run saw no IfcSpace and some models had no count of their own to check it against")
    elif total[SPACES] == 0 and models:
        print("space kinds across the run: no IfcSpace in the run, and the exports' own counts agree")
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
