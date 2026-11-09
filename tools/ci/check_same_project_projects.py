#!/usr/bin/env python3
"""The projects, the years and the pins the same-project gate holds, as each file that names them says: each
list in several places, which must agree or the verdict does not see a sample (RE-346).

  same_project_verdict.py            PROJECTS, YEARS
  check-bodies-across-saves.sh       the `for project in` list and the `for year in` list
  measure-reference-models.sh        the two `case ... in` lists of project keys (saves_gate_keeps, bodies_across_saves),
                                     the year case of saves_gate_keeps and the `202[...]` globs of bodies_across_saves
  bodies_across_saves.py             the keys of MINIMUMS, OUTLINE_LEAST, REQUIRED and CYLINDER_LEAST
  readers_across_saves.py            the keys of MINIMUMS (and a year-keyed PROPERTIES), PINNED_WALLS and
                                     ROOFS_NOT_THE_2025_BODY

PINNED_WALLS was the one list of bodies allowed to differ from the 2025 save (Griffin, 2026-11-05). It could only
shrink, and it is empty since #1471 healed its 16 walls: PINNED_MOST is 0, no project has more than the walls
ORIGINAL_PINNED_TAGS holds, and no entry is ever added. Raising ORIGINAL_PINNED_TAGS or PINNED_MOST here is adding to
it.

ROOFS_NOT_THE_2025_BODY is the other list: the roofs a 2026 or 2027 save may draw otherwise than the 2025 save
does. Griffin approved 201262 and 243274 on 2026-11-05 and 165755, 167036 and 167158 on 2026-11-09 (#1610, "Allow
these 4 as meshes"). It can only shrink: no project has a tag ORIGINAL_ROOF_TAGS does not hold, and none is added.

Run from the repository root; exits 1 and says which file differs.
"""
import ast
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools" / "ci"))

import same_project_verdict  # noqa: E402

PINNED_MOST = 0
ORIGINAL_PINNED_TAGS = {
    "racbasicsampleproject": frozenset(
        {
            "234869",
            "428588",
            "428745",
            "429964",
            "430064",
            "430318",
            "430361",
            "430412",
            "430859",
            "497540",
            "506386",
            "506797",
            "768442",
            "937935",
            "938974",
            "939084",
        }
    )
}
ORIGINAL_ROOF_TAGS = {
    "racadvancedsampleproject": frozenset({"201262", "165755", "167036", "167158"}),
    "racbasicsampleproject": frozenset({"243274"}),
}
REFERENCE_YEAR = 2025
FIRST_YEAR = 2024


def assignments(path):
    tree = ast.parse(path.read_text())
    out = {}
    for node in tree.body:
        if isinstance(node, ast.Assign):
            for target in node.targets:
                if isinstance(target, ast.Name):
                    out[target.id] = node.value
    return out


def resolve(node, names):
    while isinstance(node, ast.Name):
        node = names[node.id]
    return node


def constant_keys(node, names):
    node = resolve(node, names)
    return {key.value for key in node.keys if isinstance(key, ast.Constant)}


def dict_items(node, names):
    node = resolve(node, names)
    return [(resolve(key, names).value, value) for key, value in zip(node.keys, node.values) if isinstance(resolve(key, names), ast.Constant)]


def glob_years(text):
    """The years a `202[567]`-style glob names."""
    return {int(f"202{digit}") for digit in text}


def main():
    projects = set(same_project_verdict.PROJECTS)
    years = set(same_project_verdict.YEARS)
    later = years - {REFERENCE_YEAR}
    found = {}
    years_found = {}
    problems = []

    shell = (ROOT / "tools/ci/check-bodies-across-saves.sh").read_text()
    match = re.search(r"for project in ([a-z ]+); do", shell)
    found["tools/ci/check-bodies-across-saves.sh"] = set(match.group(1).split()) if match else set()
    match = re.search(r"for year in ([0-9 ]+); do", shell)
    years_found["tools/ci/check-bodies-across-saves.sh, for year"] = {int(y) for y in match.group(1).split()} if match else set()

    measure = (ROOT / "tools/ci/measure-reference-models.sh").read_text()
    cases = re.findall(r'case "\$\(pair_key "\$[a-z0-9]+"\)" in ([a-z |]+?)\)', measure)
    for number, keys in enumerate(cases, 1):
        found[f"tools/ci/measure-reference-models.sh, case {number}"] = {f"{key}sampleproject" for key in keys.split(" | ")}
    if len(cases) != 2:
        found["tools/ci/measure-reference-models.sh"] = {f"{len(cases)} case lists, not 2"}
    match = re.search(r'case "\$\(basename "\$1" \| cut -c1-4\)" in ([0-9 |]+)\)', measure)
    years_found["tools/ci/measure-reference-models.sh, saves_gate_keeps"] = (
        {int(y) for y in match.group(1).split(" | ")} - {FIRST_YEAR} if match else set()
    )
    match = re.search(r'for candidate in "\$SAMPLE_DIR"/202\[([0-9]+)\]-\*\.rvt', measure)
    years_found["tools/ci/measure-reference-models.sh, candidate glob"] = glob_years(match.group(1)) if match else set()
    match = re.search(r'for earlier in "\$SAMPLE_DIR"/202\[([0-9]+)\]-\*\.rvt', measure)
    earlier = glob_years(match.group(1)) if match else set()
    if earlier != {FIRST_YEAR, REFERENCE_YEAR}:
        problems.append(f"tools/ci/measure-reference-models.sh, earlier glob: {sorted(earlier)} is not {[FIRST_YEAR, REFERENCE_YEAR]}")

    bodies = assignments(ROOT / "tools/re/bodies_across_saves.py")
    readers = assignments(ROOT / "tools/re/readers_across_saves.py")
    found["tools/re/bodies_across_saves.py, MINIMUMS"] = constant_keys(bodies["MINIMUMS"], bodies)
    found["tools/re/readers_across_saves.py, MINIMUMS"] = constant_keys(readers["MINIMUMS"], readers)
    for name, expected in (("OUTLINE_LEAST", years), ("CYLINDER_LEAST", later)):
        for project, value in dict_items(bodies[name], bodies):
            if project not in projects:
                problems.append(f"tools/re/bodies_across_saves.py, {name}: {project} is not one of the projects")
            held = {int(k.value) for k in resolve(value, bodies).keys}
            if name == "OUTLINE_LEAST" and held != expected:
                problems.append(f"tools/re/bodies_across_saves.py, {name}[{project}]: years {sorted(held)}, not {sorted(expected)}")
            if name == "CYLINDER_LEAST" and not held <= expected:
                problems.append(f"tools/re/bodies_across_saves.py, {name}[{project}]: years {sorted(held)} are not later saves")
    found["tools/re/bodies_across_saves.py, OUTLINE_LEAST"] = constant_keys(bodies["OUTLINE_LEAST"], bodies)
    for name in ("OPENINGS_LEAST", "OPENINGS_LEFT_OUT_MOST"):
        for project, value in dict_items(bodies[name], bodies):
            held = {int(k.value) for k in resolve(value, bodies).keys}
            if project not in projects or held != years:
                problems.append(f"tools/re/bodies_across_saves.py, {name}[{project}]: years {sorted(held)}, not {sorted(years)}")
    if constant_keys(bodies["OPENINGS_LEAST"], bodies) != constant_keys(bodies["OPENINGS_LEFT_OUT_MOST"], bodies):
        problems.append("tools/re/bodies_across_saves.py: OPENINGS_LEAST and OPENINGS_LEFT_OUT_MOST name different projects")
    for project, value in dict_items(bodies["REQUIRED"], bodies):
        held = {int(k.value) for k in resolve(value, bodies).keys}
        if project not in projects or not held <= years:
            problems.append(f"tools/re/bodies_across_saves.py, REQUIRED[{project}]: years {sorted(held)}")
    for project, value in dict_items(readers["MINIMUMS"], readers):
        for check, number in dict_items(value, readers):
            if isinstance(resolve(number, readers), ast.Dict):
                held = {int(k.value) for k in resolve(number, readers).keys}
                if held != later:
                    problems.append(f"tools/re/readers_across_saves.py, MINIMUMS[{project}][{check}]: years {sorted(held)}, not {sorted(later)}")

    pinned = dict_items(readers["PINNED_WALLS"], readers)
    for project, value in pinned:
        if project not in ORIGINAL_PINNED_TAGS:
            problems.append(f"tools/re/readers_across_saves.py, PINNED_WALLS[{project}]: this project has no pins to hold")
            continue
        per_year = dict_items(value, readers)
        if {year for year, _ in per_year} != later:
            problems.append(f"tools/re/readers_across_saves.py, PINNED_WALLS[{project}]: years {sorted(y for y, _ in per_year)}, not {sorted(later)}")
        for year, walls in per_year:
            tags = constant_keys(walls, readers)
            if len(tags) > PINNED_MOST:
                problems.append(f"tools/re/readers_across_saves.py, PINNED_WALLS[{project}][{year}]: {len(tags)} walls, more than {PINNED_MOST}")
            extra = tags - ORIGINAL_PINNED_TAGS[project]
            if extra:
                problems.append(f"tools/re/readers_across_saves.py, PINNED_WALLS[{project}][{year}]: {sorted(extra)} were never pinned, and none is added")
    pinned_most = ast.literal_eval(readers["PINNED_MOST"])
    if pinned_most != PINNED_MOST:
        problems.append(f"tools/re/readers_across_saves.py, PINNED_MOST is {pinned_most}, not {PINNED_MOST}")
    for project, value in dict_items(readers["ROOFS_NOT_THE_2025_BODY"], readers):
        if project not in projects:
            problems.append(f"tools/re/readers_across_saves.py, ROOFS_NOT_THE_2025_BODY: {project} is not one of the projects")
        tags = {resolve(key, readers).value for key in resolve(value, readers).keys}
        extra = tags - ORIGINAL_ROOF_TAGS.get(project, frozenset())
        if extra:
            problems.append(
                f"tools/re/readers_across_saves.py, ROOFS_NOT_THE_2025_BODY[{project}]: {sorted(extra)} were never approved, and none is added"
            )

    bad = bool(problems)
    for where, held in sorted(found.items()):
        if held != projects:
            bad = True
            print(f"{where}: {sorted(held)} is not tools/ci/same_project_verdict.py's PROJECTS {sorted(projects)}")
    for where, held in sorted(years_found.items()):
        if held != years:
            bad = True
            print(f"{where}: {sorted(held)} is not tools/ci/same_project_verdict.py's YEARS {sorted(years)}")
    for line in problems:
        print(line)
    if bad:
        return 1
    print(
        f"same-project projects agree in {len(found) + 1} places: {', '.join(sorted(projects))}; "
        f"years in {len(years_found) + 1} places: {', '.join(str(y) for y in sorted(years))}; "
        f"pinned walls at most {PINNED_MOST} a year, none beyond the {sum(len(t) for t in ORIGINAL_PINNED_TAGS.values())} first pinned"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
