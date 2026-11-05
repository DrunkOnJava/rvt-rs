#!/usr/bin/env python3
"""research/re1-models.tsv is the one list CI fetches the RE1 models by (#1457).

research/witness-registry.json carries the same eight files, with their URLs on
`main`, for the witness gates and tools/fetch-reference-models.sh. This fails when
the two disagree about which files exist, or about the sha256 or byte count of one,
so the pins cannot drift apart unseen.

    python3 tools/ci/check-re1-pins.py
"""
import json
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[2]

pins = {}
source = None
for line in (root / "research" / "re1-models.tsv").read_text().splitlines():
    fields = line.split("\t")
    if fields[0] == "source" and len(fields) == 4:
        source = fields[1:]
    elif fields[0] == "file" and len(fields) == 4:
        pins[fields[1]] = (fields[2], int(fields[3]))

registry = {}
for artifact in json.loads((root / "research" / "witness-registry.json").read_text())["artifacts"]:
    url = artifact.get("url", "")
    if "/data/RE1/" in url:
        registry[url.rsplit("/", 1)[1]] = (artifact["sha256"], artifact["bytes"])

problems = []
if source is None or source[0] != "Drshelden/IFC-ECS" or source[2] != "data/RE1":
    problems.append(f"the source row of research/re1-models.tsv is {source}, not Drshelden/IFC-ECS at data/RE1")
for name in sorted(pins.keys() | registry.keys()):
    if name not in pins:
        problems.append(f"{name} is in research/witness-registry.json and not in research/re1-models.tsv")
    elif name not in registry:
        problems.append(f"{name} is in research/re1-models.tsv and not in research/witness-registry.json")
    elif pins[name] != registry[name]:
        problems.append(f"{name}: research/re1-models.tsv has {pins[name]}, research/witness-registry.json has {registry[name]}")
if not pins:
    problems.append("research/re1-models.tsv lists no file")

for problem in problems:
    print(f"::error::{problem}")
if problems:
    sys.exit(1)
print(f"research/re1-models.tsv and research/witness-registry.json agree on {len(pins)} files")
