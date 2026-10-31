"""Self-test for plan.py. Run: python3 -m unittest discover -s .github/ci"""
import unittest

from plan import GROUPS, group_of, plan, rerun_keys

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
    "packages/surfaces/helm:test",
    "packages/surfaces/manifest:test",
    "packages/components:components_test",
    "apps/pocket:pocket_test",
]


def grouped(answer):
    return {row["group"]: row["keys"].split() for row in answer["matrix"]["include"]}


class Planning(unittest.TestCase):
    def test_a_root_name_keeps_its_static_job(self):
        self.assertEqual(plan(["build", "typecheck"])["checks"], ["build", "typecheck"])

    def test_a_workspace_key_goes_to_the_group_that_runs_it(self):
        groups = grouped(plan(KEYS))
        self.assertEqual(groups["node checks"], ["apps/desktop:typecheck", "apps/desktop:bridge_build", "apps/pocket:pocket_test"])
        self.assertEqual(groups["components"], ["packages/components:storybook", "packages/components:components_test"])
        surfaces = groups.get("surface tests 1/2", []) + groups.get("surface tests 2/2", [])
        self.assertCountEqual(surfaces, ["packages/screens:screens_test", "packages/surfaces/studios:test", "packages/surfaces/helm:test", "packages/surfaces/manifest:test"])

    def test_every_workspace_key_is_in_exactly_one_group(self):
        answer = plan(KEYS)
        every = [key for keys in grouped(answer).values() for key in keys]
        self.assertEqual(len(every), len(set(every)))
        self.assertEqual(set(answer["checks"]) - set(every),
                         {"build", "typecheck", "apps/desktop/unit:desktop_test", "apps/desktop:xtask_test"})

    def test_a_group_with_no_key_is_not_in_the_matrix(self):
        self.assertEqual(grouped(plan(["apps/pocket:pocket_test"])), {"node checks": ["apps/pocket:pocket_test"]})
        self.assertEqual(plan(["build"])["matrix"]["include"], [])

    def test_the_surface_tests_split_the_same_way_every_time(self):
        keys = [f"packages/surfaces/{name}:test" for name in "studios settings manifest setup cleanup overview helm".split()] + ["packages/screens:screens_test"]
        halves = [group_of(key) for key in keys]
        self.assertEqual(set(halves), {"surface tests 1/2", "surface tests 2/2"})
        self.assertEqual(halves.count("surface tests 1/2"), 4)
        self.assertEqual([group_of(key) for key in reversed(keys)], list(reversed(halves)))

    def test_the_desktop_tests_that_read_no_surface_have_their_own_job_and_no_matrix_entry(self):
        answer = plan(KEYS)
        self.assertIn("apps/desktop/unit:desktop_test", answer["checks"])
        self.assertNotIn("apps/desktop/unit:desktop_test", [key for keys in grouped(answer).values() for key in keys])

    def test_the_old_desktop_key_is_reported_not_skipped(self):
        self.assertEqual(plan(["apps/desktop:desktop_test"])["unrun"], ["apps/desktop:desktop_test"])

    def test_the_xtask_tests_have_their_own_rust_job_and_no_matrix_entry(self):
        answer = plan(KEYS)
        self.assertIn("apps/desktop:xtask_test", answer["checks"])
        self.assertNotIn("apps/desktop:xtask_test", [key for keys in grouped(answer).values() for key in keys])
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
        rows = {r["group"]: r["browsers"] for r in plan(KEYS)["matrix"]["include"]}
        self.assertEqual(rows["node checks"], "false")
        self.assertEqual(rows["components"], "true")

    def test_a_quick_plan_runs_what_compiles_and_names_the_rest_for_the_queue(self):
        answer = plan(KEYS + ["test"], quick=True)
        self.assertEqual(answer["checks"], ["build", "typecheck", "apps/desktop:typecheck", "apps/desktop:bridge_build"])
        self.assertEqual(grouped(answer), {"node checks": ["apps/desktop:typecheck", "apps/desktop:bridge_build"]})
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
        self.assertIn("packages/screens:screens_test", [key for keys in grouped(answer).values() for key in keys])
        self.assertIn("acceptance", answer["queued"])

    def test_a_failed_group_reruns_the_keys_of_that_group_the_change_names(self):
        failed = rerun_keys(["components", "surface tests 2/2", "node checks", "ci"])
        self.assertEqual(failed, {"components", "surface tests 2/2", "node checks", "ci"})
        answer = plan(KEYS, quick=True, rerun=failed)
        named = [key for keys in grouped(plan(KEYS)).values() for key in keys]
        self.assertCountEqual(answer["rerun"], [key for key in named if group_of(key) != "surface tests 1/2"])
        self.assertTrue(any(group_of(key) == "surface tests 1/2" for key in named))
        self.assertEqual(set(grouped(answer)), {"node checks", "components", "surface tests 2/2"})
        self.assertNotIn("packages/components:components_test", answer["queued"])

    def test_a_failed_group_adds_no_key_the_change_does_not_name(self):
        answer = plan(["apps/desktop:typecheck"], quick=True, rerun=rerun_keys(["components"]))
        self.assertEqual(answer["rerun"], [])
        self.assertEqual(grouped(answer), {"node checks": ["apps/desktop:typecheck"]})

    def test_a_group_name_is_not_read_as_a_shard(self):
        self.assertTrue(all(name in rerun_keys([name]) for name in GROUPS))

    def test_a_failed_check_the_change_does_not_cover_is_not_added(self):
        answer = plan(["build"], quick=True, rerun=rerun_keys(["test 1/4"]))
        self.assertEqual(answer["checks"], ["build"])
        self.assertEqual(answer["rerun"], [])

    def test_a_shard_and_the_unit_job_name_their_check(self):
        self.assertEqual(rerun_keys(["test 4/4", "desktop_unit", "apps/desktop:xtask_test"]),
                         {"test", "apps/desktop/unit:desktop_test", "apps/desktop:xtask_test"})


if __name__ == "__main__":
    unittest.main()
