"""Replay the complete synthetic revolution package without Revit or private tools."""

import argparse
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import sys
import time

from revolution_evidence import require
from verify_bundle import digest


def verify_manifest(folder):
    manifest = json.loads((folder / "manifest.json").read_text(encoding="utf-8"))
    require(manifest["schema"] == "family-revolutions-package-v1", "unknown package")
    actual = {
        p.relative_to(folder).as_posix()
        for p in folder.rglob("*")
        if p.is_file() and p != folder / "manifest.json"
    }
    require(actual == set(manifest["files"]), "package file inventory differs")
    for name, expected in manifest["files"].items():
        part = PurePosixPath(name)
        require(
            not part.is_absolute() and ".." not in part.parts, "unsafe package path"
        )
        path = folder / name
        require(not path.is_symlink(), "package contains a symbolic link")
        require(digest(path) == expected, "package hash mismatch: " + name)
    return manifest


def verify(args):
    folder = args.fixtures.resolve(strict=True)
    manifest = verify_manifest(folder)
    binaries = args.bin_dir.resolve(strict=True)
    suffix = ".exe" if os.name == "nt" else ""
    exporter = binaries / ("rvt-ifc" + suffix)
    dump = binaries / ("rvt-dump" + suffix)
    probe = binaries / "examples" / ("probe_family_revolutions" + suffix)
    reader = binaries / "examples" / ("probe_decoded_revolutions" + suffix)
    for path in [exporter, dump, probe, reader]:
        require(path.is_file(), "missing compiled tool: " + str(path))
    args.output.mkdir(parents=True, exist_ok=False)
    scripts = Path(__file__).resolve().parent
    output = args.output.resolve()
    rectangular, curved = folder / "rectangular", folder / "curved"
    stages = [
        (
            "rectangular",
            "verify_bundle.py",
            ["--probe", probe, "--dump", dump, "--fixtures", rectangular],
        ),
        (
            "curved",
            "verify_curved.py",
            ["--probe", probe, "--dump", dump, "--fixtures", curved],
        ),
        (
            "reader",
            "verify_reader.py",
            [
                "--reader",
                reader,
                "--evidence-probe",
                probe,
                "--rectangular",
                rectangular,
                "--curved",
                curved,
            ],
        ),
        (
            "export",
            "verify_export.py",
            [
                "--exporter",
                exporter,
                "--reader-checks",
                output / "reader",
                "--rectangular",
                rectangular,
                "--curved",
                curved,
            ],
        ),
    ]
    results = []
    for name, script, flags in stages:
        print("START " + name, flush=True)
        started = time.perf_counter()
        command = [
            sys.executable,
            str(scripts / script),
            *map(str, flags),
            "--output",
            str(output / name),
        ]
        with (output / (name + ".log")).open("w", encoding="utf-8") as log:
            result = subprocess.run(
                command, stdout=log, stderr=subprocess.STDOUT, timeout=1800
            )
        require(
            result.returncode == 0,
            "replay failed; see " + str(output / (name + ".log")),
        )
        result = json.loads(
            (output / name / "verification.json").read_text(encoding="utf-8")
        )
        expected = (
            "verified_native_faces_with_recorded_ifc_discrepancies"
            if name == "curved"
            else "passed"
        )
        require(result["status"] == expected, "unexpected replay status: " + name)
        results.append(
            dict(
                stage=name,
                status=result["status"],
                seconds=time.perf_counter() - started,
            )
        )
        print("PASS " + name, flush=True)
    require(verify_manifest(folder) == manifest, "package changed during replay")
    (output / "verification.json").write_text(
        json.dumps(
            dict(
                status="passed",
                package_manifest_sha256=digest(folder / "manifest.json"),
                binaries={p.name: digest(p) for p in [exporter, dump, probe, reader]},
                stages=results,
            ),
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    print(
        "Package passed: byte evidence, native references, reader controls and analytic export",
        flush=True,
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["bin-dir", "fixtures", "output"]:
        parser.add_argument("--" + name, type=Path, required=True)
    verify(parser.parse_args())
