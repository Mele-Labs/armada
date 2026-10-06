"""Self-test for plan.py. Run: python3 -m unittest discover -s .github/ci"""
import unittest

from plan import plan

KEYS = [
    "build",
    "typecheck",
    "apps/desktop:typecheck",
    "apps/desktop:bridge_build",
    "apps/desktop:desktop_test",
    "apps/desktop:xtask_test",
    "packages/screens:screens_test",
    "packages/components:storybook",
    "packages/surfaces/studios:test",
]


class Planning(unittest.TestCase):
    def test_a_root_name_keeps_its_static_job(self):
        self.assertEqual(plan(["build", "format"])["checks"], ["build", "format"])

    def test_a_workspace_key_goes_to_the_matrix_with_its_key(self):
        keys = [row["key"] for row in plan(KEYS)["matrix"]["include"]]
        self.assertIn("packages/surfaces/studios:test", keys)
        self.assertIn("apps/desktop:bridge_build", keys)

    def test_the_desktop_tests_are_sharded_and_not_in_the_matrix(self):
        answer = plan(KEYS)
        self.assertIn("apps/desktop:desktop_test", answer["checks"])
        self.assertNotIn("apps/desktop:desktop_test", [row["key"] for row in answer["matrix"]["include"]])

    def test_the_xtask_tests_have_their_own_rust_job_and_no_matrix_entry(self):
        answer = plan(KEYS)
        self.assertIn("apps/desktop:xtask_test", answer["checks"])
        self.assertNotIn("apps/desktop:xtask_test", [row["key"] for row in answer["matrix"]["include"]])
        self.assertEqual(answer["excluded"], [])
        self.assertEqual(answer["unrun"], [])

    def test_an_excluded_key_is_named_and_not_run(self):
        import plan as module
        module.EXCLUDED["packages/x:skip"] = "test"
        try:
            answer = plan(["packages/x:skip"])
        finally:
            del module.EXCLUDED["packages/x:skip"]
        self.assertEqual(answer["excluded"], ["packages/x:skip"])
        self.assertEqual(answer["checks"], [])

    def test_a_key_nothing_runs_is_reported(self):
        self.assertEqual(plan(["packages/x:lint", "mystery"])["unrun"], ["packages/x:lint", "mystery"])

    def test_a_workspace_name_is_not_a_root_name(self):
        self.assertEqual(plan(["packages/x:build"])["unrun"], ["packages/x:build"])

    def test_browsers_only_where_a_browser_opens(self):
        rows = {r["key"]: r["browsers"] for r in plan(KEYS)["matrix"]["include"]}
        self.assertEqual(rows["apps/desktop:typecheck"], "false")
        self.assertEqual(rows["packages/components:storybook"], "true")


if __name__ == "__main__":
    unittest.main()
