#!/usr/bin/env python3
"""Run rvt-rs performance budgets and emit machine-readable JSON."""

from __future__ import annotations

import argparse
import json
import os
import platform
import statistics
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


REPO_ROOT = Path(__file__).resolve().parents[1]
CONFIG_PATH = REPO_ROOT / "tools" / "perf-budgets.json"


@dataclass(frozen=True)
class TimeTool:
    kind: str
    command: list[str]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path, default=CONFIG_PATH)
    parser.add_argument(
        "--json-out",
        type=Path,
        default=REPO_ROOT / "target" / "perf-budgets" / "latest.json",
    )
    parser.add_argument("--iterations", type=int, default=None)
    parser.add_argument("--enforce", action="store_true")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument(
        "--require-category",
        action="append",
        default=[],
        help="Fail if this fixture category cannot be resolved.",
    )
    parser.add_argument(
        "--operation",
        action="append",
        default=[],
        help="Limit to one operation; may be repeated.",
    )
    return parser.parse_args()


def load_config(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as f:
        return json.load(f)


def git_commit() -> str | None:
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=REPO_ROOT,
            text=True,
            capture_output=True,
            check=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return None
    return result.stdout.strip()


def detect_time_tool() -> TimeTool | None:
    time_bin = Path("/usr/bin/time")
    if not time_bin.exists():
        return None
    gnu_probe = subprocess.run(
        [str(time_bin), "-f", "RVT_RSS_KB=%M", "true"],
        text=True,
        capture_output=True,
    )
    if gnu_probe.returncode == 0 and "RVT_RSS_KB=" in gnu_probe.stderr:
        return TimeTool("gnu_time", [str(time_bin), "-f", "RVT_RSS_KB=%M"])
    bsd_probe = subprocess.run(
        [str(time_bin), "-l", "true"],
        text=True,
        capture_output=True,
    )
    if bsd_probe.returncode == 0 and "maximum resident set size" in bsd_probe.stderr:
        return TimeTool("bsd_time", [str(time_bin), "-l"])
    return None


def build_helper(example_name: str) -> None:
    subprocess.run(
        ["cargo", "build", "--release", "--example", example_name],
        cwd=REPO_ROOT,
        check=True,
    )


def helper_path(example_name: str) -> Path:
    suffix = ".exe" if platform.system() == "Windows" else ""
    return REPO_ROOT / "target" / "release" / "examples" / f"{example_name}{suffix}"


def candidate_paths(category: str, cfg: dict[str, Any]) -> list[Path]:
    paths: list[Path] = []
    env_name = cfg.get("env")
    if env_name and os.environ.get(env_name):
        paths.append(Path(os.environ[env_name]))
    if category == "small" and os.environ.get("RVT_SAMPLES_DIR"):
        samples = Path(os.environ["RVT_SAMPLES_DIR"])
        paths.extend(
            [
                samples / "racbasicsamplefamily-2024.rfa",
                samples / "rac_basic_sample_family-2024.rfa",
            ]
        )
    if category == "medium" and os.environ.get("RVT_PROJECT_CORPUS_DIR"):
        projects = Path(os.environ["RVT_PROJECT_CORPUS_DIR"])
        paths.extend(
            [
                projects / "2024_Core_Interior.rvt",
                projects / "Revit_IFC5_Einhoven.rvt",
            ]
        )
    for raw in cfg.get("default_paths", []):
        path = Path(raw)
        paths.append(path if path.is_absolute() else REPO_ROOT / path)
    return paths


def resolve_fixture(category: str, cfg: dict[str, Any]) -> Path | None:
    for path in candidate_paths(category, cfg):
        if path.exists() and path.is_file():
            return path
    return None


def parse_rss_bytes(kind: str, stderr: str) -> int | None:
    if kind == "gnu_time":
        for line in stderr.splitlines():
            if line.startswith("RVT_RSS_KB="):
                return int(line.split("=", 1)[1]) * 1024
    if kind == "bsd_time":
        for line in stderr.splitlines():
            stripped = line.strip()
            if stripped.endswith("maximum resident set size"):
                return int(stripped.split()[0])
    return None


def run_one(helper: Path, op: str, fixture: Path, time_tool: TimeTool | None) -> dict[str, Any]:
    command = [str(helper), op, str(fixture)]
    measured = command
    if time_tool is not None:
        measured = [*time_tool.command, *command]
    start = time.perf_counter()
    result = subprocess.run(
        measured,
        cwd=REPO_ROOT,
        text=True,
        capture_output=True,
    )
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    if result.returncode != 0:
        sys.stderr.write(result.stdout)
        sys.stderr.write(result.stderr)
        raise subprocess.CalledProcessError(result.returncode, measured)
    rss_bytes = parse_rss_bytes(time_tool.kind, result.stderr) if time_tool else None
    payload = json.loads(result.stdout)
    return {
        "elapsed_ms": elapsed_ms,
        "peak_rss_bytes": rss_bytes,
        "helper": payload,
    }


def budget_status(
    best_ms: float,
    peak_rss_bytes: int | None,
    budget: dict[str, Any],
) -> tuple[str, list[str]]:
    failures: list[str] = []
    max_ms = float(budget["max_ms"])
    if best_ms > max_ms:
        failures.append(f"best_ms {best_ms:.2f} > budget {max_ms:.2f}")
    max_rss_mb = budget.get("max_rss_mb")
    if max_rss_mb is not None and peak_rss_bytes is not None:
        max_rss_bytes = float(max_rss_mb) * 1024.0 * 1024.0
        if peak_rss_bytes > max_rss_bytes:
            failures.append(
                f"peak_rss_mb {peak_rss_bytes / 1024 / 1024:.1f} > budget {float(max_rss_mb):.1f}"
            )
    return ("fail" if failures else "pass", failures)


def run_budget(args: argparse.Namespace, config: dict[str, Any]) -> dict[str, Any]:
    example_name = config["helper_example"]
    if not args.no_build:
        build_helper(example_name)
    helper = helper_path(example_name)
    if not helper.exists():
        raise SystemExit(f"helper binary missing: {helper}")

    operations = args.operation or config["operations"]
    time_tool = detect_time_tool()
    results: list[dict[str, Any]] = []
    skipped: list[dict[str, Any]] = []
    failures: list[dict[str, Any]] = []

    for category, category_cfg in config["categories"].items():
        fixture = resolve_fixture(category, category_cfg)
        if fixture is None:
            record = {
                "category": category,
                "status": "skipped",
                "reason": "fixture not found",
                "env": category_cfg.get("env"),
            }
            skipped.append(record)
            if category in args.require_category:
                failures.append({**record, "failure": "required category missing"})
            continue

        iterations = args.iterations or int(category_cfg.get("iterations", 1))
        size = fixture.stat().st_size
        for op in operations:
            samples = [run_one(helper, op, fixture, time_tool) for _ in range(iterations)]
            elapsed = [sample["elapsed_ms"] for sample in samples]
            rss_values = [
                sample["peak_rss_bytes"]
                for sample in samples
                if sample["peak_rss_bytes"] is not None
            ]
            budget = category_cfg["budgets"][op]
            peak_rss = max(rss_values) if rss_values else None
            status, reasons = budget_status(min(elapsed), peak_rss, budget)
            record = {
                "category": category,
                "operation": op,
                "fixture": str(fixture),
                "fixture_size_bytes": size,
                "iterations": iterations,
                "best_ms": min(elapsed),
                "mean_ms": statistics.fmean(elapsed),
                "max_ms": max(elapsed),
                "peak_rss_bytes": peak_rss,
                "budget": budget,
                "status": status,
                "failure_reasons": reasons,
            }
            results.append(record)
            if status == "fail":
                failures.append(record)

    return {
        "schema_version": 1,
        "generated_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "commit": git_commit(),
        "config": str(args.config),
        "time_source": time_tool.kind if time_tool else "wall_clock_only",
        "enforced": bool(args.enforce),
        "results": results,
        "skipped": skipped,
        "failures": failures,
    }


def print_summary(report: dict[str, Any]) -> None:
    for row in report["results"]:
        rss = row["peak_rss_bytes"]
        rss_text = "n/a" if rss is None else f"{rss / 1024 / 1024:.1f} MiB"
        print(
            f"{row['status']:4} {row['category']:6} {row['operation']:19} "
            f"best={row['best_ms']:.2f}ms mean={row['mean_ms']:.2f}ms rss={rss_text}"
        )
    for row in report["skipped"]:
        print(f"skip {row['category']:6} {row['reason']} ({row.get('env')})")
    if report["failures"]:
        print("\nfailures:")
        for row in report["failures"]:
            if "operation" in row:
                print(
                    f"- {row['category']} {row['operation']}: "
                    + "; ".join(row["failure_reasons"])
                )
            else:
                print(f"- {row['category']}: {row['failure']}")


def main() -> int:
    args = parse_args()
    config = load_config(args.config)
    report = run_budget(args, config)
    args.json_out.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(
        "w", encoding="utf-8", dir=args.json_out.parent, delete=False
    ) as f:
        json.dump(report, f, indent=2, sort_keys=True)
        f.write("\n")
        tmp_name = f.name
    Path(tmp_name).replace(args.json_out)
    print_summary(report)
    print(f"\nwrote {args.json_out}")
    if args.enforce and report["failures"]:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
