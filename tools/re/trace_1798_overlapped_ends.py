#!/usr/bin/env python3
"""Research trace for #1798 (not to land): `Qto_MemberBaseQuantities`
NetSurfaceArea in rvt-rs's export and Revit's for the curtain mullions RE-441
names as the 4 chain ends under an overlapped axis (3089951, 3089980, 3090009,
3135427), and the count of mullions missing it, so that the 4 a change released
can be matched by tag.

    python3 tools/re/trace_1798_overlapped_ends.py <rvt-rs.ifc> <revit-export.ifc>
"""

import sys

import ifcopenshell
import ifcopenshell.util.element

TAGS = ["3089951", "3089980", "3090009", "3135427"]


def areas(path):
    f = ifcopenshell.open(path)
    out = {}
    for m in f.by_type("IfcMember"):
        if m.Tag:
            out[m.Tag] = ifcopenshell.util.element.get_psets(m).get("Qto_MemberBaseQuantities", {}).get("NetSurfaceArea")
    return out


def main(argv):
    if len(argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    ours, revit = areas(argv[1]), areas(argv[2])
    missing = sorted(t for t, v in revit.items() if v is not None and t in ours and ours[t] is None)
    print(f"trace 1798: mullions with Revit's NetSurfaceArea and none in rvt-rs's: {len(missing)}")
    for tag in TAGS:
        print(f"trace 1798 {tag}: Revit {revit.get(tag, 'absent')} rvt-rs {ours.get(tag, 'absent')}")
    print("trace 1798 missing tags: " + " ".join(missing))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
