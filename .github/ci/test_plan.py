"""Self-test for plan.py. Run: python3 -m unittest discover -s .github/ci"""
import unittest

from plan import GROUPS, plan, rerun_keys

KEYS = [
    "build",
    "typecheck",
    "xtask_test",
    "apps/pocket:typecheck",
    "apps/pocket:pocket_test",
    "apps/store:typecheck",
    "apps/store:store_test",
    "apps/store:bundle_current",
]


def grouped(answer):
    return {row["group"]: row["keys"].split() for row in answer["matrix"]["include"]}


class Planning(unittest.TestCase):
    def test_a_root_name_keeps_its_static_job(self):
        self.assertEqual(plan(["build", "typecheck"])["checks"], ["build", "typecheck"])

    def test_a_workspace_key_goes_to_the_node_group(self):
        groups = grouped(plan(KEYS))
        self.assertEqual(
            groups["node checks"],
            ["apps/pocket:typecheck", "apps/pocket:pocket_test", "apps/store:typecheck", "apps/store:store_test", "apps/store:bundle_current"],
        )
        self.assertEqual(list(groups), ["node checks"])

    def test_every_workspace_key_is_in_exactly_one_group(self):
        answer = plan(KEYS)
        every = [key for keys in grouped(answer).values() for key in keys]
        self.assertEqual(len(every), len(set(every)))
        self.assertEqual(set(answer["checks"]) - set(every), {"build", "typecheck", "xtask_test"})

    def test_a_group_with_no_key_is_not_in_the_matrix(self):
        self.assertEqual(grouped(plan(["apps/pocket:pocket_test"])), {"node checks": ["apps/pocket:pocket_test"]})
        self.assertEqual(plan(["build"])["matrix"]["include"], [])

    def test_a_removed_workspace_check_is_reported_not_skipped(self):
        self.assertEqual(plan(["apps/desktop:desktop_test"])["unrun"], ["apps/desktop:desktop_test"])

    def test_the_xtask_tests_have_their_own_rust_job_and_no_matrix_entry(self):
        answer = plan(KEYS)
        self.assertIn("xtask_test", answer["checks"])
        self.assertNotIn("xtask_test", [key for keys in grouped(answer).values() for key in keys])
        self.assertEqual(answer["excluded"], [])
        self.assertEqual(answer["unrun"], [])

    def test_the_native_bridge_test_is_its_own_output_and_no_linux_job(self):
        answer = plan(["crates/bridge-native:test"])
        self.assertTrue(answer["native"])
        self.assertEqual(answer["checks"], [])
        self.assertEqual(answer["excluded"], [])
        self.assertEqual(answer["unrun"], [])

    def test_the_native_bridge_test_survives_quick_and_rerun_which_drop_the_rest(self):
        answer = plan(["test", "crates/bridge-native:test"], quick=True)
        self.assertTrue(answer["native"])
        self.assertEqual(answer["queued"], ["test"])

    def test_no_native_output_when_the_change_misses_the_crate(self):
        self.assertFalse(plan(["build", "apps/pocket:typecheck"])["native"])

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

    def test_a_quick_plan_runs_what_compiles_and_names_the_rest_for_the_queue(self):
        answer = plan(KEYS + ["test"], quick=True)
        self.assertEqual(answer["checks"], ["build", "typecheck", "apps/pocket:typecheck", "apps/store:typecheck"])
        self.assertEqual(grouped(answer), {"node checks": ["apps/pocket:typecheck", "apps/store:typecheck"]})
        self.assertIn("test", answer["queued"])
        self.assertIn("xtask_test", answer["queued"])

    def test_a_quick_plan_still_reports_a_key_nothing_runs(self):
        self.assertEqual(plan(["packages/x:lint"], quick=True)["unrun"], ["packages/x:lint"])

    def test_a_quick_plan_keeps_what_the_queue_last_failed(self):
        failed = rerun_keys(["test 2/4", "xtask_test", "apps/pocket:pocket_test", "ci"])
        answer = plan(KEYS + ["test", "acceptance"], quick=True, rerun=failed)
        for key in ["test", "xtask_test", "apps/pocket:pocket_test"]:
            self.assertIn(key, answer["checks"])
            self.assertNotIn(key, answer["queued"])
        self.assertEqual(answer["rerun"], ["xtask_test", "apps/pocket:pocket_test", "test"])
        self.assertIn("apps/pocket:pocket_test", [key for keys in grouped(answer).values() for key in keys])
        self.assertIn("acceptance", answer["queued"])

    def test_a_failed_group_reruns_the_keys_of_that_group_the_change_names(self):
        failed = rerun_keys(["node checks", "ci"])
        self.assertEqual(failed, {"node checks", "ci"})
        answer = plan(KEYS, quick=True, rerun=failed)
        named = [key for keys in grouped(plan(KEYS)).values() for key in keys]
        self.assertCountEqual(answer["rerun"], named)
        self.assertEqual(set(grouped(answer)), {"node checks"})

    def test_a_failed_group_adds_no_key_the_change_does_not_name(self):
        answer = plan(["apps/pocket:typecheck"], quick=True, rerun=rerun_keys(["node checks"]))
        self.assertEqual(answer["rerun"], ["apps/pocket:typecheck"])
        self.assertEqual(grouped(answer), {"node checks": ["apps/pocket:typecheck"]})
        self.assertEqual(plan(["build"], quick=True, rerun=rerun_keys(["node checks"]))["rerun"], [])

    def test_a_group_name_is_not_read_as_a_shard(self):
        self.assertTrue(all(name in rerun_keys([name]) for name in GROUPS))

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

    def test_a_shard_names_its_check(self):
        self.assertEqual(rerun_keys(["test 4/4", "xtask_test"]), {"test", "xtask_test"})


if __name__ == "__main__":
    unittest.main()
