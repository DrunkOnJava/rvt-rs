"""rvt — Python bindings for rvt-rs (Apache-2.0, clean-room Revit reader).

This module re-exports the C extension produced by pyo3/maturin. All
the real work is in the extension; this file exists so the wheel
ships a proper Python package with type stubs (`__init__.pyi`) and
a PEP-561 `py.typed` marker, giving IDEs autocomplete and type
checkers (mypy, pyright) a target they understand.
"""

# The compiled extension is `rvt._rvt` (pyproject.toml: `module-name =
# "rvt._rvt"`), a submodule of this package, and everything it defines is
# re-exported here. There is no flat layout to fall back to: this file runs only
# when `rvt` is the package, and then `rvt._rvt` is its extension. An import that
# fails here is a broken install (a missing symbol, a wrong ABI, a library the
# extension needs), so it is raised as it is, and not left to surface at the
# first use as "module 'rvt' has no attribute 'RevitFile'" (#1884).
from rvt._rvt import *  # type: ignore  # noqa: F401,F403
from rvt._rvt import __version__  # type: ignore  # noqa: F401

from typing import Any, Dict, List, Union

# The return shapes the stubs name, so that `from rvt import FileMetadata`
# works at runtime too. `typing` generics, since the wheel supports Python 3.8.
FileMetadata = Dict[str, Any]
TypedSchemaSummary = Dict[str, int]
TypedField = Dict[str, Union[str, int, float, List[int]]]
TypedADocument = Dict[str, Union[int, List[TypedField]]]
TypedDecodedElement = Dict[str, Any]
TypedElementCounts = Dict[str, Union[int, Dict[str, int]]]
