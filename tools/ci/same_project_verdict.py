#!/usr/bin/env python3
"""The verdict of the same-project gates on the commit Measure measured.

Griffin, 2026-11-05: a body or Area no Revit export can check is held to the same
project's earlier save, and a difference blocks landing. Measure is not a required
check, so this writes what the two gates found as a check run on the measured
commit, `same-project gates`, and the landing script reads it across a pull
request's commits. success means both gates compared what they should and passed;
failure means one found a difference or left no result; neutral (NOT MEASURED)
means they compared nothing, or not everything, and that never counts as a pass.
A run with a measure shard that did not finish is neutral too, unless a result
that did arrive fails. The Areas gate has a second part, rac_advanced's 2021 save
against its 2022 save (#1434, `areas_across_saves_2021.txt`): it is required when
the run's sample years hold both saves, and a result that fails fails the verdict
whether or not it was required.

Usage: same_project_verdict.py RESULTS_DIR [--flags=FLAGS] [--samples=true|false]
                               [--sample-years="FIRST [LAST]"] [--scorers=NAMES]
                               [--measure-result=RESULT]
The `=` forms are what a caller passes: argparse reads a separate value that starts
with `--` as another option, and `--saved-meshes` is a value `ifc_flags` takes.
Prints a JSON object: conclusion, title, summary.
"""
import argparse
import json
import pathlib
import sys

PROJECTS = ("racadvancedsampleproject", "racbasicsampleproject", "rstbasicsampleproject")
YEARS = (2026, 2027)
AREAS_PASSED = "areas across saves: passed: 25 Areas, 25 with a body, on each of 4 saves"
AREAS_2021_PASSED = "areas across saves: passed: 25 Areas, 25 with a body, on each of 2 saves"


def body_state(text):
    """PASS, FAIL or a not-compared word for one bodies_across_saves.txt; None text is no result."""
    if text is None:
        return "FAIL"
    lines = text.splitlines()
    if not lines or lines[-1] != "exit 0":
        return "FAIL"
    if "bodies across saves: PASS" in lines:
        return "PASS"
    for word in ("PARTIAL", "SKIPPED"):
        if any(line.startswith(f"bodies across saves: {word}") for line in lines):
            return word
    return "FAIL"


def areas_state(texts, passed=AREAS_PASSED):
    """PASS, FAIL or SKIPPED for the areas_across_saves.txt of every shard; no file at all is no result."""
    if not texts:
        return "FAIL"
    state = "SKIPPED"
    for text in texts:
        lines = text.splitlines()
        if lines and all(line.startswith("areas across saves: skipped:") for line in lines):
            continue
        if not lines or lines[-1] != "exit 0" or not any(line.startswith(passed) for line in lines):
            return "FAIL"
        state = "PASS"
    return state


def areas_2021_state(texts, required, incomplete=None):
    """PASS, FAIL, SKIPPED or NOT REQUIRED for the areas_across_saves_2021.txt of every shard: a result that
    arrived is judged whether or not it was required, a missing one fails only when it was and the run finished."""
    if texts:
        state = areas_state(texts, AREAS_2021_PASSED)
        return state if required or state == "FAIL" else "NOT REQUIRED"
    if not required:
        return "NOT REQUIRED"
    return "SKIPPED" if incomplete else "FAIL"


def judge(bodies, areas, not_run=None, incomplete=None, areas_2021=None, expect_2021=False):
    """(conclusion, title, summary lines). `bodies` maps a sample to its result text or None, `areas` is every
    shard's text, `not_run` says why the run could not check anything, and `incomplete` that a shard did not
    finish: then a missing result says nothing, but a result that arrived and fails still fails. `areas_2021`
    is every shard's text of the 2021 save's gate and `expect_2021` whether the run's samples hold both saves."""
    if not_run:
        return "neutral", f"NOT MEASURED: {not_run}", [not_run]
    if incomplete:
        bodies = {name: text for name, text in bodies.items() if text is not None}
    states = {name: body_state(text) for name, text in bodies.items()}
    area = areas_state(areas) if areas or not incomplete else "SKIPPED"
    area_2021 = areas_2021_state(areas_2021, expect_2021, incomplete)
    lines = [f"{name}: {state}" for name, state in sorted(states.items())] + [f"Areas across rac_advanced's saves: {area}"]
    if area_2021 != "NOT REQUIRED":
        lines.append(f"Areas of rac_advanced's 2021 save against its 2022 save: {area_2021}")
    if incomplete:
        lines.insert(0, incomplete)
    failed = [name for name, state in states.items() if state == "FAIL"]
    if failed or area == "FAIL" or area_2021 == "FAIL":
        what = ", ".join(sorted(failed) + (["the Areas"] if area == "FAIL" else []) + (["the 2021 save's Areas"] if area_2021 == "FAIL" else []))
        return "failure", f"fail: {what}", lines
    if incomplete:
        return "neutral", f"NOT MEASURED: {incomplete}", lines
    if not states or any(state != "PASS" for state in states.values()) or area != "PASS" or (expect_2021 and area_2021 != "PASS"):
        return "neutral", "NOT MEASURED: the gates compared less than every save", lines
    held = f"pass: {len(states)} samples held to their earlier saves, and the Areas on 4 saves"
    return "success", held + (" and on the 2021 save" if area_2021 == "PASS" else ""), lines


def expected_samples(sample_years):
    """The 2026 and 2027 samples the bodies gate holds, within `sample_years`."""
    words = (sample_years or "").split()
    low = int(words[0]) if words else 0
    high = int(words[1]) if len(words) > 1 else 9999
    return [f"sample-{year}-{project}" for year in YEARS if low <= year <= high for project in PROJECTS]


def worst(texts):
    """The result of one sample that says least for it, where several shards hold one; None for no result."""
    rank = {"FAIL": 0, "PASS": 2}
    return min(texts, key=lambda text: rank.get(body_state(text), 1), default=None)


def expects_2021(sample_years):
    """Whether the run's sample years hold the 2021 and 2022 saves the Areas gate of the 2021 save compares."""
    words = (sample_years or "").split()
    low = int(words[0]) if words else 0
    high = int(words[1]) if len(words) > 1 else 9999
    return low <= 2021 and high >= 2022


def read_results(results, names):
    root = pathlib.Path(results)
    bodies = {}
    for name in names:
        bodies[name] = worst(path.read_text(errors="replace") for path in sorted(root.glob(f"**/{name}/bodies_across_saves.txt")))
    areas = [path.read_text(errors="replace") for path in sorted(root.glob("**/areas_across_saves.txt"))]
    areas_2021 = [path.read_text(errors="replace") for path in sorted(root.glob("**/areas_across_saves_2021.txt"))]
    return bodies, areas, areas_2021


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("results")
    parser.add_argument("--flags", default="")
    parser.add_argument("--samples", default="")
    parser.add_argument("--sample-years", default="")
    parser.add_argument("--scorers", default="")
    parser.add_argument("--measure-result", default="success")
    args = parser.parse_args()
    not_run = None
    if args.samples != "true":
        not_run = "samples=true was not set"
    elif args.flags.strip():
        not_run = f"the exports used ifc_flags ({args.flags.strip()})"
    elif args.scorers.strip() and "bodies_across_saves" not in args.scorers.split():
        not_run = "the scorers asked for exclude bodies_across_saves"
    names = expected_samples(args.sample_years)
    bodies, areas, areas_2021 = read_results(args.results, names)
    incomplete = None if args.measure_result == "success" else f"a measure shard ended {args.measure_result}"
    conclusion, title, lines = judge(bodies, areas, not_run, incomplete, areas_2021, expects_2021(args.sample_years))
    json.dump({"conclusion": conclusion, "title": title[:200], "summary": "\n".join(f"- {line}" for line in lines)}, sys.stdout)
    print()


if __name__ == "__main__":
    main()
