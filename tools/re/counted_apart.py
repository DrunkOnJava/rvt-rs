"""The categories the W7 censuses count apart: their registry, the page that lists them, and a reader of a Measure's results.

The property and quantity censuses (tools/re/psets_vs_ifc.py, tools/re/qtos_vs_ifc.py) score each value
Revit's export writes on an object rvt-rs writes as equal, different or missing. A value they set aside
because the comparison it would make is not one (a wall whose export names another type than its saved
file, a key several objects hold), or because Revit's export is not one it can be matched against (its
setup, IFC2X3), is counted apart: in none of equal, different or missing. docs/counted-apart.md lists every
such category with the decision behind it and, from one Measure run, its count per group of models, so
that "different 0" always shows what stood outside it.

    python3 tools/re/counted_apart.py --check
        the registry, the two scorers, the holdout scrub and docs/counted-apart.md agree
    python3 tools/re/counted_apart.py RESULTS_DIR
        print the counts of a Measure's `measure-head` directory, one table, as markdown
    python3 tools/re/counted_apart.py --write RESULTS_DIR --run ID --commit SHA
        rewrite the page's two generated blocks from them

A scorer that gains a counted-apart line fails --check until the line has a category here (so a new way of
setting a value aside cannot go unlisted); a category whose scorer line is gone fails it too.
"""

import ast
import collections
import os
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
PAGE = ROOT / "docs/counted-apart.md"
SCORERS = {"qtos": ROOT / "tools/re/qtos_vs_ifc.py", "psets": ROOT / "tools/re/psets_vs_ifc.py"}
SCRUB = ROOT / "tools/ci/scrub-holdout-results.sh"
CRITERIA = ROOT / "docs/release-1.0-criteria.md"
STATIC = ("<!-- counted-apart:categories -->", "<!-- /counted-apart:categories -->")
COUNTS = ("<!-- counted-apart:counts -->", "<!-- /counted-apart:counts -->")

# A scorer line that sets a value aside says so in one of these words.
KEYWORDS = re.compile(r"counted apart|not scored|left out|leaves out|out of scope")
# A summary line of the scorers that mentions the words without being a category: the Length diagnostic
# of RE-252 carries the phrase "(none counted apart)" when it trusts no curve.
DIAGNOSTIC_LINES = (re.compile(r"^walls whose Length is rvt-rs's curve's: "),)
# The literal pieces of a scorer's source that carry a keyword without naming a category.
IGNORED_LITERALS = ()


def category(id, scorer, title, what, decided, markers, pattern, example, holdout):
    """One category. `markers` are pieces of the scorer's own text that identify its line (a plain string is a
    substring of one of the scorer's string literals, ("exact", s) a whole literal); `pattern` reads the count
    (groups named n..., summed) from the line the scorer prints, which `example` is; `holdout` says the held-out
    models' count files keep the line (tools/ci/scrub-holdout-results.sh)."""
    return dict(id=id, scorer=scorer, title=title, what=what, decided=decided, markers=markers,
                pattern=re.compile(pattern), example=example, holdout=holdout)


CATEGORIES = [
    category(
        "qto-setup-off", "qtos", "Base quantities left off in Revit's export",
        "Revit's export of the model holds no base quantity set, as one made with \"Export base quantities\" off does, so there is nothing to score against. "
        "The quantities rvt-rs writes beside it are counted (`rvt-ifc --no-base-quantities` leaves them out).",
        "RE-242; base quantities on by default, as in Revit's built-in IFC4 setups",
        ["Export base quantities off, RE-242", "which rvt-ifc --no-base-quantities leaves out"],
        r"^Revit's export writes no base quantities \(its setup's Export base quantities off, RE-242\): rvt-rs writes (?P<n>\d+), which",
        "Revit's export writes no base quantities (its setup's Export base quantities off, RE-242): rvt-rs writes 3124, which rvt-ifc --no-base-quantities leaves out",
        True),
    category(
        "qto-known-wrong", "qtos", "Quantities Revit writes that are known to be wrong",
        "Mullions' `CrossSectionArea`, door openings' `Width` and `Height` (which Revit swaps) and a floor's `Perimeter` where its outline is read as feet. "
        "Revit's export writes them and rvt-rs leaves them out, so they are scored as neither missing nor different.",
        "RE-235 §5",
        ["Revit values known to be wrong, left out (RE-235 §5): "],
        r"^Revit values known to be wrong, left out \(RE-235 §5\): \S+ (?P<n>\d+)$",
        "Revit values known to be wrong, left out (RE-235 §5): Qto_MemberBaseQuantities.CrossSectionArea 10003",
        True),
    category(
        "qto-openings-unwritten", "qtos", "Quantities on openings rvt-rs does not write",
        "The openings a door or a wall cuts in a further host, and a curtain wall's or a cabinet's cut, which Revit's export writes and rvt-rs does not (RE-338 counts them by reason).",
        "#1165; RE-338",
        ["a curtain wall's or cabinet's cut, #1165), not scored: "],
        r"^Revit quantities on openings rvt-rs does not write \(a door or wall cut in a further host, a curtain wall's or cabinet's cut, #1165\), not scored: (?P<n>\d+)$",
        "Revit quantities on openings rvt-rs does not write (a door or wall cut in a further host, a curtain wall's or cabinet's cut, #1165), not scored: 188",
        True),
    category(
        "wall-two-lengths", "qtos", "A wall's Length, where Revit's export may write either of two",
        "Revit's export writes a wall's `Length` as its curve's length or as that of its curve cut back at its joins, by how it draws the wall. "
        "Where the two differ nothing in the model says which, so a missing `Length` there is counted apart.",
        "the maintainer, 2026-10-26; RE-252",
        ["counted apart (RE-252): ", ("exact", " (none counted apart)")],
        r"^walls whose curve and its cut-back curve differ, either of which Revit's export may write as their Length, counted apart \(RE-252\): (?P<n>\d+)$",
        "walls whose curve and its cut-back curve differ, either of which Revit's export may write as their Length, counted apart (RE-252): 463",
        True),
    category(
        "wall-width-other-type", "qtos", "A wall's Width, where the two exports name different types",
        "Revit's export of a held-out model was made from a state its saved file does not hold (#1048). Where a wall's own record confirms the type rvt-rs gives it "
        "and Revit's export names another, the differing `Width` is counted apart, not scored as different.",
        "the maintainer, 2026-10-27; RE-253, #1048",
        ["their differing Width counted apart (RE-253): "],
        r"^walls the two exports give different types, their differing Width counted apart \(RE-253\): (?P<n>\d+)$",
        "walls the two exports give different types, their differing Width counted apart (RE-253): 1",
        True),
    category(
        "wall-quantities-other-type", "qtos", "A wall's volumes and side areas, where the two exports name different types",
        "The same wall's `NetVolume`, `GrossVolume`, `NetSideArea` and `GrossSideArea`: rvt-rs takes them from the mesh Revit saved and the saved type's `Width`, so they follow the saved state, "
        "while Revit's export writes them from the state it was made from.",
        "the maintainer, 2026-11-09; RE-253, RE-388",
        ["their other differing wall quantities counted apart (RE-253, RE-388): "],
        r"^walls the two exports give different types, their other differing wall quantities counted apart \(RE-253, RE-388\): (?P<n>\d+)$",
        "walls the two exports give different types, their other differing wall quantities counted apart (RE-253, RE-388): 2",
        True),
    category(
        "qto-own-unit", "qtos", "Quantities with a unit of their own",
        "A quantity that carries a unit of its own, rather than the file's unit assignment, is not scored.",
        "the census's definition (tools/re/qtos_vs_ifc.py)",
        ["quantities with their own unit, not scored: rvt-rs "],
        r"^quantities with their own unit, not scored: rvt-rs (?P<n>\d+), Revit (?P<n2>\d+)$",
        "quantities with their own unit, not scored: rvt-rs 0, Revit 3",
        True),
    category(
        "qto-repeated-key", "qtos", "Quantities on a key several objects of Revit's export hold",
        "A wall Revit's export splits at its storeys keeps its ElementId as each piece's Tag, and two spaces can share a Name, which is how spaces are matched. "
        "rvt-rs's one value for the whole element is not a piece's, so Revit's values on such a key are scored as neither equal, different nor missing (the figure is Revit's values).",
        "#1576",
        ["counted apart (#1576): "],
        r"^values on a key that more than one object of Revit's export holds, counted apart \(#1576\): (?P<n>\d+) of Revit's on \d+ such keys",
        "values on a key that more than one object of Revit's export holds, counted apart (#1576): 10 of Revit's on 9 such keys, and 0 of rvt-rs's with none in the object held",
        True),
    category(
        "pset-repeated-key", "psets", "Property values on a key several objects hold, which differ",
        "The same keys as above: where the objects of Revit's export hold different values, none stands for the key, so those values are counted apart. "
        "Where they agree the value is scored against the object held.",
        "#1576",
        ["whose objects hold different values, counted apart (#1576): ", ("exact", " counted apart")],
        r"^values on a key that more than one object of Revit's export holds, whose objects hold different values, counted apart \(#1576\): (?P<n>\d+) on \d+ such keys$",
        "values on a key that more than one object of Revit's export holds, whose objects hold different values, counted apart (#1576): 12 on 6 such keys",
        True),
    category(
        "pset-sibling-openings", "psets", "Openings of a pair of exports that disagree",
        "An unfilled opening rvt-rs writes that Revit's export of this file lacks and its export of a sibling file (the same building in another state) holds is a pair inconsistency, "
        "counted apart and never scored as a match.",
        "the maintainer, 2026-11-03 and 2026-11-04; RE-321 §4, RE-338",
        ["counted apart (a pair inconsistency, RE-338): "],
        r"^rvt-rs values on openings Revit's export of this file lacks but its export of a sibling file holds, counted apart \(a pair inconsistency, RE-338\): (?P<n>\d+) on \d+ openings$",
        "rvt-rs values on openings Revit's export of this file lacks but its export of a sibling file holds, counted apart (a pair inconsistency, RE-338): 6 on 3 openings",
        True),
    category(
        "pset-known-wrong", "psets", "Property values Revit writes that are known to be wrong",
        "The stair `WalkingLineOffset` values and the others listed in RE-235 §5: Revit's export writes them and rvt-rs leaves them out.",
        "RE-235 §5",
        ["Revit values known to be wrong, left out (RE-235 §5): "],
        r"^Revit values known to be wrong, left out \(RE-235 §5\): \S+ (?P<n>\d+)$",
        "Revit values known to be wrong, left out (RE-235 §5): Pset_StairFlightCommon.WalkingLineOffset 49",
        True),
    category(
        "pset-cache-defects", "psets", "Curtain members' IsExternal in an export made from the exporter's own cache",
        "In the exports of an exporter that caches element types (Revit 24.0's, which made the BIM4LCA office's exports, 24.0.5.432), a curtain mullion's or panel's `IsExternal`, and their type's, "
        "is the cache's value, not the model's; the scorer takes an export to be one when a curtain member's type object holds the property itself. "
        "rvt-rs writes the members' by their curtain wall's value, as later exporters do, and none on the types.",
        "the maintainer, 2026-10-30; RE-269",
        ["Revit values known to be wrong, left out (RE-235 §5, RE-269): "],
        r"^Revit values known to be wrong, left out \(RE-235 §5, RE-269\): \S+ on .+ (?P<n>\d+)$",
        "Revit values known to be wrong, left out (RE-235 §5, RE-269): Pset_MemberCommon.IsExternal on curtain members 10003",
        True),
    category(
        "pset-property-set-file", "psets", "Values the export's own property set file places",
        "Where the export's user-defined property set file maps a value into a set (glazing on walls, occupancy on rooms, the `FI_` sets), the model does not store the file, so the value is out of scope.",
        "the maintainer, 2026-10-26; RE-239",
        ["out of scope (RE-239), not scored: "],
        r"^Revit values the project's own property set file places, out of scope \(RE-239\), not scored: (?P<n>\d+)$",
        "Revit values the project's own property set file places, out of scope (RE-239), not scored: 2085",
        True),
    category(
        "pset-openings-unwritten", "psets", "Property values on openings rvt-rs does not write",
        "The same openings as for quantities: those a door or a wall cuts in a further host, and a curtain wall's or a cabinet's cut.",
        "RE-338",
        ["a curtain wall's or cabinet's cut), not scored: "],
        r"^Revit values on openings rvt-rs does not write \(a door or wall cut in a further host, a curtain wall's or cabinet's cut\), not scored: (?P<n>\d+)$",
        "Revit values on openings rvt-rs does not write (a door or wall cut in a further host, a curtain wall's or cabinet's cut), not scored: 266",
        True),
    category(
        "pset-no-standard-sets", "psets", "Revit's export written with no standard property set",
        "Core Interior's export was made with no property sets, so the values rvt-rs writes beside it cannot be scored against it.",
        "the maintainer, 2026-10-29; #1115, RE-239 §5",
        ["rvt-rs values not scored: "],
        r"^Revit's export writes no standard property set \(its setup, RE-239 §5\); rvt-rs values not scored: (?P<n>\d+)$",
        "Revit's export writes no standard property set (its setup, RE-239 §5); rvt-rs values not scored: 4241",
        True),
    category(
        "pset-beyond-ifc2x3", "psets", "IFC4 properties an IFC2X3 export cannot write",
        "IFC2X3's definitions of a set lack an IFC4 property rvt-rs writes, so Revit's IFC2X3 export cannot hold it.",
        "the maintainer, 2026-10-29; #1118",
        ["which Revit's IFC2X3 export cannot write, not scored: "],
        r"^IFC4 properties IFC2X3's definitions do not have, which Revit's IFC2X3 export cannot write, not scored: (?P<n>\d+)$",
        "IFC4 properties IFC2X3's definitions do not have, which Revit's IFC2X3 export cannot write, not scored: 51",
        True),
    category(
        "ifc2x3-lengths-as-quantities", "psets", "IFC2X3 lengths that IFC4 holds as quantities",
        "Revit's IFC2X3 export writes a pipe's or duct's length in a property set; the quantity census scores it as the IFC4 base quantity, so the property census leaves it out.",
        "the census's IFC2X3 rules (tools/re/psets_vs_ifc.py)",
        ["lengths IFC4 holds as quantities"],
        r"^IFC2X3 lengths IFC4 holds as quantities, not scored here: (?P<n>\d+)$",
        "IFC2X3 lengths IFC4 holds as quantities, not scored here: 94",
        True),
    category(
        "ifc2x3-values-twice", "psets", "IFC2X3 values scored in an IFC4 set already",
        "An IFC2X3 value that also sits in an IFC4 set is scored there, not twice.",
        "the census's IFC2X3 rules (tools/re/psets_vs_ifc.py)",
        ["values also in an IFC4 set and scored there"],
        r"^IFC2X3 values also in an IFC4 set and scored there, not scored here: (?P<n>\d+)$",
        "IFC2X3 values also in an IFC4 set and scored there, not scored here: 12",
        True),
    category(
        "ifc2x3-own-set", "psets", "IFC2X3 values no IFC4 set holds",
        "A value Revit's IFC2X3 export writes that no IFC4 set of its export holds, which rvt-rs writes in its own set, cannot be matched.",
        "the census's IFC2X3 rules (tools/re/psets_vs_ifc.py)",
        ["in rvt-rs's own set, not scored here: "],
        r"^IFC2X3 values no IFC4 set Revit's export writes holds, in rvt-rs's own set, not scored here: (?P<n>\d+)$",
        "IFC2X3 values no IFC4 set Revit's export writes holds, in rvt-rs's own set, not scored here: 7",
        True),
]

# The models of a Measure's results, in the groups the page's columns name. The Autodesk samples have no
# Revit export to score.
GROUPS = [
    ("BIM4LCA office, as built", re.compile(r"^nordic-office-as-built$")),
    ("BIM4LCA office, at permit", re.compile(r"^nordic-office-permit$")),
    ("office terrain pair", re.compile(r"^nordic-office-terrain-")),
    ("BIM4LCA office, timber", re.compile(r"^nordic-office-timber-")),
    ("BIM4LCA housing", re.compile(r"^nordic-housing-")),
    ("Core Interior", re.compile(r"^core-interior$")),
    ("Einhoven", re.compile(r"^einhoven$")),
    ("RE1", re.compile(r"^re1-")),
    ("Revit 2025 models", re.compile(r"^r2025-")),
    ("IFC2X3 files (counts only)", re.compile(r"^(racadv|bimcollab)-")),
    ("held-out models (counts only)", re.compile(r"^holdout-")),
]
HEADER = {
    "qtos": re.compile(r"^Revit quantities on objects rvt-rs writes: "),
    "psets": re.compile(r"^Revit values on objects rvt-rs writes: "),
}
NOT_KEPT = "not kept"
# The first line of a held-out count file whose scrub keeps every category's line; the categories whose lines the
# scrub kept before it said so are not in NEW_IN_SCRUB, so an older file still shows those.
KEPT_LINE = "counted apart: lines kept 1"
NEW_IN_SCRUB = {
    "qto-setup-off", "qto-known-wrong", "qto-openings-unwritten", "qto-own-unit", "pset-sibling-openings",
    "pset-known-wrong", "pset-cache-defects", "pset-property-set-file", "pset-openings-unwritten", "pset-no-standard-sets",
}
NO_SCORE = "-"


def literals(source):
    """The string literals of a Python source that are not docstrings, as (line, text)."""
    tree = ast.parse(source)
    docstrings = set()
    for node in ast.walk(tree):
        if isinstance(node, (ast.Module, ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)) and node.body:
            first = node.body[0]
            if isinstance(first, ast.Expr) and isinstance(first.value, ast.Constant) and isinstance(first.value.value, str):
                docstrings.add(id(first.value))
    return [
        (node.lineno, node.value)
        for node in ast.walk(tree)
        if isinstance(node, ast.Constant) and isinstance(node.value, str) and id(node) not in docstrings
    ]


def marker_matches(marker, literal):
    if isinstance(marker, tuple):
        return literal == marker[1]
    return marker in literal


def check_scorers(registry, sources):
    """Problems between the registry and the scorers' sources (name -> text): a literal that sets a value aside and
    belongs to no category, and a marker no literal of its scorer holds. Pure."""
    problems = []
    for name, source in sorted(sources.items()):
        found = literals(source)
        mine = [c for c in registry if c["scorer"] == name]
        for line, text in found:
            if KEYWORDS.search(text) and text not in IGNORED_LITERALS:
                if not any(marker_matches(m, text) for c in mine for m in c["markers"]):
                    problems.append(f"{name}: line {line} sets a value aside ({text.strip()[:80]!r}) and no category names it")
        for c in mine:
            for m in c["markers"]:
                if not any(marker_matches(m, text) for _, text in found):
                    problems.append(f"{c['id']}: the marker {m!r} is in no string of {name}")
    return problems


def check_registry(registry):
    """Problems inside the registry: two categories with one id, an example line its own pattern does not read. Pure."""
    problems = []
    seen = set()
    for c in registry:
        if c["id"] in seen:
            problems.append(f"{c['id']}: two categories have this id")
        seen.add(c["id"])
        if not c["pattern"].search(c["example"]):
            problems.append(f"{c['id']}: its pattern does not read its example line")
    return problems


def check_scrub(registry, scrub_text):
    """Problems between the registry and the holdout scrub: a category that says the held-out models' count files keep
    its line, when the scrub's regular expressions hold none of its markers. Pure."""
    plain = scrub_text.replace("\\", "")
    return [
        f"{c['id']}: the holdout scrub keeps no line carrying {c['markers'][0]!r}"
        for c in registry
        if c["holdout"] and not any(m in plain for m in c["markers"] if isinstance(m, str))
    ]


def render_static(registry):
    rows = ["| Category | What is counted apart, and why | Decided | Scorer |", "|---|---|---|---|"]
    for c in registry:
        what = c["what"].replace("|", "\\|")
        rows.append(f"| `{c['id']}`: {c['title']} | {what} | {c['decided']} | `{c['scorer']}_vs_ifc` |")
    return "\n".join(rows)


def parse_lines(lines, c):
    """The summed counts (groups named n...) of the lines that `c`'s pattern reads, or None where none does."""
    total, matched = 0, False
    for line in lines:
        found = c["pattern"].search(line)
        if found:
            matched = True
            total += sum(int(v) for k, v in found.groupdict().items() if k.startswith("n") and v is not None)
    return total if matched else None


def model_lines(path):
    """(scorer -> (lines, ran), scrubbed, current) for a model's results directory: the scorers' own files, or the
    counts a held-out model keeps (the psets lines prefixed `psets: `); `current` says its scrub kept every category."""
    out, scrubbed, current = {}, False, True
    full = {"qtos": path / "qtos_vs_ifc.txt", "psets": path / "psets_vs_ifc.txt"}
    if all(not p.exists() for p in full.values()) and (path / "counts.txt").exists():
        scrubbed = True
        text = (path / "counts.txt").read_text(errors="replace").splitlines()
        current = KEPT_LINE in text
        psets = [line[len("psets: "):] for line in text if line.startswith("psets: ")]
        qtos = [line for line in text if not line.startswith("psets: ")]
        out = {"qtos": qtos, "psets": psets}
    else:
        for name, p in full.items():
            out[name] = p.read_text(errors="replace").splitlines() if p.exists() else []
    return {n: (lines, any(HEADER[n].search(line) for line in lines)) for n, lines in out.items()}, scrubbed, current


def collect(results, registry=CATEGORIES):
    """{group: {category id: cell}} and the lines no category reads, for a Measure's results directory."""
    cells = {}
    unread = collections.Counter()
    root = pathlib.Path(results)
    for group, pattern in GROUPS:
        models = sorted(p for p in root.iterdir() if p.is_dir() and pattern.search(p.name))
        if not models:
            continue
        row = {}
        for c in registry:
            total, seen = 0, False
            kept = True
            for model in models:
                per, scrubbed, current = model_lines(model)
                lines, ran = per[c["scorer"]]
                if not ran:
                    continue
                if scrubbed and not (c["holdout"] and (current or c["id"] not in NEW_IN_SCRUB)):
                    kept = False
                    continue
                seen = True
                total += parse_lines(lines, c) or 0
            row[c["id"]] = total if seen else (NOT_KEPT if not kept else NO_SCORE)
        cells[group] = row
        for model in models:
            per, scrubbed, _ = model_lines(model)
            if scrubbed:
                # A held-out count file keeps only the lines its scrub names; the scorers' sources are checked by --check.
                continue
            for scorer, (lines, _) in per.items():
                for line in lines:
                    if line.startswith(" ") or not KEYWORDS.search(line):
                        continue
                    if any(d.search(line) for d in DIAGNOSTIC_LINES):
                        continue
                    if not any(c["scorer"] == scorer and c["pattern"].search(line) for c in registry):
                        unread[re.sub(r"\d+", "N", line)[:140]] += 1
    return cells, unread


def render_counts(cells, registry=CATEGORIES):
    groups = [g for g, _ in GROUPS if g in cells]
    head = "| Category | " + " | ".join(groups) + " |"
    rows = [head, "|---|" + "---:|" * len(groups)]
    for c in registry:
        shown = []
        for g in groups:
            value = cells[g][c["id"]]
            shown.append(f"{value:,}" if isinstance(value, int) else value)
        rows.append(f"| `{c['id']}` | " + " | ".join(shown) + " |")
    return "\n".join(rows)


def block(text, markers):
    start, end = markers
    i, j = text.find(start), text.find(end)
    return (i, j) if 0 <= i < j else None


def page_problems(text, registry):
    """Problems of docs/counted-apart.md: its generated category block is not the registry's, or its counts table
    names other categories than the registry's. Pure."""
    problems = []
    found = block(text, STATIC)
    if not found:
        problems.append("docs/counted-apart.md has no category block")
    elif text[found[0] + len(STATIC[0]) : found[1]].strip() != render_static(registry):
        problems.append("docs/counted-apart.md's category block is not the registry's: run tools/re/counted_apart.py --write")
    found = block(text, COUNTS)
    if not found:
        problems.append("docs/counted-apart.md has no counts block")
    else:
        names = re.findall(r"^\| `([a-z0-9-]+)` \|", text[found[0] : found[1]], re.M)
        if names != [c["id"] for c in registry]:
            problems.append("docs/counted-apart.md's counts table does not list the registry's categories in order: run tools/re/counted_apart.py --write")
    return problems


def write_page(cells, run, commit, registry=CATEGORIES):
    text = PAGE.read_text()
    counts = (
        f"Counts from Measure run [{run}](https://github.com/DrunkOnJava/rvt-rs/actions/runs/{run}) on `{commit}`, "
        "the quantity and property census of each model summed over its group (`-`: the scorer did not run on that group; "
        f"`{NOT_KEPT}`: the held-out count files keep no line of that category).\n\n" + render_counts(cells, registry)
    )
    for markers, body in ((STATIC, render_static(registry)), (COUNTS, counts)):
        found = block(text, markers)
        if not found:
            sys.exit(f"docs/counted-apart.md has no {markers[0]} block")
        text = text[: found[0] + len(markers[0])] + "\n" + body + "\n" + text[found[1] :]
    PAGE.write_text(text)


def check():
    problems = check_registry(CATEGORIES)
    problems += check_scorers(CATEGORIES, {n: p.read_text() for n, p in SCORERS.items()})
    problems += check_scrub(CATEGORIES, SCRUB.read_text())
    problems += page_problems(PAGE.read_text(), CATEGORIES) if PAGE.exists() else ["docs/counted-apart.md is missing"]
    if "counted-apart.md" not in CRITERIA.read_text():
        problems.append("docs/release-1.0-criteria.md does not link counted-apart.md")
    for problem in problems:
        print(f"::error::{problem}")
    if not problems:
        print(f"Counted apart: {len(CATEGORIES)} categories, each in its scorer, the holdout scrub and docs/counted-apart.md.")
    return 1 if problems else 0


def main(argv):
    args = argv[1:]
    if args == ["--check"]:
        return check()
    write = "--write" in args
    options = {}
    for flag in ("--run", "--commit"):
        if flag in args:
            at = args.index(flag)
            options[flag] = args[at + 1]
            del args[at : at + 2]
    args = [a for a in args if a != "--write"]
    if len(args) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    cells, unread = collect(args[0])
    if write:
        if "--run" not in options or "--commit" not in options:
            print("--write needs --run and --commit", file=sys.stderr)
            return 2
        write_page(cells, options["--run"], options["--commit"])
    else:
        print(render_counts(cells))
    if unread:
        print("\nScorer lines that set a value aside and no category reads (add a category, tools/re/counted_apart.py):")
        for line, n in unread.most_common(10):
            print(f"  {n} x {line}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
