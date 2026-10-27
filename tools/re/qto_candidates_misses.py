#!/usr/bin/env python3
"""`qto_candidates_list.py --misses` (#1016), for Measure, which runs a
scorer it does not list with only the two IFC files.

Usage:

    python3 tools/re/qto_candidates_misses.py <rvt-rs.ifc> <revit-export.ifc>
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from qto_candidates_list import main  # noqa: E402

if __name__ == "__main__":
    sys.exit(main(sys.argv + ["--misses"]))
