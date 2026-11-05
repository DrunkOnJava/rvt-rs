#!/usr/bin/env python3
"""Failure-first tests of the pure comparisons in check-workflows.py: what must never read as agreement.

Each test names a way a check could pass when it should fail (a job with no timeout, a
`-latest` label hidden in a matrix, an MSRV that drifted in one of two lists, a path list
that lost an entry, a syntax the comparison cannot read) and asserts that it fails.
"""
import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("check_workflows", Path(__file__).with_name("check-workflows.py"))
cw = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cw)

STEPS = [{"run": "true"}]


def wf(**jobs):
    return {"x.yml": {"jobs": jobs}}


class Timeouts(unittest.TestCase):
    def errors(self, workflows, pending=None):
        return cw.timeout_problems(workflows, pending or {})[0]

    def test_a_job_with_steps_and_no_timeout_fails(self):
        self.assertEqual(len(self.errors(wf(a={"runs-on": "ubuntu-24.04", "steps": STEPS}))), 1)

    def test_a_called_workflow_needs_none(self):
        self.assertEqual(self.errors(wf(a={"uses": "$/.github/workflows/y.yml"})), [])

    def test_a_job_that_calls_a_workflow_is_left_out_even_if_it_also_names_steps(self):
        job = {"uses": "$/.github/workflows/y.yml", "steps": STEPS}
        self.assertEqual(self.errors(wf(a=job)), [])

    def test_a_timeout_that_is_not_a_limit_fails(self):
        for bad in (0, -5, True, 360, 361, "soon", "", None, 2.5):
            job = {"runs-on": "ubuntu-24.04", "steps": STEPS, "timeout-minutes": bad}
            self.assertEqual(len(self.errors(wf(a=job))), 1, bad)

    def test_a_number_or_an_expression_is_a_limit(self):
        for good in (1, 30, 359, "${{ inputs.x && 40 || 360 }}"):
            job = {"runs-on": "ubuntu-24.04", "steps": STEPS, "timeout-minutes": good}
            self.assertEqual(self.errors(wf(a=job)), [], good)

    def test_a_pending_job_is_a_note_not_an_error(self):
        errors, notes = cw.timeout_problems(wf(a={"steps": STEPS}), {"x.yml": {"a": "another lane"}})
        self.assertEqual(errors, [])
        self.assertEqual(len(notes), 1)

    def test_pending_names_jobs_and_a_new_job_beside_it_still_fails(self):
        workflows = wf(a={"steps": STEPS}, b={"steps": STEPS})
        self.assertEqual(len(self.errors(workflows, {"x.yml": {"a": "another lane"}})), 1)

    def test_pending_for_a_whole_file_covers_a_job_added_to_it_only_by_name(self):
        workflows = wf(a={"steps": STEPS}, b={"steps": STEPS})
        self.assertEqual(self.errors(workflows, {"x.yml": {cw.ALL: "another lane"}}), [])

    def test_pending_does_not_cover_another_file(self):
        workflows = {"y.yml": {"jobs": {"a": {"steps": STEPS}}}}
        self.assertEqual(len(self.errors(workflows, {"x.yml": {cw.ALL: "another lane"}})), 1)

    def test_a_workflow_with_no_jobs_key_does_not_crash(self):
        self.assertEqual(self.errors({"x.yml": {}}), [])


class Labels(unittest.TestCase):
    def errors(self, workflows, pending=None):
        return cw.label_problems(workflows, pending or {})[0]

    def test_a_latest_runner_fails(self):
        for label in ("ubuntu-latest", "windows-latest", "macos-latest", "macos-latest-large"):
            self.assertEqual(len(self.errors(wf(a={"runs-on": label, "steps": STEPS}))), 1, label)

    def test_a_latest_label_in_a_matrix_or_an_expression_fails(self):
        matrix = {"os": ["ubuntu-24.04", "macos-latest"]}
        self.assertEqual(len(self.errors(wf(a={"runs-on": "${{ matrix.os }}", "strategy": {"matrix": matrix}, "steps": STEPS}))), 1)
        expr = "${{ fromJSON(x && '[{\"os\":\"windows-latest\"}]') }}"
        job = {"runs-on": "${{ matrix.os }}", "strategy": {"matrix": {"include": expr}}, "steps": STEPS}
        self.assertEqual(len(self.errors(wf(a=job))), 1)

    def test_numbered_labels_pass(self):
        for label in ("ubuntu-24.04", "ubuntu-24.04-arm", "windows-2025-vs2026", "macos-26", "macos-15-intel", "windows-11-arm"):
            self.assertEqual(self.errors(wf(a={"runs-on": label, "steps": STEPS})), [], label)

    def test_a_pending_job_with_a_latest_label_is_a_note(self):
        errors, notes = cw.label_problems(wf(a={"runs-on": "macos-latest", "steps": STEPS}), {"x.yml": {"a": "another lane"}})
        self.assertEqual((errors, len(notes)), ([], 1))

    def test_a_called_workflow_has_no_label_to_read(self):
        self.assertEqual(self.errors(wf(a={"uses": "$/.github/workflows/y.yml"})), [])

    def test_a_job_that_calls_a_workflow_is_left_out_even_if_it_names_a_runner(self):
        self.assertEqual(self.errors(wf(a={"uses": "$/.github/workflows/y.yml", "runs-on": "ubuntu-latest"})), [])


class Versions(unittest.TestCase):
    def test_a_patch_zero_is_the_same_release(self):
        self.assertTrue(cw.same_version("1.87", "1.87.0"))
        self.assertFalse(cw.same_version("1.87", "1.88"))
        self.assertFalse(cw.same_version("1.87", "1.87.1"))

    def test_text_that_is_not_a_version_is_not_equal_to_anything(self):
        self.assertFalse(cw.same_version("stable", "stable"))
        self.assertFalse(cw.same_version("1.87", ""))


FAST = '[{"os":"ubuntu-24.04","rust":"stable"},{"os":"ubuntu-24.04","rust":"%s","label":"msrv"}]'
FULL = '[{"os":"ubuntu-24.04","rust":"stable"},{"os":"windows-2025-vs2026","rust":"stable"},{"os":"ubuntu-24.04","rust":"%s","label":"msrv"}]'


def matrix(fast="1.87", full="1.87"):
    return "${{ fromJSON(inputs.fast == true && '" + FAST % fast + "' || '" + FULL % full + "') }}"


class Msrv(unittest.TestCase):
    def problems(self, expr, root="1.87", manifests=None, deliberate=None):
        return cw.msrv_problems(root, expr, manifests or {}, deliberate or {})

    def test_agreement_passes(self):
        self.assertEqual(self.problems(matrix()), [])

    def test_either_matrix_string_drifting_fails(self):
        self.assertEqual(len(self.problems(matrix(fast="1.86"))), 1)
        self.assertEqual(len(self.problems(matrix(full="1.88"))), 1)
        self.assertEqual(len(self.problems(matrix(fast="1.86", full="1.86"))), 2)

    def test_the_root_moving_alone_fails_both(self):
        self.assertEqual(len(self.problems(matrix(), root="1.88")), 2)

    def test_no_matrix_array_fails_rather_than_passing_unread(self):
        for expr in ("", "${{ matrix }}", "[1]", "${{ fromJSON('[]') }}", "${{ fromJSON('{}') }}"):
            self.assertTrue(self.problems(expr), expr)

    def test_an_array_with_no_msrv_entry_fails(self):
        expr = "${{ fromJSON(x && '[{\"rust\":\"stable\"}]' || '[{\"rust\":\"1.87\",\"label\":\"msrv\"}]') }}"
        self.assertEqual(len(self.problems(expr)), 1)

    def test_two_msrv_entries_in_one_array_fail(self):
        entry = '{"rust":"1.87","label":"msrv"}'
        expr = "${{ fromJSON('[" + entry + "," + entry + "]') }}"
        self.assertEqual(len(self.problems(expr)), 1)

    def test_an_array_that_is_not_json_fails(self):
        self.assertEqual(len(self.problems("${{ fromJSON('[{rust:1.87}]') }}")), 1)

    def test_a_crate_that_drifts_fails(self):
        manifests = {"a/Cargo.toml": "1.86", "b/Cargo.toml": "1.87"}
        self.assertEqual(len(self.problems(matrix(), manifests=manifests)), 1)

    def test_a_crate_with_no_rust_version_is_skipped(self):
        self.assertEqual(self.problems(matrix(), manifests={"a/Cargo.toml": None}), [])

    def test_a_deliberate_difference_must_stay_what_it_says(self):
        manifests = {"m/Cargo.toml": "1.88"}
        self.assertEqual(self.problems(matrix(), manifests=manifests, deliberate={"m/Cargo.toml": "1.88"}), [])
        self.assertEqual(len(self.problems(matrix(), manifests={"m/Cargo.toml": "1.89"}, deliberate={"m/Cargo.toml": "1.88"})), 1)

    def test_a_deliberate_difference_that_has_gone_fails(self):
        problems = self.problems(matrix(), manifests={"m/Cargo.toml": "1.87"}, deliberate={"m/Cargo.toml": "1.88"})
        self.assertEqual(len(problems), 1)
        self.assertIn("no longer differs", problems[0])

    def test_a_deliberate_difference_for_a_missing_crate_fails(self):
        self.assertEqual(len(self.problems(matrix(), manifests={}, deliberate={"m/Cargo.toml": "1.88"})), 1)


class Globs(unittest.TestCase):
    def test_the_shapes_github_paths_use(self):
        self.assertEqual(cw.glob_rule("**.md"), ("suffix", ".md"))
        self.assertEqual(cw.glob_rule("docs/**"), ("prefix", "docs/"))
        self.assertEqual(cw.glob_rule(".github/ISSUE_TEMPLATE/**"), ("prefix", ".github/ISSUE_TEMPLATE/"))
        self.assertEqual(cw.glob_rule("LICENSE*"), ("prefix", "LICENSE"))
        self.assertEqual(cw.glob_rule(".gitignore"), ("exact", ".gitignore"))

    def test_a_shape_it_cannot_read_raises_instead_of_guessing(self):
        for glob in ("", "*.md", "docs/*", "**", "**/x.md", "src/**/*.rs", "a*b", "!docs/**", "docs/?.md", "[ab].md", "**.m*"):
            with self.assertRaises(ValueError, msg=glob):
                cw.glob_rule(glob)


class Regexes(unittest.TestCase):
    def test_the_shapes_the_scripts_use(self):
        self.assertEqual(cw.regex_rule(r"\.md$"), ("suffix", ".md"))
        self.assertEqual(cw.regex_rule("^docs/"), ("prefix", "docs/"))
        self.assertEqual(cw.regex_rule("^LICENSE"), ("prefix", "LICENSE"))
        self.assertEqual(cw.regex_rule(r"^\.gitignore$"), ("exact", ".gitignore"))
        self.assertEqual(cw.regex_rule(r"^CITATION\.cff$"), ("exact", "CITATION.cff"))

    def test_a_shape_it_cannot_read_raises_instead_of_guessing(self):
        for rx in ("", r"\.md", "docs/", ".*", r"^docs/.*", r"^a|b", r"^(a)$", r"^a.b$", "^", "$", r"^a+$", r"^[a]$"):
            with self.assertRaises(ValueError, msg=rx):
                cw.regex_rule(rx)

    def test_an_escaped_dollar_is_not_an_anchor(self):
        with self.assertRaises(ValueError):
            cw.regex_rule(r"^a\$")


class Normalise(unittest.TestCase):
    def test_a_rule_a_broader_one_covers_is_absorbed(self):
        rules = {("suffix", ".md"), ("exact", "CHANGELOG.md"), ("prefix", "docs/"), ("prefix", "docs/x/")}
        self.assertEqual(cw.normalise(rules), {("suffix", ".md"), ("prefix", "docs/")})

    def test_a_rule_nothing_covers_stays(self):
        rules = {("suffix", ".md"), ("exact", "CHANGELOG.txt")}
        self.assertEqual(cw.normalise(rules), rules)

    def test_removing_the_broad_rule_leaves_the_narrow_one_to_differ(self):
        self.assertEqual(cw.normalise({("exact", "CHANGELOG.md")}), {("exact", "CHANGELOG.md")})


PATHS = ["**.md", "docs/**", "CHANGELOG.md", "LICENSE*", ".gitignore", ".github/ISSUE_TEMPLATE/**", "CITATION.cff"]
GROUP = r"\.md$|^docs/|^LICENSE|^\.gitignore$|^\.github/ISSUE_TEMPLATE/|^CITATION\.cff$"
BUILT = {"docs/viewer-demos.json", "docs/support-matrix.json"}


def viewer(group=GROUP, built=r"^docs/(viewer-demos|support-matrix)\.json$"):
    return f"rest=$(grep -vE '({group})' <<<\"$files\" || true)\nbuilt_from=$(grep -E '{built}' <<<\"$files\" || true)\n"


def deploy(group=GROUP):
    return f"if ! grep -qvE '({group})' <<<\"$files\"; then\n  exit 0\nfi\n"


class Docs(unittest.TestCase):
    def problems(self, paths=PATHS, v=None, d=None, built=BUILT):
        return cw.docs_problems(paths, viewer() if v is None else v, deploy() if d is None else d, built)

    def test_agreement_passes(self):
        self.assertEqual(self.problems(), [])

    def test_a_path_the_list_dropped_fails(self):
        self.assertTrue(self.problems(paths=[p for p in PATHS if p != "CITATION.cff"]))

    def test_a_path_the_list_gained_fails(self):
        self.assertTrue(self.problems(paths=PATHS + ["NOTICE"]))

    def test_the_broad_markdown_rule_going_leaves_changelog_to_differ(self):
        self.assertTrue(self.problems(paths=[p for p in PATHS if p != "**.md"]))

    def test_either_script_dropping_or_gaining_an_alternative_fails(self):
        self.assertTrue(self.problems(d=deploy(GROUP.replace(r"|^CITATION\.cff$", ""))))
        self.assertTrue(self.problems(v=viewer(GROUP + "|^NOTICE")))

    def test_the_two_scripts_must_carry_the_same_text(self):
        reordered = r"^docs/|\.md$|^LICENSE|^\.gitignore$|^\.github/ISSUE_TEMPLATE/|^CITATION\.cff$"
        problems = self.problems(d=deploy(reordered))
        self.assertEqual(len(problems), 1)
        self.assertIn("same", problems[0])

    def test_the_carve_out_must_name_the_expected_files(self):
        self.assertTrue(self.problems(v=viewer(built=r"^docs/(viewer-demos)\.json$")))
        self.assertTrue(self.problems(v=viewer(built=r"^docs/(viewer-demos|support-matrix|other)\.json$")))

    def test_a_carve_out_outside_the_documentation_rules_fails(self):
        outside = {"viewer/viewer-demos.json"}
        self.assertTrue(self.problems(v=viewer(built=r"^viewer/(viewer-demos)\.json$"), built=outside))

    def test_a_script_with_no_regex_fails_rather_than_passing_unread(self):
        self.assertTrue(self.problems(d="echo nothing\n"))
        self.assertTrue(self.problems(v="echo nothing\n"))

    def test_a_script_with_two_regexes_fails(self):
        self.assertTrue(self.problems(d=deploy() + deploy()))

    def test_a_script_with_no_carve_out_line_fails(self):
        self.assertTrue(self.problems(v=f"rest=$(grep -vE '({GROUP})' <<<\"$files\")\n"))

    def test_a_syntax_the_comparison_cannot_read_fails(self):
        self.assertTrue(self.problems(paths=PATHS + ["src/**/*.rs"]))
        self.assertTrue(self.problems(d=deploy(GROUP + "|.*")))


class On(unittest.TestCase):
    def test_yamls_on_key_reads_as_true(self):
        self.assertEqual(cw.workflow_on({True: {"push": 1}}), {"push": 1})
        self.assertEqual(cw.workflow_on({"on": {"push": 2}}), {"push": 2})
        self.assertEqual(cw.workflow_on({}), {})


ACTION = ".github/actions/fetch-corpus/action.yml"
REUSABLE = ".github/workflows/viewer-test.yml"
FILES = {
    ACTION: {"inputs": {"repository": {"required": True}, "path": {"required": True}, "cache": {"required": True, "default": "true"}}},
    REUSABLE: {True: {"workflow_call": {"inputs": {"legs": {"type": "string"}, "release_wasm": {"type": "boolean", "default": True}}}}},
}


def uses_in(job):
    return {"w.yml": {"jobs": {"a": job}}}


class LocalUses(unittest.TestCase):
    def problems(self, job, files=FILES):
        return cw.local_use_problems(uses_in(job), files.get)

    def step(self, uses, **given):
        return {"steps": [{"uses": uses, "with": given}]}

    def test_a_reference_that_resolves_passes(self):
        self.assertEqual(self.problems(self.step("$/.github/actions/fetch-corpus", repository="r", path="p")), [])
        self.assertEqual(self.problems(self.step("./.github/actions/fetch-corpus", repository="r", path="p", cache="false")), [])
        self.assertEqual(self.problems({"uses": "$/.github/workflows/viewer-test.yml", "with": {"legs": "[]"}}), [])

    def test_a_typo_in_an_action_name_is_found(self):
        problems = self.problems(self.step("$/.github/actions/fetch-re1x", repository="r", path="p"))
        self.assertEqual(len(problems), 1)
        self.assertIn("no such file", problems[0])

    def test_a_typo_in_a_workflow_name_is_found(self):
        problems = self.problems({"uses": "$/.github/workflows/nothere.yml"})
        self.assertEqual(len(problems), 1)
        self.assertIn("no such file", problems[0])

    def test_an_input_the_target_does_not_declare_is_found(self):
        problems = self.problems(self.step("$/.github/actions/fetch-corpus", repository="r", path="p", repositry="x"))
        self.assertEqual(len(problems), 1)
        self.assertIn("repositry", problems[0])

    def test_an_input_the_target_requires_and_the_caller_leaves_out_is_found(self):
        problems = self.problems(self.step("$/.github/actions/fetch-corpus", repository="r"))
        self.assertEqual(len(problems), 1)
        self.assertIn("`path`", problems[0])

    def test_an_input_with_a_default_may_be_left_out(self):
        self.assertEqual(self.problems(self.step("$/.github/actions/fetch-corpus", repository="r", path="p")), [])

    def test_with_on_a_target_that_declares_no_inputs_is_found(self):
        files = {REUSABLE: {True: {"workflow_call": None}}}
        self.assertEqual(len(self.problems({"uses": "$/.github/workflows/viewer-test.yml", "with": {"legs": "[]"}}, files)), 1)

    def test_a_local_reference_in_a_shape_the_check_cannot_read_is_found(self):
        for uses in ("$/other", "$/.github/actions/with space", "./scripts/x.sh", "$/.github/workflows/x.txt", "$/.github/actions/a/b"):
            self.assertEqual(len(self.problems(self.step(uses))), 1, uses)

    def test_other_actions_are_not_this_checks_business(self):
        for uses in ("actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1", "docker://alpine:3", "owner/repo/.github/workflows/x.yml@v1"):
            self.assertEqual(self.problems(self.step(uses)), [], uses)

    def test_a_step_with_no_uses_or_with_does_not_crash(self):
        self.assertEqual(self.problems({"steps": [{"run": "true"}, {"uses": None}]}), [])

    def test_the_action_may_be_action_yaml(self):
        files = {".github/actions/fetch-corpus/action.yaml": FILES[ACTION]}
        self.assertEqual(self.problems(self.step("$/.github/actions/fetch-corpus", repository="r", path="p"), files), [])


STATEMENTS = {
    "README.md": "Rust 2024 edition (MSRV 1.87). Build: stable Rust 1.87 or newer.",
    "CONTRIBUTING.md": "You need stable Rust 1.87 or newer (`rustup` installs it).",
    ".cursor/install.sh": "# (needs Rust >= 1.87, the MSRV)\n# edition 2024 with rust-version = 1.87;",
    "AGENTS.md": "- **Rust must be stable ≥ 1.87.** (`rust-version = \"1.87\"` in `Cargo.toml`).",
    "docs/python.md": "# Needs a Rust toolchain (>= 1.87) and maturin.\n- Rust ≥ 1.87",
}


class ProseMsrv(unittest.TestCase):
    def problems(self):
        return cw.prose_msrv_problems("1.87", dict(STATEMENTS))

    def test_agreement_passes(self):
        self.assertEqual(self.problems(), [])

    def test_each_way_of_stating_it_is_read(self):
        for path, wrong in (
            ("README.md", "Rust 2024 edition (MSRV 1.86)."),
            ("CONTRIBUTING.md", "You need stable Rust 1.86 or newer."),
            (".cursor/install.sh", "# (needs Rust >= 1.86, the MSRV)"),
            ("AGENTS.md", "- **Rust must be stable ≥ 1.86.**"),
            ("AGENTS.md", "(`rust-version = \"1.86\"` in `Cargo.toml`)."),
            ("docs/python.md", "# Needs a Rust toolchain (>= 1.86) and maturin."),
        ):
            texts = dict(STATEMENTS)
            texts[path] = wrong
            problems = cw.prose_msrv_problems("1.87", texts)
            self.assertEqual(len(problems), 1, (path, wrong))
            self.assertIn("1.86", problems[0])

    def test_one_stale_statement_among_current_ones_is_found(self):
        texts = dict(STATEMENTS)
        texts["README.md"] += " Build: stable Rust 1.86 or newer."
        self.assertEqual(len(cw.prose_msrv_problems("1.87", texts)), 1)

    def test_a_file_that_states_nothing_it_can_read_fails(self):
        texts = dict(STATEMENTS)
        texts["README.md"] = "Needs a recent compiler."
        problems = cw.prose_msrv_problems("1.87", texts)
        self.assertEqual(len(problems), 1)
        self.assertIn("states no MSRV", problems[0])

    def test_a_file_that_is_gone_fails(self):
        texts = {k: v for k, v in STATEMENTS.items() if k != "AGENTS.md"}
        self.assertEqual(len(cw.prose_msrv_problems("1.87", texts)), 1)

    def test_another_release_that_is_not_the_msrv_is_ignored(self):
        texts = dict(STATEMENTS)
        texts["AGENTS.md"] += " The base image shipped Rust 1.83, which cannot build it. The server needs Rust 1.88, one above."
        self.assertEqual(cw.prose_msrv_problems("1.87", texts), [])

    def test_the_msrv_moving_alone_fails_every_file(self):
        self.assertEqual(len(cw.prose_msrv_problems("1.88", dict(STATEMENTS))), len(cw.PROSE_MSRV_FILES))


if __name__ == "__main__":
    unittest.main()
