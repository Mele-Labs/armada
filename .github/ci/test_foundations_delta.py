"""Self-test for foundations_delta.py. Run: python3 -m unittest discover -s .github/ci"""
import unittest

from foundations_delta import a_report, delta, failing_lines, new_lines

BASE = "ok    a\nFAIL  no_file_too_long\n        missing: crates/a.rs:10 is 1300 lines, over 1200\n        warn:    x\n"


class Reading(unittest.TestCase):
    def test_a_warning_is_outside_the_comparison(self):
        self.assertEqual(new_lines(BASE, BASE + "        warn:    y\n"), [])

    def test_a_renumbered_old_failure_is_not_new(self):
        moved = BASE.replace("a.rs:10", "a.rs:14")
        self.assertEqual(new_lines(BASE, moved), [])

    def test_a_second_violation_of_one_rule_in_one_file_is_new(self):
        twice = BASE + "        missing: crates/a.rs:90 is 1300 lines, over 1200\n"
        self.assertEqual(len(new_lines(BASE, twice)), 1)

    def test_a_different_file_is_new(self):
        other = BASE + "        missing: crates/b.rs:1 is 1300 lines, over 1200\n"
        self.assertEqual(len(new_lines(BASE, other)), 1)

    def test_a_fixed_failure_is_not_red(self):
        self.assertEqual(new_lines(BASE, "ok    a\n"), [])

    def test_a_nonzero_exit_naming_no_rule_is_red(self):
        red, text = delta(BASE, "error[E0433]: failed to resolve\n", 101)
        self.assertTrue(red)
        self.assertIn("E0433", text)
        self.assertFalse(a_report("error[E0433]\n", 101))

    def test_green_with_a_known_failure_is_not_red(self):
        red, text = delta(BASE, BASE, 1)
        self.assertFalse(red)
        self.assertIn("already", text)

    def test_only_fail_and_missing_lines_are_read(self):
        self.assertEqual(len(failing_lines(BASE)), 2)


if __name__ == "__main__":
    unittest.main()
