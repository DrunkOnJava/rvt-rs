#!/usr/bin/env python3
"""Hold the sealed held-out pairs sealed (W5, #408, #1565).

research/sealed-holdout-models.tsv lists the pairs kept for the final 1.0
check. This fails when:

  - a row is malformed: not 11 tab-separated fields, a release outside 2024
    to 2027, a name that is not [A-Za-z0-9._-], a URL that is not https, a
    sha256 that is not 64 hex digits, a byte count that is not a positive
    integer, a licence other than Apache-2.0, MIT, CC-BY-4.0 or CC0-1.0, a
    contributor that is not a GitHub login, a received date that is not
    YYYY-MM-DD, or a name, URL or hash used twice;
  - any tracked file other than the registry holds one of its URLs or hashes
    (in any letter case), so no fetch list, script, test or document can
    point at a sealed file;
  - any tracked file that is not Markdown names the registry, except
    tools/ci/seal-holdout-pair.sh, which adds rows, and this check. Docs may
    name it; code may not, so no workflow or script can read it.
  - the registry itself is missing.

What it cannot see: a path built at run time, a glob over research/, or a
Measure run dispatched from a branch that has not been reviewed. It holds what
is committed, which review reads.

The final 1.0 check is the one reader still to come: when W4, W6 and W7 pass
on the reference models, its Measure input is added to ALLOWED in the same
pull request that adds it, where review sees it.

Run from the repository root. The seal script also calls it with `--rows FILE`
(judge a candidate registry's rows alone) and `--pair RVT_HEAD IFC_HEAD
RELEASE` (judge the first bytes of a pair before it is sealed).
"""
import pathlib
import re
import subprocess
import sys

REGISTRY = "research/sealed-holdout-models.tsv"
ALLOWED = frozenset({"tools/ci/seal-holdout-pair.sh", "tools/ci/check_sealed_holdout.py", "tools/ci/test_check_sealed_holdout.py"})
RELEASES = frozenset({"2024", "2025", "2026", "2027"})
LICENCES = frozenset({"Apache-2.0", "MIT", "CC-BY-4.0", "CC0-1.0"})
FIELDS = ("release", "name", "rvt_url", "rvt_sha256", "rvt_bytes", "ifc_url", "ifc_sha256", "ifc_bytes", "licence", "contributor", "received")
NAME = re.compile(r"[A-Za-z0-9._-]+")
SHA = re.compile(r"[0-9a-f]{64}")
URL = re.compile(r"https://[^\s\t]+")
LOGIN = re.compile(r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,38})")
DATE = re.compile(r"\d{4}-\d{2}-\d{2}")
OLE2 = bytes.fromhex("d0cf11e0a1b11ae1")
SCHEMA = re.compile(rb"FILE_SCHEMA\s*\(\s*\(\s*'([A-Za-z0-9_]+)'")


def rows(text):
    """The registry's data lines, with their line numbers."""
    return [(n, line) for n, line in enumerate(text.splitlines(), 1) if line.strip() and not line.startswith("#")]


def row_problems(text):
    """What is wrong with the registry's rows, and the URLs and hashes they seal."""
    problems, sealed = [], set()
    seen = {"name": set(), "url": set(), "sha": set()}
    for n, line in rows(text):
        fields = line.split("\t")
        if len(fields) != len(FIELDS):
            problems.append(f"{REGISTRY}:{n}: {len(fields)} fields, not {len(FIELDS)}")
            continue
        row = dict(zip(FIELDS, fields))
        checks = [
            (row["release"] in RELEASES, f"release {row['release']!r} is not one of 2024 to 2027"),
            (NAME.fullmatch(row["name"]), f"name {row['name']!r} is not [A-Za-z0-9._-]"),
            (row["licence"] in LICENCES, f"licence {row['licence']!r} is not Apache-2.0, MIT, CC-BY-4.0 or CC0-1.0"),
            (LOGIN.fullmatch(row["contributor"]), f"contributor {row['contributor']!r} is not a GitHub login"),
            (DATE.fullmatch(row["received"]), f"received {row['received']!r} is not YYYY-MM-DD"),
        ]
        for side in ("rvt", "ifc"):
            url, sha, size = row[f"{side}_url"], row[f"{side}_sha256"], row[f"{side}_bytes"]
            checks += [
                (URL.fullmatch(url), f"{side}_url {url!r} is not an https URL"),
                (SHA.fullmatch(sha), f"{side}_sha256 {sha!r} is not 64 lowercase hex digits"),
                (size.isdigit() and int(size) > 0, f"{side}_bytes {size!r} is not a positive integer"),
            ]
            for kind, value in (("url", url), ("sha", sha)):
                if value in seen[kind]:
                    problems.append(f"{REGISTRY}:{n}: {value} is listed twice")
                seen[kind].add(value)
                sealed.add(value.lower())
        if row["name"] in seen["name"]:
            problems.append(f"{REGISTRY}:{n}: name {row['name']} is listed twice")
        seen["name"].add(row["name"])
        problems += [f"{REGISTRY}:{n}: {message}" for ok, message in checks if not ok]
    return problems, sealed


def file_problems(path, text, sealed):
    """What a tracked file other than the registry must not hold."""
    if path == REGISTRY:
        return []
    problems = []
    lowered = text.lower()
    for value in sorted(sealed):
        if value in lowered:
            problems.append(f"{path}: holds the sealed {value}")
    if REGISTRY.rsplit("/", 1)[1] in text and path not in ALLOWED and not path.endswith(".md"):
        problems.append(f"{path}: names {REGISTRY}, which only the seal script and this check may read")
    return problems


def pair_problems(rvt_head, ifc_head, release):
    """What is wrong with a pair, from the first bytes of each file only: the
    project must be an OLE2 compound file, and the IFC an ISO 10303-21 file of
    IFC4 or IFC4X3 that names `Autodesk Revit <release>` within those bytes
    (Revit's export names itself in the header or the first entities). A share
    link that answers with an HTML page, a zip, another release's export or an
    IFC2X3 view fails here, before the pair is sealed and spent."""
    problems = []
    if not rvt_head.startswith(OLE2):
        problems.append("the .rvt is not an OLE2 compound file")
    if not ifc_head.startswith(b"ISO-10303-21;"):
        problems.append("the IFC is not an ISO 10303-21 file")
    else:
        schema = SCHEMA.search(ifc_head)
        if not schema or not schema.group(1).upper().startswith(b"IFC4"):
            problems.append("the IFC's schema is not IFC4 or IFC4X3")
        if f"Autodesk Revit {release}".encode() not in ifc_head:
            problems.append(f"the IFC does not name Autodesk Revit {release} in its first bytes")
    return problems


def judge(registry_text, files):
    """Every problem with `registry_text` (None when the file is missing) and `files` (path -> text)."""
    if registry_text is None:
        return [f"{REGISTRY} is missing"]
    problems, sealed = row_problems(registry_text)
    for path, text in sorted(files.items()):
        problems += file_problems(path, text, sealed)
    return problems


def tracked_texts():
    paths = subprocess.run(["git", "ls-files", "-z"], capture_output=True, check=True).stdout.split(b"\0")
    texts = {}
    for raw in paths:
        if not raw:
            continue
        path = raw.decode()
        file = pathlib.Path(path)
        if not file.is_file() or file.stat().st_size > 20_000_000:
            continue
        data = file.read_bytes()
        if b"\0" in data[:8192]:
            continue
        texts[path] = data.decode("utf-8", errors="replace")
    return texts


def main():
    args = sys.argv[1:]
    if args[:1] == ["--rows"] and len(args) == 2:
        problems, _ = row_problems(pathlib.Path(args[1]).read_text())
    elif args[:1] == ["--pair"] and len(args) == 4:
        rvt, ifc, release = args[1:]
        problems = pair_problems(pathlib.Path(rvt).read_bytes(), pathlib.Path(ifc).read_bytes(), release)
    elif args:
        print("usage: check_sealed_holdout.py [--rows FILE | --pair RVT_HEAD IFC_HEAD RELEASE]", file=sys.stderr)
        return 2
    else:
        problems = None
    if problems is not None:
        for problem in problems:
            print(problem)
        return 1 if problems else 0
    registry = pathlib.Path(REGISTRY)
    registry_text = registry.read_text() if registry.is_file() else None
    files = tracked_texts()
    problems = judge(registry_text, files)
    for problem in problems:
        print(problem)
    if problems:
        return 1
    count = len(rows(registry_text))
    print(f"sealed held-out pairs: {count} listed, none named or pointed at by the {len(files) - 1} other tracked files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
