"""Run every ``python`` block of docs/python.md against the tier-1 fixture.

The page is read top to bottom, so its blocks run in order in one
namespace, from a scratch directory holding the synthetic project file
under the names the examples use (1.0 criteria S4, #872). Blocks that show
a signature or a result shape are fenced ``text`` and are not run.
"""

from __future__ import annotations

import os
import pathlib
import re
import shutil

import rvt  # type: ignore  # noqa: F401  (the blocks import it themselves)

CRATE = pathlib.Path(__file__).resolve().parent.parent.parent
FIXTURE = CRATE / "corpus/tier1/architectural-2024/architectural-2024.rvt"


def python_blocks() -> list[tuple[int, str]]:
    text = (CRATE / "docs/python.md").read_text(encoding="utf-8")
    return [
        (text.count("\n", 0, m.start()) + 1, m.group(1))
        for m in re.finditer(r"```python\n(.*?)```", text, re.S)
    ]


def test_every_python_block_in_the_docs_runs(tmp_path: pathlib.Path) -> None:
    shutil.copy(FIXTURE, tmp_path / "my-family.rfa")
    (tmp_path / "projects").mkdir()
    shutil.copy(FIXTURE, tmp_path / "projects" / "architectural-2024.rvt")
    namespace: dict[str, object] = {"untrusted_path": str(FIXTURE)}
    blocks = python_blocks()
    assert blocks, "docs/python.md has no python blocks"
    cwd = os.getcwd()
    os.chdir(tmp_path)
    try:
        for line, code in blocks:
            try:
                exec(compile(code, f"docs/python.md:{line}", "exec"), namespace)
            except Exception as error:  # noqa: BLE001
                raise AssertionError(f"docs/python.md block at line {line} failed: {error!r}") from error
    finally:
        os.chdir(cwd)
