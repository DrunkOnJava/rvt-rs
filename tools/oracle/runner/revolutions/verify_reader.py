"""Exercise compiled shared Rust decoding against independent native references.

The Python descriptor is a separate semantic cross-check, never an input to
Rust reconstruction. Only the Rust reader supplies the measured mesh.
"""

import argparse
import collections
import json
from pathlib import Path

import numpy as np

import measure_rectangular as rectangular
import measure_curved as curved
from revolution_evidence import driven_curve, require
from verify_bundle import digest, run


def curve_points(curve):
    t = np.linspace(0, 1, 33)
    if curve["kind"] == "line":
        return np.asarray(curve["from"]) + t[:, None] * (
            np.asarray(curve["to"]) - curve["from"]
        )
    angles = curve["start"] + t * (curve["end"] - curve["start"])
    return np.asarray(curve["center"]) + curve["radius"] * (
        np.cos(angles)[:, None] * curve["x_axis"]
        + np.sin(angles)[:, None] * curve["y_axis"]
    )


def semantic_check(mesh, doc, d):
    require(mesh["source_sha256"] == doc["file_sha256"], "reader source hash")
    require(mesh["units"] == "feet", "reader units")
    require(
        abs(mesh["start"] - d["start"]) < 1e-12 and abs(mesh["end"] - d["end"]) < 1e-12,
        "reader angle mismatch",
    )
    require(
        np.linalg.norm(np.asarray(mesh["axis_origin"]) * 0.3048 - d["axis"]) < 1e-10,
        "reader axis origin",
    )
    require(
        np.linalg.norm(np.asarray(mesh["axis_direction"]) - d["direction"]) < 1e-10,
        "reader axis direction",
    )
    points = np.concatenate([curve_points(c) for c in mesh["profile"]])
    actual = (
        np.asarray(mesh["axis_origin"])
        + points[:, 0, None] * mesh["radial_direction"]
        + points[:, 1, None] * mesh["axis_direction"]
    )
    expected = []
    for record in d["used"]:
        if record["class"] != "CurveElem" or record["element_id"] == d["axis_curve_id"]:
            continue
        obj = driven_curve(record)
        f = obj["fields"]
        if obj["class"] == "GLine":
            endpoints = (
                np.asarray(f["m_origin"])
                + np.asarray(f["m_endParams"])[:, None] * f["m_dirVec"]
            )
            c = dict(kind="line", **{"from": endpoints[0], "to": endpoints[1]})
        else:
            c = dict(
                kind="arc",
                center=f["m_center"],
                radius=f["m_radius"],
                x_axis=f["m_xVec"],
                y_axis=f["m_yVec"],
                start=f["m_endParams"][0],
                end=f["m_endParams"][1],
            )
        expected.append(curve_points(c))
    expected = np.concatenate(expected)
    distance = np.linalg.norm(actual[:, None] - expected[None, :], axis=2)
    error = float(max(distance.min(axis=0).max(), distance.min(axis=1).max()) * 0.3048)
    require(
        error < 1e-10, "shared decoded profile disagrees with independent byte proof"
    )
    return error


def mesh_checks(vertices, faces, analytic):
    require(
        np.isfinite(vertices).all()
        and faces.min() >= 0
        and faces.max() < len(vertices),
        "invalid mesh",
    )
    edges = collections.Counter(
        (int(t[i]), int(t[(i + 1) % 3])) for t in faces for i in range(3)
    )
    require(
        all(n == 1 and edges[b, a] == 1 for (a, b), n in edges.items()),
        "Rust mesh not closed/oriented",
    )
    volume = rectangular.signed_volume(vertices, faces)
    error = abs(volume / analytic - 1)
    require(volume > 0 and error < 0.0001, "Rust volume mismatch")
    return dict(
        vertices=len(vertices),
        triangles=len(faces),
        closed_oriented=True,
        volume_m3=volume,
        relative_volume_error=error,
    )


def verify(args):
    args.output.mkdir(parents=True, exist_ok=False)
    results = []
    for prefix, folder, module in [
        ("REV", args.rectangular, rectangular),
        ("ARC", args.curved, curved),
    ]:
        for suffix in ["A", "B", "BT"]:
            name = prefix + "-" + suffix
            source = folder / (name + ".rfa")
            before = digest(source)
            target = args.output / name
            run([args.reader, source, target], args.output / (name + "-reader.log"))
            result = json.loads((target / "result.json").read_text(encoding="utf-8"))
            require(result["passed"] and len(result["forms"]) == 1, "reader form count")
            mesh = json.loads(
                (target / result["forms"][0]["mesh"]).read_text(encoding="utf-8")
            )
            probe = args.output / (name + "-probe.json")
            probe.write_bytes(
                run([args.evidence_probe, source], args.output / (name + "-probe.log"))
            )
            doc = json.loads(probe.read_text(encoding="utf-8"))
            d = module.descriptor(doc)
            semantics = semantic_check(mesh, doc, d)
            vertices = np.asarray(mesh["vertices"]) * 0.3048
            faces = np.asarray(mesh["triangles"], dtype=int)
            analytic = (
                module.rectangular_volume(d)
                if prefix == "REV"
                else module.analytic_volume(d)
            )
            checks = mesh_checks(vertices, faces, analytic)
            if prefix == "REV":
                native = rectangular.measure_reconstruction(
                    probe,
                    folder / (name + "-native.ifc"),
                    target / "native",
                    rectangular.descriptor,
                    lambda descriptor, reverse=False: (
                        rectangular.rebuild(descriptor, True)
                        if reverse
                        else (vertices, faces)
                    ),
                    rectangular.rectangular_volume,
                    "compiled shared Rust reader mesh; rectangular native IFC",
                )
                require(native["passed"], "Rust rectangular IFC mismatch")
            else:
                oracle = json.loads(
                    (folder / (name + "-faces.json")).read_text(encoding="utf-8")
                )
                query = folder / (name + "-queries.json")
                queries = json.loads(query.read_text(encoding="utf-8"))
                require(
                    oracle["source_sha256"] == queries["source_sha256"] == before,
                    "native source hash",
                )
                require(oracle["query_sha256"] == digest(query), "native query hash")
                samples = np.asarray(oracle["points_m"])
                query_points = np.asarray(queries["points_m"])
                projections = np.asarray(oracle["projection_distances_m"], dtype=float)
                require(
                    len(projections) == len(query_points)
                    and np.isfinite(projections).all(),
                    "incomplete native oracle",
                )
                forward = (
                    rectangular.surface_distance(
                        samples / 0.3048, vertices[faces] / 0.3048
                    )
                    * 0.3048
                )
                fit = (
                    rectangular.surface_distance(
                        query_points / 0.3048, vertices[faces] / 0.3048
                    )
                    * 0.3048
                )
                # Each captured query has a native projection. The triangle
                # inequality bounds the distance of its nearest Rust mesh point
                # to that native point by fit + projection distance.
                bound = fit + float(projections.max())
                volume_error = abs(oracle["native_volume_m3"] / analytic - 1)
                require(
                    fit < 1e-8, "Rust mesh does not reproduce captured query locations"
                )
                passed = bool(max(forward, bound) < 0.0003048 and volume_error < 0.0001)
                native = dict(
                    passed=passed,
                    native_samples_to_rust_m=forward,
                    captured_queries_to_rust_m=fit,
                    rust_to_native_sampled_upper_bound_m=bound,
                    native_samples=len(samples),
                    captured_queries=len(query_points),
                    native_volume_relative_error=volume_error,
                    ifc="separate known limitation; not relabelled",
                )
                require(passed, "Rust circular native-face mismatch")
            require(digest(source) == before, "source changed")
            results.append(
                dict(
                    fixture=name,
                    source_sha256=before,
                    semantic_error_m=semantics,
                    mesh=checks,
                    native=native,
                )
            )
            print(
                name + ": compiled reader, closed mesh and native reference passed",
                flush=True,
            )
    cases = [
        "duplicate_record",
        "missing_curve",
        "void",
        "duplicate_angle",
        "no_axis",
        "flipped_curve",
        "old_axis",
        "missing_driver_edge",
        "profile_gap",
    ]
    refusals = []
    for name, folder, selected in [
        ("REV-B", args.rectangular, cases),
        (
            "ARC-B",
            args.curved,
            cases
            + ["negative_radius", "nonunit_arc", "filled_arc", "unsupported_curve"],
        ),
    ]:
        for case in selected:
            target = args.output / (name + "-refuse-" + case)
            run(
                [args.reader, folder / (name + ".rfa"), target, "--refusal", case],
                args.output / (name + "-" + case + ".log"),
            )
            report = json.loads((target / "result.json").read_text(encoding="utf-8"))
            require(
                report["passed"]
                and all(r["status"] == "refused" for r in report["forms"]),
                "refusal not enforced",
            )
            refusals.append(
                dict(
                    fixture=name,
                    case=case,
                    reasons=[r["reason"] for r in report["forms"]],
                )
            )
    summary = dict(
        status="passed",
        reader_sha256=digest(args.reader),
        fixtures=results,
        mutated_native_graph_refusals=refusals,
    )
    (args.output / "verification.json").write_text(
        json.dumps(summary, indent=2, allow_nan=False) + "\n", encoding="utf-8"
    )
    print(
        "All six Rust reader/native comparisons and 22 refusal controls passed",
        flush=True,
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["reader", "evidence-probe", "rectangular", "curved", "output"]:
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    args.reader = args.reader.resolve(strict=True)
    args.evidence_probe = args.evidence_probe.resolve(strict=True)
    verify(args)
