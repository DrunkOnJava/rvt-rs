"""rvt — Python bindings for rvt-rs (Apache-2.0, clean-room Revit reader).

This module re-exports the C extension produced by pyo3/maturin. All
the real work is in the extension; this file exists so the wheel
ships a proper Python package with type stubs (`__init__.pyi`) and
a PEP-561 `py.typed` marker, giving IDEs autocomplete and type
checkers (mypy, pyright) a target they understand.
"""

# Re-export everything from the compiled extension. Maturin builds
# the extension as `rvt.<ext>` when `module-name = "rvt"` is set,
# which means the package and the extension collide — so we ship
# the extension as `rvt._rvt` and re-export here.
#
# (Until the maturin build config is switched to produce `_rvt`,
# this file is a no-op passthrough; the pyo3 module `rvt` is
# discovered directly. Once the switch happens, replace the body
# with `from rvt._rvt import *`.)

# Best-effort: try the nested module first (future layout), then
# fall back to the flat layout that maturin currently produces.
try:
    from rvt._rvt import *  # type: ignore  # noqa: F401,F403
    from rvt._rvt import __version__  # type: ignore  # noqa: F401
except ImportError:
    # Flat pyo3 layout — the compiled extension IS this module.
    # Nothing to re-export; native attributes resolve directly.
    pass
