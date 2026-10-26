#!/usr/bin/env python3
"""`qto_candidates_list.py --misses` (#1016): the objects a candidate that
holds on nearly every object of a group misses.

Usage:

    python3 tools/re/qto_candidates_misses.py <rvt-rs.ifc> <revit-export.ifc>
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qto_candidates_list  # noqa: E402

if __name__ == "__main__":
    sys.exit(qto_candidates_list.main(sys.argv + ["--misses"]))
