"""Run the curved-fixture IFC tessellator in a child with a fixed time budget."""

from pathlib import Path
import subprocess
import sys
import tempfile

import numpy as np

TIMEOUT_SECONDS = 120


def load(path, geometry_library="opencascade"):
    with tempfile.TemporaryDirectory(prefix="rvt-curved-ifc-") as folder:
        output = Path(folder) / "mesh.npz"
        result = subprocess.run(
            [
                sys.executable,
                str(Path(__file__).resolve()),
                str(path.resolve()),
                str(output),
                geometry_library,
            ],
            capture_output=True,
            timeout=TIMEOUT_SECONDS,
        )
        if result.returncode:
            raise ValueError(
                "IFC tessellator failed: "
                + result.stderr.decode("utf-8", errors="replace")
            )
        with np.load(output) as data:
            return data["vertices"], data["faces"]


if __name__ == "__main__":
    import ifcopenshell
    import ifcopenshell.geom

    model = ifcopenshell.open(sys.argv[1])
    products = [p for p in model.by_type("IfcProduct") if p.Representation]
    if len(products) != 1 or not products[0].is_a("IfcBuildingElementProxy"):
        raise ValueError("Expected one represented reference proxy")
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    settings.set("mesher-linear-deflection", 0.00003)
    settings.set("mesher-angular-deflection", 0.03)
    shape = ifcopenshell.geom.create_shape(
        settings,
        products[0],
        geometry_library=sys.argv[3] if len(sys.argv) > 3 else "opencascade",
    )
    np.savez(
        sys.argv[2],
        vertices=np.asarray(shape.geometry.verts).reshape(-1, 3),
        faces=np.asarray(shape.geometry.faces).reshape(-1, 3),
    )
