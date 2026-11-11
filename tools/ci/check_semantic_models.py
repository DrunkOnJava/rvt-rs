#!/usr/bin/env python3
"""The models the semantic check judges, as each file that names them says: lists that must agree, or a complete
Measure run is judged on fewer models than it measured and its check reads NOT MEASURED (#1675).

  tools/ci/measure-reference-models.sh   MODELS_LIST (the models it measures by name) and the `case` arm that writes
                                         an MCP dump (`dump=(--dump ...)`)
  tools/re/semantic_diff.py              OPEN_MODELS (the globs of models that may be sent to a third party) and
                                         OPEN_MODEL_NAMES (every open model a complete run measures)

The ways it goes wrong, each of which this names before a run does:

  1. An open model of MODELS_LIST that no dump pattern matches: its folders hold no dump, the run skips it as
     "no dump", and the check reads a gap (the timber office on 2026-11-10, in every complete run after it).
  2. A model that gets a dump and is not open: the Compare job fails the run after the dump was written, and what
     protects the data is that only openly licensed models have one.
  3. An open model of MODELS_LIST that OPEN_MODEL_NAMES lacks: a complete run is judged without it, so a change
     that moves only that model passes unseen.
  4. A name in OPEN_MODEL_NAMES that nothing provides: no list of the fetcher names it and no generated family
     covers it (`r2025-*` names come from the archives a run fetches, so the family's pattern does), so no run
     could ever cover every open model and the check could never read a pass.
  5. A dump pattern that matches no model, which is a name that went stale.

Run from the repository root; `--root DIR` reads another tree, and `--refusals` shows each of the five on a mutated
copy of the two files.
"""
import fnmatch
import importlib.util
import pathlib
import re
import shutil
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = "tools/ci/measure-reference-models.sh"
SEMANTIC = "tools/re/semantic_diff.py"
# Models a run builds from the archives it fetches, named by the file inside (puzzbobb's Revit 2025 models).
GENERATED = ("r2025-*",)


def load_semantic(root):
    spec = importlib.util.spec_from_file_location("semantic_diff_under_check", root / SEMANTIC)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def model_names(script):
    """The names of MODELS_LIST, the first field of each `name|rvt|ifc` line."""
    found = re.search(r'^MODELS_LIST="\n(.*?)^"', script, re.S | re.M)
    if not found:
        raise SystemExit(f"{SCRIPT}: no MODELS_LIST=\"...\" block found")
    names = []
    for line in found.group(1).splitlines():
        line = line.strip()
        if line and not line.startswith("#"):
            names.append(line.split("|", 1)[0])
    return names


def dump_patterns(script):
    """The patterns of the `case "$name" in` arm that sets `dump=(--dump ...)`."""
    found = re.search(r'case "\$name" in\s*\n\s*([^)\n]*)\)\s*\n\s*dump=\(--dump', script)
    if not found:
        raise SystemExit(f"{SCRIPT}: no `case \"$name\" in ... ) dump=(--dump` arm found")
    return [pattern.strip() for pattern in found.group(1).split("|") if pattern.strip()]


def matches(name, patterns):
    return any(fnmatch.fnmatchcase(name, pattern) for pattern in patterns)


def check(root):
    """The problems of the two files under `root`, as (file, message)."""
    root = pathlib.Path(root)
    script = (root / SCRIPT).read_text(encoding="utf-8")
    sd = load_semantic(root)
    names = model_names(script)
    patterns = dump_patterns(script)
    problems = []
    for name in names:
        scope = sd.model_scope(name)
        if scope == "open" and not matches(name, patterns):
            problems.append((SCRIPT, f"{name} is an open model of MODELS_LIST that no dump pattern matches, so a complete run holds no dump for it and the semantic check reads NOT MEASURED (#1675)"))
        if scope != "open" and matches(name, patterns):
            problems.append((SCRIPT, f"{name} gets an MCP dump and is {scope}: only openly licensed models may have one"))
        if scope == "open" and name not in sd.OPEN_MODEL_NAMES:
            problems.append((SEMANTIC, f"{name} is open and measured by name but missing from OPEN_MODEL_NAMES, so a complete run is judged without it"))
    for name in sd.OPEN_MODEL_NAMES:
        if name not in names and not matches(name, GENERATED):
            problems.append((SEMANTIC, f"OPEN_MODEL_NAMES lists {name}, which no list of the fetcher provides"))
        if sd.model_scope(name) != "open":
            problems.append((SEMANTIC, f"OPEN_MODEL_NAMES lists {name}, which OPEN_MODELS does not name as open"))
        if not matches(name, patterns):
            problems.append((SCRIPT, f"OPEN_MODEL_NAMES lists {name}, which no dump pattern matches"))
    for pattern in patterns:
        if not any(fnmatch.fnmatchcase(name, pattern) for name in list(names) + list(sd.OPEN_MODEL_NAMES)):
            problems.append((SCRIPT, f"the dump pattern {pattern} matches no model of MODELS_LIST or OPEN_MODEL_NAMES"))
    return problems, len(sd.OPEN_MODEL_NAMES), len(names)


def report(problems, open_count, listed):
    for file, message in problems:
        print(f"::error file={file}::{message}")
    if problems:
        print(f"{len(problems)} problems; see docs/maintaining.md, \"The semantic check\"")
        return 1
    print(f"The semantic check names the same {open_count} open models in the dump case, OPEN_MODEL_NAMES and the {listed} models of MODELS_LIST")
    return 0


# Each refusal: edits (file, old text, new text) to a copy of the two files, and the text the check must name.
REFUSALS = [
    (
        "an open model with no dump pattern (the housing, or the timber office of #1675)",
        [(SCRIPT, "nordic-housing-* | ", "")],
        "nordic-housing-concrete-as-built is an open model of MODELS_LIST that no dump pattern matches",
    ),
    (
        "a model that joins the fetcher and matches an open glob but no dump pattern",
        [(SCRIPT, "einhoven|Revit_IFC5_Einhoven.rvt|-\n", "einhoven|Revit_IFC5_Einhoven.rvt|-\nnordic-office-new|new.rvt|-\n")],
        "nordic-office-new is an open model of MODELS_LIST that no dump pattern matches",
    ),
    (
        "that same new model, missing from OPEN_MODEL_NAMES",
        [(SCRIPT, "einhoven|Revit_IFC5_Einhoven.rvt|-\n", "einhoven|Revit_IFC5_Einhoven.rvt|-\nnordic-office-new|new.rvt|-\n")],
        "nordic-office-new is open and measured by name but missing from OPEN_MODEL_NAMES",
    ),
    (
        "a restricted model that gets a dump",
        [
            (SCRIPT, "einhoven|Revit_IFC5_Einhoven.rvt|-\n", "einhoven|Revit_IFC5_Einhoven.rvt|-\nbimcollab-example|b.rvt|-\n"),
            (SCRIPT, " | r2025-*)", " | bimcollab-* | r2025-*)"),
        ],
        "bimcollab-example gets an MCP dump and is restricted",
    ),
    (
        "a name in OPEN_MODEL_NAMES that nothing provides",
        [(SEMANTIC, '    "einhoven",\n', '    "einhoven",\n    "nordic-office-ghost",\n')],
        "OPEN_MODEL_NAMES lists nordic-office-ghost, which no list of the fetcher provides",
    ),
    (
        "a dump pattern that matches nothing",
        [(SCRIPT, " | r2025-*)", " | nordic-none-* | r2025-*)")],
        "the dump pattern nordic-none-* matches no model",
    ),
]


def refusals(root):
    problems, _, _ = check(root)
    if problems:
        print("the unmutated files already fail; fix those first:")
        report(problems, 0, 0)
        return 1
    failed = 0
    for why, edits, expected in REFUSALS:
        with tempfile.TemporaryDirectory() as scratch:
            copy = pathlib.Path(scratch)
            for relative in (SCRIPT, SEMANTIC):
                target = copy / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy(pathlib.Path(root) / relative, target)
            for relative, old, new in edits:
                text = (copy / relative).read_text(encoding="utf-8")
                if text.count(old) != 1:
                    print(f"NOT RUN  {why}: the text to mutate stands {text.count(old)} times in {relative}")
                    failed = 1
                    break
                (copy / relative).write_text(text.replace(old, new), encoding="utf-8")
            else:
                found, _, _ = check(copy)
                if any(expected in message for _, message in found):
                    print(f"refused  {why}")
                else:
                    print(f"MISSED   {why}: no problem names \"{expected}\"")
                    failed = 1
    return failed


def main(argv):
    root = pathlib.Path(argv[argv.index("--root") + 1]) if "--root" in argv else ROOT
    if "--refusals" in argv:
        return refusals(root)
    return report(*check(root))


if __name__ == "__main__":
    sys.exit(main(sys.argv))
