"""Replay the three saved-file findings, byte spans and native IFC comparisons.

Uses real synthetic RFA/IFC files, the compiled probe and the existing rvt-dump.
Never launches Revit. Mutated JSON checks are refusal checks, not observations
of native files with those variants. All generated output goes to a new folder.
"""

import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess

from measure_rectangular import descriptor, identifier, measure, one_object, require


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(command, log):
    result = subprocess.run([str(x) for x in command], capture_output=True)
    log.write_bytes(result.stderr)
    require(result.returncode == 0, "command failed; see " + str(log))
    return result.stdout


def check_bytes(doc, folder):
    streams = {}
    bodies = fields = 0
    for record in doc["records"] + doc["saved_graphics_records"]:
        stream = record["stream"]
        require(
            stream.startswith("Partitions/") and stream.split("/")[1].isdigit(),
            "unexpected partition name",
        )
        if stream not in streams:
            streams[stream] = (
                folder / (stream.replace("/", "_") + ".decomp")
            ).read_bytes()
        data = streams[stream]
        start = record["body_inflated_offset"]
        require(
            isinstance(start, int) and start >= 0,
            "fixture needs a contiguous source body",
        )
        body = data[start : start + record["body_bytes"]]
        require(
            len(body) == record["body_bytes"]
            and hashlib.sha256(body).hexdigest() == record["body_sha256"],
            "body hash mismatch",
        )
        bodies += 1
        for obj in record.get("objects", []):
            for span in obj["spans"]:
                lo, hi = span["body_start"], span["body_end"]
                raw = bytes.fromhex(span["raw_hex"])
                require(0 <= lo <= hi <= len(body), "field outside body")
                require(span["inflated_offset"] == start + lo, "field offset mismatch")
                require(
                    len(raw) <= hi - lo
                    and span["raw_truncated"] == (len(raw) < hi - lo),
                    "incorrect truncation label",
                )
                require(body[lo : lo + len(raw)] == raw, "field bytes mismatch")
                fields += 1
    return dict(bodies=bodies, field_spans=fields)


def refusals(base, read=descriptor):
    results = []
    for case in [
        "duplicate_record",
        "no_axis",
        "flipped_curve",
        "missing_profile_edge",
        "reversed_angles",
        "incomplete_record",
        "unconsumed_bytes",
        "duplicate_angle",
        "inconsistent_angle_summary",
    ]:
        doc = copy.deepcopy(base)
        d = read(doc)
        form = next(r for r in doc["records"] if r["element_id"] == d["form_id"])
        axis = next(r for r in doc["records"] if r["element_id"] == d["axis_curve_id"])
        profile = next(
            r for r in d["used"] if r["class"] == "CurveElem" and r is not axis
        )
        if case == "duplicate_record":
            doc["records"].append(copy.deepcopy(form))
        elif case == "no_axis":
            one_object(axis, "RevolutionCurveData")["fields"]["m_axis"] = False
        elif case == "flipped_curve":
            one_object(profile, "CurveElemDriver")["fields"]["m_flip"] = True
        elif case == "missing_profile_edge":
            doc["records"].remove(profile)
        elif case == "incomplete_record":
            form["decode_status"] = "partial"
        elif case == "unconsumed_bytes":
            form["graph_consumed_bytes"] -= 1
        elif case == "duplicate_angle":
            form["angle_parameter_values"][1]["parameter_id"] = form[
                "angle_parameter_values"
            ][0]["parameter_id"]
        elif case == "inconsistent_angle_summary":
            form["angle_parameter_values"][0]["value"] += 0.1
        else:
            for p in form["angle_parameter_values"]:
                p["value"] = 0 if p["parameter_id"] == -1001803 else 2
            for p in one_object(form, "ParamValueSetDouble")["fields"]["m_paramSet"]:
                p["m_value"] = 0 if identifier(p["m_paramId"]) == -1001803 else 2
        try:
            read(doc)
        except (ValueError, KeyError) as error:
            results.append(dict(case=case, refusal=str(error)))
        else:
            raise ValueError("accepted unsupported evidence: " + case)
    return results


def verify(args):
    args.output.mkdir(parents=True, exist_ok=False)
    results = []
    for name in ["REV-A", "REV-B", "REV-BT"]:
        source = args.fixtures / (name + ".rfa")
        oracle = args.fixtures / (name + "-native.ifc")
        before = {p: digest(p) for p in [source, oracle]}
        for path, sha in before.items():
            sidecar = path.with_name(path.name + ".license.json")
            if sidecar.exists():
                require(
                    json.loads(sidecar.read_text(encoding="utf-8"))["sha256"] == sha,
                    "fixture manifest hash mismatch: " + path.name,
                )
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
        require(result["passed"], "native IFC comparison failed: " + name)
        require(
            all(digest(p) == sha for p, sha in before.items()),
            "source changed during verification",
        )
        results.append(dict(fixture=name, bytes=checked, measurement=result))
        print(name + ": byte evidence and native IFC comparison passed", flush=True)
    base = json.loads((args.output / "REV-B-probe.json").read_text(encoding="utf-8"))
    rejected = refusals(base)
    wrong_pair = measure(
        args.output / "REV-B-probe.json",
        args.fixtures / "REV-A-native.ifc",
        args.output / "wrong-reference-control",
    )
    require(not wrong_pair["passed"], "wrong native reference unexpectedly passed")
    summary = dict(
        status="passed",
        fixtures=results,
        modified_evidence_refusals=rejected,
        wrong_reference_refused=True,
    )
    (args.output / "verification.json").write_text(
        json.dumps(summary, indent=2, allow_nan=False) + "\n", encoding="utf-8"
    )
    print(
        "Three fixtures passed; nine unsupported-evidence cases and wrong IFC pairing refused."
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--probe", type=Path, required=True)
    parser.add_argument("--dump", type=Path, required=True)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    args.probe = args.probe.resolve(strict=True)
    args.dump = args.dump.resolve(strict=True)
    verify(args)
