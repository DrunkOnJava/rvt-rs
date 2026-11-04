#!/usr/bin/env python3
"""Count protected scorer completion summaries without exporting their content."""

import collections
import json
import re
import sys
from pathlib import Path

MARKERS = {
    "ports_vs_ifc": re.compile(r"(?m)^ports in both exports: \d+$"),
    "slab_layers_vs_ifc": re.compile(r"(?m)^\d+ floors, roofs and ceilings drawn in layers in the GLB$"),
}


def counts(root):
    result = collections.defaultdict(collections.Counter)
    for directory in root.iterdir():
        category = next((name for name in ("holdout", "bimcollab")
                         if directory.is_dir() and directory.name.startswith(name + "-")), None)
        if category is None:
            continue
        for scorer, marker in MARKERS.items():
            path = directory / (scorer + ".txt")
            if not path.exists():
                continue
            content = path.read_text(errors="replace")
            exits = re.findall(r"(?m)^exit (\d+)\s*$", content)
            result[f"{category}:{scorer}"]["outputs"] += 1
            result[f"{category}:{scorer}"]["exit_zero"] += int(bool(exits) and exits[-1] == "0")
            result[f"{category}:{scorer}"]["summary_present"] += int(bool(marker.search(content)))
    return result


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: measure-protected-summary.py RESULTS_DIR OUTPUT")
    Path(sys.argv[2]).write_text(json.dumps(counts(Path(sys.argv[1])), indent=2, sort_keys=True) + "\n")
