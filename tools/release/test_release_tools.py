#!/usr/bin/env python3
"""Failure-first tests of the release tooling: what bump_version.py and changelog.py must not let through (#1886).

The pure parts are tested as functions (a date, the citation rewrite, which local actions a workflow names, which
items a changelog has released). `--release-workflows-from` is tested end to end: the real script runs on a
scratch git repository whose version files are the preflight tests' tree, with a branch no one has put on main,
a workflow that names the actions in either form, and a hotfix worktree to copy them into.
"""
import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, ROOT / path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


sys.path.insert(0, str(ROOT / "tools" / "ci"))
import release_preflight as preflight  # noqa: E402
import test_release_preflight as fixtures  # noqa: E402

bump = load("bump_version", "tools/release/bump_version.py")
changelog = load("changelog_tool", "tools/release/changelog.py")

V = fixtures.V
NEW = "0.16.1"


class Date(unittest.TestCase):
    def test_a_date_is_a_calendar_date_not_only_the_shape_of_one(self):
        for text in ("2026-13-45", "2026-02-30", "2026-00-10", "2026-11-00", "2026-04-31", "2025-02-29", "0000-01-01"):
            self.assertFalse(bump.valid_date(text), text)

    def test_a_date_of_another_shape_is_refused(self):
        for text in ("2026-2-3", "26-11-13", "2026/11/13", "2026-11-13\n", " 2026-11-13", "20261113", "2026-W46-5",
                     "", "today", "٢٠٢٦-١١-١٣"):
            self.assertFalse(bump.valid_date(text), repr(text))

    def test_a_real_date_passes(self):
        for text in ("2026-11-13", "2024-02-29", "2026-12-31", "2026-01-01"):
            self.assertTrue(bump.valid_date(text), text)


class Citation(unittest.TestCase):
    def moved(self, text):
        return bump.bump_citation(text, preflight, V, NEW, "2026-11-13")

    def test_every_form_the_preflight_reads_is_one_the_bump_rewrites(self):
        for line in (f"version: {V}", f'version: "{V}"', f"version: {V}  # the release", f"version:\t{V}",
                     f'version: "{V}" # quoted and commented'):
            text = f'cff-version: 1.2.0\n{line}\ndate-released: "2026-11-05"\n'
            self.assertEqual(preflight.check_versions({**fixtures.tree(), "CITATION.cff": text}, V)[1], [], line)
            moved = self.moved(text)
            self.assertEqual(preflight.CITATION_VERSION.findall(moved), [NEW], line)
            self.assertIn('date-released: "2026-11-13"', moved, line)

    def test_a_date_written_in_the_forms_yaml_allows_is_moved_too(self):
        for line in ("date-released: 2026-11-05", 'date-released: "2026-11-05"', "date-released: 2026-11-05  # when"):
            moved = self.moved(f"version: {V}\n{line}\n")
            self.assertEqual(moved, f'version: {NEW}\ndate-released: "2026-11-13"\n', line)

    def test_a_citation_the_bump_cannot_follow_stops_it_and_never_writes_half(self):
        for text in (f"version: 0.15.0\ndate-released: \"2026-11-05\"\n",  # not the version being moved
                     f'version: {V}\nversion: {V}\ndate-released: "2026-11-05"\n',  # two top-level lines
                     f"references:\n  - version: {V}\ndate-released: \"2026-11-05\"\n",  # only a nested one
                     f"version: {V}\n"):  # no date line
            with self.assertRaises(SystemExit, msg=text):
                self.moved(text)

    def test_a_nested_version_is_left_alone(self):
        moved = self.moved(f'version: {V}\nreferences:\n  - version: 9.9.9\ndate-released: "2026-11-05"\n')
        self.assertIn("  - version: 9.9.9\n", moved)
        self.assertTrue(moved.startswith(f"version: {NEW}\n"))


class Workflows(unittest.TestCase):
    def test_a_local_action_is_found_in_either_form_and_once(self):
        text = """\
jobs:
  a:
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
      - name: Rust
        uses: $/.github/actions/setup-rust
      - uses: ./.github/actions/wait-for-check # waits
      - name: Again
        uses: $/.github/actions/setup-rust
        with:
          x: 1
      # uses: $/.github/actions/commented-out
      - run: echo "uses: ./.github/actions/in-a-string"
      - uses: $/.github/actions/fetch-corpus
"""
        self.assertEqual(
            bump.local_actions(text),
            [".github/actions/setup-rust", ".github/actions/wait-for-check", ".github/actions/fetch-corpus"],
        )

    def test_a_remote_action_or_none_is_not_a_local_one(self):
        self.assertEqual(bump.local_actions("steps:\n  - uses: actions/cache@abc\n  - run: ls\n"), [])
        self.assertEqual(bump.local_actions(""), [])


class Changelog(unittest.TestCase):
    ITEM = "- **A fix (#1).** Said once."
    OTHER = "- **Another fix (#2).** Said once."

    def released(self, text):
        return changelog.released_items_of(text)

    def test_unreleased_first_gives_the_sections_below_it(self):
        text = f"# Changelog\n\n## [Unreleased]\n\n### Fixed\n\n{self.OTHER}\n\n## [0.2.0] - x\n\n### Fixed\n\n{self.ITEM}\n"
        self.assertEqual(self.released(text), {self.ITEM})

    def test_unreleased_last_still_gives_the_released_sections_above_it(self):
        # It was read as "the section above the released ones": last, it checked nothing and every item passed.
        text = f"# Changelog\n\n## [0.2.0] - x\n\n### Fixed\n\n{self.ITEM}\n\n## [Unreleased]\n\n### Fixed\n\n{self.OTHER}\n"
        self.assertEqual(self.released(text), {self.ITEM})

    def test_unreleased_in_the_middle_gives_both_sides(self):
        text = f"## [0.3.0] - x\n\n{self.ITEM}\n\n## [Unreleased]\n\n{self.OTHER}\n\n## [0.2.0] - y\n\n- **Old (#0).** Old.\n"
        self.assertEqual(self.released(text), {self.ITEM, "- **Old (#0).** Old."})

    def test_no_unreleased_section_means_every_section_is_released(self):
        self.assertEqual(self.released(f"# Changelog\n\n## [0.2.0] - x\n\n{self.ITEM}\n"), {self.ITEM})

    def test_only_unreleased_releases_nothing(self):
        self.assertEqual(self.released(f"# Changelog\n\n## [Unreleased]\n\n{self.ITEM}\n"), set())
        self.assertEqual(self.released(""), set())
        self.assertEqual(self.released(f"{self.ITEM}\n"), set())


def run(args, cwd, check=True):
    done = subprocess.run(args, cwd=cwd, capture_output=True, text=True)
    if check and done.returncode != 0:
        raise AssertionError(f"{args} failed: {done.stdout}{done.stderr}")
    return done


def git(repo, *args, check=True):
    return run(["git", "-c", "user.name=release-test", "-c", "user.email=release-test@example.invalid",
                "-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false", *args], repo, check)


WORKFLOW = """\
name: {name}
on: workflow_dispatch
jobs:
  job:
    runs-on: ubuntu-24.04
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
{uses}
"""


class CopyWorkflows(unittest.TestCase):
    """`bump_version.py --release-workflows-from`, run as a script on a scratch repository."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.repo = pathlib.Path(self.tmp.name) / "repo"
        self.repo.mkdir()
        files = dict(fixtures.at_version(V))
        # The viewer's package.json as npm writes it: indented, the version not the last key.
        files["viewer/package.json"] = json.dumps({"name": "viewer", "version": V, "private": True}, indent=2) + "\n"
        files[".github/workflows/publish.yml"] = WORKFLOW.format(
            name="publish",
            uses="      - uses: $/.github/actions/wait-for-check\n      - uses: $/.github/actions/fetch-corpus\n"
            "      - uses: $/.github/actions/setup-rust\n",
        )
        files[".github/workflows/release-binaries.yml"] = WORKFLOW.format(
            name="binaries", uses="      - uses: ./.github/actions/setup-rust\n      - uses: $/.github/actions/wasm-tools\n"
        )
        for action, names in {"wait-for-check": ("action.yml", "wait.sh"), "fetch-corpus": ("action.yml",),
                              "setup-rust": ("action.yml",), "wasm-tools": ("action.yml",),
                              "fetch-re1": ("action.yml",)}.items():
            for name in names:
                files[f".github/actions/{action}/{name}"] = f"# main's {action}/{name}\n"
        files["tools/ci/release_preflight.py"] = (ROOT / "tools/ci/release_preflight.py").read_text()
        for path, text in files.items():
            target = self.repo / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_text(text)
        git(self.repo, "init", "-q", "-b", "main")
        git(self.repo, "add", "-A")
        git(self.repo, "commit", "-q", "-m", "main")
        self.main = git(self.repo, "rev-parse", "HEAD").stdout.strip()
        git(self.repo, "update-ref", "refs/remotes/origin/main", self.main)
        # A branch no one put on main, whose publish.yml is another file.
        git(self.repo, "checkout", "-q", "-b", "feature")
        (self.repo / ".github/workflows/publish.yml").write_text("name: not main's\n")
        git(self.repo, "commit", "-q", "-am", "feature")
        self.feature = git(self.repo, "rev-parse", "HEAD").stdout.strip()
        # The hotfix checkout: main's tree, the workflows older than main's.
        git(self.repo, "checkout", "-q", "-b", "hotfix/v0.16.1", self.main)
        for path in (".github/workflows/publish.yml", ".github/workflows/release-binaries.yml",
                     ".github/actions/wait-for-check/action.yml", ".github/actions/wait-for-check/wait.sh"):
            (self.repo / path).write_text("# the old one\n")
        for path in (".github/actions/fetch-corpus", ".github/actions/setup-rust", ".github/actions/wasm-tools",
                     ".github/actions/fetch-re1"):
            for file in (self.repo / path).glob("*"):
                file.unlink()
            (self.repo / path).rmdir()
        (self.repo / "tools/ci/release_preflight.py").write_text("# the old preflight\n")
        git(self.repo, "commit", "-q", "-am", "the hotfix branch, cut from an old tag")

    def bump(self, *extra, check=False):
        return run([sys.executable, str(ROOT / "tools/release/bump_version.py"), NEW, "--root", str(self.repo), *extra],
                   self.repo, check)

    def test_a_ref_that_main_does_not_hold_is_refused_and_nothing_is_copied(self):
        for ref in ("feature", self.feature, "hotfix/v0.16.1", "no-such-ref"):
            done = self.bump("--release-workflows-from", ref)
            self.assertNotEqual(done.returncode, 0, ref)
            self.assertTrue("is not on origin/main" in done.stderr or "is not a commit" in done.stderr, done.stderr)
            # Refused before the version files were touched, too.
            self.assertEqual((self.repo / ".github/workflows/publish.yml").read_text(), "# the old one\n", ref)
            self.assertIn(f"version: {V}", (self.repo / "CITATION.cff").read_text(), ref)

    def test_a_main_that_lacks_a_file_to_copy_is_refused_with_nothing_written(self):
        # Each lack is met after the files read before it (the two workflows, for the action), which a script that
        # wrote as it read had already replaced, along with the version files.
        for lacking in (".github/actions/wasm-tools", ".github/workflows/release-binaries.yml"):
            git(self.repo, "checkout", "-q", "-f", "-B", "lacking", self.main)
            git(self.repo, "rm", "-q", "-r", lacking)
            git(self.repo, "commit", "-q", "-m", f"main without {lacking}")
            main = git(self.repo, "rev-parse", "HEAD").stdout.strip()
            git(self.repo, "update-ref", "refs/remotes/origin/main", main)
            git(self.repo, "checkout", "-q", "-f", "hotfix/v0.16.1")
            done = self.bump("--release-workflows-from", main)
            self.assertNotEqual(done.returncode, 0, lacking)
            self.assertIn(f"has no {lacking}", done.stderr)
            self.assertEqual(git(self.repo, "status", "--porcelain").stdout, "", lacking)

    def test_main_copies_the_workflows_and_every_local_action_they_use(self):
        for ref in ("origin/main", self.main):
            git(self.repo, "checkout", "-q", "-f", "hotfix/v0.16.1")
            git(self.repo, "reset", "-q", "--hard", "hotfix/v0.16.1")
            git(self.repo, "clean", "-fdq")
            done = self.bump("--release-workflows-from", ref, check=True)
            for path in (".github/actions/fetch-corpus/action.yml", ".github/actions/setup-rust/action.yml",
                         ".github/actions/wasm-tools/action.yml", ".github/actions/wait-for-check/action.yml",
                         ".github/actions/wait-for-check/wait.sh"):
                self.assertEqual((self.repo / path).read_text(), f"# main's {path.split('/')[2]}/{path.split('/')[3]}\n",
                                 f"{ref}: {path}")
            self.assertNotEqual((self.repo / ".github/workflows/publish.yml").read_text(), "# the old one\n")
            # Only what the workflows use: main's fetch-re1 is not theirs.
            self.assertFalse((self.repo / ".github/actions/fetch-re1").exists(), ref)
            self.assertIn("replaces a file that differs", done.stdout)
            self.assertRegex(done.stdout, r"fetch-corpus/action\.yml from \S+ \(\w+\): new")
            self.assertEqual((self.repo / "tools/ci/release_preflight.py").read_text(),
                             (ROOT / "tools/ci/release_preflight.py").read_text(), ref)

    def test_a_calendar_date_is_required_before_anything_is_written(self):
        done = self.bump("--date", "2026-13-45")
        self.assertNotEqual(done.returncode, 0)
        self.assertIn("calendar date", done.stderr)
        self.assertIn(f"version: {V}", (self.repo / "CITATION.cff").read_text())

    def test_a_citation_the_preflight_reads_is_bumped(self):
        text = f'cff-version: 1.2.0\ntitle: "x"\nversion: "{V}"  # the release\ndate-released: 2026-11-05\n'
        (self.repo / "CITATION.cff").write_text(text)
        done = self.bump("--date", "2026-11-13", check=True)
        moved = (self.repo / "CITATION.cff").read_text()
        self.assertIn(f'version: "{NEW}"  # the release', moved, done.stdout)
        self.assertIn('date-released: "2026-11-13"', moved)


if __name__ == "__main__":
    unittest.main()
