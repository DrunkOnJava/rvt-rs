#!/usr/bin/env python3
"""IfcOpenShell's schema validation of one IFC file, as a short, stable summary.

Measure (tools/ci/measure-reference-models.sh) runs this on every sample's
export, so the compare job's diff of base against head shows a change in what
the IFC4 schema finds (C9). The output carries no timing and lists findings in
a fixed order:

    findings <n>
    <entity> <attribute or rule>: <message>     (at most 20, sorted)

With STATEMENTS.jsonl, each finding is also written there in full, one JSON
object a line in the validator's order, so another tool reading the same file
(`tools/re/ports_vs_ifc.py --validation`) need not validate it again.

Usage: ifc_validate_summary.py MODEL.ifc [STATEMENTS.jsonl]
"""

import json
import sys

import ifcopenshell
import ifcopenshell.validate


def describe(statement):
    instance = statement.get("instance")
    entity = instance.is_a() if hasattr(instance, "is_a") else str(instance or "")
    attribute = statement.get("attribute") or statement.get("type") or ""
    message = " ".join(str(statement.get("message", "")).split())
    return f"{entity} {attribute}: {message}"[:300]


def main(path, statements=None):
    logger = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(ifcopenshell.open(path), logger)
    if statements:
        with open(statements, "w", encoding="utf-8") as out:
            for statement in logger.statements:
                out.write(json.dumps(statement, default=str) + "\n")
    lines = sorted(describe(s) for s in logger.statements)
    print(f"findings {len(lines)}")
    for line in lines[:20]:
        print(line)


if __name__ == "__main__":
    if len(sys.argv) not in (2, 3):
        sys.exit(__doc__)
    try:
        main(sys.argv[1], sys.argv[2] if len(sys.argv) == 3 else None)
    except Exception as error:  # an unreadable file is a finding too
        print(f"unreadable: {type(error).__name__}: {json.dumps(str(error))[:300]}")
