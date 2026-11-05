#!/usr/bin/env python3
"""Failure-first tests of clean_caches.unusable for Fuzz's corpus family (#1457).

What must never be deleted: the newest generation of a target (the next night restores it),
another target's caches, a branch's own caches, and a key that only looks like a corpus cache.
What must be: the older generations of a target on `main`, however many targets there are.
"""
import unittest

import clean_caches as cc

MAIN = "refs/heads/main"


def cache(key, n, ref=MAIN):
    return {"id": n, "key": key, "ref": ref, "last_accessed_at": f"2026-11-{n:02d}T07:00:00Z", "size_in_bytes": 1000}


def doomed_keys(caches, keep=2, live=(), prs=()):
    return sorted(c["key"] for c, _ in cc.unusable(caches, set(prs), set(live), keep))


class FuzzCorpus(unittest.TestCase):
    def test_only_the_generations_past_the_newest_are_deleted(self):
        caches = [cache(f"fuzz-corpus-fuzz_open_bytes-{100 + n}", n) for n in range(1, 5)]
        self.assertEqual(doomed_keys(caches, keep=2), ["fuzz-corpus-fuzz_open_bytes-101", "fuzz-corpus-fuzz_open_bytes-102"])
        self.assertEqual(len(doomed_keys(caches, keep=1)), 3)

    def test_the_newest_is_found_by_when_it_was_used_not_by_its_run_id(self):
        caches = [cache("fuzz-corpus-fuzz_a-900", 1), cache("fuzz-corpus-fuzz_a-100", 2)]
        self.assertEqual(doomed_keys(caches, keep=1), ["fuzz-corpus-fuzz_a-900"])

    def test_each_target_keeps_its_own_newest(self):
        caches = [cache("fuzz-corpus-fuzz_a-1", 1), cache("fuzz-corpus-fuzz_b-2", 2), cache("fuzz-corpus-fuzz_a-3", 3)]
        self.assertEqual(doomed_keys(caches, keep=1), ["fuzz-corpus-fuzz_a-1"])

    def test_a_target_whose_name_starts_another_is_its_own_family(self):
        caches = [cache("fuzz-corpus-fuzz_open-1", 1), cache("fuzz-corpus-fuzz_open_bytes-2", 2)]
        self.assertEqual(doomed_keys(caches, keep=1), [])

    def test_a_key_that_only_looks_like_a_corpus_is_left_alone(self):
        keys = ["fuzz-corpus-fuzz_a", "fuzz-corpus-fuzz_a-", "fuzz-corpus-fuzz_a-x1", "fuzz-corpus-Fuzz_A-1", "fuzz-corpus--1"]
        caches = [cache(k, n) for n, k in enumerate(keys, 1)] + [cache("fuzz-corpus-fuzz_a-1", 9)]
        self.assertEqual(doomed_keys(caches, keep=1), [])

    def test_a_live_branchs_corpus_is_kept_and_a_deleted_branchs_is_not(self):
        caches = [cache("fuzz-corpus-fuzz_a-1", 1, "refs/heads/live"), cache("fuzz-corpus-fuzz_a-2", 2, "refs/heads/gone")]
        self.assertEqual(doomed_keys(caches, keep=1, live=["live"]), ["fuzz-corpus-fuzz_a-2"])

    def test_the_reason_names_the_family(self):
        caches = [cache(f"fuzz-corpus-fuzz_a-{n}", n) for n in (1, 2, 3)]
        (_, why), = cc.unusable(caches, set(), set(), 2)
        self.assertEqual(why, "superseded generation of fuzz-corpus-fuzz_a")

    def test_the_other_families_are_still_judged(self):
        caches = [cache(f"v0-rust-ci-release-Linux-x64-cdee7d24-{h}", n) for n, h in enumerate(("11111111", "22222222", "33333333"), 1)]
        self.assertEqual(len(doomed_keys(caches, keep=2)), 1)


if __name__ == "__main__":
    unittest.main()
