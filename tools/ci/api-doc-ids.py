"""Fail when the API docs cite an RE finding that has no report, or carry a
maintainer's planning label.

The API docs are the rustdoc of the crate and of the Python bindings (pyo3
turns `///` into the Python docstring), the type stubs IDEs show, and the
repository README and the user pages `docs/README.md` lists under "For
users" (the Python guide among them). The research record keeps its ids as
written.

The crate root tells docs.rs readers that every `RE-NN` is written up in
`reports/**/RE-NN-*.md`, and that `#NNN` is an issue or pull request; those
are the only tracker ids the docs use. Backlog and task ids (`B55`,
`SEC-12`, `VW1-13`, audit items such as "P0 item 4") and work-plan labels
("Lane C", "Phase 4d", "spike 257", "the attack plan") mean nothing
outside the project: one that is a citation belongs in a `// Tracker:`
comment beside the item. Code spans and quoted strings are
skipped, so a Revit phase named "Phase 1" stays.
"""

import pathlib
import re
import sys

root = pathlib.Path(__file__).resolve().parents[2]
reports = sorted(p.name for p in (root / "reports").rglob("RE-*.md"))
citation = re.compile(r"\bRE-\d+(?:[.-]\d+)*")


def has_report(cited):
    return any(name.startswith((f"{cited}-", f"{cited}.")) for name in reports)


planning = re.compile(
    r"\b(?:Lane (?:[A-Z0-9]|One|Two|Three|Four|Five|Six|Seven|Eight|Nine|Ten|Eleven|Twelve)"
    r"|Phase (?:[A-D]|\d+[a-z]?(?:\.\d+)?)"
    r"|[Aa]ttack[- ]plan|[Ss]pike \d+|VW\d+(?:-\d+)?|[Aa]udit P[0-3]|P[0-3] items?|Finding \d+|PARSE-\d+"
    r"|Q-\d+|COR-\d+|TEST-\d+|DOC-\d+|WF-\d+"
    # A bare research-question id; a section of a report is cited as §Q6.5.
    r"|(?<!§)Q\d+(?:\.\d+)?"
    r"|B\d{2,3}|SEC-\d+|VW\d-\d+|L5B-\d+|WRT-\d+|IFC-\d+|GEO-\d+|ELEM-\d+|API-\d+|VAL-\d+|M\d-\d+)\b"
)
missing = {}
labels = []
paths = sorted((root / "src").rglob("*.rs")) + sorted((root / "rvt-py/src").rglob("*.rs"))
paths += sorted((root / "python/rvt").glob("*.pyi"))
index = (root / "docs/README.md").read_text(encoding="utf-8")
users = index.split("## For users", 1)[1].split("\n## ", 1)[0]
paths += sorted({root / "docs" / link for link in re.findall(r"\]\(([^)#]+\.md)\)", users)}) + [root / "README.md"]
for path in paths:
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.lstrip().replace('"""', "")
        if path.suffix == ".rs" and not stripped.startswith(("///", "//!")):
            continue
        place = f"{path.relative_to(root)}:{number}"
        for cited in citation.findall(stripped):
            if not has_report(cited):
                missing.setdefault(cited, []).append(place)
        # Code spans are code. In Rust and the stubs a quoted string is data
        # (a Revit phase named "Phase 1"); on a markdown page it is prose.
        quoted = r'`[^`]*`|"[^"]*"' if path.suffix in (".rs", ".pyi") else r"`[^`]*`"
        prose = re.sub(quoted, "", stripped)
        labels += [(place, label) for label in planning.findall(prose)]

for cited, places in sorted(missing.items()):
    print(f"::error::{cited} is cited in the API docs but no report is named {cited}-*.md: {', '.join(places[:5])}")
for place, label in labels:
    print(f"::error file={place.split(':')[0]},line={place.split(':')[1]}::the API docs carry the planning label {label!r}; move it to a // Tracker: comment")
if missing or labels:
    sys.exit(1)
print(f"API doc ids: every cited RE finding has a report ({len(reports)} reports), and no planning label is left.")
