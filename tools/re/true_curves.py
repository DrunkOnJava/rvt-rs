"""Whether an IFC element's body is made of true curves (#1365, RE-331 section 8).

The body scorers judge a body in one of two tessellations (Griffin, #1355,
2026-11-04: "Fine where both draw curves"): the fine one where both files write
the body as true curves, which IfcOpenShell must cut into chords and cuts at
different counts for the forms one circle takes, and IfcOpenShell's default one
for every other body. A body is made of true curves in a file when its items,
which are those of its `Body` representations (mapped items followed), of the
openings that cut it (`HasOpenings`, which IfcOpenShell subtracts when it meshes
the body) and of its aggregated parts, include at least one curve item and no mesh
item and no item of a class this module does not know. Each item is one of four
kinds:

- curve: an extrusion (`IfcExtrudedAreaSolid`, and `IfcExtrudedAreaSolidTapered`
  with its end profile) of a profile that is curved: an `IfcCircleProfileDef` (or
  its hollow form), `IfcEllipseProfileDef` or `IfcRoundedRectangleProfileDef`; a
  parameterised profile with a fillet or edge radius above zero (an L, U, T, I, C
  or Z shape, a hollow rectangle, and the like: any attribute named for a radius);
  an arbitrary profile whose outline has an arc (an `IfcIndexedPolyCurve` with an
  `IfcArcIndex`, an `IfcTrimmedCurve` or an `IfcCompositeCurve` segment on a circle
  or ellipse, an `IfcCircle`, `IfcEllipse` or `IfcBSplineCurve`); a composite or
  derived profile of one of these. Also an `IfcRevolvedAreaSolid`; an
  `IfcSweptDiskSolid`; a sweep (`IfcSurfaceCurveSweptAreaSolid`,
  `IfcFixedReferenceSweptAreaSolid`) of such a profile or along such a curve; an
  `IfcAdvancedBrep` with a face that is not on a plane; an `IfcSphere`,
  `IfcRightCircularCylinder` or `IfcRightCircularCone`; a Boolean result with such
  an operand;
- straight: an extrusion of a polygon or of a parameterised profile with no
  radius, an `IfcBlock`, an `IfcRectangularPyramid`, an `IfcBoundingBox`, a half
  space on a plane, a Boolean result of straight operands. It neither makes a
  body a curve nor stops it from being one;
- mesh: any `IfcTessellatedItem` (a triangulated or polygonal face set), a faceted
  or other manifold B-rep that is not an `IfcAdvancedBrep`, a face-based or
  shell-based surface model, or a Boolean result with such an operand;
- other: an item of any class not named above, and a profile or an outline of a
  class not named above (an `IfcOffsetCurve2D`, an `IfcPcurve`, a profile type this
  module does not know). It fails closed: the body is not one of true curves.

A body that cannot be read (an attribute the file does not have) is not one of
true curves either. Both files have to write a body as true curves for the fine
tessellation to decide it, and the saved mesh rvt-rs writes is a mesh item, so no
saved mesh is ever in the set. Whatever this module does not know leaves a body in
the default tessellation, which is the bar before #1365.
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


def all_items(element):
    """`body_items` and the items of the openings that cut the element and its parts."""
    for representation in (element.Representation.Representations if element.Representation else ()):
        if representation.RepresentationIdentifier == "Body":
            yield from items_of(representation)
    for rel in getattr(element, "HasOpenings", None) or ():
        yield from all_items(rel.RelatedOpeningElement)
    for rel in getattr(element, "IsDecomposedBy", None) or ():
        for part in rel.RelatedObjects:
            yield from all_items(part)


def combine(kinds):
    """The kind of what is made of parts of these kinds."""
    kinds = set(kinds)
    for kind in ("mesh", "other", "curve"):
        if kind in kinds:
            return kind
    return "straight"


def curve_kind(curve):
    """"curve", "straight" or "other" for an outline or a path."""
    if curve.is_a("IfcPolyline") or curve.is_a("IfcLine"):
        return "straight"
    if curve.is_a("IfcIndexedPolyCurve"):
        return "curve" if any(segment.is_a("IfcArcIndex") for segment in curve.Segments or ()) else "straight"
    if curve.is_a("IfcCompositeCurve"):
        return combine(curve_kind(segment.ParentCurve) for segment in curve.Segments)
    if curve.is_a("IfcTrimmedCurve"):
        return curve_kind(curve.BasisCurve)
    if curve.is_a("IfcConic") or curve.is_a("IfcBSplineCurve"):
        return "curve"
    return "other"


def has_radius(profile):
    """Whether a parameterised profile has a fillet or edge radius above zero."""
    for name, value in profile.get_info(recursive=False).items():
        if "Radius" in name and isinstance(value, (int, float)) and value > 0:
            return True
    return False


def profile_kind(profile):
    """"curve", "straight" or "other" for a profile."""
    if any(profile.is_a(name) for name in CURVED_PROFILES):
        return "curve"
    if profile.is_a("IfcArbitraryClosedProfileDef"):
        outlines = [profile.OuterCurve]
        if profile.is_a("IfcArbitraryProfileDefWithVoids"):
            outlines += list(profile.InnerCurves)
        return combine(curve_kind(curve) for curve in outlines)
    if profile.is_a("IfcArbitraryOpenProfileDef"):
        return curve_kind(profile.Curve)
    if profile.is_a("IfcCompositeProfileDef"):
        return combine(profile_kind(part) for part in profile.Profiles)
    if profile.is_a("IfcDerivedProfileDef"):
        return profile_kind(profile.ParentProfile)
    if profile.is_a("IfcParameterizedProfileDef"):
        return "curve" if has_radius(profile) else "straight"
    return "other"


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
        return combine([profile_kind(item.SweptArea), curve_kind(item.Directrix)])
    if item.is_a("IfcExtrudedAreaSolid"):
        profiles = [item.SweptArea]
        if item.is_a("IfcExtrudedAreaSolidTapered"):
            profiles.append(item.EndSweptArea)
        return combine(profile_kind(profile) for profile in profiles)
    if item.is_a("IfcBooleanResult"):
        return combine([item_kind(item.FirstOperand), item_kind(item.SecondOperand)])
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
        kinds = {item_kind(item) for item in all_items(element)}
    except (AttributeError, IndexError, KeyError, TypeError, ValueError, RuntimeError):
        return False
    return "curve" in kinds and not kinds & {"mesh", "other"}
