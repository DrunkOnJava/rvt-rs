"""Whether an IFC element's body is made of true curves (#1365, RE-331 section 8).

The body scorers judge a body in one of two tessellations (Griffin, #1355,
2026-11-04: "Fine where both draw curves"): the fine one where both files write
the body as true curves, which IfcOpenShell must cut into chords and cuts at
different counts for the forms one circle takes, and IfcOpenShell's default one
for every other body. A body is made of true curves in a file when its `Body`
representation, mapped items followed and aggregated parts included, holds at
least one item that is a true curve and none that is a mesh:

- a true curve item: an extrusion (`IfcExtrudedAreaSolid`) of a profile that is
  an `IfcCircleProfileDef` (or its hollow form), an `IfcEllipseProfileDef`, an
  `IfcRoundedRectangleProfileDef`, a derived or composite profile of one of
  them, or an arbitrary profile whose outline has an arc (an `IfcIndexedPolyCurve`
  with an `IfcArcIndex`, an `IfcCompositeCurve` with a segment on a circle,
  ellipse, B-spline or other curve that is not a line or a polyline, an
  `IfcTrimmedCurve` on a circle or ellipse); an `IfcRevolvedAreaSolid`; an
  `IfcSweptDiskSolid`; a sweep (`IfcSurfaceCurveSweptAreaSolid`,
  `IfcFixedReferenceSweptAreaSolid`) of such a profile or along such a curve; an
  `IfcAdvancedBrep` with a face that is not on a plane; an `IfcSphere`,
  `IfcRightCircularCylinder` or `IfcRightCircularCone`; a Boolean result with such
  an operand;
- a mesh item: any `IfcTessellatedItem` (a triangulated or polygonal face set), a
  faceted or other manifold B-rep that is not an `IfcAdvancedBrep`, a face-based or
  shell-based surface model, or a Boolean result with such an operand;
- an item that is neither (an extrusion of a polygon, an `IfcBlock`, a bounding
  box, a half space on a plane) is exact and straight: it does not make a body a
  curve and does not stop it from being one;
- an item of any other class makes the body not one of true curves: a class this
  list does not know is judged in the default tessellation.

A body that cannot be read (an attribute the file does not have) is not one of true
curves either, so the default tessellation, the bar before #1365, decides it.
Both files have to write a body as true curves for the fine tessellation to decide
it: the saved mesh rvt-rs writes is a mesh item, so no saved mesh is ever in the
set.
"""

CURVE_PRIMITIVES = ("IfcSphere", "IfcRightCircularCylinder", "IfcRightCircularCone")
STRAIGHT_PRIMITIVES = ("IfcBlock", "IfcRectangularPyramid", "IfcBoundingBox")
CURVED_PROFILES = ("IfcCircleProfileDef", "IfcEllipseProfileDef", "IfcRoundedRectangleProfileDef")
MESH_ITEMS = (
    "IfcTessellatedItem",
    "IfcFacetedBrep",
    "IfcFacetedBrepWithVoids",
    "IfcFaceBasedSurfaceModel",
    "IfcShellBasedSurfaceModel",
)


def items_of(representation):
    """The items of a representation, mapped items followed to their source."""
    for item in representation.Items:
        if item.is_a("IfcMappedItem"):
            yield from items_of(item.MappingSource.MappedRepresentation)
        else:
            yield item


def body_items(element):
    """The items of the element's `Body` representations and of its aggregated parts'."""
    for representation in (element.Representation.Representations if element.Representation else ()):
        if representation.RepresentationIdentifier == "Body":
            yield from items_of(representation)
    for rel in getattr(element, "IsDecomposedBy", None) or ():
        for part in rel.RelatedObjects:
            yield from body_items(part)


def curve_is_true(curve):
    """Whether an outline or a path has an arc, or a curve that is not a line."""
    if curve.is_a("IfcIndexedPolyCurve"):
        return any(segment.is_a("IfcArcIndex") for segment in curve.Segments or ())
    if curve.is_a("IfcCompositeCurve"):
        return any(curve_is_true(segment.ParentCurve) for segment in curve.Segments)
    if curve.is_a("IfcTrimmedCurve"):
        return curve_is_true(curve.BasisCurve)
    if curve.is_a("IfcPolyline") or curve.is_a("IfcLine"):
        return False
    return curve.is_a("IfcConic") or curve.is_a("IfcBSplineCurve")


def profile_is_curved(profile):
    if any(profile.is_a(name) for name in CURVED_PROFILES):
        return True
    if profile.is_a("IfcArbitraryClosedProfileDef"):
        outlines = [profile.OuterCurve]
        if profile.is_a("IfcArbitraryProfileDefWithVoids"):
            outlines += list(profile.InnerCurves)
        return any(curve_is_true(curve) for curve in outlines)
    if profile.is_a("IfcArbitraryOpenProfileDef"):
        return curve_is_true(profile.Curve)
    if profile.is_a("IfcCompositeProfileDef"):
        return any(profile_is_curved(part) for part in profile.Profiles)
    if profile.is_a("IfcDerivedProfileDef"):
        return profile_is_curved(profile.ParentProfile)
    return False


def item_kind(item):
    """"mesh", "curve", "straight" or "other" for one item of a body."""
    if any(item.is_a(name) for name in MESH_ITEMS) or (item.is_a("IfcManifoldSolidBrep") and not item.is_a("IfcAdvancedBrep")):
        return "mesh"
    if item.is_a("IfcAdvancedBrep"):
        faces = item.Outer.CfsFaces
        return "curve" if any(not face.FaceSurface.is_a("IfcPlane") for face in faces) else "straight"
    if item.is_a("IfcRevolvedAreaSolid") or item.is_a("IfcSweptDiskSolid"):
        return "curve"
    if item.is_a("IfcSurfaceCurveSweptAreaSolid") or item.is_a("IfcFixedReferenceSweptAreaSolid"):
        return "curve" if profile_is_curved(item.SweptArea) or curve_is_true(item.Directrix) else "straight"
    if item.is_a("IfcExtrudedAreaSolid"):
        return "curve" if profile_is_curved(item.SweptArea) else "straight"
    if item.is_a("IfcBooleanResult"):
        kinds = {item_kind(item.FirstOperand), item_kind(item.SecondOperand)}
        for kind in ("mesh", "other", "curve"):
            if kind in kinds:
                return kind
        return "straight"
    if item.is_a("IfcHalfSpaceSolid"):
        return "straight" if item.BaseSurface.is_a("IfcPlane") else "other"
    if item.is_a("IfcCsgSolid"):
        return item_kind(item.TreeRootExpression)
    if any(item.is_a(name) for name in CURVE_PRIMITIVES):
        return "curve"
    if any(item.is_a(name) for name in STRAIGHT_PRIMITIVES):
        return "straight"
    return "other"


def is_true_curve_body(element):
    """Whether the element's body is made of true curves (this module's docstring)."""
    try:
        kinds = {item_kind(item) for item in body_items(element)}
    except (AttributeError, IndexError, KeyError, TypeError, ValueError, RuntimeError):
        return False
    return "curve" in kinds and not kinds & {"mesh", "other"}
