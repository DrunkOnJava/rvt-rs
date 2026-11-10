"""Verify analytic IFC revolutions and bounded OpenCascade import on native fixtures."""

import argparse
import json
import math
import os
from pathlib import Path
import subprocess
import sys
import time

import numpy as np
import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement
import ifcopenshell.validate

import measure_rectangular as rectangular
import measure_curved as curved
from revolution_evidence import require
from verify_bundle import digest
from verify_reader import mesh_checks


def import_child(path, output, fine=True):
    model = ifcopenshell.open(str(path))
    products = [p for p in model.by_type("IfcProduct") if p.Representation]
    require(len(products) == 1, "expected one represented product")
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    if fine:
        settings.set("mesher-linear-deflection", 0.00002)
        settings.set("mesher-angular-deflection", 0.03)
    started = time.perf_counter()
    shape = ifcopenshell.geom.create_shape(
        settings, products[0], geometry_library="opencascade"
    )
    np.savez(
        output,
        vertices=np.asarray(shape.geometry.verts).reshape(-1, 3),
        faces=np.asarray(shape.geometry.faces).reshape(-1, 3),
        import_seconds=time.perf_counter() - started,
    )


def readback(output, mesh_path, fine):
    started = time.perf_counter()
    command = [
        sys.executable,
        str(Path(__file__).resolve()),
        "--import-child",
        str(output.resolve()),
        str(mesh_path.resolve()),
        "fine" if fine else "default",
    ]
    try:
        result = subprocess.run(command, capture_output=True, timeout=120)
    except subprocess.TimeoutExpired as error:
        mesh_path.with_suffix(".log").write_bytes(
            (error.stdout or b"")
            + (error.stderr or b"")
            + b"OpenCascade timeout: 120s\n"
        )
        raise
    mesh_path.with_suffix(".log").write_bytes(result.stdout + result.stderr)
    require(result.returncode == 0, "OpenCascade import failed")
    return time.perf_counter() - started


def verify(args):
    args.output.mkdir(parents=True, exist_ok=False)
    rows = []
    reader = json.loads((args.reader_checks / "verification.json").read_text("utf-8"))
    require(reader["status"] == "passed", "reader replay did not pass")
    for row in reader["fixtures"]:
        name = row["fixture"]
        folder = args.rectangular if name.startswith("REV-") else args.curved
        source = folder / (name + ".rfa")
        require(digest(source) == row["source_sha256"], "source changed")
        output = args.output / (name + ".ifc")
        diagnostic_path = args.output / (name + ".json")
        started = time.perf_counter()
        command = [
            str(args.exporter),
            str(source),
            "-o",
            str(output),
            "--diagnostics",
            str(diagnostic_path),
        ]
        result = subprocess.run(
            command,
            capture_output=True,
            timeout=120,
            env=dict(os.environ, SOURCE_DATE_EPOCH="0"),
        )
        export_seconds = time.perf_counter() - started
        (args.output / (name + "-export.log")).write_bytes(
            result.stdout + result.stderr
        )
        require(result.returncode == 0, "export failed")
        model = ifcopenshell.open(str(output))
        log = ifcopenshell.validate.json_logger()
        ifcopenshell.validate.validate(model, log, express_rules=True)
        require(not log.statements, "IFC validation: " + str(log.statements[:3]))
        if name in ("ARC-B", "ARC-BT"):
            diag = json.loads(diagnostic_path.read_text("utf-8"))
            reason = "partial revolution with an arc profile lacks native export confirmation"
            require(
                not model.by_type("IfcRevolvedAreaSolid")
                and not any(p.Representation for p in model.by_type("IfcProduct")),
                "unsupported partial arc exported a body",
            )
            require(not diag["confidence"]["has_geometry"], "refused geometry counted")
            require(reason in result.stderr.decode("utf-8"), "missing partial arc refusal")
            require(digest(source) == row["source_sha256"], "source modified")
            rows.append(
                dict(
                    fixture=name,
                    status="refused",
                    reason=reason,
                    source_sha256=row["source_sha256"],
                )
            )
            print(f"{name}: partial arc correctly refused", flush=True)
            continue
        solids = model.by_type("IfcRevolvedAreaSolid")
        require(
            len(solids) == 1 and not model.by_type("IfcTriangulatedFaceSet"),
            "not one analytic revolution",
        )
        solid = solids[0]
        result = json.loads(
            (args.reader_checks / name / "result.json").read_text("utf-8")
        )
        saved = json.loads(
            (args.reader_checks / name / result["forms"][0]["mesh"]).read_text("utf-8")
        )
        matrix = ifcopenshell.util.placement.get_axis2placement(solid.Position)
        axis = np.asarray(saved["axis_direction"])
        radial = np.asarray(saved["radial_direction"])
        expected_radial = math.cos(saved["start"]) * radial + math.sin(
            saved["start"]
        ) * np.cross(axis, radial)
        require(
            np.max(np.abs(matrix[:3, 3] - np.asarray(saved["axis_origin"]) * 0.3048))
            <= 0.500001e-6,
            "axis origin",
        )
        require(
            np.linalg.norm(matrix[:3, 0] - expected_radial) < 1e-12
            and np.linalg.norm(matrix[:3, 1] - axis) < 1e-12,
            "start angle or tilted frame",
        )
        require(
            tuple(solid.Axis.Location.Coordinates) == (0.0, 0.0, 0.0)
            and tuple(solid.Axis.Axis.DirectionRatios) == (0.0, 1.0, 0.0),
            "IFC local axis",
        )
        require(
            abs(solid.Angle - (saved["end"] - saved["start"])) < 1e-12, "sweep angle"
        )
        profile = solid.SweptArea.OuterCurve
        require(
            profile.is_a("IfcIndexedPolyCurve") and profile.Dim == 2, "analytic profile"
        )
        kinds = [s.is_a() for s in profile.Segments]
        require(
            kinds
            == (
                ["IfcLineIndex"] * 4 if name.startswith("REV-") else ["IfcArcIndex"] * 2
            ),
            "lines/arcs lost",
        )
        proxies = model.by_type("IfcBuildingElementProxy")
        require(
            len(proxies) == 1 and proxies[0].Tag == str(saved["id"]), "form identity"
        )
        diag = json.loads(diagnostic_path.read_text("utf-8"))
        require(
            diag["exported"]["body_sources"].get("family_revolution") == 1
            and diag["confidence"]["has_geometry"],
            "body diagnostics",
        )
        require(
            "No building elements were exported; output is scaffold-only."
            not in diag["warnings"],
            "stale warning",
        )
        default_path = args.output / (name + "-default.npz")
        default_process_seconds = readback(output, default_path, fine=False)
        with np.load(default_path) as mesh:
            require(
                np.isfinite(mesh["vertices"]).all() and len(mesh["faces"]) > 0,
                "default OpenCascade import has no finite geometry",
            )
            default_import_seconds = float(mesh["import_seconds"])
        print(f"{name}: default OpenCascade {default_import_seconds:.3f}s", flush=True)
        mesh_path = args.output / (name + "-opencascade.npz")
        readback_seconds = readback(output, mesh_path, fine=True)
        with np.load(mesh_path) as mesh:
            vertices, faces = mesh["vertices"], mesh["faces"]
            import_seconds = float(mesh["import_seconds"])
        probe = args.reader_checks / (name + "-probe.json")
        doc = json.loads(probe.read_text("utf-8"))
        module = rectangular if name.startswith("REV-") else curved
        descriptor = module.descriptor(doc)
        analytic = (
            rectangular.rectangular_volume(descriptor)
            if name.startswith("REV-")
            else curved.analytic_volume(descriptor)
        )
        checks = mesh_checks(vertices, faces, analytic)
        if name.startswith("REV-"):
            native = rectangular.measure_reconstruction(
                probe,
                folder / (name + "-native.ifc"),
                args.output / (name + "-native"),
                rectangular.descriptor,
                lambda d, reverse=False: (
                    rectangular.rebuild(d, True) if reverse else (vertices, faces)
                ),
                rectangular.rectangular_volume,
                "analytic IFC read back by OpenCascade",
            )
            require(native["passed"], "native rectangular mismatch")
        else:
            oracle = json.loads((folder / (name + "-faces.json")).read_text("utf-8"))
            query_path = folder / (name + "-queries.json")
            queries = json.loads(query_path.read_text("utf-8"))
            require(
                oracle["source_sha256"]
                == queries["source_sha256"]
                == row["source_sha256"]
                and oracle["query_sha256"] == digest(query_path),
                "oracle identity",
            )
            forward = (
                rectangular.surface_distance(
                    np.asarray(oracle["points_m"]) / 0.3048, vertices[faces] / 0.3048
                )
                * 0.3048
            )
            fit = (
                rectangular.surface_distance(
                    np.asarray(queries["points_m"]) / 0.3048, vertices[faces] / 0.3048
                )
                * 0.3048
            )
            projections = np.asarray(oracle["projection_distances_m"])
            require(
                len(projections) == len(queries["points_m"])
                and np.isfinite(projections).all(),
                "projection data",
            )
            bound = fit + float(projections.max())
            require(max(forward, bound) < 0.0003048, "native circular mismatch")
            require(
                abs(oracle["native_volume_m3"] / analytic - 1) < 0.0001,
                "native volume mismatch",
            )
            native = dict(
                passed=True,
                native_samples_to_ifc_m=forward,
                ifc_to_native_sampled_upper_bound_m=bound,
            )
        require(digest(source) == row["source_sha256"], "source modified")
        rows.append(
            dict(
                fixture=name,
                source_sha256=row["source_sha256"],
                ifc_sha256=digest(output),
                bytes=output.stat().st_size,
                export_seconds=export_seconds,
                default_import_seconds=default_import_seconds,
                default_process_seconds=default_process_seconds,
                opencascade_import_seconds=import_seconds,
                readback_process_seconds=readback_seconds,
                mesh=checks,
                native=native,
            )
        )
        print(
            f"{name}: analytic IFC, OpenCascade {import_seconds:.3f}s, {output.stat().st_size} bytes, native checks passed",
            flush=True,
        )
    (args.output / "verification.json").write_text(
        json.dumps(
            dict(
                status="passed",
                exporter_sha256=digest(args.exporter),
                geometry_library="opencascade",
                ifcopenshell_version=ifcopenshell.version,
                fine_linear_deflection_m=0.00002,
                fine_angular_deflection_rad=0.03,
                distance_limit_m=0.0003048,
                relative_volume_error_limit=0.0001,
                fixtures=rows,
            ),
            indent=2,
            allow_nan=False,
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    if len(sys.argv) == 5 and sys.argv[1] == "--import-child":
        require(sys.argv[4] in ("fine", "default"), "unknown mesh settings")
        import_child(Path(sys.argv[2]), Path(sys.argv[3]), sys.argv[4] == "fine")
    else:
        parser = argparse.ArgumentParser(description=__doc__)
        for name in ["exporter", "reader-checks", "rectangular", "curved", "output"]:
            parser.add_argument("--" + name, type=Path, required=True)
        verify(parser.parse_args())
