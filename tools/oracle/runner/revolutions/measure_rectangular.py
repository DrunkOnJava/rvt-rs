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


def require(condition, message):
    if not condition:
        raise ValueError(message)


def identifier(value):
    while isinstance(value, dict):
        value = value.get("m_id", value.get("m_id64"))
    return value


def one_object(record, name):
    found = [o for o in record["objects"] if o["class"] == name]
    require(len(found) == 1, "expected exactly one " + name)
    return found[0]


def descriptor(doc):
    require(
        doc.get("schema") == "family-revolution-evidence-v1", "unsupported probe schema"
    )
    require(doc["status"] == "observed", "incomplete decode")
    records = {}
    for record in doc["records"]:
        require(
            record.get("decode_status") == "decoded"
            and record.get("graph_consumed_bytes") == record["body_bytes"],
            "incomplete record decode",
        )
        key = (record["class"], record["element_id"])
        require(
            key not in records, "repeated saved records require explicit resolution"
        )
        records[key] = record
    forms = [r for r in records.values() if r["class"] == "RevolutionElem"]
    require(len(forms) == 1, "expected exactly one revolution")
    form = forms[0]
    fields = one_object(form, "RevolutionElem")["fields"]
    require(fields["m_cutting"] is False, "void revolution unsupported")
    sketch_id = identifier(fields["m_sketchId"])
    sketch = records["VarSketch", sketch_id]
    sketch_fields = one_object(sketch, "VarSketch")["fields"]
    require(
        sketch_fields["m_flipXY"] is False and sketch_fields["m_flipZ"] is False,
        "flipped sketch unsupported",
    )
    plane_id = identifier(sketch_fields["m_sketchPlaneId"])
    plane = records["SketchPlane", plane_id]
    require(
        one_object(plane, "SketchPlane")["fields"]["m_flipZ"] is False,
        "flipped plane unsupported",
    )
    params = form["angle_parameter_values"]
    require(len(params) == 2, "ambiguous angle parameters")
    angles = {p["parameter_id"]: p["value"] for p in params}
    require(set(angles) == {-1001802, -1001803}, "missing or duplicate angle parameter")
    saved_angles = one_object(form, "ParamValueSetDouble")["fields"]["m_paramSet"]
    require(
        len(saved_angles) == 2
        and {identifier(p["m_paramId"]): p["m_value"] for p in saved_angles} == angles,
        "angle summary disagrees with decoded fields",
    )
    start, end = angles[-1001802], angles[-1001803]
    require(
        math.isfinite(start)
        and math.isfinite(end)
        and 0 <= start < end <= 2 * math.pi + 1e-12,
        "unsupported angular range",
    )
    curves, used = [], [form, sketch, plane]
    for r in records.values():
        if r["class"] != "CurveElem":
            continue
        membership = [o for o in r["objects"] if o["class"] == "SketchMembership"]
        if not any(
            identifier(o["fields"]["m_groupId"]) == sketch_id for o in membership
        ):
            continue
        require(len(membership) == 1, "ambiguous sketch membership")
        require(
            identifier(one_object(r, "CurveElem")["fields"]["m_sketchPlaneId"])
            == plane_id,
            "curve plane mismatch",
        )
        require(
            one_object(r, "CurveElemDriver")["fields"]["m_flip"] is False,
            "flipped curve unsupported",
        )
        data = one_object(r, "RevolutionCurveData")["fields"]
        require(type(data["m_axis"]) is bool, "invalid axis marker")
        require(data["m_bOldAxis"] is False, "old-axis flag unsupported")
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
                data["m_axis"],
                origin + t[0] * direction,
                origin + t[1] * direction,
                r["element_id"],
            )
        )
        used.append(r)
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
        start=start,
        end=end,
        r0=r0,
        r1=r1,
        z0=z0,
        z1=z1,
        used=used,
        form_id=form["element_id"],
        sketch_id=sketch_id,
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


def measure(probe_path, ifc_path, output):
    doc = json.loads(probe_path.read_text(encoding="utf-8-sig"))
    d = descriptor(doc)
    ours, faces = rebuild(d)
    model = ifcopenshell.open(str(ifc_path))
    products = [p for p in model.by_type("IfcProduct") if p.Representation]
    require(
        len(products) == 1 and products[0].is_a("IfcBuildingElementProxy"),
        "expected exactly one represented product, a reference proxy",
    )
    shape = ifcopenshell.geom.create_shape(fine_mesh.settings(), products[0])
    reference = np.array(shape.geometry.verts).reshape(-1, 3)
    ref_faces = np.array(shape.geometry.faces).reshape(-1, 3)
    require(len(reference) > 0 and len(ref_faces) > 0, "empty IFC geometry")
    require(np.isfinite(reference).all(), "nonfinite IFC geometry")
    # Existing repository scorer expects feet; convert both meshes consistently.
    forward = surface_distance(reference / 0.3048, ours[faces] / 0.3048) * 0.3048
    backward = surface_distance(ours / 0.3048, reference[ref_faces] / 0.3048) * 0.3048
    centre_distance = (
        surface_distance(
            ours[faces].mean(axis=1) / 0.3048, reference[ref_faces] / 0.3048
        )
        * 0.3048
    )
    analytic = (
        (d["end"] - d["start"])
        * 0.5
        * (d["r1"] ** 2 - d["r0"] ** 2)
        * (d["z1"] - d["z0"])
    )
    native_volume = abs(signed_volume(reference, ref_faces))
    relative_volume = abs(native_volume - analytic) / analytic
    rebuilt_volume = signed_volume(ours, faces)
    rebuilt_relative_volume = abs(rebuilt_volume - analytic) / analytic
    wrong_sense = None
    if abs(d["end"] - d["start"] - 2 * math.pi) > 1e-10:
        wrong, wrong_faces = rebuild(d, True)
        wrong_sense = (
            surface_distance(reference / 0.3048, wrong[wrong_faces] / 0.3048) * 0.3048
        )
    report = dict(
        file=doc["file"],
        sha256=doc["file_sha256"],
        ifc_sha256=hashlib.sha256(ifc_path.read_bytes()).hexdigest(),
        scope="one rectangular positive revolution; research proof; no exporter changes",
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
        vertices_tested=dict(ifc=len(reference), rebuild=len(ours)),
        triangles=dict(ifc=len(ref_faces), rebuild=len(faces)),
        ifcopenshell_version=ifcopenshell.version,
        tessellation=fine_mesh.LABEL,
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
