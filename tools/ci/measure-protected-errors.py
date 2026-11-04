#!/usr/bin/env python3
"""Count protected scorer failure types without exporting model data."""

import collections
import json
import re
import sys
from pathlib import Path

ERROR_TYPES = {
    "AssertionError", "AttributeError", "FileNotFoundError", "ImportError",
    "IndexError", "KeyError", "ModuleNotFoundError", "OSError", "RuntimeError",
    "TypeError", "ValueError", "ZeroDivisionError",
}
OWN_FRAME = re.compile(r'File "[^\n"]*/tools/re/(ports_vs_ifc|slab_layers_vs_ifc|wall_layers_vs_ifc|ifc_common)\.py", line (\d+)')


def count_errors(root):
    counts = collections.Counter()
    for directory in root.iterdir():
        category = next((name for name in ("holdout", "bimcollab")
                         if directory.is_dir() and directory.name.startswith(name + "-")), None)
        if category is None:
            continue
        for scorer in ("ports_vs_ifc", "slab_layers_vs_ifc"):
            path = directory / (scorer + ".txt")
            if not path.exists():
                continue
            content = path.read_text(errors="replace")
            exits = re.findall(r"(?m)^exit (\d+)\s*$", content)
            if not exits or exits[-1] == "0":
                continue
            names = re.findall(r"(?m)^([A-Za-z_][A-Za-z0-9_]*)(?::|$)", content)
            error = next((name for name in reversed(names) if name in ERROR_TYPES), "other")
            frames = OWN_FRAME.findall(content)
            frame = f"{frames[-1][0]}.py:{frames[-1][1]}" if frames else "unavailable"
            counts[(category, scorer, exits[-1], error, frame)] += 1
    return [dict(category=c, scorer=s, exit=code, error_type=error, own_frame=frame, count=count)
            for (c, s, code, error, frame), count in sorted(counts.items())]


if __name__ == "__main__":
    if len(sys.argv) != 3:
        raise SystemExit("usage: measure-protected-errors.py RESULTS_DIR OUTPUT")
    Path(sys.argv[2]).write_text(json.dumps(count_errors(Path(sys.argv[1])), indent=2, sort_keys=True) + "\n")
