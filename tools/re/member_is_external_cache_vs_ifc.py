#!/usr/bin/env python3
"""Score two readings of revit-ifc's `IsExternal` for curtain mullions and
panels against Revit's export (RE-269, #35, W7).

`probes/examples/probe_is_external_members.rs` prints, per IfcMember and
IfcPlate the export writes, the member's type, its host (the curtain wall its
record names), the host's native Function and the type's own Function entry.
revit-ifc's `CategoryUtil.IsElementExternal` (CLEANROOM.md source 5) gives a
family instance with no `IsExternal` parameter its host's value, and caches
every value under an ElementId in one dictionary:

- host rule: a member takes its curtain wall's value, the wall type's Function
  (external unless 0, interior);
- cache rule: replays IFC_v24.1.0 and IFC_v24.2.0, whose `IsElementExternal`
  also accepts an element type and caches it as not external where it has no
  Function. A curtain wall's wrapper creates the property sets of its members
  first and those of their types after, so the members of the first wall (in
  export order) that uses a type get the wall's value, and every later member
  of that type reads the type's cached false. The order is ascending host
  ElementId (`--order id`, default) or the IFC step order of the
  IfcCurtainWall entities (`--order step`).

It joins each member to Revit's element by Tag, reads `IsExternal` from its
`Pset_*Common`, and prints the file's exporter version (the FILE_NAME
header), the counts for each rule by IFC entity and, with --list, the types
and hosts whose members split and the members on which the cache rule and
Revit differ.

Usage:
    python3 tools/re/member_is_external_cache_vs_ifc.py PROBE.txt REVIT.ifc [--list] [--order id|step]

Needs IfcOpenShell (the version tools/ci/requirements-ifc.txt pins).
"""

import collections
import json
import sys

import ifcopenshell
import ifcopenshell.util.element


def common_is_external(product, inherit=False):
    value = None
    psets = ifcopenshell.util.element.get_psets(product, psets_only=True, should_inherit=inherit)
    for name, properties in psets.items():
        if name.startswith("Pset_") and name.endswith("Common") and "IsExternal" in properties:
            value = bool(properties["IsExternal"])
    return value


def curtain_parent(product):
    for relation in getattr(product, "Decomposes", None) or []:
        parent = getattr(relation, "RelatingObject", None)
        if parent is not None:
            return parent
    return None


def read_probe(path):
    build, members = None, []
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        row = json.loads(line)
        if "member" in row:
            members.append(row)
        elif "build" in row:
            build = row
    return build, members


def host_value(row):
    function = row.get("host_function")
    return None if function is None else function != 0


def host_rule(rows):
    return {row["member"]: host_value(row) for row in rows}


def cache_rule(rows, order_of):
    """Member -> value, replaying the cache in the order of `order_of` (host -> sort key)."""
    by_host = collections.defaultdict(list)
    for row in rows:
        if row.get("host") is not None:
            by_host[row["host"]].append(row)
    out = {}
    cached = {}
    for host in sorted(by_host, key=order_of):
        members = sorted(by_host[host], key=lambda r: r["member"])
        for row in members:
            kind = row.get("type")
            type_function = row.get("type_function")
            if kind in cached:
                out[row["member"]] = cached[kind]
            elif type_function is not None:
                out[row["member"]] = type_function != 0
            else:
                out[row["member"]] = host_value(row)
        for row in members:
            kind = row.get("type")
            type_function = row.get("type_function")
            if kind is not None and kind not in cached:
                cached[kind] = False if type_function is None else type_function != 0
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    order = "id"
    if "--order" in sys.argv:
        order = sys.argv[sys.argv.index("--order") + 1]
        args = [a for a in args if a != order]
    probe_path, reference = args
    build, rows = read_probe(probe_path)
    model = ifcopenshell.open(reference)
    header = model.header.file_name
    print(f"export: schema {model.schema}; originating_system {header.originating_system!r}; "
          f"preprocessor {header.preprocessor_version!r}")
    print(f"file: {build}; probe members: {len(rows)}")

    revit = {}
    for entity in ("IfcMember", "IfcPlate"):
        for product in model.by_type(entity):
            tag = getattr(product, "Tag", None)
            if not tag:
                continue
            parent = curtain_parent(product)
            type_object = ifcopenshell.util.element.get_type(product)
            revit[str(tag)] = {
                "entity": entity,
                "value": common_is_external(product),
                "wall": parent is not None and parent.is_a("IfcCurtainWall"),
                "parent_tag": str(getattr(parent, "Tag", "") or "") if parent is not None else None,
                "parent_step": parent.id() if parent is not None else None,
                "type_tag": str(getattr(type_object, "Tag", "") or "") if type_object is not None else None,
                "type_value": common_is_external(type_object, True) if type_object is not None else None,
            }
    walls = [w for w in model.by_type("IfcCurtainWall") if getattr(w, "Tag", None)]
    wall_step = {str(w.Tag): w.id() for w in walls}
    ids = sorted(int(t) for t in wall_step if t.isdigit())
    steps = [wall_step[str(i)] for i in ids]
    inversions = sum(1 for a, b in zip(steps, steps[1:]) if b < a)
    print(f"Revit: {len(revit)} members and plates with a Tag; {len(walls)} curtain walls; "
          f"curtain wall step order against ElementId order: {inversions} inversions in {len(ids)}")
    kinds = collections.Counter((v["entity"], v["value"]) for v in revit.values())
    print(f"Revit IsExternal: {dict(kinds)}")
    typed = collections.Counter(v["type_value"] for v in revit.values())
    print(f"IsExternal on the members' IFC types: {dict(typed)}")
    mismatch_types = sum(1 for tag, v in revit.items() if v["type_tag"])
    print(f"members with a type Tag: {mismatch_types}")

    joined = [(row, revit[str(row["member"])]) for row in rows if str(row["member"]) in revit]
    print(f"joined {len(joined)} of {len(rows)} probe members")
    wall_rows = [row for row, rev in joined if row.get("host") is not None]
    print(f"with a host: {len(wall_rows)}; on a curtain wall in Revit's export: "
          f"{sum(1 for row, rev in joined if rev['wall'])}; "
          f"host classes {dict(collections.Counter(row.get('host_class') for row in wall_rows))}")

    if order == "step":
        step_of = {}
        for row, rev in joined:
            if row.get("host") is not None and rev["parent_step"] is not None:
                step_of[row["host"]] = rev["parent_step"]
        order_of = lambda host: (step_of.get(host, 1 << 60), host)
    else:
        order_of = lambda host: host
    plain = host_rule([row for row, _ in joined])
    replay = cache_rule([row for row, _ in joined], order_of)

    tally = collections.Counter()
    by_type = collections.defaultdict(collections.Counter)
    by_host = collections.defaultdict(collections.Counter)
    differing = []
    for row, rev in joined:
        tag = row["member"]
        for name, rule in (("host rule", plain), ("cache rule", replay)):
            decided = rule.get(tag)
            if decided is None:
                outcome = "undecided"
            elif rev["value"] is None:
                outcome = "no Revit value"
            elif decided == rev["value"]:
                outcome = "equal"
            else:
                outcome = "different"
            tally[(rev["entity"], name, outcome)] += 1
            if name == "cache rule":
                if outcome == "different":
                    differing.append((rev["entity"], tag, row.get("type"), row.get("host"), rev["value"], decided))
                by_type[row.get("type")][(rev["value"], outcome)] += 1
                by_host[row.get("host")][(rev["value"], outcome)] += 1
    for entity in ("IfcMember", "IfcPlate"):
        total = sum(1 for _, rev in joined if rev["entity"] == entity)
        print(f"\n{entity}: {total} paired")
        for name in ("host rule", "cache rule"):
            counts = {o: tally[(entity, name, o)] for o in ("equal", "different", "undecided", "no Revit value")}
            print(f"  {name:12} {counts}")
    if listing:
        print("\nby member type (Revit true/false; cache rule outcome):")
        for kind, counts in sorted(by_type.items(), key=lambda kv: -sum(kv[1].values())):
            print(f"  type {kind}: {dict(counts)}")
        mixed = {h: c for h, c in by_host.items() if len({v for v, _ in c}) > 1}
        print(f"\nhosts whose members split: {len(mixed)}")
        for host, counts in sorted(mixed.items(), key=lambda kv: str(kv[0]))[:40]:
            print(f"  host {host}: {dict(counts)}")
        print(f"\n{len(differing)} members where the cache rule and Revit differ:")
        for entity, tag, kind, host, value, decided in differing[:60]:
            print(f"  {entity} {tag} type {kind} host {host}: Revit {value}, rule {decided}")


main()
