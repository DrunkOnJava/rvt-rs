#!/usr/bin/env python3
"""Score the placed instances of every BuiltInCategory against a Revit IFC export's Tags (RE-319, RE-350).

Reads the `any`, `rule` and `category` lines that
`probes/examples/probe_re319_category_census.rs` prints, one `<kind> <id> <name> <count> <ids>`
each per category, and compares the ElementIds with the numeric `Tag` of every element of
the export, openings and virtual elements left out. For each category it prints the
ElementIds of any decoded frame, of the placed instances (the RE-21 rule) and of the
instances rvt-rs's exporter writes (a 3D body, in the export phase, in a primary design
option), how many of each the export holds, and the IFC classes they land on. Then the
export's elements that no written instance covers, by class and by the category whose
frames hold their id.

The `unplaced` and `refused` lines of `probes/examples/probe_re350_category_stages.rs`
(`<kind> <id> <name> <reason> <count> <ids>`), when the probe prints them, say why the
ids of a category are not written: for each reason, how many ids the export holds as a
`Tag` and the classes they land on.

Usage:

    python3 tools/re/category_census_vs_ifc.py <probe-output.txt> <revit-export.ifc> [--list]

`--list` is what Measure passes to a probe's scorer; it also lists the first ElementIds of each
category that the export lacks.

The lines that start `category census` are counts, IFC class names and built-in category names
only: they are what a model with no licence to redistribute it keeps
(tools/ci/scrub-holdout-results.sh, RE-350). `category census export` counts the export's
elements by class, whether their `Tag` is a number or not; `category census` has a category's
counts; `category census missed` has an export class's elements that no written instance covers.
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.util.element


def parse(path):
    """The `any`, `rule` and `category` sets, and the `unplaced` and `refused` sets by reason.

    The first is kind -> category id -> (name, set of ElementIds); the second is
    kind -> category id -> reason -> set of ElementIds.
    """
    found = {"any": {}, "rule": {}, "category": {}}
    reasons = {"unplaced": {}, "refused": {}}
    for line in open(path):
        kind = line.split(" ", 1)[0]
        if kind in reasons:
            _, category, _name, reason, _count, ids = line.rstrip("\n").split(" ", 5)
            reasons[kind].setdefault(int(category), {})[reason] = {
                int(i) for i in ids.split(",") if i
            }
        elif kind in found:
            _, category, name, _count, ids = line.rstrip("\n").split(" ", 4)
            found[kind][int(category)] = (name, {int(i) for i in ids.split(",") if i})
    return found, reasons


def category_name(element):
    """The category an export gives an element (`Other.Category`), or "-"."""
    value = ifcopenshell.util.element.get_psets(element).get("Other", {}).get("Category")
    return str(value) if value else "-"


def type_entity(element):
    """The entity and PredefinedType of an element's type object in the export, or "-"."""
    type_object = ifcopenshell.util.element.get_type(element)
    if type_object is None:
        return "-"
    return f"{type_object.is_a()}/{getattr(type_object, 'PredefinedType', None) or '-'}"


def safe(text):
    """`text` in the alphabet tools/ci/scrub-holdout-results.sh keeps for `category census` lines."""
    return "".join(
        ch if ch.isascii() and (ch.isalnum() or ch in " _:.,;()=<>/+-") else "_" for ch in text
    )


def class_counts(ids, tags):
    """`IfcWall=3, IfcDoor=1`: the export classes of the ids that are `Tag`s."""
    counts = collections.Counter(tags[i] for i in ids if i in tags)
    return ", ".join(f"{cls}={n}" for cls, n in sorted(counts.items()))


def main(argv):
    listing = "--list" in argv
    argv = [arg for arg in argv if arg != "--list"]
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    found, reasons = parse(argv[1])
    tags = {}
    elements = {}
    export_classes = collections.defaultdict(lambda: [0, 0])
    # The file stays bound while its instances are in use: IfcOpenShell 0.9
    # frees a file its instances outlive (#773).
    model = ifcopenshell.open(argv[2])
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or element.is_a("IfcVirtualElement"):
            continue
        tag = getattr(element, "Tag", None)
        export_classes[element.is_a()][0] += 1
        if tag and tag.isdigit():
            export_classes[element.is_a()][1] += 1
            tags[int(tag)] = element.is_a()
            elements[int(tag)] = element
    tag_set = set(tags)
    print(f"export schema {model.schema}; elements with a numeric Tag: {len(tags)}")
    written = set()
    for _, (_, ids) in found["category"].items():
        written |= ids
    print(
        f"written instances {len(written)}; in the export {len(written & tag_set)}, "
        f"not in it {len(written - tag_set)}, export elements not written {len(tag_set - written)}"
    )
    print("category | name | any | rule (in export) | written (in export, not in export) | export classes of the written, categories of the instances")
    census = []
    for category in sorted(found["any"], reverse=True):
        name, any_ids = found["any"][category]
        _, rule_ids = found["rule"].get(category, (name, set()))
        _, wrote = found["category"].get(category, (name, set()))
        classes = collections.Counter(tags[i] for i in wrote if i in tags)
        shown = ", ".join(f"{cls} {n}" for cls, n in sorted(classes.items()))
        names = collections.Counter(
            category_name(elements[i]) for i in rule_ids if i in tags
        )
        shown += "".join(f" | category {name!r} {n}" for name, n in sorted(names.items()))
        kinds = collections.Counter(type_entity(elements[i]) for i in rule_ids if i in tags)
        shown += "".join(f" | type object {kind} {n}" for kind, n in sorted(kinds.items()))
        print(
            f"{category} | {name} | {len(any_ids)} | {len(rule_ids)} ({len(rule_ids & tag_set)}) | "
            f"{len(wrote)} ({len(wrote & tag_set)}, {len(wrote - tag_set)}) | {shown}"
        )
        if listing and wrote - tag_set:
            print("    not in the export:", ",".join(str(i) for i in sorted(wrote - tag_set)[:20]))
        line = (
            f"category census: {category} {name} any {len(any_ids)}; "
            f"placed {len(rule_ids)} ({len(rule_ids & tag_set)} in export"
        )
        placed_classes = class_counts(rule_ids, tags)
        line += f", {placed_classes})" if placed_classes else ")"
        line += (
            f"; written {len(wrote)} ({len(wrote & tag_set)} in export, "
            f"{len(wrote - tag_set)} not)"
        )
        for kind in ("unplaced", "refused"):
            for reason, ids in sorted(reasons[kind].get(category, {}).items()):
                line += f"; {kind} {reason} {len(ids)} ({len(ids & tag_set)} in export"
                held = class_counts(ids, tags)
                line += f", {held})" if held else ")"
        census.append(safe(line))
    missed = collections.defaultdict(collections.Counter)
    for tag, cls in tags.items():
        if tag in written:
            continue
        holders = [name for _, (name, ids) in found["any"].items() if tag in ids]
        rule_holders = [name for _, (name, ids) in found["rule"].items() if tag in ids]
        if rule_holders:
            where = "rule:" + "/".join(sorted(set(rule_holders)))
        elif holders:
            where = "frame:" + "/".join(sorted(set(holders)))
        else:
            where = "no frame"
        missed[cls][where] += 1
    print("export elements not written, by IFC class and where rvt-rs holds their id:")
    missed_lines = []
    for cls in sorted(missed):
        total = sum(missed[cls].values())
        parts = ", ".join(f"{where} {n}" for where, n in sorted(missed[cls].items()))
        print(f"  {cls}: {total} ({parts})")
        missed_lines.append(safe(f"category census missed: {cls} {total} ({parts})"))
    print(safe(
        f"category census export: schema {model.schema}; elements "
        f"{sum(counts[0] for counts in export_classes.values())}; numeric Tag {len(tags)}"
    ))
    for cls in sorted(export_classes):
        total, numeric = export_classes[cls]
        print(safe(f"category census export: {cls} {total} (numeric Tag {numeric})"))
    for line in census + missed_lines:
        print(line)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
