#!/usr/bin/env python3
"""Failure-first tests of .github/actions/wait-for-check/wait.sh: what it must do with the CI run of a commit.

The real script runs as a process. Only the GitHub API is simulated, at the `gh` boundary: a stub `gh` answers
`gh api` from canned JSON and applies the script's own `--jq` filter with the real `jq`, as `gh` does, so the
filters are exercised too. The case that matters is a commit whose push CI run was cancelled by a later push
while pending: it never makes `CI passed`, and the script waited its whole timeout for it, with the viewer deploy
of every later commit queued behind it (13 November 2026).
"""
import json
import os
import pathlib
import shutil
import subprocess
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
# WAIT_FOR_CHECK_SCRIPT runs another copy, such as the script before a change, to see these fail on it.
SCRIPT = pathlib.Path(os.environ.get("WAIT_FOR_CHECK_SCRIPT") or ROOT / ".github" / "actions" / "wait-for-check" / "wait.sh")

# `gh api [--paginate] PATH [-X GET] [-f k=v ...] [--jq FILTER]`: the canned answer for PATH, filtered. A
# check-runs answer is the next of the scenario's list (the last one stays), and every query is counted.
STUB = r"""#!/usr/bin/env bash
set -euo pipefail
[ "$1" = api ] || exit 2
shift
jq_filter=""
path=""
while [ $# -gt 0 ]; do
  case "$1" in
    --jq) jq_filter="$2"; shift 2 ;;
    -X|-f) shift 2 ;;
    --paginate) shift ;;
    *) path="$1"; shift ;;
  esac
done
case "$path" in
  */check-runs)
    n=$(cat "$SCENARIO/polls" 2>/dev/null || echo 0)
    n=$((n + 1))
    echo "$n" > "$SCENARIO/polls"
    file="$SCENARIO/check-runs.$n.json"
    [ -f "$file" ] || file=$(ls "$SCENARIO"/check-runs.*.json | sort -V | tail -n 1)
    ;;
  */actions/runs) file="$SCENARIO/runs.json" ;;
  */commits/*) file="$SCENARIO/commit.json" ;;
  *) echo "unexpected path $path" >&2; exit 3 ;;
esac
jq -r "$jq_filter" < "$file"
"""

NO_CHECK = {"total_count": 0, "check_runs": []}


def check(conclusion):
    return {"total_count": 1, "check_runs": [{"status": "completed", "conclusion": conclusion}]}


def run(name, status, conclusion):
    return {"name": name, "run_number": 7, "status": status, "conclusion": conclusion}


class WaitForCheck(unittest.TestCase):
    def setUp(self):
        if shutil.which("jq") is None:
            self.fail("jq is needed to run the stub gh as gh does")
        self.dir = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.dir.name)
        (self.root / "bin").mkdir()
        gh = self.root / "bin" / "gh"
        gh.write_text(STUB, encoding="utf-8")
        gh.chmod(0o755)
        self.addCleanup(self.dir.cleanup)

    def scenario(self, checks, runs, files=("src/lib.rs",)):
        scenario = self.root / "scenario"
        scenario.mkdir()
        for number, body in enumerate(checks, 1):
            (scenario / f"check-runs.{number}.json").write_text(json.dumps(body), encoding="utf-8")
        (scenario / "runs.json").write_text(json.dumps({"workflow_runs": runs}), encoding="utf-8")
        (scenario / "commit.json").write_text(json.dumps({"files": [{"filename": f} for f in files]}), encoding="utf-8")
        return scenario

    def wait(self, scenario, attempts=4, **env):
        output = self.root / "github-output"
        output.write_text("", encoding="utf-8")
        environment = {
            **os.environ,
            "PATH": f"{self.root / 'bin'}{os.pathsep}{os.environ['PATH']}",
            "SCENARIO": str(scenario),
            "GH_TOKEN": "x",
            "GH_REPO": "owner/repo",
            "SHA": "a" * 40,
            "CHECK_NAME": "CI passed",
            "ATTEMPTS": str(attempts),
            "POLL_SECONDS": "0",
            "GITHUB_OUTPUT": str(output),
            **env,
        }
        done = subprocess.run(
            ["bash", str(SCRIPT)], env=environment, capture_output=True, text=True, timeout=120, check=False
        )
        result = [line[7:] for line in output.read_text(encoding="utf-8").splitlines() if line.startswith("result=")]
        polls = int((scenario / "polls").read_text(encoding="utf-8")) if (scenario / "polls").exists() else 0
        return done, (result[-1] if result else None), polls

    def test_a_check_that_passed_succeeds(self):
        done, result, _ = self.wait(self.scenario([check("success")], [run("CI", "completed", "success")]))
        self.assertEqual((done.returncode, result), (0, "success"), done.stderr)

    def test_a_cancelled_ci_run_ends_the_wait_at_once_and_leaves_the_commit_out(self):
        scenario = self.scenario([NO_CHECK], [run("CI", "completed", "cancelled")])
        done, result, polls = self.wait(scenario, attempts=5, SUPERSEDED_SKIPS="true")
        self.assertEqual((done.returncode, result), (0, "superseded"), done.stdout + done.stderr)
        self.assertLessEqual(polls, 2, "it went on polling for a check that cannot come")

    def test_a_release_does_not_ride_on_a_cancelled_ci_run(self):
        scenario = self.scenario([NO_CHECK], [run("CI", "completed", "cancelled")])
        done, result, polls = self.wait(scenario, attempts=5)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("cancelled", done.stdout + done.stderr)
        self.assertIsNone(result)
        self.assertLessEqual(polls, 2)

    def test_a_ci_run_still_going_is_waited_for(self):
        scenario = self.scenario(
            [NO_CHECK, NO_CHECK, check("success")], [run("CI", "in_progress", None)]
        )
        done, result, polls = self.wait(scenario, SUPERSEDED_SKIPS="true")
        self.assertEqual((done.returncode, result, polls), (0, "success", 3), done.stderr)

    def test_a_run_that_never_finishes_still_fails_at_the_timeout(self):
        scenario = self.scenario([NO_CHECK], [run("CI", "in_progress", None)])
        done, _, polls = self.wait(scenario, attempts=2, SUPERSEDED_SKIPS="true")
        self.assertEqual(done.returncode, 1)
        self.assertIn("did not finish", done.stdout + done.stderr)
        self.assertGreaterEqual(polls, 2)

    def test_another_workflows_cancelled_run_is_not_ci_cancelled(self):
        scenario = self.scenario(
            [NO_CHECK], [run("CodeQL", "completed", "cancelled"), run("CI", "in_progress", None)]
        )
        done, result, _ = self.wait(scenario, attempts=2, SUPERSEDED_SKIPS="true")
        self.assertEqual(done.returncode, 1)
        self.assertNotEqual(result, "superseded")

    def test_a_failed_check_stops_the_wait(self):
        done, result, _ = self.wait(
            self.scenario([check("failure")], [run("CI", "completed", "failure")]), SUPERSEDED_SKIPS="true"
        )
        self.assertEqual(done.returncode, 1)
        self.assertIsNone(result)
        self.assertIn("failure", done.stdout + done.stderr)

    def test_a_commit_ci_never_ran_on_passes_when_the_caller_allows_it(self):
        scenario = self.scenario([NO_CHECK], [], files=("README.md", "docs/x.rs"))
        done, result, _ = self.wait(scenario)
        self.assertEqual((done.returncode, result), (0, "skipped-no-ci"), done.stderr)

    def test_a_commit_ci_never_ran_on_fails_a_release(self):
        scenario = self.scenario([NO_CHECK], [], files=("README.md",))
        done, _, _ = self.wait(scenario, DOCS_ONLY_SKIPS="false")
        self.assertEqual(done.returncode, 1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
