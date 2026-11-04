#!/usr/bin/env python3
"""Record benchmark inputs and check counts without exporting restricted results."""

import collections
import hashlib
import json
import re
import sys
from pathlib import Path


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def inputs(root):
    ordinary = {}
    restricted = collections.defaultdict(list)
    roots = [root / "models", root / "_samples", root / "_corpus/examples/Autodesk"]
    for directory in roots:
        if not directory.exists():
            continue
        for path in sorted(directory.rglob("*")):
            if not path.is_file() or path.suffix.lower() not in (".rvt", ".rfa", ".ifc"):
                continue
            relative = path.relative_to(root)
            if relative.parts[:2] in (("models", "holdout"), ("models", "bimcollab")):
                restricted[relative.parts[1]].append(digest(path))
            else:
                ordinary[str(relative)] = digest(path)
    protected = {}
    for category, hashes in sorted(restricted.items()):
        encoded = "\n".join(sorted(hashes)).encode()
        protected[category] = {"files": len(hashes), "sha256_multiset": hashlib.sha256(encoded).hexdigest()}
    return {"ordinary": ordinary, "restricted": protected}


def exit_status(path):
    with path.open("rb") as stream:
        stream.seek(0, 2)
        stream.seek(max(0, stream.tell() - 16384))
        content = stream.read().decode("utf-8", "replace")
    matches = re.findall(r"(?m)^exit (\d+)\s*$", content)
    return matches[-1] if matches else "missing"


def item_coverage(directory):
    timings = directory / "timings.log"
    checks = collections.Counter()
    if timings.exists():
        for line in timings.read_text().splitlines():
            step = line.split(maxsplit=1)[0]
            if step != "item":
                checks[step] += 1
    statuses = {}
    for path in sorted(directory.iterdir()):
        if not path.is_file():
            continue
        if path.name == "info.exit" or path.name in (
            "rvt-ifc.log", "rvt-gltf.log", "rvt-ifc-revit-sets.log"
        ) or (path.suffix == ".txt" and path.name != "schema.txt"):
            statuses[path.name] = exit_status(path)
    return {"checks": dict(sorted(checks.items())), "statuses": statuses}


def coverage(root):
    ordinary = {}
    protected = {}
    for directory in sorted(root.iterdir()):
        if not directory.is_dir():
            continue
        category = next((s for s in ("holdout", "bimcollab") if directory.name.startswith(s + "-")), None)
        item = item_coverage(directory)
        if category is None:
            ordinary[directory.name] = item
            continue
        bucket = protected.setdefault(category, {"models": 0, "checks": collections.Counter(), "statuses": collections.Counter()})
        bucket["models"] += 1
        bucket["checks"].update(item["checks"])
        bucket["statuses"].update(f"{name}:exit-{status}" for name, status in item["statuses"].items())
    return {"ordinary": ordinary, "restricted": protected}


if __name__ == "__main__":
    if len(sys.argv) != 4 or sys.argv[1] not in ("inputs", "coverage"):
        raise SystemExit("usage: measure-benchmark-manifest.py inputs|coverage ROOT OUTPUT")
    function = inputs if sys.argv[1] == "inputs" else coverage
    Path(sys.argv[3]).write_text(json.dumps(function(Path(sys.argv[2])), indent=2, sort_keys=True) + "\n")
