#!/usr/bin/env bash
# Fetch the redistributable reference models (#407) and verify their hashes.
#
# Usage:
#   tools/fetch-reference-models.sh [target-dir]
#
# Reads research/witness-registry.json and downloads every artifact that is
# `redistributable` and has a `url`, from its original source, into
# target-dir (default: _project_corpus/Revit, gitignored), under the file
# name the corpus tests expect. Each file is checked against the registry's
# sha256 and byte count; a mismatch is deleted and reported, and the script
# exits 1. Files already present with the right hash are left alone.
#
# Local-only artifacts (no licence, `redistributable: false`) are listed with
# their source and hash so you can obtain them yourself; they are never
# fetched or committed.
#
# Then:
#   RVT_PROJECT_CORPUS_DIR="$(realpath _project_corpus/Revit)" \
#     cargo test --profile ci --test integration -- revit_global_ids:: element_records_2025::

set -euo pipefail

TARGET_DIR="${1:-_project_corpus/Revit}"
REGISTRY="$(dirname "$0")/../research/witness-registry.json"
mkdir -p "$TARGET_DIR"

python3 - "$REGISTRY" "$TARGET_DIR" <<'PY'
import hashlib, json, os, sys, urllib.parse, urllib.request

registry, target = sys.argv[1], sys.argv[2]
artifacts = json.load(open(registry))["artifacts"]
failed = 0

def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()

for a in artifacts:
    url = a.get("url")
    if not url:
        continue
    name = urllib.parse.unquote(url.rsplit("/", 1)[1])
    if not a.get("redistributable"):
        print(f"  [local only] {name}: {a['source']} ({a['license']}), sha256 {a['sha256']}")
        continue
    path = os.path.join(target, name)
    if os.path.exists(path) and sha256(path) == a["sha256"]:
        print(f"  [ok]         {name}")
        continue
    print(f"  [fetch]      {name} <- {url}")
    request = urllib.request.Request(url, headers={"User-Agent": "rvt-rs fetch-reference-models"})
    with urllib.request.urlopen(request) as response, open(path, "wb") as out:
        while chunk := response.read(1 << 20):
            out.write(chunk)
    size, digest = os.path.getsize(path), sha256(path)
    if digest != a["sha256"] or size != a["bytes"]:
        os.remove(path)
        print(f"  [MISMATCH]   {name}: got {size} bytes sha256 {digest}, registry says {a['bytes']} bytes {a['sha256']}")
        failed += 1
    else:
        print(f"  [verified]   {name} ({size} bytes)")

for a in artifacts:
    if not a.get("redistributable") and not a.get("url"):
        print(f"  [local only] {a['id']}: {a['source']} ({a['license']}), sha256 {a['sha256']}")

sys.exit(1 if failed else 0)
PY
