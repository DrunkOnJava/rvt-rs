"""Release smoke test for an installed `rvt` wheel.

Opens a Revit file through the compiled extension, reads its release and
schema, and exports it to IFC with its diagnostics, so a wheel that installs but cannot parse
(a wrong ABI, a missing symbol, a broken build) fails the release. Runs on
CPython 3.8 (the manylinux2014 image) and later.

Usage: python tools/ci/wheel-smoke.py FILE.rvt
"""

import json
import platform
import sys

import rvt


def main() -> int:
    path = sys.argv[1]
    model = rvt.RevitFile(path)
    version = model.version
    streams = model.stream_names()
    schema = json.loads(model.schema_json())
    diagnostics = json.loads(model.export_diagnostics_json())
    ifc = model.write_ifc("scaffold")
    print(
        "{} {}: Revit {}, {} streams, {} schema classes, {} bytes of IFC, export mode {}".format(
            platform.system(),
            platform.machine(),
            version,
            len(streams),
            len(schema.get("classes", [])),
            len(ifc),
            diagnostics.get("mode"),
        )
    )
    failures = []
    if version is None:
        failures.append("no Revit release read")
    if not streams:
        failures.append("no streams listed")
    if not schema.get("classes"):
        failures.append("no schema classes parsed")
    if not ifc.startswith("ISO-10303-21;") or "IFCPROJECT" not in ifc:
        failures.append("IFC export is not a STEP file with a project")
    for failure in failures:
        print("FAIL: " + failure)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
