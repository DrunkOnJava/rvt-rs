"""Circular-profile revolution proof from saved GArc fields, checked against IFC.

Only one positive form with two oriented, bounded arcs forming one circle clear
of the axis is accepted. No fixture dimensions or native API output are inputs.
"""

import argparse
import collections
import json
import math
from pathlib import Path
import sys

import numpy as np

from revolution_evidence import require, driven_curve, sketch_evidence
from measure_rectangular import measure_reconstruction, signed_volume

STEP = math.radians(0.75)
TOL = 1e-8  # metres


def vector(value, name):
    v = np.asarray(value, dtype=float)
    require(v.shape == (3,) and np.isfinite(v).all(), "invalid " + name)
    return v


def arc(record):
    obj = driven_curve(record)
    require(obj["class"] == "GArc", "expected driven profile arc")
    f = obj["fields"]
    require(f["m_bFilled"] is False, "filled arc unsupported")
    c = vector(f["m_center"], "arc centre") * 0.3048
    x, y = vector(f["m_xVec"], "arc x"), vector(f["m_yVec"], "arc y")
    require(
        abs(np.linalg.norm(x) - 1) < 1e-9
        and abs(np.linalg.norm(y) - 1) < 1e-9
        and abs(x @ y) < 1e-9,
        "arc frame not orthonormal",
    )
    r = float(f["m_radius"]) * 0.3048
    t = np.asarray(f["m_endParams"], dtype=float)
    require(math.isfinite(r) and r > TOL, "invalid arc radius")
    require(
        t.shape == (2,)
        and np.isfinite(t).all()
        and np.max(np.abs(t)) <= 2 * math.pi
        and 0 < t[1] - t[0] <= 2 * math.pi,
        "unsupported arc limits",
    )
    n = int(math.ceil((t[1] - t[0]) / STEP))
    angles = np.linspace(t[0], t[1], n + 1)
    points = c + r * (np.cos(angles)[:, None] * x + np.sin(angles)[:, None] * y)
    return dict(
        center=c,
        radius=r,
        x=x,
        y=y,
        limits=t,
        points=points,
        element_id=record["element_id"],
    )


def descriptor(doc):
    common = sketch_evidence(doc)
    axes = [r for marked, r in common["curves"] if marked]
    profiles = [r for marked, r in common["curves"] if not marked]
    require(
        len(axes) == 1 and len(profiles) == 2, "requires one axis and two profile arcs"
    )
    axis_object = driven_curve(axes[0])
    require(axis_object["class"] == "GLine", "expected driven axis line")
    line = axis_object["fields"]
    origin = vector(line["m_origin"], "axis origin") * 0.3048
    direction = vector(line["m_dirVec"], "axis direction")
    limits = np.asarray(line["m_endParams"], dtype=float) * 0.3048
    require(abs(np.linalg.norm(direction) - 1) < 1e-9, "axis direction not unit")
    require(
        limits.shape == (2,) and np.isfinite(limits).all() and limits[1] > limits[0],
        "invalid axis limits",
    )
    axis = origin + limits[0] * direction
    arcs = [arc(r) for r in profiles]
    a, b = arcs
    require(
        np.linalg.norm(a["center"] - b["center"]) < TOL
        and abs(a["radius"] - b["radius"]) < TOL,
        "arcs do not share a circle",
    )
    require(
        np.linalg.norm(np.cross(a["x"], a["y"]) - np.cross(b["x"], b["y"])) < 1e-9,
        "inconsistent arc plane or direction",
    )
    require(
        np.linalg.norm(a["points"][-1] - b["points"][0]) < TOL
        and np.linalg.norm(b["points"][-1] - a["points"][0]) < TOL,
        "open or reversed arc loop",
    )
    require(
        abs(sum(v["limits"][1] - v["limits"][0] for v in arcs) - 2 * math.pi) < 1e-9,
        "arc loop not exactly one turn",
    )
    center, radius = a["center"], a["radius"]
    delta = center - axis
    radial = delta - (delta @ direction) * direction
    major = np.linalg.norm(radial)
    require(major - radius > TOL, "circle touches or crosses axis")
    normal = np.cross(a["x"], a["y"])
    require(
        abs(normal @ direction) < 1e-9 and abs(normal @ radial) < TOL,
        "profile not in axis plane",
    )
    polygon = np.concatenate([a["points"][:-1], b["points"][:-1]])
    return dict(
        axis=axis,
        direction=direction,
        polygon=polygon,
        center=center,
        radius=radius,
        major=major,
        arcs=arcs,
        start=common["start"],
        end=common["end"],
        used=common["used"],
        form_id=common["form_id"],
        sketch_id=common["sketch_id"],
        axis_curve_id=axes[0]["element_id"],
    )


def rebuild(d, reverse=False):
    sweep = d["end"] - d["start"]
    n = int(math.ceil(sweep / STEP))
    closed = abs(sweep - 2 * math.pi) < 1e-10
    rings = n if closed else n + 1
    axis, direction = d["axis"], d["direction"]
    v = d["polygon"] - axis
    axial = (v @ direction)[:, None] * direction
    radial = v - axial
    angles = np.linspace(d["start"], d["end"], n + 1)[:rings]
    if reverse:
        angles = -angles
    vertices = (
        axis
        + axial[None]
        + radial[None] * np.cos(angles)[:, None, None]
        + np.cross(direction, radial)[None] * np.sin(angles)[:, None, None]
    ).reshape(-1, 3)
    size = len(v)
    j, i = np.meshgrid(np.arange(n), np.arange(size), indexing="ij")
    k, h = (j + 1) % rings, (i + 1) % size
    a, b, c, e = j * size + i, k * size + i, k * size + h, j * size + h
    faces = np.concatenate(
        [
            np.stack([a, b, c], axis=-1).reshape(-1, 3),
            np.stack([a, c, e], axis=-1).reshape(-1, 3),
        ]
    )
    if not closed:
        # Circle is convex; a centroid fan gives nondegenerate caps.
        first, last = vertices[:size].mean(axis=0), vertices[-size:].mean(axis=0)
        cap0, cap1 = len(vertices), len(vertices) + 1
        vertices = np.concatenate([vertices, [first, last]])
        i = np.arange(size)
        caps = np.concatenate(
            [
                np.stack([np.full(size, cap0), i, (i + 1) % size], axis=-1),
                np.stack(
                    [np.full(size, cap1), n * size + (i + 1) % size, n * size + i],
                    axis=-1,
                ),
            ]
        )
        faces = np.concatenate([faces, caps])
    if signed_volume(vertices, faces) < 0:
        faces = faces[:, ::-1]
    edges = collections.Counter(
        (int(t[i]), int(t[(i + 1) % 3])) for t in faces for i in range(3)
    )
    require(
        all(count == 1 and edges[b, a] == 1 for (a, b), count in edges.items()),
        "rebuild not closed/oriented",
    )
    return vertices, faces


def analytic_volume(d):
    return (d["end"] - d["start"]) * d["major"] * math.pi * d["radius"] ** 2


def measure(probe, native_ifc, output):
    import hashlib
    import subprocess
    from bounded_ifc_mesh import load, TIMEOUT_SECONDS

    label = "IfcOpenShell linear deflection 0.00003 m, angular deflection 0.03 rad"
    try:
        result = measure_reconstruction(
            probe,
            native_ifc,
            output,
            descriptor,
            rebuild,
            analytic_volume,
            "one circular positive revolution from two bounded arcs; research proof",
            control_point_limit=400,
            sample_limit=4000,
            tessellation_label=label,
            mesh_loader=load,
        )
        result["comparison_completed"] = True
        return result
    except subprocess.TimeoutExpired:
        doc = json.loads(probe.read_text(encoding="utf-8-sig"))
        result = dict(
            file=doc["file"],
            sha256=doc["file_sha256"],
            ifc_sha256=hashlib.sha256(native_ifc.read_bytes()).hexdigest(),
            passed=False,
            comparison_completed=False,
            status="ifc_tessellation_timeout",
            timeout_seconds=TIMEOUT_SECONDS,
            tessellation=label,
            note="No geometry-match verdict; timed-out child was terminated.",
        )
        output.mkdir(parents=True, exist_ok=True)
        (output / (probe.stem + "-measurement.json")).write_text(
            json.dumps(result, indent=2) + "\n", encoding="utf-8"
        )
        return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("probe", type=Path)
    parser.add_argument("ifc", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    result = measure(args.probe, args.ifc, args.output)
    print(json.dumps(result, indent=2))
    sys.exit(0 if result["passed"] else 1)
