#!/usr/bin/env python3
"""Schema-conformance gate for every instance in an emitted IFC file.

IfcOpenShell's ``open()`` is tolerant: it parses a record that lists
fewer attributes than its entity type declares, and only a consumer
that *reads* the missing attribute finds out, with

    RuntimeError: Index 8 is out of range for variant of size 8

That is how rvt-rs shipped ``IFCCOLUMN`` with eight attributes against
a declared nine for as long as it did (#214). This module closes that
hole by re-deriving the declared attribute count from the IFC4 EXPRESS
schema and comparing it against the count actually written in the STEP
text, for every instance in the file.

Two checks, both fail-closed:

1. **Arity** — the number of top-level attributes written must equal
   ``declaration_by_name(<entity>).attribute_count()``.
2. **PredefinedType** — for entity types where every ``category_map``
   row supplies a value, the attribute must not be null. Types whose
   Revit classes legitimately have no mapped value (``IfcSpace`` via
   ``Area`` / ``Space``, ``IfcBuildingElementProxy`` via
   ``GenericModel``) are not in that set: writing ``$`` there is the
   honest answer, and forcing a value would be an invention.

Usage::

    python tools/ci/ifc_schema_arity.py FILE.ifc [FILE.ifc ...]
"""

from __future__ import annotations

import re
import sys
from collections import Counter
from pathlib import Path

import ifcopenshell
import ifcopenshell.ifcopenshell_wrapper as wrapper


# Entity types whose every `src/ifc/category_map.rs` row carries a
# `predefined_type`, so a null in the emitted file means the writer
# dropped a value it had rather than reporting one it lacked.
PREDEFINED_TYPE_REQUIRED = {
    "IfcBeam",
    "IfcColumn",
    "IfcCovering",
    "IfcMember",
    "IfcSlab",
    "IfcWall",
}

_INSTANCE = re.compile(r"^#(\d+)\s*=\s*([A-Za-z0-9_]+)\s*\((.*)\)\s*;\s*$")


class ArityError(AssertionError):
    """Raised when an emitted instance disagrees with the schema."""


def step_argument_count(args: str) -> int:
    """Count top-level attributes, honouring quotes and aggregates."""
    if not args.strip():
        return 0
    count = 1
    depth = 0
    in_string = False
    index = 0
    while index < len(args):
        char = args[index]
        if in_string:
            if char == "'":
                if index + 1 < len(args) and args[index + 1] == "'":
                    index += 2
                    continue
                in_string = False
        elif char == "'":
            in_string = True
        elif char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
        elif char == "," and depth == 0:
            count += 1
        index += 1
    return count


def check_arity(ifc_path: Path, schema_name: str) -> Counter:
    """Compare every written instance against its schema declaration."""
    schema = wrapper.schema_by_name(schema_name)
    seen: Counter = Counter()
    failures: list[str] = []
    with ifc_path.open("r", encoding="utf-8", errors="replace") as handle:
        for lineno, line in enumerate(handle, start=1):
            match = _INSTANCE.match(line.strip())
            if match is None:
                continue
            _, name, args = match.groups()
            try:
                declared = schema.declaration_by_name(name).as_entity().attribute_count()
            except Exception:  # noqa: BLE001 - schema lookup is the check
                failures.append(f"{ifc_path}:{lineno}: {name} is not in {schema_name}")
                continue
            written = step_argument_count(args)
            seen[name] += 1
            if written != declared:
                failures.append(
                    f"{ifc_path}:{lineno}: {name} wrote {written} attribute(s), "
                    f"{schema_name} declares {declared} — every attribute a type "
                    f"declares must occupy its slot, `$` when unknown"
                )
    if failures:
        raise ArityError(
            "IFC schema arity check failed:\n  " + "\n  ".join(failures[:40])
        )
    return seen


def check_predefined_types(ifc_path: Path) -> Counter:
    """Assert mapped `PredefinedType` values survived to the file."""
    model = ifcopenshell.open(str(ifc_path))
    populated: Counter = Counter()
    failures: list[str] = []
    for ifc_class in sorted(PREDEFINED_TYPE_REQUIRED):
        try:
            instances = model.by_type(ifc_class, include_subtypes=False)
        except Exception:  # noqa: BLE001 - class absent from this schema
            continue
        for instance in instances:
            try:
                value = instance.PredefinedType
            except Exception as exc:  # noqa: BLE001 - short attribute list
                failures.append(f"{ifc_path}: #{instance.id()} {ifc_class}: {exc}")
                continue
            if value is None:
                failures.append(
                    f"{ifc_path}: #{instance.id()} {ifc_class} has a null "
                    f"PredefinedType, but every category_map row for "
                    f"{ifc_class} supplies one"
                )
            else:
                populated[ifc_class] += 1
    if failures:
        raise ArityError(
            "IFC PredefinedType check failed:\n  " + "\n  ".join(failures[:40])
        )
    return populated


def validate(ifc_path: Path) -> None:
    model = ifcopenshell.open(str(ifc_path))
    schema_name = model.schema
    seen = check_arity(ifc_path, schema_name)
    populated = check_predefined_types(ifc_path)
    print(f"IFC schema conformance passed: {ifc_path}")
    print(f"  schema: {schema_name}")
    print(f"  instances checked: {sum(seen.values())} across {len(seen)} entity type(s)")
    if populated:
        summary = ", ".join(f"{k}={v}" for k, v in sorted(populated.items()))
        print(f"  PredefinedType populated: {summary}")
    else:
        print("  PredefinedType populated: none of the required types are present")


def main(argv: list[str]) -> None:
    if len(argv) < 2:
        print("usage: ifc_schema_arity.py FILE.ifc [FILE.ifc ...]", file=sys.stderr)
        raise SystemExit(2)
    for raw in argv[1:]:
        path = Path(raw)
        if not path.is_file():
            raise ArityError(f"IFC file is missing: {path}")
        validate(path)


if __name__ == "__main__":
    main(sys.argv)
