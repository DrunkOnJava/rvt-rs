"""Whether a body's volume differs from Revit's only because of how Revit's export writes it: its
round parts' end faces drawn as polygons inscribed in the circle rvt-rs's mesh draws, or faces it
writes twice (#1658, RE-397 section 5.2).

A saved mesh draws a round section with many segments (a 32-gon on the BIM4LCA housing's
30 mm rails); Revit's own export draws the same section as a polygon inscribed in that
circle with few corners (a hexagon there, a quadrilateral on the office's `KAI-3` posts), its
other vertices lying on the polygon's edges. Every vertex of each body then lies within
0.01 ft of the other's surface, and the volumes differ by the ratio of the two polygons'
areas, which a volume bar of 1% reads as a difference.

Revit's export also writes some bodies with every face twice, in the same winding (the BIM4LCA
roofs, slabs and ramp flights that read exactly 0.5000: 12 of a roof's 24 triangles are distinct),
and the volume of such a mesh is twice the body's.

`read_body` takes the triangles of one body in rvt-rs's export and in Revit's, in internal
feet, and says for each shell (a set of triangles joined through shared vertices) whether it
is equal (volume within SAME), doubled (equal once Revit's repeated triangles are counted once),
explained by its section, or differs for another reason:

* a triangle repeats an earlier one of the same shell of Revit's when it has the same vertices
  in the same cyclic order; the shell's volume is taken over its distinct triangles (rvt-rs's own
  triangles are never dropped, so a mesh of rvt-rs's that repeated a face would still read
  wrong);

* a cap is a set of coplanar triangles, joined through shared edges, whose boundary is one
  closed loop: at least three for rvt-rs's, at least two for Revit's, whose bar cut at a slant is a
  prism of 8 vertices with a quadrilateral for an end face (#1712);
* rvt-rs's cap is a circle when it has at least MIN_CIRCLE vertices that all lie within CIRCLE
  of one radius from their mean. A shell that has no circle is read by its elliptical caps (the
  end faces of a round bar cut at a slant): at least MIN_CIRCLE vertices within CIRCLE of one
  ellipse, mapped, with Revit's caps, to the circle of its minor radius by the affine map that
  scales the major axis (every area in the plane alike), then read as a circle is. A shell that
  has a circle is read by its circles alone, as it always was;
* Revit's cap is inscribed in it when it is coplanar with it to within the circle's own
  radius, none of its vertices lies beyond the circle's radius plus CIRCLE, at least three
  lie on the circle within CIRCLE (the corners) and surround its centre, the polygon through
  the corners has the area of the cap to within AREA, and every other vertex lies on one of the
  polygon's edges to within CIRCLE;
* a shell is explained when each of its circles has an inscribed cap of Revit's (or an equal
  one), the area ratios of its circles agree to within AREA, and the shell's volume ratio is
  within EXPLAINED of that area ratio.

A shell with no such circle, or whose volume ratio is not the area ratio, stays "other", and
so does a shell that has no partner in the other file, so a body that is wrong still reads
wrong. Needs numpy only.
"""

import collections

import numpy as np

# Vertices equal to this many feet are one vertex, as shell_volumes in mesh_bodies_vs_ifc.py joins them.
KEY = 1e6
# Two triangles that share an edge are one plane when the opposite vertex of each lies this
# close (feet) to the other's plane. The files write six decimals, so a vertex is off by up to
# 5e-7 in each coordinate and the triangles of a slanted cap are coplanar to about 3e-6: at 1e-6 the
# 55-vertex end face of a bar cut at a slant came out as 5 to 10 caps of 5 to 11 vertices (#1712).
COPLANAR = 1e-5
# Vertices of a cap that rvt-rs draws as a circle or an ellipse, at least.
MIN_CIRCLE = 12
# Feet: the spread of a circle's radii, the distance of a corner from the circle and of
# another vertex from the polygon's edge.
CIRCLE = 1e-3
# Relative: Revit's cap against the polygon through its corners, and the area ratios of one
# shell's circles against each other.
AREA = 5e-3
# A shell's volume ratio this near 1 is equal.
SAME = 5e-3
# Relative: a shell's volume ratio against its circles' area ratio.
EXPLAINED = 2e-2
# Bodies with more triangles than this are not read.
MAX_TRIANGLES = 200_000
# Feet: a body whose volume is rvt-rs's once Revit's repeated triangles count once is `doubled` only
# where every vertex of each lies within this of the other's surface, since a volume says nothing of shape.
FAR = 1e-2
# Shells are paired by the middle of their boxes: within this many feet, and this fraction of the
# shell's own bounding box diagonal.
PAIR_FEET = 0.05
PAIR_FRACTION = 0.01


def vertex_ids(triangles):
    """(unique vertices (V, 3) in feet, vertex ids of each triangle (T, 3))."""
    keys = np.round(triangles.reshape(-1, 3) * KEY).astype(np.int64)
    _, first, inverse = np.unique(keys, axis=0, return_index=True, return_inverse=True)
    return triangles.reshape(-1, 3)[first], inverse.reshape(-1, 3)


class Union:
    def __init__(self, n):
        self.parent = list(range(n))

    def find(self, a):
        parent = self.parent
        while parent[a] != a:
            parent[a] = parent[parent[a]]
            a = parent[a]
        return a

    def join(self, a, b):
        self.parent[self.find(a)] = self.find(b)


def shells(ids):
    """The triangle indices of each set of triangles joined through shared vertices."""
    union = Union(int(ids.max()) + 1 if len(ids) else 0)
    for a, b, c in ids:
        union.join(a, b)
        union.join(a, c)
    groups = collections.defaultdict(list)
    for t, (a, _, _) in enumerate(ids):
        groups[union.find(a)].append(t)
    return [np.array(g) for g in groups.values()]


def signed_volume(points, ids):
    """The volume enclosed by the triangles, about their own centre (as `volume` of
    mesh_bodies_vs_ifc.py takes it), unsigned."""
    p = points[ids]
    centre = p.reshape(-1, 3).mean(axis=0)
    a, b, c = p[:, 0] - centre, p[:, 1] - centre, p[:, 2] - centre
    return abs(float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum()) / 6.0)


def distinct(ids):
    """(the triangles of `ids` without those that repeat an earlier one, how many were dropped). A
    triangle repeats another when it has the same vertices in the same cyclic order."""
    seen = set()
    keep = []
    for t, (a, b, c) in enumerate(ids):
        a, b, c = int(a), int(b), int(c)
        key = (a, b, c) if a <= b and a <= c else (b, c, a) if b <= a and b <= c else (c, a, b)
        if key in seen:
            continue
        seen.add(key)
        keep.append(t)
    return ids[keep], len(ids) - len(keep)


def caps(points, ids, least=3):
    """Each cap of the triangles `ids`: a dict of its boundary loop (ordered vertices, (n, 3)),
    its area, its unit normal and its centre (the mean of the loop). A cap is a set of at
    least `least` coplanar triangles, joined through shared edges, whose boundary is one closed
    loop. Revit's caps are read with `least=2`: it draws a bar cut at a slant as a prism of 8
    vertices, whose end faces are quadrilaterals of two triangles (#1712)."""
    a, b, c = (points[ids[:, k]] for k in range(3))
    cross = np.cross(b - a, c - a)
    twice = np.linalg.norm(cross, axis=1)
    live = twice > 1e-12
    normal = cross / np.maximum(twice, 1e-24)[:, None]
    edges = collections.defaultdict(list)
    for t, tri in enumerate(ids):
        if not live[t]:
            continue
        for i in range(3):
            u, v = int(tri[i]), int(tri[(i + 1) % 3])
            edges[(min(u, v), max(u, v))].append(t)
    union = Union(len(ids))
    for (u, v), pair in edges.items():
        if len(pair) != 2:
            continue
        s, t = pair
        # The vertex of each triangle that is not on the shared edge.
        other_s = next(int(k) for k in ids[s] if k != u and k != v)
        other_t = next(int(k) for k in ids[t] if k != u and k != v)
        if (
            abs(float(np.dot(normal[s], points[other_t] - points[u]))) <= COPLANAR
            and abs(float(np.dot(normal[t], points[other_s] - points[u]))) <= COPLANAR
        ):
            union.join(s, t)
    groups = collections.defaultdict(list)
    for t in range(len(ids)):
        if live[t]:
            groups[union.find(t)].append(t)
    out = []
    for members in groups.values():
        if len(members) < least:
            continue
        inside = set(members)
        boundary = collections.defaultdict(list)
        for t in members:
            tri = ids[t]
            for i in range(3):
                u, v = int(tri[i]), int(tri[(i + 1) % 3])
                if sum(1 for other in edges[(min(u, v), max(u, v))] if other in inside) == 1:
                    boundary[u].append(v)
                    boundary[v].append(u)
        if not boundary or any(len(near) != 2 for near in boundary.values()):
            continue
        start = next(iter(boundary))
        loop, previous, current = [start], None, start
        while True:
            near = boundary[current]
            following = near[0] if near[0] != previous else near[1]
            if following == start:
                break
            loop.append(following)
            previous, current = current, following
            if len(loop) > len(boundary):
                break
        if len(loop) != len(boundary):
            continue
        loop_points = points[loop]
        # The cap's normal is the sum of its triangles' (twice-area weighted): a sliver's own normal
        # is off by the error of its vertices over its width, 0.5 degrees on the end of a bar cut at a slant.
        n = cross[members].sum(axis=0)
        n = n / max(float(np.linalg.norm(n)), 1e-24)
        out.append(
            {
                "loop": loop_points,
                "area": float(twice[members].sum()) / 2.0,
                "normal": n,
                "centre": loop_points.mean(axis=0),
            }
        )
    return out


def circle_of(cap):
    """(centre, radius) when the cap is a circle: at least MIN_CIRCLE vertices on one radius."""
    loop = cap["loop"]
    if len(loop) < MIN_CIRCLE:
        return None
    radii = np.linalg.norm(loop - cap["centre"], axis=1)
    if float(radii.max() - radii.min()) > CIRCLE:
        return None
    return cap["centre"], float(radii.mean())


def ellipse_of(cap):
    """(centre, minor radius, unit major direction, minor over major) when the cap is an ellipse that
    is no circle: at least MIN_CIRCLE vertices, every one within CIRCLE of one ellipse in the cap's
    plane, whose semi-axes differ by more than CIRCLE. A round bar cut at a slant ends in one (#1712).

    The conic is fitted to the loop by least squares, in the plane's own coordinates about the
    loop's mean (which lies inside it); the vertices' distance from the fitted ellipse is measured on
    the circle the fit maps it to, in feet of the minor radius."""
    loop = cap["loop"]
    if len(loop) < MIN_CIRCLE:
        return None
    normal, mean = cap["normal"], cap["centre"]
    first = loop[0] - mean
    first = first - normal * float(first @ normal)
    length = float(np.linalg.norm(first))
    if length < 1e-12:
        return None
    e1 = first / length
    e2 = np.cross(normal, e1)
    x, y = (loop - mean) @ e1, (loop - mean) @ e2
    coefficients, *_ = np.linalg.lstsq(np.c_[x * x, x * y, y * y, x, y], np.ones(len(x)), rcond=None)
    a, b, c, d, e = (float(v) for v in coefficients)
    hessian = np.array([[2 * a, b], [b, 2 * c]])
    if not np.isfinite(hessian).all() or abs(float(np.linalg.det(hessian))) < 1e-18:
        return None
    try:
        centre2 = np.linalg.solve(hessian, -np.array([d, e]))
    except np.linalg.LinAlgError:
        return None
    rhs = 1.0 - (a * centre2[0] ** 2 + b * centre2[0] * centre2[1] + c * centre2[1] ** 2 + d * centre2[0] + e * centre2[1])
    if rhs <= 0:
        return None
    quadratic = np.array([[a, b / 2], [b / 2, c]]) / rhs
    eigenvalues, axes = np.linalg.eigh(quadratic)
    if not (eigenvalues > 0).all():
        return None
    major, minor = (1.0 / np.sqrt(eigenvalues)).tolist()
    if major - minor <= CIRCLE:
        return None
    local = np.c_[x - centre2[0], y - centre2[1]] @ axes
    radius = np.sqrt((local[:, 0] / major) ** 2 + (local[:, 1] / minor) ** 2)
    if float(np.abs(radius - 1.0).max()) * minor > CIRCLE:
        return None
    centre = mean + centre2[0] * e1 + centre2[1] * e2
    direction = axes[0, 0] * e1 + axes[1, 0] * e2
    return centre, float(minor), direction / float(np.linalg.norm(direction)), float(minor / major)


def squash(points, centre, direction, k):
    """`points` with their component along the unit vector `direction` about `centre` scaled by k: the
    affine map that takes an ellipse whose major axis lies along `direction` to the circle of its minor
    radius when k is minor over major. It scales every area in that plane by k alike."""
    along = (points - centre) @ direction
    return points + np.outer((k - 1.0) * along, direction)


def squashed(cap, centre, direction, k):
    """`cap` after `squash`: its loop, centre and area."""
    return dict(
        cap,
        loop=squash(cap["loop"], centre, direction, k),
        centre=squash(cap["centre"][None], centre, direction, k)[0],
        area=cap["area"] * k,
    )


def inscribed_ratio(circle, normal, cap_ours_area, theirs):
    """The area ratio (rvt-rs's circle over Revit's cap) when Revit's cap `theirs` is a polygon
    inscribed in the circle, else None. A cap that is the circle itself (the same vertices)
    has a ratio of 1."""
    centre, radius = circle
    loop = theirs["loop"]
    if abs(float(np.dot(normal, theirs["normal"]))) < 1.0 - 1e-6:
        return None
    if abs(float(np.dot(normal, theirs["centre"] - centre))) > radius:
        return None
    flat = loop - np.outer((loop - centre) @ normal, normal)
    offset = flat - centre
    rho = np.linalg.norm(offset, axis=1)
    if float(rho.max()) > radius + CIRCLE:
        return None
    corner = rho >= radius - CIRCLE
    if int(corner.sum()) < 3:
        return None
    # The corners in angular order about the centre, in the cap's own plane.
    e1 = offset[int(np.argmax(rho))] / float(rho.max())
    e2 = np.cross(normal, e1)
    angle = np.arctan2(offset @ e2, offset @ e1)
    order = np.argsort(angle[corner])
    corners = flat[corner][order]
    turned = np.sort(angle[corner])
    gaps = np.diff(np.r_[turned, turned[0] + 2 * np.pi])
    if float(gaps.max()) >= np.pi:
        return None
    area = 0.0
    for i in range(len(corners)):
        area += float(np.linalg.norm(np.cross(corners[i] - centre, corners[(i + 1) % len(corners)] - centre))) / 2.0
    if abs(area - theirs["area"]) > AREA * theirs["area"]:
        return None
    # Every vertex that is not a corner is on one of the polygon's edges.
    for point in flat[~corner]:
        best = float("inf")
        for i in range(len(corners)):
            a, b = corners[i], corners[(i + 1) % len(corners)]
            ab = b - a
            t = float(np.clip(np.dot(point - a, ab) / max(float(np.dot(ab, ab)), 1e-24), 0.0, 1.0))
            best = min(best, float(np.linalg.norm(point - (a + t * ab))))
        if best > CIRCLE:
            return None
    return cap_ours_area / theirs["area"]


def read_shell(ours_points, ours_ids, ref_points, ref_ids):
    """('equal' | 'doubled' | 'explained' | 'other', volume ratio, area ratio or None, reason,
    triangles of Revit's shell that repeat another) of a pair of shells. The volume ratio is
    taken over Revit's distinct triangles."""
    ref_ids, repeated = distinct(ref_ids)
    v_ours, v_ref = signed_volume(ours_points, ours_ids), signed_volume(ref_points, ref_ids)
    ratio = v_ours / v_ref if v_ref > 0 else float("inf")
    if abs(ratio - 1.0) <= SAME:
        return ("doubled" if repeated else "equal"), ratio, None, "", repeated
    # Each round end face: its circle and the cap. A shell with none is read by its elliptical end
    # faces (a bar cut at a slant), each taken to the circle of its minor radius by the map that
    # takes its ellipse there; Revit's caps are read through the same map, which scales every area
    # in the plane alike, so the area ratio is the one of the plane. A shell that has a circle is
    # read by its circles alone, as it always was: its elliptical end faces are not asked for.
    own = caps(ours_points, ours_ids)
    circles = [(found, cap, None) for cap in own if (found := circle_of(cap)) is not None]
    if not circles:
        for cap in own:
            ellipse = ellipse_of(cap)
            if ellipse is not None:
                centre, minor, direction, k = ellipse
                circles.append(((centre, minor), squashed(cap, centre, direction, k), (centre, direction, k)))
    if not circles:
        return "other", ratio, None, "no round end face in rvt-rs's shell", repeated
    theirs = caps(ref_points, ref_ids, least=2)
    ratios = []
    for circle, cap, mapping in circles:
        best = None
        for candidate in theirs:
            if mapping is not None:
                candidate = squashed(candidate, *mapping)
            got = inscribed_ratio(circle, cap["normal"], cap["area"], candidate)
            if got is not None:
                best = got
                break
        if best is None:
            return "other", ratio, None, "a round end face of rvt-rs's has no inscribed polygon in Revit's", repeated
        ratios.append(best)
    mean = float(np.mean(ratios))
    if max(ratios) - min(ratios) > AREA * mean:
        return "other", ratio, mean, "the end faces' area ratios disagree", repeated
    if abs(ratio - mean) > EXPLAINED * mean:
        return "other", ratio, mean, "the volume ratio is not the end faces' area ratio", repeated
    return "explained", ratio, mean, "", repeated


def read_body(ours_triangles, ref_triangles, far=None):
    """The reading of one body: a dict of 'shells' (a list of (kind, volume ratio, area ratio,
    reason, repeated triangles)), 'unpaired_ours' and 'unpaired_ref' (shells with no partner in
    the other file), or {'unread': why} for a body too large to read. `far` is the farthest vertex
    of either body from the other's surface, in feet, when known: a `doubled` reading asks it to be
    within FAR. A body of Revit's with repeated triangles whose whole volume, over its distinct
    triangles, is rvt-rs's is read as one `doubled` shell, whether or not the shells pair up
    (rvt-rs's mesh can join two shells that Revit's export keeps apart)."""
    if len(ours_triangles) > MAX_TRIANGLES or len(ref_triangles) > MAX_TRIANGLES:
        return {"unread": "more than %d triangles" % MAX_TRIANGLES}
    ours_points, ours_ids = vertex_ids(ours_triangles)
    ref_points, ref_ids = vertex_ids(ref_triangles)
    whole_ids, repeated_total = distinct(ref_ids)
    if repeated_total:
        v_ours, v_ref = signed_volume(ours_points, ours_ids), signed_volume(ref_points, whole_ids)
        whole = v_ours / v_ref if v_ref > 0 else float("inf")
        if abs(whole - 1.0) <= SAME and (far is None or far <= FAR):
            return {
                "shells": [("doubled", whole, None, "", repeated_total)],
                "unpaired_ours": 0,
                "unpaired_ref": 0,
            }

    def describe(points, ids):
        used = points[np.unique(ids)]
        box = (used.max(axis=0) + used.min(axis=0)) / 2.0
        return {
            "ids": ids,
            # The middle of the shell's box: the mean of its vertices moves with how a surface is cut.
            "centre": box,
            "diagonal": float(np.linalg.norm(used.max(axis=0) - used.min(axis=0))),
            "volume": max(signed_volume(points, ids), 1e-12),
        }

    ours_shells = [describe(ours_points, ours_ids[g]) for g in shells(ours_ids)]
    ref_shells = [describe(ref_points, ref_ids[g]) for g in shells(ref_ids)]
    free = set(range(len(ours_shells)))
    order = sorted(range(len(ref_shells)), key=lambda i: -ref_shells[i]["volume"])
    result = []
    unpaired_ref = 0
    for i in order:
        theirs = ref_shells[i]
        limit = PAIR_FEET + PAIR_FRACTION * theirs["diagonal"]
        # Among the shells within reach, the one whose volume is nearest: two shells side by side
        # that differ in size must not be taken for each other's partner.
        best, best_cost = None, float("inf")
        for j in free:
            near = float(np.linalg.norm(ours_shells[j]["centre"] - theirs["centre"]))
            if near > limit:
                continue
            cost = abs(float(np.log(ours_shells[j]["volume"] / theirs["volume"]))) + near / limit * 1e-3
            if cost < best_cost:
                best, best_cost = j, cost
        if best is None:
            unpaired_ref += 1
            continue
        free.discard(best)
        reading = read_shell(ours_points, ours_shells[best]["ids"], ref_points, theirs["ids"])
        if reading[0] == "doubled" and far is not None and far > FAR:
            reading = (
                "other",
                reading[1],
                None,
                "equal once repeated faces count once, but a vertex lies %.4f ft from the other's surface" % far,
                reading[4],
            )
        result.append(reading)
    return {"shells": result, "unpaired_ours": len(free), "unpaired_ref": unpaired_ref}


def verdict(reading):
    """'equal' (every shell equal), 'doubled' (every shell equal, doubled or explained, at least one
    doubled), 'explained' (every shell equal or explained, at least one explained), 'other' (a
    shell differs for another reason or has no partner) or 'unread'."""
    if "unread" in reading:
        return "unread"
    if reading["unpaired_ours"] or reading["unpaired_ref"]:
        return "other"
    kinds = {kind for kind, *_ in reading["shells"]}
    if "other" in kinds:
        return "other"
    if "doubled" in kinds:
        return "doubled"
    return "explained" if "explained" in kinds else "equal"
