#!/usr/bin/env python3
"""Check or assemble the changelog fragments under changelog.d/ (#938).

A pull request writes its changelog entry as a fragment,
`changelog.d/<section>/<name>.md`, where `<section>` is one of the
directories below. Every pull request added its entry at the same place in
CHANGELOG.md, so each one that landed left the others in conflict with main.
A fragment is the entry exactly as it reads in the changelog: one or more
`- **...**` items, wrapped as the changelog wraps them.

    python3 tools/release/changelog.py check
    python3 tools/release/changelog.py assemble

`check` fails on a fragment outside a known section, an empty one, or one
whose first line does not start an item. `assemble`, a release step
(RELEASE.md), moves every fragment into CHANGELOG.md's `[Unreleased]`, at
the top of its section's heading (made if missing), in name order, one
blank line apart, and deletes the fragments.
"""

import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[2]
FRAGMENTS = ROOT / "changelog.d"
CHANGELOG = ROOT / "CHANGELOG.md"
# The sections in the order a release section lists them.
SECTIONS = {
    "added": "Added",
    "changed": "Changed",
    "fixed": "Fixed",
    "removed": "Removed",
    "security": "Security",
    "research": "Research",
}


def fragments():
    """(section heading, path, entry text) of every fragment, by section and
    name; the problems with any that is malformed."""
    found, problems = [], []
    for path in sorted(FRAGMENTS.rglob("*.md")):
        relative = path.relative_to(FRAGMENTS)
        if relative.parts == ("README.md",):
            continue
        if len(relative.parts) != 2 or relative.parts[0] not in SECTIONS:
            problems.append(f"{path.relative_to(ROOT)}: not in a section directory ({', '.join(SECTIONS)})")
            continue
        text = path.read_text(encoding="utf-8").strip("\n")
        if not text.strip():
            problems.append(f"{path.relative_to(ROOT)}: empty")
            continue
        if not text.startswith("- **"):
            problems.append(f"{path.relative_to(ROOT)}: the first line does not start an item (`- **`)")
            continue
        found.append((SECTIONS[relative.parts[0]], path, text))
    order = list(SECTIONS.values())
    found.sort(key=lambda f: (order.index(f[0]), f[1].name))
    return found, problems


def assemble(found):
    lines = CHANGELOG.read_text(encoding="utf-8").split("\n")
    try:
        start = lines.index("## [Unreleased]")
    except ValueError:
        sys.exit("CHANGELOG.md has no `## [Unreleased]` heading")
    end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## ")), len(lines))
    for heading in reversed(list(dict.fromkeys(section for section, _, _ in found))):
        entries = [text for section, _, text in found if section == heading]
        block = "\n\n".join(entries).split("\n")
        at = next((i for i in range(start, end) if lines[i] == f"### {heading}"), None)
        if at is None:
            # A new heading after the ones before it in SECTIONS' order.
            order = list(SECTIONS.values())
            later = [
                i for i in range(start, end)
                if lines[i].startswith("### ") and lines[i][4:] in order
                and order.index(lines[i][4:]) > order.index(heading)
            ]
            at = later[0] if later else end
            lines[at:at] = [f"### {heading}", "", *block, ""]
        else:
            # Directly under the heading and its blank line.
            body = at + 2
            if body < end and lines[body].strip():
                block = [*block, ""]
            lines[body:body] = block
        end = next((i for i in range(start + 1, len(lines)) if lines[i].startswith("## ")), len(lines))
    CHANGELOG.write_text("\n".join(lines), encoding="utf-8")
    for _, path, _ in found:
        path.unlink()
    print(f"assembled {len(found)} fragment(s) into CHANGELOG.md's [Unreleased]")


def main(argv):
    if len(argv) != 2 or argv[1] not in ("check", "assemble"):
        print(__doc__, file=sys.stderr)
        return 2
    found, problems = fragments()
    for problem in problems:
        print(f"::error::{problem}")
    if problems:
        return 1
    if argv[1] == "check":
        print(f"changelog.d: {len(found)} fragment(s), each in a section and starting an item")
        return 0
    assemble(found)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
