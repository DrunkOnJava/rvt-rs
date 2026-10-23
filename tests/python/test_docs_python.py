"""Run the Python documentation's examples against the tier-1 fixture.

Every ``python`` block of docs/python.md runs, top to bottom in one
namespace, from a scratch directory holding the synthetic project file
under the names the examples use (1.0 criteria S4, #872). Blocks that show
a signature or a result shape are fenced ``text`` and are not run.

The PyPI page is the installed distribution's own description, so it is
read from the wheel under test (#905): it must be ``rvt-py/README.md``,
link only to absolute URLs (PyPI does not resolve relative links), and its
example must run.
"""

from __future__ import annotations

import importlib.metadata
import os
import pathlib
import re
import shutil

import rvt  # type: ignore  # noqa: F401  (the blocks import it themselves)

CRATE = pathlib.Path(__file__).resolve().parent.parent.parent
FIXTURE = CRATE / "corpus/tier1/architectural-2024/architectural-2024.rvt"


def python_blocks(text: str) -> list[tuple[int, str]]:
    return [
        (text.count("\n", 0, m.start()) + 1, m.group(1))
        for m in re.finditer(r"```python\n(.*?)```", text, re.S)
    ]


def pypi_description() -> str:
    meta = importlib.metadata.metadata("rvt")
    return meta.get("Description") or meta.get_payload()  # type: ignore[attr-defined]


def run_blocks(name: str, text: str, tmp_path: pathlib.Path) -> None:
    shutil.copy(FIXTURE, tmp_path / "my-family.rfa")
    shutil.copy(FIXTURE, tmp_path / "model.rvt")
    (tmp_path / "projects").mkdir()
    shutil.copy(FIXTURE, tmp_path / "projects" / "architectural-2024.rvt")
    namespace: dict[str, object] = {"untrusted_path": str(FIXTURE)}
    blocks = python_blocks(text)
    assert blocks, f"{name} has no python blocks"
    cwd = os.getcwd()
    os.chdir(tmp_path)
    try:
        for line, code in blocks:
            try:
                exec(compile(code, f"{name}:{line}", "exec"), namespace)
            except Exception as error:  # noqa: BLE001
                raise AssertionError(f"{name} block at line {line} failed: {error!r}") from error
    finally:
        os.chdir(cwd)


def test_every_python_block_in_the_docs_runs(tmp_path: pathlib.Path) -> None:
    run_blocks("docs/python.md", (CRATE / "docs/python.md").read_text(encoding="utf-8"), tmp_path)


def test_the_pypi_page_is_the_python_readme_with_absolute_links() -> None:
    description = pypi_description()
    readme = (CRATE / "rvt-py/README.md").read_text(encoding="utf-8")
    lines = [line.rstrip() for line in description.strip().splitlines()]
    assert lines == [line.rstrip() for line in readme.strip().splitlines()], "the wheel's description is not rvt-py/README.md"
    targets = re.findall(r"\]\(([^)\s]+)\)", description)
    targets += re.findall(r"(?m)^\s{0,3}\[[^\]]+\]:\s*<?([^>\s]+)", description)  # reference definitions
    relative = [t for t in targets if not t.startswith("https://")]
    assert not relative, f"the PyPI page has links PyPI cannot resolve: {relative}"


def test_the_pypi_page_example_runs(tmp_path: pathlib.Path) -> None:
    run_blocks("PyPI page (rvt-py/README.md)", pypi_description(), tmp_path)
