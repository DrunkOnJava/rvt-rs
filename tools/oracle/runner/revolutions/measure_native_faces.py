"""Separate native-face oracle for the bounded circular-profile byte proof.

Queries come from reconstruction; native samples/projections only validate it.
No native field is an input to descriptor() or rebuild().
"""

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np

from measure_curved import descriptor, rebuild, analytic_volume
from measure_rectangular import require, surface_distance, signed_volume


def query_points(vertices, faces):
    centres = vertices[faces].mean(axis=1)
    return np.concatenate(
        [
            vertices[np.linspace(0, len(vertices) - 1, 400, dtype=int)],
            centres[np.linspace(0, len(centres) - 1, 400, dtype=int)],
        ]
    )


def write_queries(probe, output):
    require(not output.exists(), "query output must be new")
    doc = json.loads(probe.read_text(encoding="utf-8-sig"))
    vertices, faces = rebuild(descriptor(doc))
    result = dict(
        source_sha256=doc["file_sha256"],
        points_m=query_points(vertices, faces).tolist(),
        sampling="400 evenly spaced vertices and 400 triangle centres from byte reconstruction",
    )
    output.write_text(json.dumps(result, indent=2, allow_nan=False), encoding="utf-8")


def measure(probe, query, oracle, output):
    doc = json.loads(probe.read_text(encoding="utf-8-sig"))
    native = json.loads(oracle.read_text(encoding="utf-8"))
    queries = json.loads(query.read_text(encoding="utf-8"))
    require(
        native["schema"] == "native-revolution-face-oracle-v1", "unknown native oracle"
    )
    require(
        native["source_sha256"] == queries["source_sha256"] == doc["file_sha256"],
        "native/probe source mismatch",
    )
    require(
        native["query_sha256"] == hashlib.sha256(query.read_bytes()).hexdigest(),
        "native query hash mismatch",
    )
    d = descriptor(doc)
    vertices, faces = rebuild(d)
    expected_queries = query_points(vertices, faces)
    actual = np.asarray(queries["points_m"], dtype=float)
    require(
        actual.shape == expected_queries.shape
        and np.array_equal(actual, expected_queries),
        "native projections were not made from this reconstruction",
    )
    points = np.asarray(native["points_m"], dtype=float)
    require(
        points.ndim == 2
        and points.shape[1] == 3
        and len(points) > 0
        and np.isfinite(points).all(),
        "invalid native face samples",
    )
    require(
        sum(native["face_sample_counts"]) == len(points)
        and all(c > 0 for c in native["face_sample_counts"]),
        "unsampled native face",
    )
    distances = native["projection_distances_m"]
    require(
        len(distances) == len(expected_queries)
        and all(v is not None for v in distances),
        "incomplete native projections",
    )
    distances = np.asarray(distances, dtype=float)
    require(
        np.isfinite(distances).all() and (distances >= 0).all(),
        "invalid native distances",
    )
    forward = surface_distance(points / 0.3048, vertices[faces] / 0.3048) * 0.3048
    reverse = float(distances.max())
    analytic = analytic_volume(d)
    volume_error = abs(native["native_volume_m3"] / analytic - 1)
    rebuilt_volume_error = abs(signed_volume(vertices, faces) / analytic - 1)
    result = dict(
        file=doc["file"],
        source_sha256=doc["file_sha256"],
        oracle_sha256=hashlib.sha256(oracle.read_bytes()).hexdigest(),
        query_sha256=native["query_sha256"],
        native_face_samples=len(points),
        projected_queries=len(distances),
        native_points_to_rebuilt_m=forward,
        rebuilt_queries_to_native_faces_m=reverse,
        analytic_volume_m3=analytic,
        native_volume_m3=native["native_volume_m3"],
        relative_volume_error=volume_error,
        rebuilt_relative_volume_error=rebuilt_volume_error,
        sampling="21 by 21 UV grid per trimmed face; 400 rebuilt vertices and 400 triangle centres",
        passed=bool(
            max(forward, reverse) < 0.0003048
            and volume_error < 0.0001
            and rebuilt_volume_error < 0.0001
        ),
    )
    output.write_text(
        json.dumps(result, indent=2, allow_nan=False) + "\n", encoding="utf-8"
    )
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    q = sub.add_parser("queries")
    q.add_argument("probe", type=Path)
    q.add_argument("output", type=Path)
    m = sub.add_parser("measure")
    for name in ["probe", "query", "oracle", "output"]:
        m.add_argument(name, type=Path)
    args = parser.parse_args()
    if args.command == "queries":
        write_queries(args.probe, args.output)
    else:
        result = measure(args.probe, args.query, args.oracle, args.output)
        print(json.dumps(result, indent=2))
        raise SystemExit(0 if result["passed"] else 1)
