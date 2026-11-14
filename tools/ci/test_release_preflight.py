#!/usr/bin/env python3
"""Failure-first tests of the pure judgement in release_preflight.py: what must never let a tag publish.

Each case asserts the exact problems, not that some line mentions a file, so a
comparison the code drops (one source out of the fourteen) fails a test."""
import json
import unittest

import release_preflight as p

V = "0.16.0"
STALE = "0.15.9"
COMMIT = "a" * 40
TAG_SHA = "b" * 40
CRATES = "registry+https://github.com/rust-lang/crates.io-index"

# the label release_preflight.py gives each version it compares, and the tree() keyword that sets it
SOURCES = {
    "Cargo.toml version": "root",
    "rvt-py/Cargo.toml version": "py",
    "rvt-py/Cargo.toml pin on rvt": "py_pin",
    "rvt-mcp/Cargo.toml version": "mcp",
    "rvt-mcp/Cargo.toml pin on rvt": "mcp_pin",
    "rvt-wasm/Cargo.toml version": "wasm",
    "tools/stream_evidence/Cargo.toml pin on rvt": "evidence_pin",
    "Cargo.lock entry for rvt": "lock_rvt",
    "Cargo.lock entry for rvt-py": "lock_py",
    "Cargo.lock entry for rvt-rs-mcp": "lock_mcp",
    "Cargo.lock entry for rvt-wasm": "lock_wasm",
    "fuzz/Cargo.lock entry for rvt": "fuzz_lock",
    "viewer/package.json version": "package",
    "CITATION.cff version": "citation",
}


def manifest(name, version, pin=None):
    text = f'[package]\nname = "{name}"\nversion = "{version}"\n'
    if pin is not None:
        text += f'\n[dependencies]\nrvt = {{ path = "..", version = "{pin}" }}\nserde = "1"\n'
    return text


def lock(*packages):
    out = "version = 4\n"
    for name, version, source in packages:
        out += f'\n[[package]]\nname = "{name}"\nversion = "{version}"\n'
        if source:
            out += f'source = "{source}"\n'
    return out


def tree(changelog=V, **versions):
    def at(key):
        return versions.get(key, V)

    return {
        "Cargo.toml": manifest("rvt", at("root")),
        "rvt-py/Cargo.toml": manifest("rvt-py", at("py"), f"={at('py_pin')}"),
        "rvt-mcp/Cargo.toml": manifest("rvt-rs-mcp", at("mcp"), f"={at('mcp_pin')}"),
        "rvt-wasm/Cargo.toml": manifest("rvt-wasm", at("wasm")),
        "tools/stream_evidence/Cargo.toml": manifest("stream-evidence", "0.1.0", f"={at('evidence_pin')}"),
        "viewer/package.json": json.dumps({"name": "viewer", "version": at("package")}),
        "CITATION.cff": f'cff-version: 1.2.0\ntitle: "x"\nversion: {at("citation")}\ndate-released: "2026-11-05"\n',
        "Cargo.lock": lock(
            ("rvt", at("lock_rvt"), None), ("rvt-py", at("lock_py"), None),
            ("rvt-rs-mcp", at("lock_mcp"), None), ("rvt-wasm", at("lock_wasm"), None),
            ("stream-evidence", "0.1.0", None), ("serde", "1.0.1", CRATES),
        ),
        "fuzz/Cargo.lock": lock(("rvt", at("fuzz_lock"), None), ("rvt-fuzz", "0.0.0", None)),
        "CHANGELOG.md": f"# Changelog\n\n## [Unreleased]\n\n## [{changelog}] — 2026-11-06\n\n## [0.15.0] — 2026-11-05\n",
        "pyproject.toml": '[project]\nname = "rvt"\ndynamic = ["version"]\n\n[tool.maturin]\nmanifest-path = "rvt-py/Cargo.toml"\n',
    }


def at_version(version):
    return tree(changelog=version, **{key: version for key in SOURCES.values()})


class TagName(unittest.TestCase):
    def test_a_name_that_is_not_a_release_version_has_none(self):
        for name in ("0.16.0", "V0.16.0", "v0.16", "v0.16.0.1", "v01.2.3", "v1.02.3", "v1.2.3\n", "v1.2.3 ",
                     " v1.2.3", "v1.2.3+build", "v1.2.3-", "v1.2.3-01", "v1.2.3-rc..1", "vx", "v", "", "latest",
                     "v1.2.3\nv1.2.4", "v١.٢.٣", "refs/heads/v0.16.0"):
            self.assertIsNone(p.tag_version(name), repr(name))

    def test_a_release_and_a_prerelease_have_their_version(self):
        for name, version in (("v0.16.0", "0.16.0"), ("refs/tags/v0.16.0", "0.16.0"),
                              ("v0.3.0-rc1", "0.3.0-rc1"), ("v10.20.30", "10.20.30")):
            self.assertEqual(p.tag_version(name), version)


class Versions(unittest.TestCase):
    def test_a_consistent_bump_is_clean(self):
        self.assertEqual(p.check_versions(tree(), V), (V, []))
        self.assertEqual(p.check_versions(at_version("0.3.0-rc1"), "0.3.0-rc1"), ("0.3.0-rc1", []))

    def test_each_of_the_fourteen_sources_alone_being_behind_is_named_exactly(self):
        for label, key in SOURCES.items():
            self.assertEqual(p.check_versions(tree(**{key: STALE}), V),
                             (V, [f"the tag says {V}, but {STALE} is in {label}"]), label)

    def test_a_tree_of_the_previous_release_names_all_fourteen_and_the_changelog(self):
        _, problems = p.check_versions(at_version("0.15.0"), V)
        self.assertEqual(problems, [f"the tag says {V}, but 0.15.0 is in {'; '.join(SOURCES)}",
                                    f"CHANGELOG.md has no '## [{V}]' section"])

    def test_two_different_stale_versions_are_two_problems(self):
        _, problems = p.check_versions(tree(package=STALE, citation="0.14.0"), V)
        self.assertEqual(problems, [f"the tag says {V}, but 0.14.0 is in CITATION.cff version",
                                    f"the tag says {V}, but {STALE} is in viewer/package.json version"])

    def test_a_changelog_without_the_versions_section_is_named(self):
        for text in ("# Changelog\n\n## [Unreleased]\n\n## [0.15.0] — x\n", f"# x\n\nmentions [{V}] in prose\n",
                     f"### [{V}]\n", f"## [{V}x]\n", ""):
            self.assertEqual(p.check_versions({**tree(), "CHANGELOG.md": text}, V)[1],
                             [f"CHANGELOG.md has no '## [{V}]' section"], repr(text))
        files = {**at_version("0.3.0-rc1"), "CHANGELOG.md": "## [0.3.0] — x\n"}
        self.assertEqual(p.check_versions(files, "0.3.0-rc1")[1], ["CHANGELOG.md has no '## [0.3.0-rc1]' section"])

    def test_an_unreadable_or_missing_file_is_named_and_never_skipped(self):
        for path in tree():
            self.assertEqual(p.check_versions({**tree(), path: None}, V)[1], [f"{path} could not be read"], path)
        files = tree()
        del files["viewer/package.json"]
        self.assertEqual(p.check_versions(files, V)[1], ["viewer/package.json could not be read"])

    def test_malformed_files_are_named_and_never_crash(self):
        for path in ("Cargo.toml", "rvt-py/Cargo.toml", "Cargo.lock", "fuzz/Cargo.lock", "pyproject.toml"):
            problems = p.check_versions({**tree(), path: "[[[ nope"}, V)[1]
            self.assertEqual(len(problems), 1, path)
            self.assertTrue(problems[0].startswith(f"{path} is not valid TOML"), problems)
        problems = p.check_versions({**tree(), "viewer/package.json": "{not json"}, V)[1]
        self.assertEqual(len(problems), 1)
        self.assertTrue(problems[0].startswith("viewer/package.json is not valid JSON"), problems)
        for text in ("[]", '{"version": 16}', "{}", "null"):
            self.assertEqual(p.check_versions({**tree(), "viewer/package.json": text}, V)[1],
                             ["viewer/package.json has no version"], text)

    def test_a_manifest_that_is_not_the_one_expected_is_named_exactly(self):
        mcp, py, wasm = "rvt-mcp/Cargo.toml", "rvt-py/Cargo.toml", "rvt-wasm/Cargo.toml"
        cases = (
            (mcp, manifest("rvt-mcp", V, f"={V}"), f"{mcp} is the manifest of 'rvt-mcp', not rvt-rs-mcp"),
            (mcp, manifest("rvt-rs-mcp", V), f"{mcp} has no version pin on rvt"),
            (wasm, '[package]\nname = "rvt-wasm"\nversion.workspace = true\n', f"{wasm} has no literal package version"),
            (py, 'package = "x"\n', f"{py} is the manifest of None, not rvt-py"),
            (py, f'[package]\nname = "rvt-py"\nversion = "{V}"\n[dependencies]\nrvt = "={V}"\n', f"{py} has no version pin on rvt"),
            (py, f'[package]\nname = "rvt-py"\nversion = "{V}"\ndependencies = 3\n', f"{py} has no version pin on rvt"),
        )
        for path, text, problem in cases:
            self.assertEqual(p.check_versions({**tree(), path: text}, V)[1], [problem], text)
        for pin in (V, f"^{V}", f">={V}", f"~{V}", ""):
            self.assertEqual(p.check_versions({**tree(), py: manifest("rvt-py", V, pin)}, V)[1],
                             [f"{py} pins rvt to {pin!r}, which is not an exact =X.Y.Z"], pin)

    def test_lock_entries_are_the_workspace_ones(self):
        short = lock(("rvt", V, None), ("rvt-py", V, None), ("rvt-wasm", V, None))
        self.assertEqual(p.check_versions({**tree(), "Cargo.lock": short}, V)[1],
                         ["Cargo.lock has 0 workspace entries for rvt-rs-mcp, not one"])
        twice = lock(("rvt", V, None), ("rvt", "0.15.0", None), ("rvt-py", V, None), ("rvt-rs-mcp", V, None),
                     ("rvt-wasm", V, None))
        self.assertEqual(p.check_versions({**tree(), "Cargo.lock": twice}, V)[1],
                         ["Cargo.lock has 2 workspace entries for rvt, not one"])
        registry_copy = lock(("rvt", "0.1.0", CRATES), ("rvt", V, None), ("rvt-py", V, None), ("rvt-rs-mcp", V, None),
                             ("rvt-wasm", V, None))
        self.assertEqual(p.check_versions({**tree(), "Cargo.lock": registry_copy}, V), (V, []))

    def test_only_a_top_level_citation_version_counts(self):
        quoted = tree()["CITATION.cff"].replace(f"version: {V}", f'version: "{V}"')
        nested = f"cff-version: 1.2.0\nreferences:\n  - version: 9.9.9\nversion: {V}\n"
        for text in (quoted, nested):
            self.assertEqual(p.check_versions({**tree(), "CITATION.cff": text}, V), (V, []))
        self.assertEqual(p.check_versions({**tree(), "CITATION.cff": f"version: {V}\nversion: 0.15.0\n"}, V)[1],
                         ["CITATION.cff has 2 top-level version lines, not one"])

    def test_the_wheels_version_must_come_from_the_file_checked(self):
        dynamic_but_static = '[project]\nname = "rvt"\nversion = "0.15.0"\ndynamic = ["version"]\n[tool.maturin]\nmanifest-path = "rvt-py/Cargo.toml"\n'
        static_only = '[project]\nname = "rvt"\nversion = "0.16.0"\n[tool.maturin]\nmanifest-path = "rvt-py/Cargo.toml"\n'
        elsewhere = '[project]\nname = "rvt"\ndynamic = ["version"]\n[tool.maturin]\nmanifest-path = "Cargo.toml"\n'
        unset = '[project]\nname = "rvt"\ndynamic = ["version"]\n'
        dynamic = "pyproject.toml must leave the wheel's version dynamic, so rvt-py/Cargo.toml is its one source"
        for text, problem in ((dynamic_but_static, dynamic), (static_only, dynamic),
                              (elsewhere, "pyproject.toml builds the wheel from 'Cargo.toml', not rvt-py/Cargo.toml"),
                              (unset, "pyproject.toml builds the wheel from None, not rvt-py/Cargo.toml")):
            self.assertEqual(p.check_versions({**tree(), "pyproject.toml": text}, V)[1], [problem], text)

    def test_without_a_tag_the_files_must_agree_with_each_other(self):
        self.assertEqual(p.check_versions(tree(), None), (V, []))
        others = "; ".join(label for label in SOURCES if label != "viewer/package.json version")
        self.assertEqual(p.check_versions(tree(package=STALE), None),
                         (None, [f"the version files disagree. {STALE} is in viewer/package.json version. {V} is in {others}"]))
        self.assertEqual(p.check_versions({**tree(), "Cargo.lock": None}, None)[1], ["Cargo.lock could not be read"])


def ref(sha=TAG_SHA, kind="tag"):
    return {"ref": "refs/tags/v0.16.0", "object": {"sha": sha, "type": kind}}


def tag_object(**over):
    obj = {
        "sha": TAG_SHA,
        "tag": "v0.16.0",
        "object": {"sha": COMMIT, "type": "commit"},
        "verification": {"verified": True, "reason": "valid"},
    }
    obj.update(over)
    return obj


def verification(verified, reason="valid"):
    return {"verification": {"verified": verified, "reason": reason}}


class SignedTag(unittest.TestCase):
    def problems(self, ref_json, tag_json, name="v0.16.0", commit=COMMIT):
        return p.tag_problems(ref_json, tag_json, name, commit)

    def test_a_signed_annotated_tag_on_the_pushed_commit_passes(self):
        self.assertEqual(self.problems(ref(), tag_object()), [])

    def test_each_refusal_says_exactly_why(self):
        unsigned = "the tag's signature is not verified by GitHub (reason: {})"
        cases = [
            (None, None, ["tag v0.16.0 was not found on GitHub"]),
            (ref(COMMIT, "commit"), None,
             ["tag v0.16.0 is lightweight (it points straight at a commit); a release tag is annotated and signed: git tag -s"]),
            (ref(), None, [f"the tag object {TAG_SHA} of v0.16.0 could not be read"]),
            (ref("d" * 40), tag_object(), [f"the tag object {TAG_SHA} is not the one v0.16.0 points at ({'d' * 40})"]),
            (ref(), tag_object(tag="v0.15.0"), ["the tag object is for v0.15.0, not v0.16.0"]),
            (ref(), tag_object(object={"sha": "c" * 40, "type": "commit"}),
             [f"the tag points at {'c' * 12} but this run is for {COMMIT[:12]}"]),
            (ref(), tag_object(verification=None), ["GitHub gave no verification for the tag"]),
            (ref(), tag_object(verification="valid"), ["GitHub gave no verification for the tag"]),
            (ref(), tag_object(verification={"verified": True, "reason": "unknown_key"}),
             ["the tag's signature reason is unknown_key, not valid"]),
            (ref(), tag_object(verification={"verified": True}), ["the tag's signature reason is None, not valid"]),
        ]
        cases += [(ref(), tag_object(**verification(False, reason)), [unsigned.format(reason)])
                  for reason in ("unsigned", "unknown_key", "expired_key", "bad_email", "malformed_signature", "invalid")]
        cases += [(ref(), tag_object(**verification(flag)), [unsigned.format("valid")])
                  for flag in ("true", 1, None, "yes", [True])]
        cases += [(ref(), tag_object(object={"sha": COMMIT, "type": kind}), [f"the tag points at a {kind}, not a commit"])
                  for kind in ("tag", "tree", "blob")]
        for ref_json, tag_json, expected in cases:
            self.assertEqual(self.problems(ref_json, tag_json), expected, tag_json)

    def test_replies_of_the_wrong_shape_are_refused_and_never_crash(self):
        for bad in ("x", [], 3, {"object": "x"}, {"object": []}, {"object": {"type": 5}}):
            self.assertTrue(self.problems(bad, tag_object()), repr(bad))
            self.assertTrue(self.problems(ref(), bad), repr(bad))

    def test_only_a_github_side_hiccup_is_worth_asking_again(self):
        for reason in ("gpgverify_unavailable", "gpgverify_error", "ocsp_pending"):
            self.assertTrue(p.verification_is_transient(tag_object(**verification(False, reason))))
        for reason in ("unsigned", "unknown_key", "valid", "invalid"):
            self.assertFalse(p.verification_is_transient(tag_object(**verification(False, reason))))
        self.assertFalse(p.verification_is_transient(tag_object(**verification(True, "ocsp_pending"))))
        self.assertFalse(p.verification_is_transient(None))
        self.assertFalse(p.verification_is_transient({"verification": "x"}))


class Branch(unittest.TestCase):
    NONE = [f"the tagged commit is on neither origin/main nor origin/hotfix/v{V}"]
    UNFETCHED = ["origin/main is not in the checkout, so the fetch was incomplete (the job needs fetch-depth: 0)"]

    def test_a_commit_on_main_or_its_own_hotfix_branch_passes(self):
        self.assertEqual(p.branch_problems({"origin/main": True}, V), [])
        self.assertEqual(p.branch_problems({"origin/main": False, "origin/hotfix/v0.15.1": True}, "0.15.1"), [])

    def test_a_commit_on_no_release_branch_is_refused(self):
        self.assertEqual(p.branch_problems({"origin/main": False}, V), self.NONE)
        refs = {"origin/main": False, "origin/feature/x": True, "origin/lane/1454-gates": True, "origin/release": True}
        self.assertEqual(p.branch_problems(refs, V), self.NONE)

    def test_look_alike_branch_names_are_not_main_or_hotfix(self):
        for name in ("origin/main-old", "origin/mainline", "origin/hotfix", "origin/hotfix/", "origin/hotfixes/v1",
                     "origin/x/hotfix/v1", "upstream/main", "main", "origin/main/x"):
            self.assertEqual(p.branch_problems({"origin/main": False, name: True}, V), self.NONE, name)

    def test_only_the_hotfix_branch_named_for_the_version_makes_a_release_of_it(self):
        # RELEASE.md: the tag is accepted while `hotfix/vX.Y.(Z+1)` holds the commit, not any hotfix branch.
        for name in ("origin/hotfix/v0.16.1", "origin/hotfix/v0.15.9", "origin/hotfix/0.16.0", "origin/hotfix/v0.16.0-rc1",
                     "origin/hotfix/v0.16.0/x", "origin/hotfix/V0.16.0"):
            self.assertEqual(p.branch_problems({"origin/main": False, name: True}, V), self.NONE, name)
        self.assertEqual(p.branch_problems({"origin/main": False, f"origin/hotfix/v{V}": True}, V), [])
        # And a branch of that name that does not hold the commit is not enough.
        self.assertEqual(p.branch_problems({"origin/main": False, f"origin/hotfix/v{V}": False}, V), self.NONE)

    def test_a_missing_origin_main_means_the_fetch_was_incomplete(self):
        self.assertEqual(p.branch_problems({"origin/hotfix/v0.15.1": True}, "0.15.1"), self.UNFETCHED)
        self.assertEqual(p.branch_problems({}, V), self.UNFETCHED)


class RunKind(unittest.TestCase):
    """A tag decides whether the tag checks apply, not the event (#1886)."""

    def test_a_push_is_a_release_of_a_tag_and_says_so_when_it_is_another_ref(self):
        self.assertEqual(p.run_plan("push", "tag", "v0.16.0"), (True, []))
        self.assertEqual(p.run_plan("push", "branch", "main"),
                         (True, ["publish.yml releases tags; this run is for the branch main"]))

    def test_a_manual_run_on_a_tag_is_checked_as_that_release(self):
        # `gh workflow run publish.yml --ref v0.20.0` was passed as having no tag, with no signature check.
        self.assertEqual(p.run_plan("workflow_dispatch", "tag", "v0.20.0"), (True, []))

    def test_a_manual_run_on_a_branch_has_no_tag_to_check(self):
        self.assertEqual(p.run_plan("workflow_dispatch", "branch", "hotfix/v0.16.1"), (False, []))
        self.assertEqual(p.run_plan("workflow_dispatch", "branch", "checkout"), (False, []))


class CrateState(unittest.TestCase):
    def state(self, status, body, version="0.16.0"):
        return p.crate_state(status, body, "rvt", version)

    def body(self, num="0.16.0", yanked=False):
        return json.dumps({"version": {"num": num, "yanked": yanked}})

    def test_absent_means_publish_and_present_means_skip(self):
        self.assertEqual(self.state(404, "{}")[0], "publish")
        self.assertEqual(self.state(200, self.body())[0], "skip")

    def test_a_yanked_version_is_refused_never_skipped(self):
        state, message = self.state(200, self.body(yanked=True))
        self.assertEqual(state, "error")
        self.assertIn("yanked", message)
        for yanked in (None, "false", 0, "no"):
            self.assertEqual(self.state(200, self.body(yanked=yanked))[0], "error", repr(yanked))
        self.assertEqual(self.state(200, json.dumps({"version": {"num": "0.16.0"}}))[0], "error")

    def test_an_answer_that_is_not_about_this_version_is_refused_never_read_as_absent(self):
        self.assertEqual(self.state(200, self.body("0.15.0"))[0], "error")
        for body in ("", "not json", "[]", "{}", '{"version": null}', '{"version": {"num": 3}}', '{"version": "x"}'):
            self.assertEqual(self.state(200, body)[0], "error", body)

    def test_any_other_status_is_refused_even_with_a_body_that_says_published(self):
        for status in (0, 201, 301, 400, 401, 403, 410, 429, 500, 502, 503):
            self.assertEqual(self.state(status, self.body())[0], "error", status)
            self.assertEqual(self.state(status, "{}")[0], "error", status)


if __name__ == "__main__":
    unittest.main()
