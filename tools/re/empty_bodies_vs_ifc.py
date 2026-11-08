#!/usr/bin/env python3
"""Score a rule that leaves out family instances whose saved body has no surface (RE-363, #1496).

Reads what `probes/examples/probe_re363_empty_bodies.rs` prints for a model: for each category the
ids of the family instances rvt-rs writes that the probe analysed (`analysed`), and of those the
ids whose saved graphics at the Fine detail level decode to no surface with no diagnostic at all
(`empty ... empty_none`) or with only the view-component or nested-detail-filter diagnostics
(`empty ... empty_view_only`), and of the empty ones how many branches the walk left out as
flagged not visible (`emptyinfo`): the third rule drops an `empty_none` instance only where some
branch was, so an instance with no saved graphics at all is kept. It joins them with the numeric `Tag`s of Revit's own export and says,
for each category and in total, how many instances in Revit's export the rule would drop (a false
negative: an element Revit writes would be lost) and how many of those Revit's export lacks it
would drop. A door or a window is counted apart: revit-ifc's `FamilyInstanceExporter` writes one
with no geometry, so a rule would not touch it.

Usage:

    python3 tools/re/empty_bodies_vs_ifc.py <probe-output.txt> <revit-export.ifc> [--list]

`--list` is what Measure passes to a probe's scorer; it also lists the ids of each category that the
rule would drop.

The lines that start `re363 ` are counts, built-in category names, rule names and the IFC schema
name only: they are what a model with no licence to redistribute it keeps
(tools/ci/scrub-holdout-results.sh, RE-363).
"""

import collections
import sys

DOORS_AND_WINDOWS = {-2000023, -2000014}
RULES = (
    "empty_none",
    "empty_none_or_view_only",
    "empty_none_and_hidden_branches",
    "type_level_empty_hidden",
)


def parse(path):
    """(analysed, empty, names, notes, hidden): the ids analysed by category, the empty ids by
    (category, outcome), the category names, the other lines and, for each empty id, how many
    branches of its graphics the walk left out as flagged not visible."""
    analysed = {}
    empty = collections.defaultdict(set)
    names = {}
    notes = []
    hidden = {}
    for raw in open(path):
        line = raw.rstrip("\n")
        head = line.split(" ", 1)[0]
        if head == "analysed":
            _, category, name, _count, ids = (line.split(" ", 4) + [""])[:5]
            analysed[int(category)] = {int(i) for i in ids.split(",") if i}
            names[int(category)] = name
        elif head == "empty":
            _, category, name, outcome, _count, ids = (line.split(" ", 5) + [""])[:6]
            empty[(int(category), outcome)] = {int(i) for i in ids.split(",") if i}
            names[int(category)] = name
        elif head == "typedrop":
            _, category, name, _count, ids = (line.split(" ", 4) + [""])[:5]
            empty[(int(category), "typedrop")] = {int(i) for i in ids.split(",") if i}
            names[int(category)] = name
        elif head == "emptyinfo":
            word = tokens(line)
            hidden[int(word["id"])] = int(word.get("vis_branches", "0"))
        elif head in ("truncated", "written", "done", "release", "timing", "memory"):
            notes.append(line)
    return analysed, empty, names, notes, hidden


def tokens(line):
    """The `key=value` words of a line, as a dict."""
    return dict(word.split("=", 1) for word in line.split()[1:] if "=" in word)


def dropped_by(rule, category, empty, hidden):
    """The ids of `category` that `rule` leaves out."""
    none = empty.get((category, "empty_none"), set())
    if rule == "empty_none":
        return set(none)
    if rule == "empty_none_or_view_only":
        return none | empty.get((category, "empty_view_only"), set())
    if rule == "type_level_empty_hidden":
        return set(empty.get((category, "typedrop"), set()))
    return {i for i in none if hidden.get(i, 0) > 0}


def safe(text):
    """`text` in the alphabet tools/ci/scrub-holdout-results.sh keeps for `re363` lines."""
    return "".join(
        ch if ch.isascii() and (ch.isalnum() or ch in " _:.,;()=#<>/+-") else "_" for ch in text
    )


def export_tags(path):
    """(schema, {Tag: IFC class}) of the elements of Revit's export."""
    import ifcopenshell

    model = ifcopenshell.open(path)
    tags = {}
    for element in model.by_type("IfcElement"):
        if element.is_a("IfcOpeningElement") or element.is_a("IfcVirtualElement"):
            continue
        tag = getattr(element, "Tag", None)
        if tag and tag.isdigit():
            tags[int(tag)] = element.is_a()
    return model.schema, tags


def main(argv):
    listing = "--list" in argv
    argv = [arg for arg in argv if arg != "--list"]
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    analysed, empty, names, notes, hidden = parse(argv[1])
    schema, tags = export_tags(argv[2])
    truncated = [line for line in notes if line.startswith("truncated")]
    complete = any(line == "done" for line in notes) and not truncated
    total_analysed = sum(len(ids) for ids in analysed.values())
    print(
        f"export schema {schema}; elements with a numeric Tag: {len(tags)}; analysed family "
        f"instances {total_analysed}; probe {'complete' if complete else 'incomplete'}"
    )
    print(safe(f"re363 empty bodies: schema {schema}; analysed {total_analysed}; complete {'yes' if complete else 'no'}"))
    for rule in RULES:
        totals = {"other": collections.Counter(), "door_window": collections.Counter()}
        for category in sorted(analysed):
            ids = analysed[category]
            dropped = dropped_by(rule, category, empty, hidden)
            held = ids & set(tags)
            wrong = dropped & set(tags)
            right = dropped - set(tags)
            scope = "door_window" if category in DOORS_AND_WINDOWS else "other"
            bucket = totals[scope]
            bucket["analysed"] += len(ids)
            bucket["in_export"] += len(held)
            bucket["not_in_export"] += len(ids - held)
            bucket["drops"] += len(dropped)
            bucket["wrong"] += len(wrong)
            bucket["right"] += len(right)
            if dropped or (ids - held):
                print(
                    safe(
                        f"re363 empty bodies {rule}: category {category} {names.get(category, '-')}; "
                        f"schema {schema}; analysed {len(ids)}; in export {len(held)}; not in export "
                        f"{len(ids - held)}; rule drops {len(dropped)}; dropped in export {len(wrong)}; "
                        f"dropped not in export {len(right)}"
                    )
                )
            if rule == "type_level_empty_hidden":
                by_instance = dropped_by("empty_none_and_hidden_branches", category, empty, hidden)
                if dropped or by_instance:
                    print(
                        safe(
                            f"re363 empty bodies type level against instance level: category "
                            f"{category} {names.get(category, '-')}; schema {schema}; instance "
                            f"level drops {len(by_instance)}; type level drops {len(dropped)}; "
                            f"both {len(dropped & by_instance)}; type level only "
                            f"{len(dropped - by_instance)} (in export "
                            f"{len((dropped - by_instance) & set(tags))}); instance level only "
                            f"{len(by_instance - dropped)} (in export "
                            f"{len((by_instance - dropped) & set(tags))})"
                        )
                    )
            if listing and dropped:
                print(
                    f"  {rule} category {category} drops: in export "
                    f"{','.join(map(str, sorted(wrong)[:30])) or '-'}; not in export "
                    f"{','.join(map(str, sorted(right)[:30])) or '-'}"
                )
        for scope, bucket in totals.items():
            print(
                safe(
                    f"re363 empty bodies {rule} total {'doors and windows' if scope == 'door_window' else 'other categories'}: "
                    f"schema {schema}; analysed {bucket['analysed']}; in export {bucket['in_export']}; "
                    f"not in export {bucket['not_in_export']}; rule drops {bucket['drops']}; "
                    f"dropped in export {bucket['wrong']}; dropped not in export {bucket['right']}; "
                    f"not in export and kept {bucket['not_in_export'] - bucket['right']}"
                )
            )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
