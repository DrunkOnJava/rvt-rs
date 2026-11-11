#!/usr/bin/env python3
"""What names a railing's type in its `BaseRailing` data object (RE-431,
#1193, W7).

Reads `probes/examples/probe_re431_railing_types.rs`'s output and Revit's IFC
export. Revit's housing export has no `IfcRailingType`: a railing's type is
the second part of its Name (`Railing:KAI-7:1347781`), so that is the oracle.
For each railing of Revit's export the probe lists, it counts:

- in how many the type's name is that of a `StairsRailingAttr` object named
  by a `BaseRailing` copy, by the current copy and by every copy;
- how many `StairsRailingAttr` ids the words of the copies hold together and
  those of the current copy alone (0, 1, 2, ...), which decides whether the
  rule of `railing_type_picks` (B45, RE-153: the one such id) types it;
- how many railings that rule types over the copies together and over the
  current copy alone, and how many of those it types as Revit does;
- the classes of the objects the words of a railing no `StairsRailingAttr`
  id is found for name (the most common ones).

`--list` also prints up to eight railings the union rule leaves untyped.

Usage:

    python3 tools/re/railing_types_vs_ifc.py <probe.txt> <revit.ifc> [--list]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import sys

import ifcopenshell


def read_name(entry):
    """`name@partition` or `name@partition*` (the routed copy) as a triple."""
    name, _, partition = entry.rpartition("@")
    routed = partition.endswith("*")
    number = int(partition.rstrip("*")) if partition.rstrip("*").isdigit() else -1
    return (name, number, routed)


def read_probe(path):
    copies = collections.defaultdict(list)
    for line in open(path, encoding="utf-8", errors="replace"):
        parts = line.split()
        if len(parts) < 6 or parts[0] != "copy":
            continue
        words = {}
        for item in parts[6:]:
            word, _, classes = item.partition(":")
            words[word] = classes
        attrs = {}
        if parts[5] != "-":
            for item in parts[5].split(","):
                attr_id, _, names = item.partition("=")
                attrs[attr_id] = [read_name(entry) for entry in names.split("|")]
        copies[parts[1]].append(
            {
                "partition": parts[2],
                "current": parts[3] == "1",
                "bytes": parts[4],
                "attrs": attrs,
                "words": words,
            }
        )
    return copies


def revit_type_names(path):
    model = ifcopenshell.open(path)
    out = {}
    for railing in model.by_type("IfcRailing"):
        tag = getattr(railing, "Tag", None)
        parts = (railing.Name or "").split(":")
        if tag and len(parts) >= 2:
            out[str(tag)] = parts[1].replace(" ", "~")
    return out


def ids_named(copies):
    """Every StairsRailingAttr id the copies' words hold, with the set of
    names its copies hold ('?' where a copy holds none)."""
    found = collections.defaultdict(set)
    for copy in copies:
        for attr_id, entries in copy["attrs"].items():
            found[attr_id] |= {name for name, _, _ in entries}
    return found


def entries_of(copies, attr_id):
    seen = set()
    for copy in copies:
        seen |= set(copy["attrs"].get(attr_id, []))
    return seen


def named_by(policy, entries):
    """The name a policy gives a type object from its copies' entries, or None."""
    read = [(name, part, routed) for name, part, routed in entries if name != "?"]
    if policy == "agree":
        names = {name for name, _, _ in read}
        return names.pop() if len(names) == 1 else None
    if policy == "latest":
        if not read:
            return None
        newest = max(part for _, part, _ in read)
        names = {name for name, part, _ in read if part == newest}
        return names.pop() if len(names) == 1 else None
    if policy == "routed":
        names = {name for name, _, routed in read if routed}
        return names.pop() if len(names) == 1 else None
    return None


def main():
    args = [arg for arg in sys.argv[1:] if not arg.startswith("--")]
    if len(args) != 2:
        print(__doc__)
        return 2
    listing = "--list" in sys.argv
    probe = read_probe(args[0])
    revit = revit_type_names(args[1])
    counted = collections.Counter()
    attr_counts_all = collections.Counter()
    attr_counts_current = collections.Counter()
    other_classes = collections.Counter()
    untyped = []
    for tag, type_name in sorted(revit.items()):
        held = probe.get(tag)
        if not held:
            counted["railings with no BaseRailing copy listed"] += 1
            continue
        counted["railings scored"] += 1
        current = [copy for copy in held if copy["current"]]
        union = ids_named(held)
        current_ids = ids_named(current)
        attr_counts_all[len(union)] += 1
        attr_counts_current[len(current_ids)] += 1
        counted["railings with several copies"] += len(held) > 1
        counted["railings with no copy marked current"] += not current
        counted["a copy names a type object of Revit's type name"] += any(
            type_name in names for names in union.values()
        )
        counted["the current copy names one of Revit's type name"] += any(
            type_name in names for names in current_ids.values()
        )
        if len(union) == 1:
            ((only, names),) = union.items()
            counted["union rule (one id over all copies) types the railing"] += 1
            entries = entries_of(held, only)
            for policy in ("agree", "latest", "routed"):
                picked = named_by(policy, entries)
                counted[f"  name by the {policy} policy: found"] += picked is not None
                counted[f"  name by the {policy} policy: Revit's"] += picked == type_name
        else:
            untyped.append((tag, type_name, held))
            if not union:
                for copy in held:
                    for word, classes in copy["words"].items():
                        other_classes[classes] += 1
        if len(current_ids) == 1:
            counted["current-copy rule types the railing"] += 1
    print(f"railings in Revit's export: {len(revit)}; with BaseRailing copies listed: {len(probe)}")
    for key, value in counted.items():
        print(f"  {key}: {value}")
    print(f"  StairsRailingAttr ids in the words of all copies (ids: railings): {dict(sorted(attr_counts_all.items()))}")
    print(f"  StairsRailingAttr ids in the words of the current copy: {dict(sorted(attr_counts_current.items()))}")
    print(f"  classes of the words of railings with no attribute id: {other_classes.most_common(8)}")
    if listing:
        for tag, type_name, held in untyped[:8]:
            print(f"untyped {tag}: Revit's type {type_name}")
            for copy in held:
                print(
                    f"  partition {copy['partition']} current {int(copy['current'])} "
                    f"{copy['bytes']} bytes attrs {copy['attrs']}"
                )
    return 0


if __name__ == "__main__":
    sys.exit(main())
