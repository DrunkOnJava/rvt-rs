import collections
import csv
import json
import sys


def area(path):
    for prefix, name in (
        ("src/bin/", "bin"),
        ("src/", "lib"),
        ("rvt-py/", "rvt-py"),
        ("rvt-wasm/", "rvt-wasm"),
        ("tools/stream_evidence/", "stream-evidence"),
        ("tests/", "tests"),
        ("examples/", "examples"),
        ("benches/", "benches"),
    ):
        if path.startswith(prefix):
            return name
    return "other"


def load(path):
    out = {}
    with open(path, encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line.startswith("{"):
                continue
            msg = json.loads(line)
            if msg.get("reason") != "compiler-message":
                continue
            m = msg["message"]
            code = (m.get("code") or {}).get("code")
            if not code or m.get("level") not in ("warning", "error"):
                continue
            spans = [s for s in m.get("spans", []) if s.get("is_primary")]
            if not spans:
                continue
            s = spans[0]
            key = (code, s["file_name"], s["line_start"], s["column_start"])
            out[key] = m["message"]
    return out


prod = load(sys.argv[1])
everything = load(sys.argv[2])
test_only = {k: v for k, v in everything.items() if k not in prod}

with open("census.csv", "w", newline="", encoding="utf-8") as fh:
    w = csv.writer(fh)
    w.writerow(["pass", "lint", "area", "file", "line", "col", "message"])
    for name, rows in (("production", prod), ("test-and-examples", test_only)):
        for (code, f, line, col), text in sorted(rows.items()):
            w.writerow([name, code, area(f), f, line, col, text])

areas = ["lib", "bin", "rvt-py", "rvt-wasm", "stream-evidence", "tests", "examples", "benches", "other"]
lines = []
for name, rows in (("production (--lib --bins)", prod), ("tests, examples, benches, cfg(test)", test_only)):
    counts = collections.Counter((k[0], area(k[1])) for k in rows)
    lints = sorted({k[0] for k in rows})
    lines.append(f"## {name}\n")
    lines.append("| lint | " + " | ".join(areas) + " | total |")
    lines.append("|---|" + "---|" * (len(areas) + 1))
    for lint in lints:
        cells = [counts.get((lint, a), 0) for a in areas]
        lines.append(f"| {lint} | " + " | ".join(str(c) for c in cells) + f" | {sum(cells)} |")
    lines.append("")
    lines.append(f"### top files per lint ({name})\n")
    for lint in lints:
        files = collections.Counter(k[1] for k in rows if k[0] == lint)
        top = ", ".join(f"{f} ({n})" for f, n in files.most_common(6))
        lines.append(f"- {lint}: {top}")
    lines.append("")

with open("summary.md", "w", encoding="utf-8") as fh:
    fh.write("\n".join(lines))
print("\n".join(lines))
