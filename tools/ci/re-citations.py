"""Fail when rustdoc cites an RE finding that has no report.

The crate's docs cite reverse-engineering findings as `RE-NN`, and the crate
root tells readers that each one is written up in
`reports/**/RE-NN-*.md`. This keeps that true: every `RE-NN` in a `///` or
`//!` comment under `src/` must have a report.
"""

import pathlib
import re
import sys

root = pathlib.Path(__file__).resolve().parents[2]
reports = {m.group(1) for p in (root / "reports").rglob("RE-*.md") if (m := re.match(r"(RE-\d+)[-.]", p.name))}
missing = {}
for path in sorted((root / "src").rglob("*.rs")):
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.lstrip()
        if not stripped.startswith(("///", "//!")):
            continue
        for cited in re.findall(r"\bRE-\d+", stripped):
            if cited not in reports:
                missing.setdefault(cited, []).append(f"{path.relative_to(root)}:{number}")

for cited, places in sorted(missing.items()):
    print(f"::error::{cited} is cited in rustdoc but reports/ has no {cited}-*.md: {', '.join(places[:5])}")
if missing:
    sys.exit(1)
print(f"RE citations: every RE id cited in rustdoc has a report ({len(reports)} reports).")
