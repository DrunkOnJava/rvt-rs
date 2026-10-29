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

With --rules it runs IfcOpenShell's check of the schema's EXPRESS rules (WHERE
rules and global rules, `ifcopenshell.express.rule_executor`) instead, which
the default validation leaves out: every reference export passes that one, so
it tells no export from another. It prints the findings by kind, the message
with its instance numbers replaced by `#`, most frequent first:

    findings <n>
    <count> <entity> <attribute or rule>: <message>     (at most 30)

Usage: ifc_validate_summary.py [--rules] MODEL.ifc [STATEMENTS.jsonl]
"""

import collections
import json
import re
import sys

import ifcopenshell
import ifcopenshell.express.rule_executor
import ifcopenshell.validate


def describe(statement):
    instance = statement.get("instance")
    entity = instance.is_a() if hasattr(instance, "is_a") else str(instance or "")
    attribute = statement.get("attribute") or statement.get("type") or ""
    message = " ".join(str(statement.get("message", "")).split())
    return f"{entity} {attribute}: {message}"[:300]


def rules(path):
    logger = ifcopenshell.validate.json_logger()
    ifcopenshell.express.rule_executor.run(ifcopenshell.open(path), logger)
    kinds = collections.Counter(re.sub(r"#\d+", "#", describe(s)) for s in logger.statements)
    print(f"findings {len(logger.statements)}")
    for kind, count in sorted(kinds.items(), key=lambda item: (-item[1], item[0]))[:30]:
        print(f"{count} {kind}")


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
    args = sys.argv[1:]
    with_rules = args[:1] == ["--rules"]
    if with_rules:
        args = args[1:]
    if len(args) not in (1, 2) or (with_rules and len(args) != 1):
        sys.exit(__doc__)
    try:
        if with_rules:
            rules(args[0])
        else:
            main(args[0], args[1] if len(args) == 2 else None)
    except Exception as error:  # an unreadable file is a finding too
        print(f"unreadable: {type(error).__name__}: {json.dumps(str(error))[:300]}")
