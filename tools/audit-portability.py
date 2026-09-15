#!/usr/bin/env python3
"""Audit upstream portability/provenance hazards without opening build artifacts.

The inventory is deliberately factual: it reports references and requirements;
it does not decide whether Revit, ODA, API witnesses, or historical attribution
are acceptable for a particular research claim.
"""
from __future__ import annotations

import argparse
import json
import re
import subprocess
from collections import Counter
from pathlib import Path

CANDIDATE_SUFFIXES = {
    ".rs", ".py", ".pyi", ".md", ".txt", ".toml", ".json", ".yaml", ".yml",
    ".sh", ".ps1", ".cs", ".js", ".ts", ".tsx", ".jsx", ".html", ".ipynb",
}
EXCLUDED_PARTS = {".git", "target", "node_modules", ".venv", "__pycache__"}
MAX_READ_BYTES = 8 * 1024 * 1024

RULES: dict[str, tuple[str, ...]] = {
    "absolute_developer_path": (
        r"/(?:home|Users|private|tmp|var|mnt)/[A-Za-z0-9_.-]+",
        r"[A-Za-z]:\\(?:Users|home|private|tmp|Program Files)\\",
    ),
    "sibling_or_workspace_reference": (
        r"\.\./(?:rvt-rs|spatiallogic)(?:[/\\]|$)",
        r"(?:^|[/\\])spatiallogic(?:[/\\])tools[/\\]bim[/\\]data",
    ),
    "private_fixture_or_receipt": (
        r"(?:revit-research|RVT_ORACLE_RUN_DIR|RVT_PROJECT_CORPUS_DIR|RVT_SAMPLES_DIR)",
        r"(?:gpubox|oracle/out|private/|world-model-[A-Za-z0-9_-]+)",
    ),
    "process_or_external_invocation": (
        r"\b(?:subprocess\.(?:run|Popen)|Command::new|ssh|scp|powershell|dotnet|docker|Revit\.exe|wasm-pack|maturin|npm)\b",
        r"\bcargo\s+(?:run|build|test|bench)\b",
    ),
    "revit_or_api_runtime_reference": (
        r"\b(?:Revit(?:API)?|pyRevit|Autodesk\.Revit|RevitAPI\.dll)\b",
        r"\b(?:ODA|BimRv|DDC|API oracle|API witness)\b",
    ),
}
COMPILED_RULES = {k: [re.compile(p, re.I) for p in v] for k, v in RULES.items()}


def git_files(root: Path, tracked: bool) -> list[Path]:
    args = ["git", "-C", str(root), "ls-files"]
    if not tracked:
        args += ["--others", "--exclude-standard"]
    out = subprocess.check_output(args, text=True)
    return [root / line for line in out.splitlines() if line]


def candidate(path: Path, root: Path) -> bool:
    try:
        rel = path.relative_to(root)
    except ValueError:
        return False
    if any(part in EXCLUDED_PARTS for part in rel.parts):
        return False
    return path.is_file() and path.suffix.lower() in CANDIDATE_SUFFIXES


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--json-out", type=Path)
    parser.add_argument("--max-read-bytes", type=int, default=MAX_READ_BYTES)
    args = parser.parse_args()
    root = args.root.resolve()
    paths = sorted({p for tracked in (True, False) for p in git_files(root, tracked) if candidate(p, root)})
    counts = Counter()
    findings: list[dict[str, object]] = []
    for path in paths:
        rel = path.relative_to(root).as_posix()
        size = path.stat().st_size
        counts["candidate_files"] += 1
        if size > args.max_read_bytes:
            counts["oversized_files"] += 1
            findings.append({"path": rel, "kind": "oversized_file", "bytes": size, "max_read_bytes": args.max_read_bytes})
            continue
        try:
            text = path.read_text(encoding="utf-8", errors="replace")
        except OSError as exc:
            counts["unreadable_files"] += 1
            findings.append({"path": rel, "kind": "unreadable_file", "error": str(exc)})
            continue
        for kind, patterns in COMPILED_RULES.items():
            matches = []
            for pattern in patterns:
                matches.extend(m.group(0) for m in pattern.finditer(text))
            if matches:
                counts[kind] += 1
                findings.append({"path": rel, "kind": kind, "match_count": len(matches), "examples": sorted(set(matches))[:8]})
    report = {
        "format": "rvt-upstream-portability-audit/v1",
        "root": str(root),
        "candidate_suffixes": sorted(CANDIDATE_SUFFIXES),
        "excluded_parts": sorted(EXCLUDED_PARTS),
        "max_read_bytes": args.max_read_bytes,
        "counts": dict(sorted(counts.items())),
        "findings": findings,
        "interpretation": {
            "historical_docs_are_reported_references_not_runtime_dependencies": True,
            "revit_or_api_references_require_source_scope_review": True,
            "no_sdk_source_access_inference": "This tool detects text references only; it cannot establish what a contributor accessed.",
        },
    }
    encoded = json.dumps(report, indent=2, sort_keys=True) + "\n"
    if args.json_out:
        args.json_out.parent.mkdir(parents=True, exist_ok=True)
        args.json_out.write_text(encoded, encoding="utf-8")
    print(json.dumps({"format": report["format"], "root": str(root), "counts": report["counts"]}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
