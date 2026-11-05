#!/usr/bin/env python3
"""Check every structured answer the MCP server's end-to-end test was given
against the `outputSchema` its tool declared in the same run (#1415).

`rvt-mcp/tests/mcp_stdio.rs`, run with `RVT_MCP_ANSWERS_DIR=<dir>`, writes
each tool's schema under `<dir>/schemas/<tool>/` (one file per session that
listed the tools) and each answer under `<dir>/answers/<tool>/`. A tool's
schema must be the same in every session, every tool that answered must have
one, and every answer must validate. A field the schema requires but the
answer leaves out fails here: the output schema is what a client that
validates `structuredContent` holds the server to.

Usage: validate-mcp-answers.py <dir>
"""

import json
import sys
from pathlib import Path

import jsonschema


def load_all(folder: Path) -> list[tuple[Path, object]]:
    return [(path, json.loads(path.read_text())) for path in sorted(folder.glob("*.json"))]


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__.strip().splitlines()[-1], file=sys.stderr)
        return 2
    root = Path(sys.argv[1])
    schemas_root, answers_root = root / "schemas", root / "answers"
    if not answers_root.is_dir() or not schemas_root.is_dir():
        print(f"error: no answers or no schemas under {root}: the test kept nothing")
        return 1
    failures = 0
    schemas: dict[str, dict] = {}
    for folder in sorted(p for p in schemas_root.iterdir() if p.is_dir()):
        copies = load_all(folder)
        if not copies:
            print(f"{folder.name}: no outputSchema file in its folder")
            failures += 1
            continue
        first = copies[0][1]
        if any(copy != first for _, copy in copies[1:]):
            print(f"{folder.name}: its outputSchema differs between sessions")
            failures += 1
        jsonschema.validators.validator_for(first).check_schema(first)
        schemas[folder.name] = first
    answered = sorted(p for p in answers_root.iterdir() if p.is_dir())
    if not answered:
        print("error: no tool answered")
        return 1
    for folder in answered:
        tool = folder.name
        schema = schemas.get(tool)
        answers = load_all(folder)
        if schema is None:
            print(f"{tool}: {len(answers)} answers and no outputSchema")
            failures += 1
            continue
        validator = jsonschema.validators.validator_for(schema)(schema)
        bad = 0
        for path, answer in answers:
            errors = sorted(validator.iter_errors(answer), key=lambda e: list(e.absolute_path))
            if errors:
                bad += 1
                for error in errors[:5]:
                    where = "/".join(str(part) for part in error.absolute_path) or "(root)"
                    print(f"{tool}: {path.name}: at {where}: {error.message}")
        print(f"{tool}: {len(answers)} answers, {len(answers) - bad} valid against its outputSchema")
        failures += bad
    unanswered = sorted(set(schemas) - {p.name for p in answered})
    if unanswered:
        print(f"tools with a schema and no answer in this run: {', '.join(unanswered)}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
