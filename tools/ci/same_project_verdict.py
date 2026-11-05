#!/usr/bin/env python3
"""The verdict of the same-project gates on the commit Measure measured.

Griffin, 2026-11-05: a body or Area no Revit export can check is held to the same
project's earlier save, and a difference blocks landing. Measure is not a required
check, so this writes what the two gates found as a check run on the measured
commit, `same-project gates`, and the landing script reads it across a pull
request's commits. success means both gates compared what they should and passed;
failure means one found a difference or left no result; neutral (NOT MEASURED)
means they compared nothing, or not everything, and that never counts as a pass.

Usage: same_project_verdict.py RESULTS_DIR [--flags FLAGS] [--samples true|false]
                               [--sample-years "FIRST LAST"] [--scorers NAMES]
Prints a JSON object: conclusion, title, summary.
"""
import argparse
import json
import pathlib
import sys

PROJECTS = ("racadvancedsampleproject", "racbasicsampleproject", "rstbasicsampleproject")
YEARS = (2026, 2027)
AREAS_PASSED = "areas across saves: passed: 25 Areas, 25 with a body, on each of 4 saves"


def body_state(text):
    """PASS, FAIL or a not-compared word for one bodies_across_saves.txt; None text is no result."""
    if text is None:
        return "FAIL"
    lines = text.splitlines()
    if "exit 0" not in lines:
        return "FAIL"
    if "bodies across saves: PASS" in lines:
        return "PASS"
    for word in ("PARTIAL", "SKIPPED"):
        if any(line.startswith(f"bodies across saves: {word}") for line in lines):
            return word
    return "FAIL"


def areas_state(texts):
    """PASS, FAIL or SKIPPED for the areas_across_saves.txt of every shard."""
    state = "SKIPPED"
    for text in texts:
        lines = text.splitlines()
        if lines and all(line.startswith("areas across saves: skipped:") for line in lines):
            continue
        if not lines or lines[-1] != "exit 0" or not any(line.startswith(AREAS_PASSED) for line in lines):
            return "FAIL"
        state = "PASS"
    return state


def judge(bodies, areas, not_run=None):
    """(conclusion, title, summary lines). `bodies` maps a sample to its result text or None, `areas` is every
    shard's text, `not_run` says why the run could not check anything."""
    if not_run:
        return "neutral", f"NOT MEASURED: {not_run}", [not_run]
    states = {name: body_state(text) for name, text in bodies.items()}
    area = areas_state(areas)
    lines = [f"{name}: {state}" for name, state in sorted(states.items())] + [f"Areas across rac_advanced's saves: {area}"]
    failed = [name for name, state in states.items() if state == "FAIL"]
    if failed or area == "FAIL":
        what = ", ".join(sorted(failed) + (["the Areas"] if area == "FAIL" else []))
        return "failure", f"fail: {what}", lines
    if not states or any(state != "PASS" for state in states.values()) or area != "PASS":
        return "neutral", "NOT MEASURED: the gates compared less than every save", lines
    return "success", f"pass: {len(states)} samples held to their earlier saves, and the Areas on 4 saves", lines


def expected_samples(sample_years):
    """The 2026 and 2027 samples the bodies gate holds, within `sample_years`."""
    words = (sample_years or "").split()
    low, high = (int(words[0]), int(words[1])) if len(words) == 2 else (0, 9999)
    return [f"sample-{year}-{project}" for year in YEARS if low <= year <= high for project in PROJECTS]


def read_results(results, names):
    root = pathlib.Path(results)
    bodies = {}
    for name in names:
        found = sorted(root.glob(f"**/{name}/bodies_across_saves.txt"))
        bodies[name] = found[0].read_text(errors="replace") if found else None
    return bodies, [path.read_text(errors="replace") for path in sorted(root.glob("**/areas_across_saves.txt"))]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("results")
    parser.add_argument("--flags", default="")
    parser.add_argument("--samples", default="")
    parser.add_argument("--sample-years", default="")
    parser.add_argument("--scorers", default="")
    args = parser.parse_args()
    not_run = None
    if args.samples != "true":
        not_run = "samples=true was not set"
    elif args.flags.strip():
        not_run = f"the exports used ifc_flags ({args.flags.strip()})"
    elif args.scorers.strip() and "bodies_across_saves" not in args.scorers.split():
        not_run = "the scorers asked for exclude bodies_across_saves"
    names = expected_samples(args.sample_years)
    bodies, areas = read_results(args.results, names)
    conclusion, title, lines = judge(bodies, areas, not_run)
    json.dump({"conclusion": conclusion, "title": title[:200], "summary": "\n".join(f"- {line}" for line in lines)}, sys.stdout)
    print()


if __name__ == "__main__":
    main()
