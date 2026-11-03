"""Render the README's support-by-release table from docs/support-matrix.json.

    python3 tools/ci/readme-matrix.py            rewrite the table in README.md
    python3 tools/ci/readme-matrix.py --check    fail when README.md's table is stale

The table sits between the lines `<!-- support-matrix:releases -->` and
`<!-- /support-matrix -->` of README.md. Each release of the matrix's
`releases` list is one row of the matrix; consecutive releases that share a
project capability are one row of the table ("2008 to 2010"). "Projects" is
the status word of that capability exactly as the matrix has it,
"What a project gives" its name, and "Families" the distinct statuses of the
family capabilities, or "not measured" for a release with none. When the
releases of one row differ in their family capabilities, the cell says which
releases have which.

The statuses and names live only in the matrix, so the README cannot claim
more than the matrix does.
"""

import json
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parents[2]
readme_path = root / "README.md"
matrix_path = root / "docs/support-matrix.json"
begin = "<!-- support-matrix:releases -->"
end = "<!-- /support-matrix -->"
header = ("Revit release", "Projects", "What a project gives", "Families")


def fail(message):
    print(f"::error::{message}")
    sys.exit(1)


def years(releases):
    first, last = releases[0], releases[-1]
    return str(first) if first == last else f"{first} to {last}"


def runs(pairs):
    """Consecutive (release, value) pairs with the same value, as (value, [releases])."""
    out = []
    for release, value in pairs:
        if out and out[-1][0] == value:
            out[-1][1].append(release)
        else:
            out.append((value, [release]))
    return out


def render(matrix):
    vocabulary = matrix["status_vocabulary"]
    capabilities = {c["id"]: c for c in matrix["capabilities"]}

    def capability(identifier, release):
        if identifier not in capabilities:
            fail(f"docs/support-matrix.json: release {release} names the capability {identifier}, which is not in the matrix")
        status = capabilities[identifier]["status"]
        if status not in vocabulary:
            fail(f"docs/support-matrix.json: {identifier} has the status {status!r}, which is not in status_vocabulary")
        return capabilities[identifier]

    def families(release):
        found = {capability(i, release["release"])["status"] for i in release["family_capabilities"]}
        return ", ".join(s for s in vocabulary if s in found) or "not measured"

    releases = sorted(matrix["releases"], key=lambda r: r["release"])
    groups = []
    for release in releases:
        if groups and groups[-1][0] == release["project_capability"]:
            groups[-1][1].append(release)
        else:
            groups.append((release["project_capability"], [release]))

    rows = [header]
    for identifier, members in groups:
        project = capability(identifier, members[0]["release"])
        cells = runs([(m["release"], families(m)) for m in members])
        if len(cells) == 1:
            family_cell = cells[0][0]
        else:
            family_cell = "; ".join(f"{cell} ({years(rels)})" for cell, rels in cells)
        rows.append((years([m["release"] for m in members]), project["status"], project["name"], family_cell))

    def line(cells):
        return "| " + " | ".join(c.replace("|", "\\|") for c in cells) + " |"

    table = [line(rows[0]), "|---|---|---|---|"] + [line(r) for r in rows[1:]]
    return "\n".join(table), len(rows) - 1, len(releases)


def main(argv):
    check = argv[1:] == ["--check"]
    if argv[1:] and not check:
        print(__doc__)
        return 2
    matrix = json.loads(matrix_path.read_text(encoding="utf-8"))
    table, row_count, release_count = render(matrix)
    text = readme_path.read_text(encoding="utf-8")
    if text.count(begin) != 1 or text.count(end) != 1 or text.index(begin) > text.index(end):
        fail(f"README.md must hold exactly one {begin} line followed by one {end} line")
    head, rest = text.split(begin)
    _, tail = rest.split(end)
    block = f"{begin}\n\n{table}\n\n{end}"
    updated = head + block + tail
    if check:
        if updated != text:
            fail(
                "README.md's support-by-release table differs from docs/support-matrix.json; "
                "run python3 tools/ci/readme-matrix.py and commit the result"
            )
        print(f"README support table: matches docs/support-matrix.json ({row_count} rows for {release_count} releases).")
        return 0
    if updated == text:
        print(f"README support table: already current ({row_count} rows for {release_count} releases).")
        return 0
    with open(readme_path, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(updated)
    print(f"README support table: rewritten ({row_count} rows for {release_count} releases).")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
