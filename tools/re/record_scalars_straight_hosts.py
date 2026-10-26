#!/usr/bin/env python3
"""`record_scalars_vs_ifc.py` on the records whose host is not an ArcWall
(#1016): which field tells the curtain mullions on straight hosts that
Revit's export writes as anything but extruded solids.

Usage:

    python3 tools/re/record_scalars_straight_hosts.py <probe.txt> <revit.ifc>
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.environ["RECORD_FILTER"] = "H.ArcWall.m_id=-"
import record_scalars_vs_ifc  # noqa: E402

if __name__ == "__main__":
    sys.exit(record_scalars_vs_ifc.main())
