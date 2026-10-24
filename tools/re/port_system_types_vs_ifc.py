#!/usr/bin/env python3
"""Where a port's SystemType comes from, against Revit's export (#834).

Takes the output of `examples/probe_re212_connector_data.rs` and Revit's IFC
export. Revit names each port `InPort_`, `OutPort_` or
`Port_<ElementId>_<index>` (revit-ifc `ConnectorExporter`), and an IFC4
export writes the port's `SystemType` (an `IfcDistributionSystemEnum`) as
its tenth attribute; an IFC2X3 export has no `SystemType`. For every one of
Revit's ports this finds its connector's calculation object
(`SegmentConnectorCalculation` or `MEPFamilyConnectorCalculation*`, keyed by
the connector's `m_nIndex`, as `port_directions_vs_ifc.py` does) and, for a
family instance, its family's connector parameters (`FamilyParams` after
the family's `ConnectorDataCell`s, in cell order). It reports:

- Revit's `SystemType` values by `PredefinedType`;
- for every scalar field of the calculation whose name holds "System", and
  every integer parameter of the family's connector, how its values split
  over Revit's `SystemType` (a field that is the system classification puts
  each value on one `SystemType`).

Pure standard library.

Usage:

    python3 tools/re/port_system_types_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import json
import re
import sys


def attributes(text):
    """The top-level attributes of a STEP instance's argument list."""
    out, depth, quoted, current = [], 0, False, []
    for ch in text:
        if quoted:
            current.append(ch)
            if ch == "'":
                quoted = False
            continue
        if ch == "'":
            quoted = True
        elif ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        elif ch == "," and depth == 0:
            out.append("".join(current).strip())
            current = []
            continue
        current.append(ch)
    out.append("".join(current).strip())
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    listing = "--list" in sys.argv
    probe, ifc = args[0], args[1]
    token_index, calculation = {}, {}
    cells, params = collections.defaultdict(list), collections.defaultdict(list)
    symbol_family, instance_symbol = {}, {}
    system_fields = set()
    for line in open(probe, encoding="utf-8", errors="replace"):
        kind, _, rest = line.rstrip("\n").partition(" ")
        if kind == "object":
            owner, cls, _tag, start, token, data = rest.split(" ", 5)
            owner, fields = int(owner), json.loads(data)
            if cls == "Connector":
                token_index[(owner, int(token))] = fields["m_nIndex"]
            elif cls == "SegmentConnectorCalculation" or cls.startswith("MEPFamilyConnectorCalculation"):
                picked = {
                    f"{cls.split('<')[0]}.{name}": value
                    for name, value in fields.items()
                    if "system" in name.lower() and isinstance(value, (int, float, str, bool))
                }
                system_fields.update(picked)
                calculation[(owner, fields["m_pConnector"]["pointer_token"])] = picked
            elif cls == "FamilyParams":
                params[owner].append(
                    (int(start), {p["m_paramId"]["m_id"]["m_id64"]: p["m_int"] for p in fields.get("m_params", []) if "m_int" in p})
                )
        elif kind == "cell":
            family, data = rest.split(" ", 1)
            cells[int(family)].append(json.loads(data))
        elif kind == "symbol":
            symbol, family, _ = rest.split(" ", 2)
            if family != "-":
                symbol_family[int(symbol)] = int(family)
        elif kind == "instance":
            instance, symbol, _ = rest.split(" ", 2)
            if symbol != "-":
                instance_symbol[int(instance)] = int(symbol)
    calculated = {(o, token_index[(o, t)]): v for (o, t), v in calculation.items() if (o, t) in token_index}
    family_params = {}
    for family, family_cells in cells.items():
        family_cells.sort(key=lambda c: c["m_oRelevantParams"]["offset"])
        first = family_cells[0]["m_oRelevantParams"]["offset"]
        owned = [p for start, p in sorted(params.get(family, [])) if start > first]
        if len(owned) != len(family_cells):
            continue
        for cell, owned_params in zip(family_cells, owned):
            family_params[(family, cell["m_index"])] = owned_params

    revit = {}
    for line in open(ifc, encoding="utf-8", errors="replace"):
        m = re.match(r"#\d+=\s*IFCDISTRIBUTIONPORT\((.*)\);\s*$", line)
        if not m:
            continue
        attrs = attributes(m.group(1))
        name = re.match(r"'(?:In|Out)?Port_(\d+)_(\d+)'", attrs[2]) if len(attrs) > 2 else None
        if not name:
            continue
        predefined = attrs[8].strip(".") if len(attrs) > 8 else "-"
        system = attrs[9].strip(".") if len(attrs) > 9 else "(no SystemType: IFC2X3)"
        revit[(int(name.group(1)), int(name.group(2)))] = (predefined, system)

    print(f"Revit's ports: {len(revit)}")
    by_type = collections.Counter(revit.values())
    for (predefined, system), n in sorted(by_type.items()):
        print(f"  {predefined}: SystemType {system} {n}")

    splits = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    missing = collections.Counter()
    for key, (predefined, system) in sorted(revit.items()):
        calc = calculated.get(key)
        if calc is None:
            missing["no calculation object"] += 1
        else:
            for field, value in calc.items():
                splits[(predefined, field)][value][system] += 1
        symbol = instance_symbol.get(key[0])
        if symbol is not None:
            owned = family_params.get((symbol_family.get(symbol), key[1]))
            if owned is None:
                missing["family instance, connector parameters not matched"] += 1
            else:
                for pid, value in owned.items():
                    splits[(predefined, f"FamilyParams {pid}")][value][system] += 1
    print(f"calculation fields named System: {sorted(system_fields) or 'none'}")
    for reason, n in sorted(missing.items()):
        print(f"  {reason}: {n}")
    print("\nfields whose every value falls on one SystemType (a candidate classification):")
    candidates = []
    for (predefined, field), values in sorted(splits.items(), key=lambda kv: (kv[0][0], str(kv[0][1]))):
        clean = all(len(systems) == 1 for systems in values.values())
        varied = len({s for systems in values.values() for s in systems}) > 1
        total = sum(sum(s.values()) for s in values.values())
        if clean and varied:
            candidates.append((predefined, field))
            mapping = ", ".join(f"{v} -> {next(iter(s))} ({sum(s.values())})" for v, s in sorted(values.items(), key=lambda kv: str(kv[0])))
            print(f"  {predefined} {field} over {total}: {mapping}")
    if not candidates:
        print("  none")
    if listing:
        print("\nevery field, value -> Revit's SystemType counts:")
        for (predefined, field), values in sorted(splits.items(), key=lambda kv: (kv[0][0], str(kv[0][1]))):
            for value, systems in sorted(values.items(), key=lambda kv: str(kv[0])):
                print(f"  {predefined} {field} = {value}: {dict(systems)}")


if __name__ == "__main__":
    main()
