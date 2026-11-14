"""A broken extension is an ImportError at ``import rvt`` (#1884).

``python/rvt/__init__.py`` re-exports the compiled extension ``rvt._rvt``. It
used to catch ``ImportError`` and go on, so a failed load of the extension (a
missing symbol, a wrong ABI, a library it needs) surfaced at the first use as
``module 'rvt' has no attribute 'RevitFile'``. Each case runs the real package
file in a fresh interpreter, beside a stand-in for the extension that fails in
one way: the ways a load can fail first, then the one way it works.
"""

from __future__ import annotations

import pathlib
import shutil
import subprocess
import sys

import pytest

PACKAGE = pathlib.Path(__file__).resolve().parent.parent.parent / "python" / "rvt"

IMPORT = "import sys; sys.path.insert(0, sys.argv[1]); import rvt; print(rvt.RevitFile.__name__, rvt.__version__)"


def run(tmp_path: pathlib.Path, extension: str | None) -> subprocess.CompletedProcess[str]:
    """``import rvt`` against the package's ``__init__.py`` and ``extension`` as ``rvt/_rvt.py``."""
    package = tmp_path / "rvt"
    package.mkdir()
    shutil.copy(PACKAGE / "__init__.py", package / "__init__.py")
    if extension is not None:
        (package / "_rvt.py").write_text(extension)
    return subprocess.run(
        [sys.executable, "-c", IMPORT, str(tmp_path)],
        capture_output=True,
        text=True,
        timeout=120,
    )


@pytest.mark.parametrize(
    "extension",
    [
        None,
        'raise ImportError("undefined symbol: PyInit__rvt")',
        'raise ImportError("libsomething.so: cannot open shared object file")',
    ],
    ids=["no extension", "a missing symbol", "a missing library"],
)
def test_a_failed_load_of_the_extension_is_an_import_error(
    tmp_path: pathlib.Path, extension: str | None
) -> None:
    done = run(tmp_path, extension)
    assert done.returncode != 0, done.stdout
    # A missing module is the ImportError subclass ModuleNotFoundError.
    assert "ImportError" in done.stderr or "ModuleNotFoundError" in done.stderr, done.stderr
    assert "has no attribute" not in done.stderr, done.stderr


def test_an_extension_that_loads_is_re_exported(tmp_path: pathlib.Path) -> None:
    done = run(tmp_path, '__version__ = "9.9.9"\nclass RevitFile:\n    pass\n')
    assert done.returncode == 0, done.stderr
    assert done.stdout.split() == ["RevitFile", "9.9.9"]
