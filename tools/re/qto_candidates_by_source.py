#!/usr/bin/env python3
"""`qto_candidates_list.py --by-source` (#1016): each count split by the
`BodySource` rvt-rs gives the object.

Usage:

    python3 tools/re/qto_candidates_by_source.py <rvt-rs.ifc> <revit-export.ifc>
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import qto_candidates_list  # noqa: E402

if __name__ == "__main__":
    sys.exit(qto_candidates_list.main(sys.argv + ["--by-source"]))
