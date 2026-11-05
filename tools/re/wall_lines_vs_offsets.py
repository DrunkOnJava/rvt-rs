#!/usr/bin/env python3
"""Check where each area boundary line lies against the wall it is picked from
(`probe_re332_wall_hosted_lines`) with the offsets the file stores for it
(`probe_re333_area_plan_topology`) (RE-332).

Usage: wall_lines_vs_offsets.py PROBE_RE332.txt PROBE_RE333.txt

The first probe classifies a line by its record box against its host wall's:
`centre` (on the wall's centre line), `face` (half the wall's thickness off it),
`compound` (a box half the thickness across, from the centre line to a face),
`other` (neither) and `nohost` (the line names no wall). The second prints each
line's offset parts, `from-to:offset`, in feet. A line agrees when its stored
offsets are what its class says:

- `centre`, `face` and `compound` on a wall under 1.5 ft thick: every offset is
  0 for `centre`; half the thickness for `face`; 0 and half the thickness, both
  present, for `compound`;
- a curtain wall (the record box is over 3 ft across): every offset within
  0.05 ft of 0;
- `nohost`: no offset other than 0.

Prints the lines by class and stored offsets, and ends `exit 1` when a line
disagrees, a line of one probe is missing from the other, or a line's copies
differ. Counts, ElementIds and feet only.
"""

import collections
import sys

TOLERANCE = 0.001
CURTAIN_THICKNESS = 3.0
CURTAIN_OFFSET = 0.05


def read_host_rows(path):
    rows = {}
    for row in open(path).read().splitlines():
        fields = row.split("\t")
        if fields[0] == "host":
            rows[int(fields[1])] = (fields[8], float(fields[4]))
        elif fields[0] == "nohost":
            rows[int(fields[1])] = ("nohost", None)
    return rows


def read_offset_rows(path):
    rows = {}
    conflicts = set()
    for row in open(path).read().splitlines():
        fields = row.split("\t")
        if fields[0] != "line":
            continue
        parts = []
        if len(fields) > 3:
            for part in fields[3].split():
                span, offset = part.split(":")
                parts.append(float(offset))
        line = int(fields[1])
        if line in rows and rows[line] != parts:
            conflicts.add(line)
        rows[line] = parts
    return rows, conflicts


def agrees(kind, thickness, offsets):
    if kind == "nohost":
        return all(abs(o) <= TOLERANCE for o in offsets)
    if thickness > CURTAIN_THICKNESS:
        return all(abs(o) <= CURTAIN_OFFSET for o in offsets)
    half = thickness / 2.0
    zero = [abs(o) <= TOLERANCE for o in offsets]
    half_off = [abs(abs(o) - half) <= TOLERANCE for o in offsets]
    if kind == "centre":
        return all(zero)
    if kind == "face":
        return bool(offsets) and all(half_off)
    if kind == "compound":
        return any(zero) and any(half_off) and all(z or h for z, h in zip(zero, half_off))
    return False


def main():
    hosts = read_host_rows(sys.argv[1])
    offsets, conflicts = read_offset_rows(sys.argv[2])
    problems = []
    table = collections.Counter()
    for line in sorted(set(hosts) | set(offsets)):
        if line in conflicts:
            problems.append(f"line {line}: copies differ in the offsets probe")
        elif line not in hosts or line not in offsets:
            problems.append(f"line {line}: in only one probe")
        else:
            kind, thickness = hosts[line]
            parts = offsets[line]
            seen = tuple(sorted({round(o, 3) for o in parts}))
            table[(kind, None if thickness is None else round(thickness, 4), seen)] += 1
            if not agrees(kind, thickness or 0.0, parts):
                problems.append(f"line {line}: {kind}, offsets {seen}")
    for (kind, thickness, seen), count in sorted(table.items(), key=str):
        print(f"{count}\t{kind}\tthickness {thickness}\toffsets {seen}")
    with_offset = sum(
        1 for line in offsets if any(abs(o) > TOLERANCE for o in offsets[line])
    )
    compared = sum(table.values())
    if problems:
        for problem in problems:
            print(problem)
        print(f"wall lines vs offsets: failed: {len(problems)} problems in {len(set(hosts) | set(offsets))} lines")
        print("exit 1")
        sys.exit(1)
    print(
        f"wall lines vs offsets: passed: {compared} lines, {with_offset} with an offset, "
        "every position matches its stored offsets"
    )


if __name__ == "__main__":
    main()
