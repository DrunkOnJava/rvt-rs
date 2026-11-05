#!/usr/bin/env python3
"""Print how many walls of an export take the wall base and end rules of RE-336,
RE-349 and RE-348, from the export's `diagnostics.json` (`rvt-ifc --diagnostics`).

Reads `exported.wall_base_sources`, `exported.wall_end_sources`,
`exported.wall_ends_order_dependent`, `exported.wall_base_refusals` and
`exported.wall_end_refusals` and `exported.wall_end_reads_unmarked`, and prints one line each, in the restricted
alphabet that tools/ci/scrub-holdout-results.sh keeps for a model without a
licence to redistribute it (counts beside fixed words, never an ElementId):

    wall rules: base sources A=1 B=2
    wall rules: end sources C=3
    wall rules: ends order dependent 2
    wall rules: base refusals D=1
    wall rules: end refusals E=2
    wall rules: end reads unmarked F=1

A row with no count prints `none`. A diagnostics file without the counters (an
older rvt-ifc) prints `wall rules: not measured`.

Usage:

    python3 tools/re/wall_rule_counts.py <diagnostics.json>
"""

import json
import re
import sys

NAME = re.compile(r"^[A-Za-z0-9_]+$")


def counts(label, mapping):
    if not mapping:
        return f"wall rules: {label} none"
    parts = []
    for name in sorted(mapping):
        value = mapping[name]
        if not NAME.match(name) or not isinstance(value, int):
            continue
        parts.append(f"{name}={value}")
    return f"wall rules: {label} " + (" ".join(parts) if parts else "none")


def main(path):
    exported = json.load(open(path)).get("exported", {})
    if "wall_end_sources" not in exported:
        print("wall rules: not measured")
        return
    print(counts("base sources", exported.get("wall_base_sources", {})))
    print(counts("end sources", exported.get("wall_end_sources", {})))
    print(f"wall rules: ends order dependent {int(exported.get('wall_ends_order_dependent', 0))}")
    print(counts("base refusals", exported.get("wall_base_refusals", {})))
    print(counts("end refusals", exported.get("wall_end_refusals", {})))
    print(counts("end reads unmarked", exported.get("wall_end_reads_unmarked", {})))


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
