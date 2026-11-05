"""Fail when a number in the README or the supported profile does not name its model.

W10 of docs/release-1.0-criteria.md: every measured number on those two pages
names the model it was measured on, and says when nobody else can fetch that
model, so that a reader can tell which figures they can re-run. The models and
the names the pages use for them are the "Model index" table of
docs/reference-models.md, with one of four fetchability words each:

  redistributable     licensed for redistribution; tools/fetch-reference-models.sh
  fetched by Measure  a public download a Measure run fetches by its pinned hash
  local only          no documented command fetches it with Revit's export of it
  not shared          files their owner has not shared; the numbers are a report

The rule is closed, not a list of what counts as a measurement. Every number
on the two pages needs a model, and what is exempt is what a reader cannot
mistake for a measurement, by its spelling alone:

  - code spans (but not one that is only a number, `4,321 walls`), fenced
    blocks, link targets, URLs and HTML comments
  - an issue or pull request (#1192), a report (RE-345, §8), a hash, a date
    (2026-11-05), a version with two dots or a v (v0.15.0), a Revit release or
    any year (2024), a number of 8 digits or more with no comma (a Measure run
    id), and a layout width (32-bit, 12-byte)
  - the version of a named tool (Rust 1.87, Apache License 2.0, glTF 2.0) and
    the name of the release (the 1.0 range)
  - 2D and 3D, an ordinal (2nd), and a list's own numbering

A count of 1,000 or more is written with its comma, as the pages do, so that a
year is never mistaken for a count and a count for a year.

Anything else with a digit in it is a number: a count, a ratio, a percentage, a
tolerance, a default, a version of the project. A number that is not a
measurement of a model (a default limit, a count another check pins, the 1.0
release) carries `<!-- w10: why -->` in its clause, and the reason is
required. A marker with nothing to excuse fails too, so markers do not outlive
their numbers. Numbers spelled out in words are not read.

A clause is the text between semicolons, between sentences, and between table
cells and list items. A number names its model when its own clause does: a
continuation after a semicolon says which model again. When the model is
`local only` or `not shared`, the clause says so, or an earlier clause of the
same cell, paragraph or list item did for that model.

    python3 tools/ci/measured_numbers.py            check the repository's pages
    python3 tools/ci/measured_numbers.py --root D   check the pages under D
    python3 tools/ci/measured_numbers.py --refusals check that it fails on each
                                                    kind of number it must refuse
"""

import pathlib
import re
import shutil
import sys
import tempfile

PAGES = ("README.md", "docs/supported-profile.md")
INDEX = "docs/reference-models.md"
HEADING = "## Model index"
COLUMNS = ("Model", "Names the pages use", "Who can fetch it", "How")
CLASSES = ("redistributable", "fetched by Measure", "local only", "not shared")
SAYS = {
    "local only": re.compile(r"\blocal[ -]only\b"),
    "not shared": re.compile(r"\bnot shared\b"),
}

MARKER = re.compile(r"<!--\s*w10:(.*?)-->", re.S)
COMMENT = re.compile(r"<!--.*?-->", re.S)
PLACEHOLDER = re.compile("\x01(\\d+)\x01")
SENTENCE_END = r"(?<!e\.g)(?<!i\.e)(?<!etc)(?<!vs)(?<=[A-Za-z0-9)`\"'%\]])\.\s+(?=[A-Z0-9(\[`\"*])"
CLAUSE_END = re.compile(rf";\s+|{SENTENCE_END}")
LIST_ITEM = re.compile(r"^(?:[-*]|\d+\.)\s+")

# What is not a measurement by its spelling. Order matters: a citation is cut
# out before a number is looked for.
IDENTIFIERS = [
    re.compile(r"`(?!\d[\d,.]*(?:[ ]?[A-Za-z%]+|[ ]\d[\d,.]*)*`)[^`]*`"),
    re.compile(r"\]\([^)]*\)"),
    re.compile(r"<https?://[^>]*>|https?://\S+"),
    re.compile(r"\bRE-\d+(?:[.-]\d+)*"),
    re.compile(r"#\d+"),
    re.compile(r"§\s?[\w.]+"),
    re.compile(r"\b\d{4}-\d{2}-\d{2}\b"),
    re.compile(r"\bv?\d+\.\d+\.\d+\b|\bv\d+(?:\.\d+)?\b"),
    re.compile(r"\b(?=[0-9a-f]*\d)(?=[0-9a-f]*[a-f])[0-9a-f]{7,40}\b"),
    re.compile(r"\b20[0-2]\d\b"),
    re.compile(r"\b\d{8,}\b"),
    re.compile(r"\b\d+-(?:bit|byte)s?\b"),
    re.compile(r"\b(?:Rust|MSRV|License|glTF|Python|Node)\s+\d+(?:\.\d+)*\b"),
    re.compile(r"\bRevit\s+\d+(?:\.\d+)+\b"),
    re.compile(r"\b1\.0\s+(?:range|release|criteria)\b"),
    re.compile(r"\b[23]D\b|\b\d+(?:st|nd|rd|th)\b"),
]
NUMBER = re.compile(
    r"(?<![\w.#§/-])(?:\d+(?:\.\d+)?e-?\d+|(?:\d{1,3}(?:,\d{3})+|\d+)(?:\.\d+)?)(?!\d)"
)
MIN_REASON = 12


def units(text):
    """(line, text) of each table cell, paragraph and list item of a Markdown
    page. Fenced blocks, headings and comment-only lines are not text."""
    out, para, start = [], [], 0
    fenced = in_comment = False

    def flush():
        nonlocal para
        if para:
            out.append((start, " ".join(para)))
        para = []

    for number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if in_comment:
            in_comment = "-->" not in stripped
            continue
        if stripped.startswith(("```", "~~~")):
            fenced = not fenced
            flush()
            continue
        if fenced:
            continue
        if stripped.startswith("<!--") and "-->" not in stripped:
            in_comment = True
            flush()
            continue
        if not COMMENT.sub("", stripped).strip() and stripped.startswith("<!--"):
            flush()
            continue
        if re.match(r"#{1,6}\s", stripped):
            flush()
            continue
        if stripped.startswith("|"):
            flush()
            cells = [c.strip() for c in stripped.strip("|").split("|")]
            if not re.fullmatch(r"[\s:|-]*", stripped):
                out.extend((number, cell) for cell in cells)
            continue
        if not stripped:
            flush()
            continue
        if LIST_ITEM.match(stripped):
            flush()
            stripped = LIST_ITEM.sub("", stripped)
        if not para:
            start = number
        para.append(stripped)
    flush()
    return out


def clauses(unit):
    """The clauses of one unit, each as (text, reasons of the markers in it).
    A marker's reason may hold a semicolon or a full stop."""
    reasons = []

    def hold(match):
        marker = MARKER.fullmatch(match.group(0))
        if not marker:
            return " "
        reasons.append(marker.group(1).strip())
        return f"\x01{len(reasons) - 1}\x01"

    held = COMMENT.sub(hold, unit)
    out = []
    for part in CLAUSE_END.split(held):
        found = [reasons[int(i)] for i in PLACEHOLDER.findall(part)]
        part = PLACEHOLDER.sub("", part).strip()
        if part or found:
            out.append((part, found))
    return out


def numbers(clause):
    """The numbers of a clause: its digits once what spells something else is
    cut out."""
    for pattern in IDENTIFIERS:
        clause = pattern.sub(" ", clause)
    return NUMBER.findall(clause)


def named(clause, models):
    """The models whose names the clause uses, by model index."""
    clause = re.sub(r"\]\([^)]*\)|<https?://[^>]*>|https?://\S+", " ", clause)
    found = [
        (m.start(), m.end(), i)
        for i, model in enumerate(models)
        for name in model["names"]
        for m in re.finditer(rf"(?<![\w-]){re.escape(name)}(?!\w)", clause)
    ]
    # The longest name wins where one name sits inside another ("Snowdon
    # Towers samples" is not "Snowdon").
    return {
        i
        for start, end, i in found
        if not any(s <= start and end <= e and e - s > end - start for s, e, _ in found)
    }


def read_index(text):
    """(models, problems) of the "Model index" table of docs/reference-models.md."""
    problems = []
    if HEADING not in text:
        return [], [f'{INDEX} has no "{HEADING}" section']
    rows = []
    for line in text.split(HEADING, 1)[1].splitlines()[1:]:
        if line.startswith("#"):
            break
        if line.startswith("|"):
            rows.append([c.strip() for c in line.strip("|").split("|")])
    if not rows or tuple(rows[0]) != COLUMNS:
        return [], [f"the model index's columns are not {' | '.join(COLUMNS)}"]
    models, owner = [], {}
    for row in rows[2:]:
        label = row[0] if row else "(empty row)"
        if len(row) != len(COLUMNS):
            problems.append(f"model index row {label!r} has {len(row)} cells, not {len(COLUMNS)}")
            continue
        model, names, fetch, how = row
        names = [n.strip().strip("`") for n in names.split(",") if n.strip().strip("`")]
        if not names:
            problems.append(f"model index row {model!r} names no name the pages use")
        if fetch not in CLASSES:
            problems.append(
                f"model index row {model!r} says {fetch!r} for who can fetch it; "
                f"the words are {', '.join(CLASSES)}"
            )
        if not how:
            problems.append(f"model index row {model!r} does not say how it is fetched or why it cannot be")
        for name in names:
            if name in owner:
                problems.append(f"model index: the name {name!r} is used by {owner[name]!r} and {model!r}")
            owner[name] = model
        models.append({"model": model, "names": names, "fetch": fetch})
    if not models:
        problems.append("the model index has no rows")
    return models, problems


def check_page(page, text, models):
    """(line, message) for each number of the page that breaks the rule, and
    the counts (numbers read, clauses with a model, markers)."""
    found = []
    seen = {"numbers": 0, "named": 0, "markers": 0}
    for line, unit in units(text):
        flagged = set()
        for clause, reasons in clauses(unit):
            digits = numbers(clause)
            who = named(clause, models)
            for i in who:
                if models[i]["fetch"] in SAYS and SAYS[models[i]["fetch"]].search(clause):
                    flagged.add(i)
            shown = clause if len(clause) <= 110 else clause[:107] + "..."
            for reason in reasons:
                seen["markers"] += 1
                if len(reason) < MIN_REASON or not re.search(r"[A-Za-z]{3}", reason):
                    found.append((line, f"w10 marker without a reason: {reason!r} in: {shown}"))
            if reasons and (not digits or who):
                found.append(
                    (line, "w10 marker with nothing to excuse (" + ("no number" if not digits else "the clause names its model") + f") in: {shown}")
                )
                continue
            if not digits:
                continue
            seen["numbers"] += len(digits)
            if reasons:
                continue
            if not who:
                found.append((line, f"{', '.join(digits)} names no model of the model index in: {shown}"))
                continue
            seen["named"] += len(digits)
            for i in sorted(who):
                fetch = models[i]["fetch"]
                if fetch in SAYS and i not in flagged:
                    found.append(
                        (line, f"{', '.join(digits)} rests on {models[i]['names'][0]}, which is {fetch}, and the clause does not say \"{fetch}\" in: {shown}")
                    )
    return [(page, line, message) for line, message in found], seen


def check(root):
    """(problems, counts) of the two pages under root."""
    root = pathlib.Path(root)
    problems = []
    try:
        models, index_problems = read_index((root / INDEX).read_text(encoding="utf-8"))
    except OSError as error:
        return [(INDEX, 1, f"cannot read the model index: {error}")], {}
    problems += [(INDEX, 1, message) for message in index_problems]
    totals = {"numbers": 0, "named": 0, "markers": 0, "models": len(models)}
    if index_problems:
        return problems, totals
    for page in PAGES:
        try:
            text = (root / page).read_text(encoding="utf-8")
        except OSError as error:
            problems.append((page, 1, f"cannot read the page: {error}"))
            continue
        found, seen = check_page(page, text, models)
        problems += found
        for key, value in seen.items():
            totals[key] += value
    return problems, totals


def report(problems, totals):
    for page, line, message in problems:
        print(f"::error file={page},line={line}::{message}")
    if problems:
        print(f"{len(problems)} problems; see docs/maintaining.md, \"Numbers on the README and the supported profile\"")
    else:
        print(
            f"Measured numbers: {totals['named']} of {totals['numbers']} numbers name a model of the index "
            f"({totals['models']} models), {totals['markers']} are marked as no measurement"
        )
    return 1 if problems else 0


# Each refusal: a mutation of the real pages or index (text appended to a
# page, or the first occurrence of one string replaced by another), and the
# text the check must name.
REFUSALS = [
    (
        "a number with no model",
        "docs/supported-profile.md",
        "\nOn the model, 4,321 of 4,322 walls were exact.\n",
        "names no model",
    ),
    (
        "a number on a local-only model that does not say so",
        "docs/supported-profile.md",
        "\nOn Snowdon Towers, 4,321 of 4,322 walls were exact.\n",
        "does not say",
    ),
    (
        "a marker with no reason",
        "README.md",
        "\nIt reads 4,321 files <!-- w10: x -->.\n",
        "without a reason",
    ),
    (
        "a marker with nothing to excuse",
        "README.md",
        "\nIt reads files <!-- w10: nothing numeric is excused here -->.\n",
        "nothing to excuse",
    ),
    (
        "a model whose row does not say who can fetch it",
        INDEX,
        ("| fetched by Measure |", "|  |"),
        "who can fetch it",
    ),
]


def refusals(root):
    """Run the check on a scratch copy of the pages with each mutation
    appended, and fail unless the check refuses it for the stated reason."""
    root = pathlib.Path(root)
    problems, _ = check(root)
    if problems:
        print("the pages themselves fail the check; its refusals cannot be shown on them")
        return report(problems, {})
    failed = 0
    for what, page, addition, reason in REFUSALS:
        with tempfile.TemporaryDirectory() as scratch:
            scratch = pathlib.Path(scratch)
            for name in (*PAGES, INDEX):
                (scratch / name).parent.mkdir(parents=True, exist_ok=True)
                shutil.copy(root / name, scratch / name)
            if isinstance(addition, tuple):
                text = (scratch / page).read_text(encoding="utf-8")
                (scratch / page).write_text(text.replace(*addition, 1), encoding="utf-8")
            else:
                with open(scratch / page, "a", encoding="utf-8") as out:
                    out.write(addition)
            found, _ = check(scratch)
        if any(reason in message for _, _, message in found):
            print(f"refused as it must be: {what}")
        else:
            print(f"::error::the check did not refuse {what}: it found {[m for _, _, m in found]}")
            failed = 1
    return failed


def main(argv):
    root = pathlib.Path(__file__).resolve().parents[2]
    if "--root" in argv:
        root = pathlib.Path(argv[argv.index("--root") + 1])
    if "--refusals" in argv:
        return refusals(root)
    return report(*check(root))


if __name__ == "__main__":
    sys.exit(main(sys.argv))
