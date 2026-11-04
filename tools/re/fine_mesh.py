"""The fine tessellation the body scorers can measure beside IfcOpenShell's
default one (#1355, RE-331).

`mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc` and `family_extrusion_bodies_vs_ifc`
mesh both IFC files with IfcOpenShell's default settings and check Revit's
vertices against rvt-rs's surface. Revit's two exporters write one circle two
ways (two half arcs; an `IfcCircleProfileDef`), and IfcOpenShell's default
tessellation cuts the two into chords that differ at some radii (27 against
28 chords at 69.85 mm), so the check then measures two tessellations of one
exact circle, not two shapes (RE-327 section 11.2). In fine mode both files
are also meshed at one equal, fine deflection: every vertex of a body then
lies within the deflection of its true surface, and two tessellations of one
surface lie within about twice that of each other whichever chords they use.

Fine mode is off unless RVT_SCORER_FINE_MESH is set to anything but "" or "0"
(Measure's `fine_mesh` input sets it), and then adds labelled "fine" rows
after the scorer's own output, which it leaves as it was.

LINEAR_DEFLECTION is 1e-5 ft, in metres, the unit of IfcOpenShell's
`mesher-linear-deflection`; ANGULAR_DEFLECTION is its `mesher-angular-deflection`
in radians (the chords of a circle are about half of it apart). They are the
names IfcOpenShell 0.9.0.post1, the version tools/ci/requirements-ifc.txt pins,
accepts, and the values at which the two forms of a circle from 0.02 to 2 ft
in radius coincide within 1e-5 ft at every start (RE-331 section 2).
"""

import os

import ifcopenshell.geom

FEET = 0.3048
LINEAR_DEFLECTION = 1e-5 * FEET
ANGULAR_DEFLECTION = 0.03
LABEL = (
    f"IfcOpenShell mesher-linear-deflection {LINEAR_DEFLECTION:.4g} m (1e-5 ft)"
    f" and mesher-angular-deflection {ANGULAR_DEFLECTION:g} rad"
)


def enabled():
    return os.environ.get("RVT_SCORER_FINE_MESH", "") not in ("", "0")


def settings():
    """IfcOpenShell geometry settings: the scorers' own (world coordinates) at the fine deflection."""
    fine = ifcopenshell.geom.settings()
    fine.set("use-world-coords", True)
    fine.set("mesher-linear-deflection", LINEAR_DEFLECTION)
    fine.set("mesher-angular-deflection", ANGULAR_DEFLECTION)
    return fine
