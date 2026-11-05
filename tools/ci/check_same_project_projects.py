#!/usr/bin/env python3
"""The projects the same-project gate holds, as each file that names them says: one list in five places, which
must agree or the verdict does not see a sample (RE-346).

  same_project_verdict.py            PROJECTS
  check-bodies-across-saves.sh       the `for project in` list
  measure-reference-models.sh        the two `case ... in` lists of project keys (saves_gate_keeps, bodies_across_saves)
  bodies_across_saves.py             the keys of MINIMUMS
  readers_across_saves.py            the keys of MINIMUMS

Run from the repository root; exits 1 and says which file differs.
"""
import ast
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "ci"))

import same_project_verdict  # noqa: E402


def minimums_keys(path):
    tree = ast.parse(path.read_text())
    for node in ast.walk(tree):
        if isinstance(node, ast.Assign) and any(isinstance(t, ast.Name) and t.id == "MINIMUMS" for t in node.targets):
            return {key.value for key in node.value.keys if isinstance(key, ast.Constant)}
    raise SystemExit(f"{path}: no MINIMUMS")


def main():
    expected = set(same_project_verdict.PROJECTS)
    found = {}
    shell = (ROOT / "tools/ci/check-bodies-across-saves.sh").read_text()
    match = re.search(r"for project in ([a-z ]+); do", shell)
    found["tools/ci/check-bodies-across-saves.sh"] = set(match.group(1).split()) if match else set()
    measure = (ROOT / "tools/ci/measure-reference-models.sh").read_text()
    cases = re.findall(r'case "\$\(pair_key "\$[a-z0-9]+"\)" in ([a-z |]+?)\)', measure)
    for number, keys in enumerate(cases, 1):
        found[f"tools/ci/measure-reference-models.sh, case {number}"] = {f"{key}sampleproject" for key in keys.split(" | ")}
    if len(cases) != 2:
        found["tools/ci/measure-reference-models.sh"] = {f"{len(cases)} case lists, not 2"}
    for name in ("bodies_across_saves", "readers_across_saves"):
        found[f"tools/re/{name}.py"] = minimums_keys(ROOT / "tools" / "re" / f"{name}.py")
    bad = False
    for where, projects in sorted(found.items()):
        if projects != expected:
            bad = True
            print(f"{where}: {sorted(projects)} is not tools/ci/same_project_verdict.py's PROJECTS {sorted(expected)}")
    if bad:
        return 1
    print(f"same-project projects agree in {len(found) + 1} places: {', '.join(sorted(expected))}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
