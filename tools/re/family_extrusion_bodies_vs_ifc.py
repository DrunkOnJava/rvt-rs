#!/usr/bin/env python3
"""Score rvt-rs bodies drawn from a family's own extrusions
(`rvt-ifc --family-extrusions`, RE-327) against the body Revit's own IFC
export writes for the same element.

This is `mesh_bodies_vs_ifc.py` on the elements whose `BodySource` is
`family_extrusions` instead of `saved_graphics_mesh`: per class and Revit
type, how many have Revit's volume within 1% and 0.01%, every vertex of each
body within 0.01 ft and 0.001 ft of the other's surface, and "ref on ours":
every tested vertex of Revit's body within 0.001 ft of rvt-rs's surface.
Bodies only exist in an export made with `--family-extrusions`; without it
this prints none.

Needs IfcOpenShell and numpy.

Usage:

    python3 tools/re/family_extrusion_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import mesh_bodies_vs_ifc  # noqa: E402

mesh_bodies_vs_ifc.SOURCE = "family_extrusions"

if __name__ == "__main__":
    sys.exit(mesh_bodies_vs_ifc.main())
