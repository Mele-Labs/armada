"""Self-test for plan.py. Run: python3 -m unittest discover -s .github/ci"""
import unittest

from plan import plan, rerun_keys

KEYS = [
    "build",
    "typecheck",
    "apps/desktop:typecheck",
    "apps/desktop:bridge_build",
    "apps/desktop/unit:desktop_test",
    "apps/desktop:xtask_test",
    "packages/screens:screens_test",
    "packages/components:storybook",
    "packages/surfaces/studios:test",
]


class Planning(unittest.TestCase):
    def test_a_root_name_keeps_its_static_job(self):
        self.assertEqual(plan(["build", "typecheck"])["checks"], ["build", "typecheck"])

    def test_a_workspace_key_goes_to_the_matrix_with_its_key(self):
        keys = [row["key"] for row in plan(KEYS)["matrix"]["include"]]
        self.assertIn("packages/surfaces/studios:test", keys)
        self.assertIn("apps/desktop:bridge_build", keys)

    def test_the_desktop_tests_that_read_no_surface_have_their_own_job_and_no_matrix_entry(self):
        answer = plan(KEYS)
        self.assertIn("apps/desktop/unit:desktop_test", answer["checks"])
        self.assertNotIn("apps/desktop/unit:desktop_test", [row["key"] for row in answer["matrix"]["include"]])

    def test_the_old_desktop_key_is_reported_not_skipped(self):
        self.assertEqual(plan(["apps/desktop:desktop_test"])["unrun"], ["apps/desktop:desktop_test"])

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

    def test_a_quick_plan_runs_what_compiles_and_names_the_rest_for_the_queue(self):
        answer = plan(KEYS + ["test"], quick=True)
        self.assertEqual(answer["checks"], ["build", "typecheck", "apps/desktop:typecheck", "apps/desktop:bridge_build"])
        self.assertEqual([row["key"] for row in answer["matrix"]["include"]], ["apps/desktop:typecheck", "apps/desktop:bridge_build"])
        self.assertIn("test", answer["queued"])
        self.assertIn("apps/desktop/unit:desktop_test", answer["queued"])

    def test_a_quick_plan_still_reports_a_key_nothing_runs(self):
        self.assertEqual(plan(["packages/x:lint"], quick=True)["unrun"], ["packages/x:lint"])

    def test_a_quick_plan_keeps_what_the_queue_last_failed(self):
        failed = rerun_keys(["test 2/4", "desktop_unit", "packages/screens:screens_test", "desktop_test", "ci"])
        answer = plan(KEYS + ["test", "acceptance"], quick=True, rerun=failed)
        for key in ["test", "apps/desktop/unit:desktop_test", "packages/screens:screens_test"]:
            self.assertIn(key, answer["checks"])
            self.assertNotIn(key, answer["queued"])
        self.assertEqual(answer["rerun"], ["apps/desktop/unit:desktop_test", "packages/screens:screens_test", "test"])
        self.assertIn("packages/screens:screens_test", [row["key"] for row in answer["matrix"]["include"]])
        self.assertIn("acceptance", answer["queued"])

    def test_a_failed_check_the_change_does_not_cover_is_not_added(self):
        answer = plan(["build"], quick=True, rerun=rerun_keys(["test 1/4"]))
        self.assertEqual(answer["checks"], ["build"])
        self.assertEqual(answer["rerun"], [])

    def test_a_quick_plan_compiles_the_rust_tests_it_leaves_to_the_queue(self):
        answer = plan(KEYS + ["test", "acceptance"], quick=True)
        self.assertEqual(answer["compile"], ["test", "acceptance"])

    def test_a_rust_check_the_pull_request_runs_is_not_compiled_again(self):
        answer = plan(["build", "test", "acceptance"], quick=True, rerun=rerun_keys(["test 3/4"]))
        self.assertEqual(answer["compile"], ["acceptance"])

    def test_a_whole_plan_compiles_nothing_on_its_own(self):
        self.assertEqual(plan(KEYS + ["test"])["compile"], [])

    def test_a_shard_and_the_unit_job_name_their_check(self):
        self.assertEqual(rerun_keys(["test 4/4", "desktop_unit", "apps/desktop:xtask_test"]),
                         {"test", "apps/desktop/unit:desktop_test", "apps/desktop:xtask_test"})


if __name__ == "__main__":
    unittest.main()
