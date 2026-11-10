"""Why bodies are still an element's record box, summed over one Measure run (#1645).

Each export's `diagnostics.json` holds `exported.bounding_box_bodies`, how many of its
bodies are only their element record's box, and `exported.saved_mesh_refusals`, how many
of the boxed elements keep their box for each reason the saved-mesh pass gives (no saved
graphics, no mesh, no triangles, a shared ElementId, or the decoder diagnostics that left
the mesh partial). This sums both over the models and samples whose diagnostics the run
kept (a held-out model keeps counts only, and is not in it) and prints each reason, and
with a base side the count on each side.

Usage: python3 -I box_reasons.py <measure-head directory> [<measure-base directory>]
"""
import collections
import glob
import json
import os
import sys

if not 2 <= len(sys.argv) <= 3:
    sys.exit(__doc__)

WIDTH = 110


def read(root):
    """(models read, record boxes, reasons) of a results directory, or None where it is absent."""
    if not os.path.isdir(root):
        return None
    models = boxes = 0
    reasons = collections.Counter()
    for path in sorted(glob.glob(f"{root}/*/diagnostics.json")):
        try:
            with open(path, encoding="utf-8") as handle:
                exported = json.load(handle)["exported"]
            boxed = int(exported.get("bounding_box_bodies", 0))
            counted = exported.get("saved_mesh_refusals", {})
            if not isinstance(counted, dict):
                continue
        except (OSError, ValueError, KeyError, TypeError):
            continue
        models += 1
        boxes += boxed
        for reason, number in counted.items():
            reasons[reason] += int(number)
    return models, boxes, reasons


def clip(text):
    return text if len(text) <= WIDTH else text[: WIDTH - 3] + "..."


head = read(sys.argv[1])
base = read(sys.argv[2]) if len(sys.argv) == 3 else None
if head is None:
    sys.exit(f"no results directory {sys.argv[1]}")
head_models, head_boxes, head_reasons = head
print(f"record boxes across the run (head): {head_boxes} in {head_models} models and samples whose diagnostics were kept")
if base is not None:
    base_models, base_boxes, base_reasons = base
    print(f"record boxes across the run (base): {base_boxes} in {base_models} models and samples")
else:
    base_reasons = collections.Counter()
explained = sum(head_reasons.values())
print(f"boxes with a saved-mesh reason (head): {explained}; without one: {head_boxes - explained} (a box that was never offered a mesh, such as one of a release before 2023, or no pass ran)")
if not head_reasons and not base_reasons:
    print("no saved_mesh_refusals in any diagnostics: the exports predate #1645")
    sys.exit(0)
print()
print("  count         reason" if base is None else "  base -> head  reason")
for reason in sorted(set(head_reasons) | set(base_reasons), key=lambda r: (-head_reasons[r], r)):
    if base is None:
        print(f"  {head_reasons[reason]:>12}  {clip(reason)}")
    else:
        print(f"  {base_reasons[reason]:>5} -> {head_reasons[reason]:<5}  {clip(reason)}")
