"""Replay local circular-profile RFA/IFC evidence; never launches Revit."""

import argparse
import copy
import json
from pathlib import Path

from measure_curved import descriptor, measure, analytic_volume
from measure_native_faces import measure as measure_native
from revolution_evidence import require, one_object, driven_curve
from verify_bundle import digest, run, check_bytes, refusals


def arc_refusals(base):
    results = refusals(base, descriptor)
    for case in [
        "negative_radius",
        "nonunit_frame",
        "equal_arc_limits",
        "filled_arc",
        "missing_driver_edge",
        "driver_points_at_center_marker",
        "different_centres",
        "nonfinite_center",
        "axis_outside_profile_plane",
    ]:
        doc = copy.deepcopy(base)
        d = descriptor(doc)
        profiles = [
            r
            for r in d["used"]
            if r["class"] == "CurveElem" and r["element_id"] != d["axis_curve_id"]
        ]
        r = profiles[0]
        f = driven_curve(r)["fields"]
        if case == "negative_radius":
            f["m_radius"] *= -1
        elif case == "nonunit_frame":
            f["m_xVec"][0] += 1
        elif case == "equal_arc_limits":
            f["m_endParams"][1] = f["m_endParams"][0]
        elif case == "filled_arc":
            f["m_bFilled"] = True
        elif case == "different_centres":
            f["m_center"][0] += 0.1
        elif case == "nonfinite_center":
            f["m_center"][0] = float("nan")
        elif case == "axis_outside_profile_plane":
            axis = next(r for r in d["used"] if r["element_id"] == d["axis_curve_id"])
            driven_curve(axis)["fields"]["m_origin"][1] += 0.1
        else:
            driver = one_object(r, "CurveElemDriver")
            edge = next(
                e
                for e in r["graph_edges"]
                if e["source_object_index"] == driver["object_index"]
                and e["pointer_offset"] == driver["fields"]["m_pCrv"]["offset"]
            )
            if case == "missing_driver_edge":
                r["graph_edges"].remove(edge)
            else:
                marker = one_object(r, "GLine")
                driver["fields"]["m_pCrv"]["class_tag"] = marker["class_tag"]
                edge["target_class_tag"] = marker["class_tag"]
                edge["target_object_index"] = marker["object_index"]
        try:
            descriptor(doc)
        except (ValueError, KeyError) as error:
            results.append(dict(case=case, refusal=str(error)))
        else:
            raise ValueError("accepted unsupported evidence: " + case)
    return results


def verify(args):
    args.output.mkdir(parents=True, exist_ok=False)
    results = []
    for name in ["ARC-A", "ARC-B", "ARC-BT"]:
        source, oracle = args.fixtures / (name + ".rfa"), args.fixtures / (
            name + "-native.ifc"
        )
        before = {p: digest(p) for p in [source, oracle]}
        probe = args.output / (name + "-probe.json")
        probe.write_bytes(
            run([args.probe, source], args.output / (name + "-probe.log"))
        )
        doc = json.loads(probe.read_text(encoding="utf-8"))
        require(doc["file_sha256"] == before[source], "probe source hash mismatch")
        dump = args.output / (name + "-streams")
        run([args.dump, source, "-o", dump], args.output / (name + "-dump.log"))
        checked = check_bytes(doc, dump)
        result = measure(probe, oracle, args.output / "measurements")
        native = measure_native(
            probe,
            args.fixtures / (name + "-queries.json"),
            args.fixtures / (name + "-faces.json"),
            args.output / (name + "-native-face-measurement.json"),
        )
        require(native["passed"], "native face comparison failed: " + name)
        require(
            (name == "ARC-BT" or result["comparison_completed"])
            and result["passed"] == (name == "ARC-A"),
            "unexpected IFC result for archived reference: " + name,
        )
        if name != "ARC-A" and result["comparison_completed"]:
            require(
                result["opposite_sense_ifc_to_rebuild_m"] > 0.0003048,
                "opposite sweep control did not fail",
            )
        require(all(digest(p) == h for p, h in before.items()), "input changed")
        results.append(
            dict(fixture=name, bytes=checked, native_faces=native, ifc=result)
        )
        print(
            name + ": native faces passed; IFC passed=" + str(result["passed"]),
            flush=True,
        )
    base = json.loads((args.output / "ARC-B-probe.json").read_text(encoding="utf-8"))
    rejected = arc_refusals(base)
    # Independently measured full IFC must fail the partial reconstruction's volume bar.
    wrong_volume_error = abs(
        results[0]["ifc"]["ifc_mesh_volume_m3"] / analytic_volume(descriptor(base)) - 1
    )
    require(wrong_volume_error > 0.0001, "wrong reference volume unexpectedly passed")
    summary = dict(
        status="verified_native_faces_with_recorded_ifc_discrepancies",
        fixtures=results,
        modified_evidence_refusals=rejected,
        wrong_reference_relative_volume_error=wrong_volume_error,
    )
    (args.output / "verification.json").write_text(
        json.dumps(summary, indent=2, allow_nan=False) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--dump", type=Path, required=True)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.probe, args.dump = args.probe.resolve(strict=True), args.dump.resolve(
        strict=True
    )
    verify(args)
