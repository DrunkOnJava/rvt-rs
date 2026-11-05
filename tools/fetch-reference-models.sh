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
# exits 1. A download that fails or comes back short of the registry's byte
# count resumes from the bytes it has (an HTTP Range request; a server that
# ignores the range starts it again), six times in all with a growing pause,
# so a connection the CDN keeps dropping does not fail the run: the 33.7 MB
# Core Interior came back short three times running in Measure run
# 41907340876, each fresh restart dropped again, and the job failed after 22
# minutes. A file of the full size whose hash differs is a real mismatch and
# is fetched again from the start. Files already present with the right hash
# are left alone.
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
import hashlib, http.client, json, os, sys, time, urllib.parse, urllib.request

registry, target = sys.argv[1], sys.argv[2]
artifacts = json.load(open(registry))["artifacts"]
failed = 0
ATTEMPTS = 6

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
    for attempt in range(1, ATTEMPTS + 1):
        have = os.path.getsize(path) if os.path.exists(path) else 0
        if have >= a["bytes"]:
            os.remove(path)
            have = 0
        resume = f" from byte {have}" if have else ""
        print(f"  [fetch]      {name} <- {url}" + (f" (attempt {attempt} of {ATTEMPTS}{resume})" if attempt > 1 else ""))
        headers = {"User-Agent": "rvt-rs fetch-reference-models"}
        if have:
            headers["Range"] = f"bytes={have}-"
        request = urllib.request.Request(url, headers=headers)
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                # 206 continues the file; any other success is the whole file.
                mode = "ab" if have and response.status == 206 else "wb"
                with open(path, mode) as out:
                    while chunk := response.read(1 << 20):
                        out.write(chunk)
        except (OSError, http.client.HTTPException) as error:
            print(f"  [error]      {name}: {error}")
        size = os.path.getsize(path) if os.path.exists(path) else 0
        if size == a["bytes"]:
            digest = sha256(path)
            if digest == a["sha256"]:
                print(f"  [verified]   {name} ({size} bytes)")
                break
            print(f"  [MISMATCH]   {name}: got {size} bytes sha256 {digest}, registry says {a['bytes']} bytes {a['sha256']}")
            os.remove(path)
        elif size > a["bytes"]:
            print(f"  [MISMATCH]   {name}: got {size} bytes, registry says {a['bytes']}")
            os.remove(path)
        else:
            print(f"  [short]      {name}: {size} of {a['bytes']} bytes; the next attempt resumes")
        if attempt < ATTEMPTS:
            time.sleep(min(10 * attempt, 60))
    else:
        if os.path.exists(path):
            os.remove(path)
        failed += 1

for a in artifacts:
    if not a.get("redistributable") and not a.get("url"):
        print(f"  [local only] {a['id']}: {a['source']} ({a['license']}), sha256 {a['sha256']}")

sys.exit(1 if failed else 0)
PY
