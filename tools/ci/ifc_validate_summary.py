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

With --both MODEL.ifc STATEMENTS.jsonl RULES.txt it does both from one parse
of the file: the default validation's summary on stdout and its findings in
STATEMENTS.jsonl, and what --rules prints in RULES.txt, each exactly as its own
run writes it.

Usage: ifc_validate_summary.py [--rules] MODEL.ifc [STATEMENTS.jsonl]
       ifc_validate_summary.py --both MODEL.ifc STATEMENTS.jsonl RULES.txt
"""

import collections
import contextlib
import io
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


def rules(path, model=None):
    logger = ifcopenshell.validate.json_logger()
    ifcopenshell.express.rule_executor.run(model if model is not None else ifcopenshell.open(path), logger)
    kinds = collections.Counter(re.sub(r"#\d+", "#", describe(s)) for s in logger.statements)
    print(f"findings {len(logger.statements)}")
    for kind, count in sorted(kinds.items(), key=lambda item: (-item[1], item[0]))[:30]:
        print(f"{count} {kind}")


def unreadable(error):
    return f"unreadable: {type(error).__name__}: {json.dumps(str(error))[:300]}"


def both(path, statements, rules_out):
    """The default validation on stdout and the rules check in RULES_OUT, each
    exactly what its own run prints, from one parse of the file: a file that
    does not open, or a check that fails, is the finding of both or of that
    one, as it is when they run apart. It returns 1 when anything failed, which
    the apart runs do not (they print the failure and exit 0), so that Measure's
    cache never keeps a failure that a rerun could mend, a MemoryError say."""
    try:
        model = ifcopenshell.open(path)
    except Exception as error:
        print(unreadable(error))
        with open(rules_out, "w", encoding="utf-8") as out:
            out.write(unreadable(error) + "\n")
        return 1
    failed = 0
    try:
        main(path, statements, model)
    except Exception as error:
        print(unreadable(error))
        failed = 1
    captured = io.StringIO()
    failure = None
    with contextlib.redirect_stdout(captured):
        try:
            rules(path, model)
        except Exception as error:
            failure = error
    with open(rules_out, "w", encoding="utf-8") as out:
        out.write(captured.getvalue() + (unreadable(failure) + "\n" if failure else ""))
    return 1 if failed or failure else 0


def main(path, statements=None, model=None):
    logger = ifcopenshell.validate.json_logger()
    ifcopenshell.validate.validate(model if model is not None else ifcopenshell.open(path), logger)
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
    if args[:1] == ["--both"]:
        if len(args) != 4:
            sys.exit(__doc__)
        sys.exit(both(args[1], args[2], args[3]))
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
        print(unreadable(error))
