#!/usr/bin/env python3
"""Failure-first tests of the pure judgement in release_preflight.py: what must never let a tag publish."""
import json
import unittest

import release_preflight as p

V = "0.16.0"
COMMIT = "a" * 40
TAG_SHA = "b" * 40


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


def tree(v=V):
    return {
        "Cargo.toml": manifest("rvt", v),
        "rvt-py/Cargo.toml": manifest("rvt-py", v, f"={v}"),
        "rvt-mcp/Cargo.toml": manifest("rvt-rs-mcp", v, f"={v}"),
        "rvt-wasm/Cargo.toml": manifest("rvt-wasm", v),
        "tools/stream_evidence/Cargo.toml": manifest("stream-evidence", "0.1.0", f"={v}"),
        "viewer/package.json": json.dumps({"name": "viewer", "version": v}),
        "CITATION.cff": f'cff-version: 1.2.0\ntitle: "x"\nversion: {v}\ndate-released: "2026-11-05"\n',
        "Cargo.lock": lock(
            ("rvt", v, None), ("rvt-py", v, None), ("rvt-rs-mcp", v, None), ("rvt-wasm", v, None),
            ("stream-evidence", "0.1.0", None), ("serde", "1.0.1", "registry+https://github.com/rust-lang/crates.io-index"),
        ),
        "fuzz/Cargo.lock": lock(("rvt", v, None), ("rvt-fuzz", "0.0.0", None)),
        "CHANGELOG.md": f"# Changelog\n\n## [Unreleased]\n\n## [{v}] — 2026-11-06\n\n## [0.15.0] — 2026-11-05\n",
        "pyproject.toml": '[project]\nname = "rvt"\ndynamic = ["version"]\n\n[tool.maturin]\nmanifest-path = "rvt-py/Cargo.toml"\n',
    }


PATHS = {
    "mcp_toml": "rvt-mcp/Cargo.toml",
    "py_toml": "rvt-py/Cargo.toml",
    "wasm_toml": "rvt-wasm/Cargo.toml",
    "package_json": "viewer/package.json",
    "root_lock": "Cargo.lock",
    "fuzz_lock": "fuzz/Cargo.lock",
    "citation": "CITATION.cff",
    "changelog": "CHANGELOG.md",
    "pyproject": "pyproject.toml",
}


def with_(files, **changes):
    out = dict(files)
    for key, value in changes.items():
        out[PATHS[key]] = value
    return out


def mentions(problems, *words):
    return any(all(w in line for w in words) for line in problems)


class TagName(unittest.TestCase):
    def test_a_name_that_is_not_a_release_version_has_none(self):
        for name in ("0.16.0", "V0.16.0", "v0.16", "v0.16.0.1", "v01.2.3", "v1.02.3", "v1.2.3\n", "v1.2.3 ",
                     " v1.2.3", "v1.2.3+build", "v1.2.3-", "v1.2.3-01", "v1.2.3-rc..1", "vx", "v", "", "latest",
                     "v1.2.3\nv1.2.4", "v١.٢.٣"):
            self.assertIsNone(p.tag_version(name), repr(name))

    def test_a_branch_ref_is_not_a_tag(self):
        self.assertIsNone(p.tag_version("refs/heads/v0.16.0"))

    def test_a_release_and_a_prerelease_have_their_version(self):
        self.assertEqual(p.tag_version("v0.16.0"), "0.16.0")
        self.assertEqual(p.tag_version("refs/tags/v0.16.0"), "0.16.0")
        self.assertEqual(p.tag_version("v0.3.0-rc1"), "0.3.0-rc1")
        self.assertEqual(p.tag_version("v10.20.30"), "10.20.30")


class Versions(unittest.TestCase):
    def check(self, files, expected=V):
        return p.check_versions(files, expected)

    def test_a_consistent_bump_is_clean(self):
        self.assertEqual(self.check(tree()), (V, []))

    def test_every_file_a_tag_disagrees_with_is_named(self):
        version, problems = self.check(tree("0.15.0"), V)
        for label in ("Cargo.toml", "rvt-py/Cargo.toml", "rvt-mcp/Cargo.toml", "rvt-wasm/Cargo.toml",
                      "viewer/package.json", "CITATION.cff", "Cargo.lock", "fuzz/Cargo.lock"):
            self.assertTrue(mentions(problems, label, "0.15.0", V), label)
        self.assertTrue(mentions(problems, "CHANGELOG.md", V))

    def test_a_package_one_patch_behind_is_named(self):
        for path, label in (("viewer/package.json", "viewer/package.json"),
                            ("rvt-wasm/Cargo.toml", "rvt-wasm/Cargo.toml")):
            files = dict(tree())
            files[path] = files[path].replace(V, "0.15.9")
            self.assertTrue(mentions(self.check(files)[1], label, "0.15.9"), path)

    def test_the_mcp_servers_pin_on_rvt_lagging_is_named(self):
        files = with_(tree(), mcp_toml=manifest("rvt-rs-mcp", V, "=0.15.0"))
        self.assertTrue(mentions(self.check(files)[1], "rvt-mcp/Cargo.toml", "pin", "0.15.0"))

    def test_a_pin_that_is_not_exact_is_named(self):
        for pin in (V, f"^{V}", f">={V}", f"~{V}", ""):
            files = with_(tree(), py_toml=manifest("rvt-py", V, pin))
            self.assertTrue(mentions(self.check(files)[1], "rvt-py/Cargo.toml", "exact"), repr(pin))

    def test_a_missing_pin_is_named(self):
        files = with_(tree(), mcp_toml=manifest("rvt-rs-mcp", V))
        self.assertTrue(mentions(self.check(files)[1], "rvt-mcp/Cargo.toml", "pin"))

    def test_a_manifest_for_another_package_is_named(self):
        files = with_(tree(), mcp_toml=manifest("rvt-mcp", V, f"={V}"))
        self.assertTrue(mentions(self.check(files)[1], "rvt-mcp/Cargo.toml", "rvt-rs-mcp"))

    def test_a_workspace_inherited_version_is_not_a_literal_one(self):
        text = '[package]\nname = "rvt-wasm"\nversion.workspace = true\n'
        self.assertTrue(mentions(self.check(with_(tree(), wasm_toml=text))[1], "rvt-wasm/Cargo.toml"))

    def test_a_stale_lock_entry_is_named(self):
        stale = lock(("rvt", V, None), ("rvt-py", V, None), ("rvt-rs-mcp", V, None), ("rvt-wasm", "0.15.0", None))
        self.assertTrue(mentions(self.check(with_(tree(), root_lock=stale))[1], "Cargo.lock", "rvt-wasm", "0.15.0"))

    def test_a_lock_without_a_workspace_crate_is_named(self):
        short = lock(("rvt", V, None), ("rvt-py", V, None), ("rvt-wasm", V, None))
        self.assertTrue(mentions(self.check(with_(tree(), root_lock=short))[1], "Cargo.lock", "rvt-rs-mcp"))

    def test_a_registry_copy_of_a_crate_is_not_the_workspace_one(self):
        crates = "registry+https://github.com/rust-lang/crates-io-index"
        both = lock(("rvt", "0.1.0", crates), ("rvt", V, None), ("rvt-py", V, None), ("rvt-rs-mcp", V, None),
                    ("rvt-wasm", V, None))
        self.assertEqual(self.check(with_(tree(), root_lock=both)), (V, []))

    def test_two_workspace_entries_of_one_name_are_ambiguous(self):
        twice = lock(("rvt", V, None), ("rvt", "0.15.0", None), ("rvt-py", V, None), ("rvt-rs-mcp", V, None),
                     ("rvt-wasm", V, None))
        self.assertTrue(mentions(self.check(with_(tree(), root_lock=twice))[1], "Cargo.lock", "rvt"))

    def test_the_fuzz_lock_being_stale_is_named(self):
        files = with_(tree(), fuzz_lock=lock(("rvt", "0.15.0", None)))
        self.assertTrue(mentions(self.check(files)[1], "fuzz/Cargo.lock", "0.15.0"))

    def test_the_citation_version_off_is_named_and_quotes_are_read(self):
        old = tree()["CITATION.cff"].replace(f"version: {V}", "version: 0.15.0")
        self.assertTrue(mentions(self.check(with_(tree(), citation=old))[1], "CITATION.cff", "0.15.0"))
        quoted = tree()["CITATION.cff"].replace(f"version: {V}", f'version: "{V}"')
        self.assertEqual(self.check(with_(tree(), citation=quoted)), (V, []))

    def test_an_indented_version_line_is_not_the_citations_version(self):
        text = f'cff-version: 1.2.0\nreferences:\n  - version: 9.9.9\nversion: {V}\n'
        self.assertEqual(self.check(with_(tree(), citation=text)), (V, []))

    def test_two_top_level_citation_versions_are_ambiguous(self):
        text = f"version: {V}\nversion: 0.15.0\n"
        self.assertTrue(mentions(self.check(with_(tree(), citation=text))[1], "CITATION.cff"))

    def test_a_changelog_without_the_versions_section_is_named(self):
        for text in ("# Changelog\n\n## [Unreleased]\n\n## [0.15.0] — x\n", f"# x\n\nmentions [{V}] in prose\n",
                     f"### [{V}]\n", f"## [{V}x]\n"):
            self.assertTrue(mentions(self.check(with_(tree(), changelog=text))[1], "CHANGELOG.md"), text)

    def test_a_prerelease_needs_its_own_section(self):
        files = tree("0.3.0-rc1")
        self.assertEqual(self.check(files, "0.3.0-rc1"), ("0.3.0-rc1", []))
        files["CHANGELOG.md"] = "## [0.3.0] — x\n"
        self.assertTrue(mentions(self.check(files, "0.3.0-rc1")[1], "CHANGELOG.md"))

    def test_an_unreadable_file_is_named_and_never_skipped(self):
        for path in tree():
            files = dict(tree())
            files[path] = None
            self.assertTrue(mentions(self.check(files)[1], path), path)

    def test_a_missing_file_is_named_and_never_skipped(self):
        files = dict(tree())
        del files["viewer/package.json"]
        self.assertTrue(mentions(self.check(files)[1], "viewer/package.json"))

    def test_malformed_files_are_named_and_never_crash(self):
        for path in ("Cargo.toml", "rvt-py/Cargo.toml", "Cargo.lock", "fuzz/Cargo.lock", "pyproject.toml"):
            self.assertTrue(mentions(self.check({**tree(), path: "[[[ nope"})[1], path), path)
        for text in ("{not json", "[]", '{"version": 16}', "{}", "null"):
            self.assertTrue(mentions(self.check(with_(tree(), package_json=text))[1], "viewer/package.json"), text)

    def test_a_manifest_whose_tables_are_not_tables_never_crashes(self):
        for text in ('package = "x"\n', '[package]\nname = "rvt-py"\nversion = "0.16.0"\n[dependencies]\nrvt = "=0.16.0"\n',
                     '[package]\nname = "rvt-py"\nversion = "0.16.0"\ndependencies = 3\n'):
            self.assertTrue(self.check(with_(tree(), py_toml=text))[1], text)

    def test_the_wheels_version_must_come_from_the_file_checked(self):
        static = '[project]\nname = "rvt"\nversion = "0.15.0"\ndynamic = ["version"]\n[tool.maturin]\nmanifest-path = "rvt-py/Cargo.toml"\n'
        no_dynamic = '[project]\nname = "rvt"\nversion = "0.16.0"\n[tool.maturin]\nmanifest-path = "rvt-py/Cargo.toml"\n'
        elsewhere = '[project]\nname = "rvt"\ndynamic = ["version"]\n[tool.maturin]\nmanifest-path = "Cargo.toml"\n'
        unset = '[project]\nname = "rvt"\ndynamic = ["version"]\n'
        for text in (static, no_dynamic, elsewhere, unset):
            self.assertTrue(mentions(self.check(with_(tree(), pyproject=text))[1], "pyproject.toml"), text)

    def test_problems_are_all_reported_at_once(self):
        files = with_(tree(), package_json=json.dumps({"version": "0.15.0"}), citation="version: 0.15.0\n")
        problems = self.check(files)[1]
        self.assertTrue(mentions(problems, "viewer/package.json"))
        self.assertTrue(mentions(problems, "CITATION.cff"))


class NoTag(unittest.TestCase):
    def test_files_that_agree_with_each_other_pass_and_name_the_version(self):
        self.assertEqual(p.check_versions(tree(), None), (V, []))

    def test_files_that_disagree_with_each_other_fail_naming_both_versions(self):
        files = with_(tree(), package_json=json.dumps({"version": "0.15.0"}))
        version, problems = p.check_versions(files, None)
        self.assertIsNone(version)
        self.assertTrue(mentions(problems, "disagree", "0.15.0", V, "viewer/package.json"))

    def test_a_missing_file_still_fails_without_a_tag(self):
        files = dict(tree())
        files["Cargo.lock"] = None
        self.assertTrue(mentions(p.check_versions(files, None)[1], "Cargo.lock"))


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


class SignedTag(unittest.TestCase):
    def problems(self, ref_json, tag_json, name="v0.16.0", commit=COMMIT):
        return p.tag_problems(ref_json, tag_json, name, commit)

    def test_a_signed_annotated_tag_on_the_pushed_commit_passes(self):
        self.assertEqual(self.problems(ref(), tag_object()), [])

    def test_a_tag_the_api_does_not_know_is_refused(self):
        self.assertTrue(mentions(self.problems(None, None), "v0.16.0", "not found"))

    def test_a_lightweight_tag_is_refused(self):
        self.assertTrue(mentions(self.problems(ref(COMMIT, "commit"), None), "lightweight"))

    def test_an_annotated_tag_the_api_cannot_read_is_refused(self):
        self.assertTrue(self.problems(ref(), None))

    def test_an_unverified_signature_is_refused_with_its_reason(self):
        for reason in ("unsigned", "unknown_key", "expired_key", "bad_email", "malformed_signature", "invalid"):
            tag = tag_object(verification={"verified": False, "reason": reason})
            self.assertTrue(mentions(self.problems(ref(), tag), "not verified", reason), reason)

    def test_verified_must_be_the_boolean_true(self):
        for verified in ("true", 1, None, "yes", [True]):
            tag = tag_object(verification={"verified": verified, "reason": "valid"})
            self.assertTrue(self.problems(ref(), tag), repr(verified))

    def test_a_verification_that_is_missing_is_refused(self):
        tag = tag_object()
        del tag["verification"]
        self.assertTrue(mentions(self.problems(ref(), tag), "verification"))
        self.assertTrue(self.problems(ref(), tag_object(verification=None)))
        self.assertTrue(self.problems(ref(), tag_object(verification="valid")))

    def test_a_verified_tag_whose_reason_is_not_valid_is_refused(self):
        self.assertTrue(self.problems(ref(), tag_object(verification={"verified": True, "reason": "unknown_key"})))
        self.assertTrue(self.problems(ref(), tag_object(verification={"verified": True})))

    def test_a_tag_object_that_names_another_tag_is_refused(self):
        self.assertTrue(mentions(self.problems(ref(), tag_object(tag="v0.15.0")), "v0.15.0", "v0.16.0"))

    def test_a_tag_of_a_tag_or_a_tree_is_refused(self):
        for kind in ("tag", "tree", "blob"):
            tag = tag_object(object={"sha": COMMIT, "type": kind})
            self.assertTrue(mentions(self.problems(ref(), tag), kind), kind)

    def test_a_tag_on_another_commit_than_the_pushed_one_is_refused(self):
        tag = tag_object(object={"sha": "c" * 40, "type": "commit"})
        self.assertTrue(mentions(self.problems(ref(), tag), "c" * 12, COMMIT[:12]))

    def test_a_tag_object_the_ref_does_not_point_at_is_refused(self):
        self.assertTrue(self.problems(ref("d" * 40), tag_object()))

    def test_replies_of_the_wrong_shape_are_refused_and_never_crash(self):
        for bad in ("x", [], 3, {"object": "x"}, {"object": []}, {"object": {"type": 5}}):
            self.assertTrue(self.problems(bad, tag_object()), repr(bad))
            self.assertTrue(self.problems(ref(), bad), repr(bad))

    def test_only_a_github_side_hiccup_is_worth_asking_again(self):
        for reason in ("gpgverify_unavailable", "gpgverify_error", "ocsp_pending"):
            self.assertTrue(p.verification_is_transient(tag_object(verification={"verified": False, "reason": reason})))
        for reason in ("unsigned", "unknown_key", "valid", "invalid"):
            self.assertFalse(p.verification_is_transient(tag_object(verification={"verified": False, "reason": reason})))
        self.assertFalse(p.verification_is_transient(None))
        self.assertFalse(p.verification_is_transient({"verification": "x"}))


class Branch(unittest.TestCase):
    def test_a_commit_on_no_branch_is_refused(self):
        self.assertTrue(mentions(p.branch_problems({"origin/main": False}), "main", "hotfix"))

    def test_a_commit_only_on_some_other_branch_is_refused(self):
        refs = {"origin/main": False, "origin/feature/x": True, "origin/lane/1454-gates": True, "origin/release": True}
        self.assertTrue(p.branch_problems(refs))

    def test_look_alike_branch_names_are_not_main_or_hotfix(self):
        for name in ("origin/main-old", "origin/mainline", "origin/hotfix", "origin/hotfix/", "origin/hotfixes/v1",
                     "origin/x/hotfix/v1", "upstream/main", "main", "origin/main/x"):
            self.assertTrue(p.branch_problems({"origin/main": False, name: True}), name)

    def test_a_missing_origin_main_means_the_fetch_was_incomplete(self):
        self.assertTrue(mentions(p.branch_problems({"origin/hotfix/v0.15.1": True}), "origin/main", "fetch"))
        self.assertTrue(mentions(p.branch_problems({}), "origin/main", "fetch"))

    def test_main_passes(self):
        self.assertEqual(p.branch_problems({"origin/main": True}), [])

    def test_a_hotfix_branch_passes_even_when_main_does_not_contain_it(self):
        self.assertEqual(p.branch_problems({"origin/main": False, "origin/hotfix/v0.15.1": True}), [])


class CrateState(unittest.TestCase):
    def state(self, status, body, version="0.16.0"):
        return p.crate_state(status, body, "rvt", version)

    def body(self, num="0.16.0", yanked=False):
        return json.dumps({"version": {"num": num, "yanked": yanked}})

    def test_not_found_means_publish(self):
        self.assertEqual(self.state(404, "{}")[0], "publish")

    def test_a_version_already_there_is_skipped(self):
        self.assertEqual(self.state(200, self.body())[0], "skip")

    def test_a_yanked_version_is_refused_never_skipped(self):
        state, message = self.state(200, self.body(yanked=True))
        self.assertEqual(state, "error")
        self.assertIn("yanked", message)

    def test_an_answer_for_another_version_is_refused(self):
        self.assertEqual(self.state(200, self.body("0.15.0"))[0], "error")

    def test_an_unreadable_answer_is_refused_never_read_as_absent(self):
        for body in ("", "not json", "[]", "{}", '{"version": null}', '{"version": {"num": 3}}', '{"version": "x"}'):
            self.assertEqual(self.state(200, body)[0], "error", body)

    def test_any_other_status_is_refused_even_with_a_body_that_says_published(self):
        for status in (0, 201, 301, 400, 401, 403, 410, 429, 500, 502, 503):
            self.assertEqual(self.state(status, self.body())[0], "error", status)
            self.assertEqual(self.state(status, "{}")[0], "error", status)

    def test_a_yanked_flag_that_is_not_false_is_refused(self):
        for yanked in (None, "false", 0, "no"):
            self.assertEqual(self.state(200, self.body(yanked=yanked))[0], "error", repr(yanked))
        self.assertEqual(self.state(200, json.dumps({"version": {"num": "0.16.0"}}))[0], "error")


if __name__ == "__main__":
    unittest.main()
