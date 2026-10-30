// A group that goes to a Job of its own is a run of its own: its Job lands its own pull request, so the label it was given is ignored.

import { landingsOf } from "@armada/components";
import { expect, test } from "vitest";

const pull = { number: 7, title: "Start the Job on the workflow a rule names", state: "open" as const };

test("a group that goes to a Job of its own is a run of its own, and the groups around it that share a label stay one run", () => {
  const runs = landingsOf(
    [
      { id: "g1", landsIn: { label: "PR 1" } },
      { id: "g2", landsIn: { label: "PR 1", pullRequest: pull }, ownJob: true as const },
      { id: "g3", landsIn: { label: "PR 1" } },
    ],
    true,
  );
  expect(runs.map((run) => [run.key, run.groups.map((one) => one.id)])).toEqual([["PR 1", ["g1", "g3"]], ["job:g2", ["g2"]]]);
  expect(runs[1]?.own?.id).toBe("g2");
  expect(runs[0]).not.toHaveProperty("own");
});

test("the landsIn of a group with its own Job is ignored: no chip heads its run, and its pull request is not read onto the groups that share its label", () => {
  const runs = landingsOf(
    [
      { id: "g1", landsIn: { label: "PR 1" } },
      { id: "g2", landsIn: { label: "PR 1", pullRequest: pull }, ownJob: true as const },
    ],
    true,
  );
  expect(runs[1]).not.toHaveProperty("landsIn");
  expect(runs[0]?.landsIn).toEqual({ label: "PR 1" });
});

test("two groups with Jobs of their own are two runs, and so on a Job that opens no pull requests", () => {
  const groups = [{ id: "g1", ownJob: true as const }, { id: "g2" }, { id: "g3", ownJob: true as const }];
  expect(landingsOf(groups, true).map((run) => run.key)).toEqual(["job:g1", "PR 1", "job:g3"]);
  expect(landingsOf(groups, false).map((run) => run.key)).toEqual(["job:g1", "", "job:g3"]);
});
