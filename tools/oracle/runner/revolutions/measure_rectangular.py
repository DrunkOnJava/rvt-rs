"""Bounded rectangular-revolution proof from probe JSON, checked against native IFC.

No fixture dimensions or native API geometry feed reconstruction. Refuses anything
outside one unflipped, positive revolution with four rectangular profile lines
and one explicitly marked axis. This is research, not a production reader.
"""

import argparse
import collections
import hashlib
import json
import math
from pathlib import Path
import sys

import numpy as np
import ifcopenshell
import ifcopenshell.geom

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "re"))
from mesh_bodies_vs_ifc import surface_distance
import fine_mesh


from revolution_evidence import require, identifier, one_object, sketch_evidence


def descriptor(doc):
    common = sketch_evidence(doc)
    curves = []
    for is_axis, r in common["curves"]:
        line = one_object(r, "GLine")["fields"]
        origin, direction = np.array(line["m_origin"]) * 0.3048, np.array(
            line["m_dirVec"]
        )
        t = np.array(line["m_endParams"]) * 0.3048
        require(
            origin.shape == (3,) and direction.shape == (3,) and t.shape == (2,),
            "invalid line dimensions",
        )
        require(
            np.isfinite(origin).all()
            and np.isfinite(direction).all()
            and np.isfinite(t).all(),
            "nonfinite line",
        )
        require(
            abs(np.linalg.norm(direction) - 1) < 1e-9 and t[1] > t[0],
            "invalid line range",
        )
        curves.append(
            (
                is_axis,
                origin + t[0] * direction,
                origin + t[1] * direction,
                r["element_id"],
            )
        )
    axes = [c for c in curves if c[0]]
    profile = [c for c in curves if c[0] is False]
    require(
        len(axes) == 1 and len(profile) == 4,
        "requires one marked axis and four profile lines",
    )
    axis = axes[0][1]
    direction = axes[0][2] - axis
    direction /= np.linalg.norm(direction)
    # Join by measured endpoints, never by record or element-id order.
    pending = list(profile)
    first = pending.pop(0)
    polygon, tail = [first[1]], first[2]
    while pending:
        candidates = [
            (i, c) for i, c in enumerate(pending) if np.linalg.norm(c[1] - tail) < 1e-8
        ]
        require(len(candidates) == 1, "open, reversed or branching profile")
        i, c = candidates[0]
        polygon.append(c[1])
        tail = c[2]
        pending.pop(i)
    require(np.linalg.norm(tail - polygon[0]) < 1e-8, "profile does not close")
    polygon = np.array(polygon)
    delta = polygon - axis
    z = delta @ direction
    radial = delta - z[:, None] * direction
    radius = np.linalg.norm(radial, axis=1)
    require(radius.min() > 1e-8, "profile touching axis unsupported")
    x = radial[0] / radius[0]
    require(
        np.max(np.linalg.norm(radial - radius[:, None] * x, axis=1)) < 1e-8,
        "profile not in one radial half-plane",
    )
    r0, r1, z0, z1 = radius.min(), radius.max(), z.min(), z.max()
    require(r1 - r0 > 1e-8 and z1 - z0 > 1e-8, "degenerate rectangle")
    corners = []
    for r, h in zip(radius, z):
        ir, iz = int(abs(r - r1) < 1e-8), int(abs(h - z1) < 1e-8)
        require(
            abs(r - [r0, r1][ir]) < 1e-8 and abs(h - [z0, z1][iz]) < 1e-8,
            "nonrectangular profile",
        )
        corners.append((ir, iz))
    require(
        len(set(corners)) == 4
        and all(
            sum(a != b for a, b in zip(corners[i], corners[(i + 1) % 4])) == 1
            for i in range(4)
        ),
        "rectangle edges not radial/axial",
    )
    return dict(
        axis=axis,
        direction=direction,
        x=x,
        polygon=polygon,
        start=common["start"],
        end=common["end"],
        r0=r0,
        r1=r1,
        z0=z0,
        z1=z1,
        used=common["used"],
        form_id=common["form_id"],
        sketch_id=common["sketch_id"],
        axis_curve_id=axes[0][3],
    )


def rebuild(d, reverse=False):
    sweep = d["end"] - d["start"]
    n = int(math.ceil(math.degrees(sweep) / 0.25))
    closed = abs(sweep - 2 * math.pi) < 1e-10
    rings = n if closed else n + 1
    axis, direction = d["axis"], d["direction"]
    v = d["polygon"] - axis
    axial = (v @ direction)[:, None] * direction
    radial = v - axial
    vertices = []
    for j in range(rings):
        a = (d["start"] + sweep * j / n) * (-1 if reverse else 1)
        vertices.extend(
            axis
            + axial
            + radial * math.cos(a)
            + np.cross(direction, radial) * math.sin(a)
        )
    triangles = []

    def quad(a, b, c, e):
        triangles.extend([[a, b, c], [a, c, e]])

    for j in range(n):
        k = (j + 1) % rings
        for i in range(4):
            quad(4 * j + i, 4 * k + i, 4 * k + (i + 1) % 4, 4 * j + (i + 1) % 4)
    if not closed:
        quad(0, 1, 2, 3)
        quad(4 * n + 3, 4 * n + 2, 4 * n + 1, 4 * n)
    vertices, triangles = np.array(vertices), np.array(triangles)
    if signed_volume(vertices, triangles) < 0:
        triangles = triangles[:, ::-1]
    edges = collections.Counter(
        (int(t[i]), int(t[(i + 1) % 3])) for t in triangles for i in range(3)
    )
    require(
        all(n == 1 and edges[b, a] == 1 for (a, b), n in edges.items()),
        "rebuild not closed/oriented",
    )
    return vertices, triangles


def signed_volume(vertices, faces):
    # Shift near the body to reduce cancellation for translated fixtures.
    a, b, c = (vertices[faces] - vertices.mean(axis=0)).transpose(1, 0, 2)
    return float(np.einsum("ij,ij->i", a, np.cross(b, c)).sum() / 6)


def evidence(doc, records):
    names = {
        "m_sketchId",
        "m_groupId",
        "m_axis",
        "m_bOldAxis",
        "m_value",
        "m_paramId",
        "m_paramSet",
        "m_endParams",
        "m_center",
        "m_xVec",
        "m_yVec",
        "m_radius",
        "m_bFilled",
        "m_pCrv",
        "m_oArcCntr",
        "m_pCurveDriver",
        "m_origin",
        "m_dirVec",
        "m_3x3",
        "m_or",
        "m_cutting",
        "m_flip",
        "m_flipXY",
        "m_flipZ",
        "m_sketchPlaneId",
    }
    rows = []
    for r in records:
        for o in r["objects"]:
            if o["class"] not in {
                "RevolutionElem",
                "SketchMembership",
                "RevolutionCurveData",
                "GLine",
                "GArc",
                "Trf",
                "ParamValueSetDouble",
                "VarSketch",
                "SketchPlane",
                "CurveElem",
                "CurveElemDriver",
            }:
                continue
            for s in o["spans"]:
                if s["field"] in names:
                    rows.append(
                        dict(
                            file=doc["file"],
                            sha256=doc["file_sha256"],
                            element_id=r["element_id"],
                            stream=r["stream"],
                            object_class=o["class"],
                            field=s["field"],
                            inflated_offset=s["inflated_offset"],
                            body_start=s["body_start"],
                            body_end=s["body_end"],
                            group_marker_offset=r["group_marker_offset"],
                            group_body_offset=r["group_body_offset"],
                            raw_hex=s["raw_hex"],
                            raw_truncated=s["raw_truncated"],
                            decoded_object_fields=o["fields"],
                        )
                    )
    return rows


def rectangular_volume(d):
    return (
        (d["end"] - d["start"])
        * 0.5
        * (d["r1"] ** 2 - d["r0"] ** 2)
        * (d["z1"] - d["z0"])
    )


def measure(probe_path, ifc_path, output):
    return measure_reconstruction(
        probe_path,
        ifc_path,
        output,
        descriptor,
        rebuild,
        rectangular_volume,
        "one rectangular positive revolution; research proof; no exporter changes",
    )


def measure_reconstruction(
    probe_path,
    ifc_path,
    output,
    read,
    build,
    volume,
    scope,
    control_point_limit=None,
    sample_limit=None,
    geometry_settings=None,
    tessellation_label=None,
    mesh_loader=None,
):
    doc = json.loads(probe_path.read_text(encoding="utf-8-sig"))
    d = read(doc)
    ours, faces = build(d)
    if mesh_loader is None:
        model = ifcopenshell.open(str(ifc_path))
        products = [p for p in model.by_type("IfcProduct") if p.Representation]
        require(
            len(products) == 1 and products[0].is_a("IfcBuildingElementProxy"),
            "expected exactly one represented product, a reference proxy",
        )
        shape = ifcopenshell.geom.create_shape(
            fine_mesh.settings() if geometry_settings is None else geometry_settings,
            products[0],
        )
        reference = np.array(shape.geometry.verts).reshape(-1, 3)
        ref_faces = np.array(shape.geometry.faces).reshape(-1, 3)
    else:
        reference, ref_faces = mesh_loader(ifc_path)
    require(len(reference) > 0 and len(ref_faces) > 0, "empty IFC geometry")
    require(np.isfinite(reference).all(), "nonfinite IFC geometry")

    # Existing repository scorer expects feet; convert both meshes consistently.
    def sample(points):
        if sample_limit is None or len(points) <= sample_limit:
            return points
        return points[np.linspace(0, len(points) - 1, sample_limit, dtype=int)]

    reference_points, rebuilt_points = sample(reference), sample(ours)
    centres = sample(ours[faces].mean(axis=1))
    forward = surface_distance(reference_points / 0.3048, ours[faces] / 0.3048) * 0.3048
    backward = (
        surface_distance(rebuilt_points / 0.3048, reference[ref_faces] / 0.3048)
        * 0.3048
    )
    centre_distance = (
        surface_distance(centres / 0.3048, reference[ref_faces] / 0.3048) * 0.3048
    )
    analytic = volume(d)
    native_volume = abs(signed_volume(reference, ref_faces))
    relative_volume = abs(native_volume - analytic) / analytic
    rebuilt_volume = signed_volume(ours, faces)
    rebuilt_relative_volume = abs(rebuilt_volume - analytic) / analytic
    wrong_sense = None
    control_count = 0
    if abs(d["end"] - d["start"] - 2 * math.pi) > 1e-10:
        wrong, wrong_faces = build(d, True)
        control = reference
        if control_point_limit is not None and len(control) > control_point_limit:
            control = reference[
                np.linspace(0, len(reference) - 1, control_point_limit, dtype=int)
            ]
        control_count = len(control)
        wrong_sense = (
            surface_distance(control / 0.3048, wrong[wrong_faces] / 0.3048) * 0.3048
        )
    report = dict(
        comparison_completed=True,
        file=doc["file"],
        sha256=doc["file_sha256"],
        ifc_sha256=hashlib.sha256(ifc_path.read_bytes()).hexdigest(),
        scope=scope,
        units="m",
        form_id=d["form_id"],
        sketch_id=d["sketch_id"],
        axis_curve_id=d["axis_curve_id"],
        start_radians=d["start"],
        end_radians=d["end"],
        axis_origin_m=d["axis"].tolist(),
        axis_direction=d["direction"].tolist(),
        analytic_volume_m3=analytic,
        ifc_mesh_volume_m3=native_volume,
        relative_volume_error=relative_volume,
        rebuilt_mesh_volume_m3=rebuilt_volume,
        rebuilt_relative_volume_error=rebuilt_relative_volume,
        closed_oriented_rebuild=True,
        ifc_vertices_to_rebuild_m=forward,
        rebuild_vertices_to_ifc_m=backward,
        rebuild_triangle_centres_to_ifc_m=centre_distance,
        opposite_sense_ifc_to_rebuild_m=wrong_sense,
        opposite_sense_vertices_tested=control_count,
        vertices_tested=dict(ifc=len(reference_points), rebuild=len(rebuilt_points)),
        triangle_centres_tested=len(centres),
        total_vertices=dict(ifc=len(reference), rebuild=len(ours)),
        sampling=(
            "all points"
            if sample_limit is None
            else "even index stride, cap %d" % sample_limit
        ),
        triangles=dict(ifc=len(ref_faces), rebuild=len(faces)),
        ifcopenshell_version=ifcopenshell.version,
        tessellation=(
            fine_mesh.LABEL if tessellation_label is None else tessellation_label
        ),
        evidence=evidence(doc, d["used"]),
    )
    # Fixed, explicit research bar. Sampling is not a full Hausdorff proof.
    report["passed"] = bool(
        forward < 0.0003048
        and backward < 0.0003048
        and centre_distance < 0.0003048
        and relative_volume < 0.0001
        and rebuilt_relative_volume < 0.0001
    )
    output.mkdir(parents=True, exist_ok=True)
    (output / (probe_path.stem + "-measurement.json")).write_text(
        json.dumps(report, indent=2, allow_nan=False) + "\n", encoding="utf-8"
    )
    with (output / (probe_path.stem + "-rebuilt.obj")).open("w") as stream:
        stream.write("# Reconstructed only from byte-probe fields; metres\n")
        for p in ours:
            stream.write("v %.17g %.17g %.17g\n" % tuple(p))
        for t in faces:
            stream.write("f %d %d %d\n" % tuple(t + 1))
    return {k: v for k, v in report.items() if k != "evidence"}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("probe", type=Path)
    parser.add_argument("ifc", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    result = measure(args.probe, args.ifc, args.output)
    print(json.dumps(result, indent=2))
    sys.exit(0 if result["passed"] else 1)
