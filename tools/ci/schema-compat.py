"""Print the breaking changes between two versions of the CLIs' JSON schemas.

    python3 tools/ci/schema-compat.py <base docs/schemas> <head docs/schemas>

The schemas below describe what the CLIs and the Python package print, so
they are read as output contracts: a consumer may rely on every field the
base lists, on every field it requires being there, and on a field's type
and enum values being among the base's. A schema or field that is gone, a
field no longer required, a type or an enum value a field gains, or a
`$ref` that changed is breaking; one that narrows or adds a field is not.
Exits 1 when there is any.
"""

import json
import pathlib
import sys

OUTPUTS = [
    "analyze.schema.json",
    "capability-manifest.schema.json",
    "corpus-report.schema.json",
    "decoded-elements.schema.json",
    "elem-table.schema.json",
    "element-counts.schema.json",
    "element-records.schema.json",
    "export-diagnostics.schema.json",
    "file-metadata.schema.json",
    "history.schema.json",
    "ifc-compare.schema.json",
    "inspect.schema.json",
    "inventory-row.schema.json",
    "schema-diagnostics.schema.json",
    "summary.schema.json",
    "witness-observation.schema.json",
]


def types(node):
    t = node.get("type")
    if t is None:
        return None
    found = set([t] if isinstance(t, str) else t)
    if "number" in found:
        found.add("integer")
    return found


def compare(base, head, path, found):
    if not isinstance(base, dict) or not isinstance(head, dict):
        return
    if base.get("$ref") != head.get("$ref"):
        found.append(f"{path}: $ref {base.get('$ref')} is now {head.get('$ref')}")
    bt, ht = types(base), types(head)
    if bt is not None and (ht is None or ht - bt):
        found.append(f"{path}: may now be {sorted((ht or {'anything'}) - (bt or set()))}")
    if "enum" in base and set(map(json.dumps, head.get("enum", []))) - set(map(json.dumps, base["enum"])):
        found.append(f"{path}: gained enum values")
    if "enum" in base and "enum" not in head:
        found.append(f"{path}: is no longer limited to its enum values")
    for name in base.get("required", []):
        if name not in head.get("required", []):
            found.append(f"{path}: {name} is no longer required")
    for name, child in base.get("properties", {}).items():
        if name not in head.get("properties", {}):
            found.append(f"{path}: {name} removed")
        else:
            compare(child, head["properties"][name], f"{path}.{name}", found)
    for key in ("items", "additionalProperties"):
        if isinstance(base.get(key), dict):
            compare(base[key], head.get(key, {}), f"{path}[{key}]", found)
    for key in ("anyOf", "oneOf"):
        if key in base:
            if len(head.get(key, [])) != len(base[key]):
                found.append(f"{path}: {key} has {len(head.get(key, []))} branches, had {len(base[key])}")
            else:
                for i, (b, h) in enumerate(zip(base[key], head[key])):
                    compare(b, h, f"{path}.{key}[{i}]", found)
    for name, child in base.get("$defs", {}).items():
        if name not in head.get("$defs", {}):
            found.append(f"{path}: $defs.{name} removed")
        else:
            compare(child, head["$defs"][name], f"{path}.$defs.{name}", found)


def main(base_dir, head_dir):
    found = []
    for name in OUTPUTS:
        base = pathlib.Path(base_dir) / name
        head = pathlib.Path(head_dir) / name
        if not base.exists():
            continue
        if not head.exists():
            found.append(f"{name}: removed")
            continue
        compare(json.loads(base.read_text()), json.loads(head.read_text()), name, found)
    for line in found:
        print(f"breaking: {line}")
    if found:
        return 1
    print("The CLIs' JSON schemas have no breaking change against the base.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2]))
