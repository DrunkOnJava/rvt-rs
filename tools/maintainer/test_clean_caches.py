#!/usr/bin/env python3
"""Failure-first tests of clean_caches.py (#1457): what must never be deleted, and when to stop.

Never deleted: the newest generation of a family (the next build restores it), another family's
caches, a live branch's own caches. Never gone on with: an empty open-pull-request list (exit 0: a
repository can have none) or a branch list without `main` (exit 1: it cannot be true), because
GitHub answers an empty body now and then and every live cache would read as dead; a permission
403 read as a rate limit.
"""
import unittest

import clean_caches as cc

MAIN = "refs/heads/main"


def cache(key, n, ref=MAIN):
    return {"id": n, "key": key, "ref": ref, "last_accessed_at": f"2026-11-{n:02d}T07:00:00Z", "size_in_bytes": 1000}


def rust(job, n, ref=MAIN):
    return cache(f"v0-rust-{job}-Linux-x64-cdee7d24-{n:08x}", n, ref)


def doomed_keys(caches, keep=2, live=(), prs=()):
    return sorted(c["key"] for c, _ in cc.unusable(caches, set(prs), set(live), keep))


class Generations(unittest.TestCase):
    def test_only_the_generations_past_the_newest_are_deleted(self):
        caches = [rust("ci-release", n) for n in range(1, 5)]
        self.assertEqual(len(doomed_keys(caches, keep=2)), 2)
        self.assertEqual(len(doomed_keys(caches, keep=1)), 3)

    def test_the_newest_is_found_by_when_it_was_used_not_by_its_hash(self):
        caches = [cache("v0-rust-x-Linux-x64-cdee7d24-ffffffff", 1), cache("v0-rust-x-Linux-x64-cdee7d24-00000001", 2)]
        self.assertEqual(doomed_keys(caches, keep=1), ["v0-rust-x-Linux-x64-cdee7d24-ffffffff"])

    def test_each_family_keeps_its_own_newest(self):
        caches = [rust("a", 1), rust("b", 2), rust("a", 3)]
        self.assertEqual(doomed_keys(caches, keep=1), [rust("a", 1)["key"]])

    def test_a_key_of_no_family_is_left_alone(self):
        caches = [cache(k, n) for n, k in enumerate(["playwright-Linux-1.64.1", "re1-oracle-9e8b37", "fuzz-corpus-fuzz_a-1", "sccache/a/b/c/abc"], 1)]
        self.assertEqual(doomed_keys(caches, keep=1), [])

    def test_a_live_branchs_caches_are_kept_and_a_deleted_branchs_are_not(self):
        caches = [rust("a", 1, "refs/heads/live"), rust("a", 2, "refs/heads/gone")]
        self.assertEqual(doomed_keys(caches, keep=1, live=["live"]), [rust("a", 2)["key"]])

    def test_an_open_pull_requests_caches_are_kept_and_a_closed_ones_are_not(self):
        caches = [rust("a", 1, "refs/pull/10/merge"), rust("a", 2, "refs/pull/11/merge")]
        self.assertEqual(doomed_keys(caches, prs=[10]), [rust("a", 2)["key"]])

    def test_the_reason_names_the_family(self):
        caches = [rust("a", n) for n in (1, 2, 3)]
        ((_, why),) = cc.unusable(caches, set(), set(), 2)
        self.assertEqual(why, "superseded generation of v0-rust-a-Linux-x64-cdee7d24")


HEX40 = "9e8b37175a0fba1397f7011aeee917fe52de7623"
HEX64 = HEX40 + "0123456789abcdef01234567"


class Re1(unittest.TestCase):
    def test_every_shape_of_the_key_is_one_family(self):
        caches = [cache(f"re1-{HEX40}", 1), cache(f"re1-oracle-{HEX40}", 2), cache(f"re1-{HEX64}", 3)]
        self.assertEqual(doomed_keys(caches, keep=2), [f"re1-{HEX40}"])

    def test_the_newest_generation_stays(self):
        caches = [cache(f"re1-{HEX64[:-1]}{c}", n) for n, c in enumerate("abc", 1)]
        self.assertEqual(doomed_keys(caches, keep=1), [caches[0]["key"], caches[1]["key"]])

    def test_a_key_that_only_looks_like_it_is_left_alone(self):
        keys = ["re1-", "re1-x", f"re1-{HEX40[:-1]}", f"re1-{HEX64}f", f"xre1-{HEX40}", f"re1-models-{HEX40}"]
        caches = [cache(k, n) for n, k in enumerate(keys, 1)]
        self.assertEqual(doomed_keys(caches, keep=0), [])

    def test_a_pull_requests_key_is_judged_by_its_pull_request_not_as_a_generation(self):
        caches = [cache(f"re1-{HEX40}", 1, "refs/pull/10/merge"), cache(f"re1-{HEX64}", 2, "refs/pull/10/merge")]
        self.assertEqual(doomed_keys(caches, keep=0, prs=[10]), [])


class Refusal(unittest.TestCase):
    def test_an_empty_open_pull_request_list_deletes_nothing_and_is_not_a_failure(self):
        status, why = cc.refusal(set(), {"main", "x"})
        self.assertEqual(status, 0)
        self.assertIn("pull request", why)

    def test_a_branch_list_without_main_is_a_failure(self):
        self.assertEqual(cc.refusal({1}, {"x"})[0], 1)
        self.assertEqual(cc.refusal({1}, set())[0], 1)

    def test_the_branch_list_is_judged_first_when_both_are_wrong(self):
        self.assertEqual(cc.refusal(set(), set())[0], 1)

    def test_both_lists_as_the_repository_has_them_do_not_refuse(self):
        self.assertIsNone(cc.refusal({1459}, {"main", "lane/1457-ci-gates"}))


class RateLimit(unittest.TestCase):
    def test_a_permission_403_is_not_a_rate_limit(self):
        self.assertFalse(cc.is_rate_limited("gh: Resource not accessible by integration (HTTP 403)"))

    def test_the_rate_limits_are(self):
        for text in ("gh: API rate limit exceeded for installation ID 1 (HTTP 403)", "HTTP 429",
                     "You have exceeded a secondary rate limit (HTTP 403)", "abuse detection mechanism (HTTP 403)"):
            self.assertTrue(cc.is_rate_limited(text), text)

    def test_a_server_error_is_neither(self):
        self.assertFalse(cc.is_rate_limited("gh: Server Error (HTTP 502)"))


if __name__ == "__main__":
    unittest.main()
