"""Self-test for queued_pr.py. Run: python3 -m unittest discover -s .github/ci"""
import unittest

from queued_pr import queued_pr


class Reading(unittest.TestCase):
    def test_a_queue_ref_names_its_pull_request(self):
        self.assertEqual(queued_pr("refs/heads/gh-readonly-queue/main/pr-4039-c934b2c577e12cfc7d4edc06228d3a1b2c3d4e5f"), "4039")

    def test_a_rebuilt_group_names_the_same_pull_request(self):
        first = queued_pr("refs/heads/gh-readonly-queue/main/pr-4041-28db3e39de7929d9a5ecdd67b9f57aaaaaaaaaaa")
        rebuilt = queued_pr("refs/heads/gh-readonly-queue/main/pr-4041-c934b2c577e12cfc7d4edc06228d3aaaaaaaaaaa")
        self.assertEqual(first, rebuilt)

    def test_a_ref_without_the_prefix_still_reads(self):
        self.assertEqual(queued_pr("gh-readonly-queue/main/pr-12-abc123"), "12")

    def test_a_base_branch_with_a_slash_still_reads(self):
        self.assertEqual(queued_pr("refs/heads/gh-readonly-queue/release/v2/pr-7-abc123"), "7")

    def test_anything_else_names_none(self):
        for ref in ("", "refs/heads/main", "refs/pull/4039/merge", "refs/heads/armada/pr-4039-abc"):
            self.assertEqual(queued_pr(ref), "", ref)


if __name__ == "__main__":
    unittest.main()
