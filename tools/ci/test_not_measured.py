#!/usr/bin/env python3
"""Failure-first tests of the pure parts of not_measured.py and open_issue.py.

What must never open an issue (a single warning, a streak a measured run broke, a run in which the
job did not succeed read as one that measured) and what must never be missed (a streak that a run
with a failed job in the middle appears to break, a job that timed out), then the label and title
an issue gets.
"""
import unittest
from datetime import datetime, timedelta, timezone

import advisory_check as ac
import not_measured as nm
import open_issue as oi

T0 = datetime(2026, 11, 5, 12, 0, tzinfo=timezone.utc)
CORPUS = "Corpus health (tier 2 — external)"
AUDIT = "viewer dependency audit"
PII = "PII guard"
BIM = "BIM4LCA models"
R25 = "R2025 models"
CITED = "cited commits"


def run(n, hours, **jobs):
    """Run n, `hours` before T0; each keyword is a job (corpus, audit or pii) and the subjects it warned."""
    names = {"corpus": CORPUS, "audit": AUDIT, "pii": PII}
    return {"id": n, "at": T0 - timedelta(hours=hours), "jobs": {names[k]: set(v) for k, v in jobs.items()}}


def only(found, subject):
    return [s for s in found if s["subject"] == subject]


class Streaks(unittest.TestCase):
    def test_a_newest_run_that_measured_has_no_streak(self):
        runs = [run(3, 0, corpus=[]), run(2, 12, corpus=[BIM]), run(1, 30, corpus=[BIM])]
        self.assertEqual(nm.streaks(runs), [])

    def test_three_runs_over_a_day_persist(self):
        runs = [run(3, 0, corpus=[BIM]), run(2, 12, corpus=[BIM]), run(1, 25, corpus=[BIM])]
        (s,) = nm.streaks(runs)
        self.assertTrue(s["persistent"])
        self.assertEqual((s["runs"], s["newest"], s["oldest"]), (3, 3, 1))

    def test_two_runs_do_not_however_long_they_span(self):
        runs = [run(2, 0, corpus=[BIM]), run(1, 100, corpus=[BIM])]
        self.assertFalse(nm.streaks(runs)[0]["persistent"])

    def test_three_runs_inside_a_day_do_not(self):
        runs = [run(3, 0, corpus=[BIM]), run(2, 10, corpus=[BIM]), run(1, 23, corpus=[BIM])]
        self.assertFalse(nm.streaks(runs)[0]["persistent"])

    def test_a_run_that_measured_in_the_middle_ends_the_streak(self):
        runs = [run(5, 0, corpus=[BIM]), run(4, 5, corpus=[BIM]), run(3, 10, corpus=[]), run(2, 40, corpus=[BIM]), run(1, 60, corpus=[BIM])]
        (s,) = nm.streaks(runs)
        self.assertEqual(s["runs"], 2)
        self.assertFalse(s["persistent"])
        self.assertFalse(s["open"])

    def test_a_run_whose_job_failed_neither_ends_nor_counts(self):
        runs = [run(4, 0, corpus=[BIM]), run(3, 9), run(2, 13, corpus=[BIM]), run(1, 26, corpus=[BIM])]
        (s,) = nm.streaks(runs)
        self.assertEqual(s["runs"], 3)
        self.assertTrue(s["persistent"])

    def test_the_newest_run_with_its_job_failed_starts_at_the_next(self):
        runs = [run(4, 0), run(3, 9, corpus=[BIM]), run(2, 20, corpus=[BIM]), run(1, 34, corpus=[BIM])]
        (s,) = nm.streaks(runs)
        self.assertEqual((s["newest"], s["runs"]), (3, 3))
        self.assertTrue(s["persistent"])

    def test_another_jobs_clean_run_does_not_end_a_streak(self):
        runs = [run(3, 0, corpus=[BIM]), run(2, 12, audit=[]), run(1, 30, corpus=[BIM])]
        (s,) = nm.streaks(runs)
        self.assertEqual(s["runs"], 2)

    def test_subjects_are_judged_apart(self):
        runs = [run(3, 0, corpus=[BIM, R25]), run(2, 12, corpus=[BIM]), run(1, 30, corpus=[BIM, R25])]
        self.assertTrue(only(nm.streaks(runs), BIM)[0]["persistent"])
        self.assertFalse(only(nm.streaks(runs), R25)[0]["persistent"])

    def test_a_job_nobody_listed_is_read_like_any_other(self):
        runs = [run(3, 0, pii=[CITED]), run(2, 12, pii=[CITED]), run(1, 25, pii=[CITED])]
        (s,) = nm.streaks(runs)
        self.assertEqual((s["job"], s["subject"], s["persistent"]), (PII, CITED, True))

    def test_the_order_runs_come_in_does_not_matter(self):
        runs = [run(1, 30, corpus=[BIM]), run(3, 0, corpus=[BIM]), run(2, 12, corpus=[BIM])]
        self.assertTrue(nm.streaks(runs)[0]["persistent"])

    def test_no_runs_no_streaks(self):
        self.assertEqual(nm.streaks([]), [])

    def test_a_streak_to_the_oldest_run_read_is_open(self):
        runs = [run(2, 0, corpus=[BIM]), run(1, 5, corpus=[BIM])]
        self.assertTrue(nm.streaks(runs)[0]["open"])


class Settled(unittest.TestCase):
    def settled(self, runs):
        return nm.settled(runs, nm.streaks(runs))

    def test_not_before_the_minimum_runs_are_read(self):
        self.assertFalse(self.settled([run(1, 0, corpus=[])]))
        self.assertFalse(self.settled([run(2, 0, corpus=[]), run(1, 2, corpus=[])]))

    def test_a_warned_job_that_failed_in_the_newest_runs_is_read_on(self):
        self.assertFalse(self.settled([run(3, 0), run(2, 1), run(1, 2, corpus=[BIM])]))

    def test_not_while_a_streak_may_still_continue_into_an_older_run(self):
        runs = [run(3, 0, corpus=[BIM]), run(2, 2, corpus=[BIM]), run(1, 3, corpus=[BIM])]
        self.assertFalse(self.settled(runs))

    def test_once_enough_runs_measured_everything(self):
        self.assertTrue(self.settled([run(3, 0, corpus=[]), run(2, 1, corpus=[]), run(1, 2, corpus=[])]))

    def test_once_a_streak_is_persistent_or_ended(self):
        runs = [run(3, 0, corpus=[BIM]), run(2, 12, corpus=[BIM]), run(1, 25, corpus=[BIM])]
        self.assertTrue(self.settled(runs))
        runs = [run(3, 0, corpus=[BIM]), run(2, 2, corpus=[BIM]), run(1, 3, corpus=[])]
        self.assertTrue(self.settled(runs))


class Messages(unittest.TestCase):
    def test_the_warnings_ci_writes(self):
        self.assertEqual(nm.subject_of("BIM4LCA models NOT MEASURED: the host was unreachable, so the targets skip"), BIM)
        self.assertEqual(nm.subject_of("viewer dependency audit NOT MEASURED: the advisory endpoint was unreachable"), AUDIT)
        self.assertEqual(nm.subject_of("cited commits NOT MEASURED: 2 of 40 could not be looked up"), CITED)
        self.assertEqual(nm.subject_of("folded-pieces.spec.ts NOT MEASURED: the host of the BIM4LCA office was unreachable"), "folded-pieces.spec.ts")

    def test_text_that_is_not_such_a_warning(self):
        for text in ("", "all measured", "not measured", "NOT MEASURED: no subject", "BIM4LCA models were measured"):
            self.assertIsNone(nm.subject_of(text), text)


class Reportable(unittest.TestCase):
    def names(self, jobs, exclude=()):
        return [j["name"] for j in oi.reportable(jobs, exclude)]

    def job(self, name, conclusion):
        return {"name": name, "conclusion": conclusion}

    def test_failed_timed_out_and_cancelled_jobs_are_reported(self):
        jobs = [self.job("a", "failure"), self.job("b", "timed_out"), self.job("c", "cancelled")]
        self.assertEqual(self.names(jobs), ["a", "b", "c"])

    def test_a_job_that_passed_or_did_not_run_is_not(self):
        jobs = [self.job(n, c) for n, c in (("a", "success"), ("b", "skipped"), ("c", "neutral"), ("d", None), ("e", "action_required"))]
        self.assertEqual(self.names(jobs), [])

    def test_an_excluded_name_is_not(self):
        jobs = [self.job("a", "failure"), self.job("b", "failure")]
        self.assertEqual(self.names(jobs, ["b"]), ["a"])

    def test_a_job_with_no_conclusion_key_does_not_crash(self):
        self.assertEqual(self.names([{"name": "a"}]), [])


class Issues(unittest.TestCase):
    def test_a_label_is_prefixed_short_and_not_empty(self):
        for name in ("Audit fuzz", "Tier 2 — wider project corpus", "x" * 200, "—", ""):
            label = oi.PREFIX + oi.slug(name)
            self.assertLessEqual(len(label), oi.LABEL_LIMIT, name)
            self.assertFalse(label.endswith("-"), name)
            self.assertGreater(len(label), len(oi.PREFIX), name)

    def test_a_slug_cut_at_its_limit_never_ends_on_a_hyphen(self):
        self.assertEqual(oi.slug("a" * 39 + " b"), "a" * 39)

    def test_a_slug_joins_words_with_one_hyphen(self):
        self.assertEqual(oi.slug("Tier 2 — wider project corpus"), "tier-2-wider-project-corpus")
        self.assertEqual(oi.slug("Audit (fuzz)"), "audit-fuzz")

    def test_a_failed_job_gets_one_label_a_title_and_the_run(self):
        job = {"name": "Audit fuzz", "conclusion": "failure", "html_url": "https://example.test/job/1", "steps": ["Audit fuzz/Cargo.lock"]}
        issue = oi.job_issue(job, "Scheduled checks", "o/r", 42)
        self.assertEqual(issue["labels"][0], "scheduled:audit-fuzz")
        self.assertEqual(issue["title"], "Scheduled check failed: Audit fuzz")
        self.assertIn("https://github.com/o/r/actions/runs/42", issue["body"])
        self.assertIn("Audit fuzz/Cargo.lock", issue["body"])
        self.assertNotIn("timeout", issue["body"])

    def test_a_cancelled_job_says_it_hit_its_timeout_and_names_the_step_that_was_running(self):
        job = {"name": "Tier 2", "conclusion": "cancelled", "html_url": "https://example.test/job/2", "steps": ["Tier-two open"]}
        issue = oi.job_issue(job, "Corpus nightly", "o/r", 7)
        self.assertEqual(issue["title"], "Scheduled check did not finish: Tier 2")
        self.assertIn("timeout-minutes", issue["body"])
        self.assertIn("Tier-two open", issue["body"])
        self.assertEqual(issue["labels"][0], "scheduled:tier-2")

    def test_a_job_with_no_recorded_step_still_says_so(self):
        issue = oi.job_issue({"name": "x", "conclusion": "failure", "steps": []}, "w", "o/r", 1)
        self.assertIn("none recorded", issue["body"])

    def test_the_not_measured_issue_names_the_streak(self):
        streak = {"job": CORPUS, "subject": BIM, "runs": 5, "hours": 30.4, "newest": 2, "oldest": 1, "workflow": "ci.yml"}
        issue = nm.issue_for(streak, "o/r", {2: "https://example.test/2", 1: "https://example.test/1"})
        self.assertEqual(issue["labels"][0], "scheduled:not-measured-bim4lca-models")
        self.assertIn("30 hours", issue["title"])
        self.assertIn("https://example.test/1", issue["body"])
        self.assertIn("ci.yml", issue["body"])


class Advisories(unittest.TestCase):
    def test_a_pass_is_a_pass_whatever_the_output_says(self):
        self.assertEqual(ac.classify(0, "warning: failed to download a crate, retried"), "pass")

    def test_the_fetch_failures_the_tools_print_are_not_findings(self):
        for text in (
            "error: couldn't fetch advisory database: git operation failed: An IO error occurred when talking to the server",
            "2026-11-05 19:41:41 [ERROR] failed to fetch advisory database https://github.com/RustSec/advisory-db with cli: failed to clone",
            "warning: spurious network error (3 tries remaining): [6] Could not resolve host: index.crates.io",
            "error: unable to update registry `crates-io`",
            "curl: (6) Could not resolve host: github.com",
        ):
            self.assertEqual(ac.classify(1, text), "fetch", text)

    def test_an_advisory_is_a_failure(self):
        for text in (
            "error: 1 vulnerability found!",
            "Crate: time\nVersion: 0.1.43\nID: RUSTSEC-2020-0071\nerror: 2 vulnerabilities found!",
            "error[vulnerability]: Potential segfault in the time crate",
            "error[yanked]: detected yanked crate (try `cargo update -p foo`)",
        ):
            self.assertEqual(ac.classify(1, text), "fail", text)

    def test_an_advisory_is_a_failure_even_beside_a_fetch_message(self):
        text = "spurious network error: could not resolve host\nID: RUSTSEC-2026-0001\nerror: 1 vulnerability found!"
        self.assertEqual(ac.classify(1, text), "fail")

    def test_any_other_failure_is_a_failure(self):
        for text in ("", "error: couldn't load Cargo.lock: parse error", "error: unexpected argument '--config' found"):
            self.assertEqual(ac.classify(2, text), "fail", text)


if __name__ == "__main__":
    unittest.main()
