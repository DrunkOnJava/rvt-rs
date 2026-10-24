#!/usr/bin/env python3
"""Where a port's flow direction comes from, against Revit's export (RE-212,
#834).

Takes the output of `examples/probe_re212_connector_data.rs` and Revit's IFC
export, whose ports are named `InPort_`, `OutPort_` or
`Port_<ElementId>_<index>` by their direction (revit-ifc
`ConnectorExporter`). For every port it applies the rule rvt-rs writes and
reports how many have Revit's prefix:

- a family instance's HVAC or piping connector takes the Flow Direction its
  family sets in the connector's own `FamilyParams` (-1140248 piping,
  -1140219 HVAC; 1 In, 2 Out), and where the family leaves it
  bidirectional (0), the `m_eDirection` of the instance connector's
  `MEPFamilyConnectorCalculation*` object;
- a duct's or pipe's connector takes its `SegmentConnectorCalculation`'s
  `m_eDirection`;
- a connector that is still bidirectional, joined to one with a direction,
  takes the opposite one;
- any other domain is bidirectional.

Pure standard library.

Usage:

    python3 tools/re/port_directions_vs_ifc.py <probe.txt> <revit.ifc>
"""

import collections
import json
import re
import sys

PIPE, DUCT = -1140248, -1140219
NAME = {1: "In", 2: "Out", 0: "-"}


def main():
    probe, ifc = sys.argv[1], sys.argv[2]
    token_index, calculation, joins = {}, {}, {}
    cells, params = collections.defaultdict(list), collections.defaultdict(list)
    symbol_family, instance_symbol = {}, {}
    for line in open(probe, encoding="utf-8", errors="replace"):
        kind, _, rest = line.rstrip("\n").partition(" ")
        if kind == "object":
            owner, cls, _tag, start, token, data = rest.split(" ", 5)
            owner, fields = int(owner), json.loads(data)
            if cls == "Connector":
                token_index[(owner, int(token))] = fields["m_nIndex"]
                joins[(owner, fields["m_nIndex"])] = [
                    (r["m_id"]["m_id"]["m_id64"], r["m_nIndex"]) for r in fields.get("m_arrRefs", []) if r["m_connType"] == 1
                ]
            elif cls == "SegmentConnectorCalculation" or cls.startswith("MEPFamilyConnectorCalculation"):
                calculation[(owner, fields["m_pConnector"]["pointer_token"])] = fields.get("m_eDirection")
            elif cls == "FamilyParams":
                params[owner].append((int(start), {p["m_paramId"]["m_id"]["m_id64"]: p["m_int"] for p in fields.get("m_params", [])}))
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
    calculated = {(o, token_index[(o, t)]): d for (o, t), d in calculation.items() if (o, t) in token_index}
    family_direction = {}
    for family, family_cells in cells.items():
        family_cells.sort(key=lambda c: c["m_oRelevantParams"]["offset"])
        first = family_cells[0]["m_oRelevantParams"]["offset"]
        owned = [p for start, p in sorted(params.get(family, [])) if start > first]
        if len(owned) != len(family_cells):
            continue
        for cell, owned_params in zip(family_cells, owned):
            wanted = PIPE if cell["m_domain"] == 3 else DUCT if cell["m_domain"] == 1 else None
            family_direction[(family, cell["m_index"])] = (cell["m_domain"], owned_params.get(wanted) if wanted else None)

    def own(key):
        symbol = instance_symbol.get(key[0])
        if symbol is None:
            return calculated.get(key)
        domain, value = family_direction.get((symbol_family.get(symbol), key[1]), (None, None))
        if domain is None:
            return None
        if domain not in (1, 3):
            return 0
        return value if value in (1, 2) else calculated.get(key) if value == 0 else None

    revit = {}
    for line in open(ifc, encoding="utf-8", errors="replace"):
        m = re.match(r"#\d+=\s*IFCDISTRIBUTIONPORT\('[^']*',[^,]*,'(In|Out)?Port_(\d+)_(\d+)'", line)
        if m:
            revit[(int(m.group(2)), int(m.group(3)))] = m.group(1) or "-"
    counts = collections.Counter()
    for key, side in sorted(revit.items()):
        source = "segment" if key[0] not in instance_symbol else "family instance"
        direction = own(key)
        if direction == 0:
            directed = [d for d in (own(p) for p in joins.get(key, []) if p[0] != key[0]) if d in (1, 2)]
            if directed:
                direction, source = 3 - directed[0], source + ", opposite its partner"
        if direction is None:
            counts[(source, "not read")] += 1
            continue
        ok = NAME[direction] == side
        counts[(source, "Revit's" if ok else "differs")] += 1
        if not ok:
            print(f"  {key}: rule {NAME[direction]}, Revit {side}")
    print(f"Revit's ports: {len(revit)}")
    for (source, verdict), n in sorted(counts.items()):
        print(f"  {source}: {verdict} {n}")


if __name__ == "__main__":
    main()
