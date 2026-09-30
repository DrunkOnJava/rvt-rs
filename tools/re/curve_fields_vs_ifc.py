#!/usr/bin/env python3
"""Score the curve fields read at a duct's or pipe's `RbsCurveConnectorManager`
anchor against Revit's own IFC export, per element (#96).

Research tool for `reports/element-framing/RE-134-curve-fields-at-the-anchor.md`.
The input is the output of `examples/probe_re134_duct_connectors.rs` on a
model, which prints each duct's and pipe's record, its connector entries and
every anchor with the bytes that follow it. An element's anchor is the nearest
one before its first connector entry, within 2,000 bytes. From it this reads

    +8   f64 width, or a pipe's nominal diameter
    +16  f64 height
    +64  u64 the ElementId of the element's type
    +514 the size text, UTF-16, on a duct

and compares them with what Revit's export gives the same element (same `Tag`):
the type's `Tag`, and the extrusion's profile and depth in feet. It also says
what the probe could not attach to an element.

Needs IfcOpenShell (`pip install ifcopenshell`).

Usage:

    python3 tools/re/curve_fields_vs_ifc.py <probe output> <revit.ifc> [--list]
"""

import argparse
import bisect
import json
import re
import struct
from collections import Counter, defaultdict

import ifcopenshell
import ifcopenshell.util.element as element_util
import ifcopenshell.util.unit as unit

FT = 0.3048
MM = 304.8
ANCHOR_REACH = 2000
DIMENSION_TOLERANCE_FT = 1e-4
WIDTH, HEIGHT, TYPE_ID, SIZE_TEXT = 8, 16, 64, 514


def read_probe(path):
    """Records, entries and anchors from the probe's JSON lines."""
    records, entries, anchors = {}, defaultdict(list), defaultdict(list)
    for line in open(path, encoding="utf-8", errors="replace"):
        line = line.strip()
        if not line.startswith("{"):
            continue
        # Rust's Debug text can hold \u{..} and \0 escapes JSON does not have.
        line = re.sub(r"\\u\{([0-9a-fA-F]+)\}", lambda m: "\\u%04x" % int(m.group(1), 16), line)
        line = re.sub(r'\\(?![\\"/bfnrtu])', r"\\\\", line)
        try:
            row = json.loads(line)
        except ValueError:
            continue
        if "mep" in row:
            records[row["mep"]] = row
        elif "entry" in row:
            entries[row["entry"]].append(row)
        elif row.get("anchor"):
            anchors[row["stream"]].append(row)
    for stream in anchors:
        anchors[stream].sort(key=lambda a: a["at"])
    return records, entries, anchors


def anchor_of(element, entries, anchors):
    """The nearest anchor before one of the element's entries, within reach."""
    best = None
    for entry in entries.get(element, []):
        starts = [a["at"] for a in anchors.get(entry["stream"], [])]
        i = bisect.bisect_right(starts, entry["at"]) - 1
        if i < 0:
            continue
        gap = entry["at"] - starts[i]
        if gap < ANCHOR_REACH and (best is None or gap < best[0]):
            best = (gap, anchors[entry["stream"]][i])
    return best[1] if best else None


def utf16_text(buf, at):
    chars, j = [], at
    while j + 1 < len(buf) and buf[j + 1] == 0 and 0x20 <= buf[j] < 0x7F:
        chars.append(chr(buf[j]))
        j += 2
    return "".join(chars)


def revit_segments(path):
    """Tag -> (type Tag or None, [profile dimensions in feet], extrusion depth in feet or None)."""
    model = ifcopenshell.open(path)
    scale = unit.calculate_unit_scale(model) / FT
    out = {}
    for element in model.by_type("IfcFlowSegment"):
        if not element.Tag:
            continue
        kind = element_util.get_type(element)
        dims, depth, outside = [], None, None
        if element.Representation:
            for representation in element.Representation.Representations:
                for item in representation.Items:
                    if not item.is_a("IfcExtrudedAreaSolid"):
                        continue
                    profile = item.SweptArea
                    depth = float(item.Depth) * scale
                    if profile.is_a("IfcRectangleProfileDef"):
                        dims = [float(profile.XDim) * scale, float(profile.YDim) * scale]
                    elif profile.is_a("IfcCircleProfileDef"):
                        outside = 2 * float(profile.Radius) * scale
        out[element.Tag] = (kind.Tag if kind is not None else None, dims, depth, outside, element.Name or "")
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("probe")
    ap.add_argument("reference")
    ap.add_argument("--list", action="store_true", help="print every element that disagrees")
    args = ap.parse_args()
    records, entries, anchors = read_probe(args.probe)
    reference = revit_segments(args.reference)
    kinds = Counter(r["kind"] for r in records.values())
    if not kinds:
        print("no ducts or pipes in the probe output")
        return
    print(f"ducts in the probe: {kinds['duct']}; pipes: {kinds['pipe']}")
    tally = Counter()
    sizes = Counter()
    misses = []
    for element, record in sorted(records.items()):
        kind = record["kind"]
        tally[kind, "records"] += 1
        anchor = anchor_of(element, entries, anchors)
        if anchor is None:
            misses.append((element, kind, "no anchor within reach of its entries"))
            continue
        tally[kind, "anchored"] += 1
        tail = bytes.fromhex(anchor["tail"])
        width, height = struct.unpack_from("<dd", tail, WIDTH)
        type_id = struct.unpack_from("<Q", tail, TYPE_ID)[0]
        revit = reference.get(str(element))
        if revit is None:
            continue
        tally[kind, "in Revit's export"] += 1
        if revit[0] == str(type_id):
            tally[kind, "type"] += 1
        else:
            misses.append((element, kind, f"type {type_id}, Revit's {revit[0]}"))
        if kind == "duct":
            extents = revit[1] + ([revit[2]] if revit[2] is not None else [])
            found = all(any(abs(d - e) <= DIMENSION_TOLERANCE_FT for e in extents) for d in (width, height))
            tally[kind, "cross-section"] += found
            if not found:
                misses.append((element, kind, f"width {width:.5f} and height {height:.5f} ft, Revit's {extents}"))
            text = utf16_text(tail, SIZE_TEXT)
            want = f"{round(width * MM)}x{round(height * MM)}"
            tally[kind, "size text"] += text == want
            if text != want:
                misses.append((element, kind, f"size text {text!r}, width by height {want}"))
        else:
            tally[kind, "square"] += abs(width - height) < 1e-12
            sizes[round(width * MM)] += 1
    for kind in ("duct", "pipe"):
        n = tally[kind, "records"]
        if not n:
            continue
        print(f"{kind}s with an anchor before their entries: {tally[kind, 'anchored']} of {n}")
        m = tally[kind, "in Revit's export"]
        print(f"{kind}s also in Revit's export: {m}")
        print(f"type id at anchor+{TYPE_ID} is the Tag of Revit's type: {tally[kind, 'type']} of {m}")
    if kinds["duct"]:
        m = tally["duct", "in Revit's export"]
        print(
            f"duct width and height at anchor+{WIDTH} and +{HEIGHT} are two of Revit's profile and depth: "
            f"{tally['duct', 'cross-section']} of {m}"
        )
        print(f"duct size text at anchor+{SIZE_TEXT} is width x height in millimetres: {tally['duct', 'size text']} of {m}")
    if kinds["pipe"]:
        print(f"pipe height equals width at the anchor: {tally['pipe', 'square']} of {tally['pipe', 'anchored']}")
        print("pipe nominal sizes, millimetres:", dict(sorted(sizes.items())))
    if args.list:
        for element, kind, why in misses:
            print(f"  {kind} {element}: {why}")


if __name__ == "__main__":
    main()
