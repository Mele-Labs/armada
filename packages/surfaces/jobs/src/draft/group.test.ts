// One task per group, in plan order, while the wire has no groups.

import { describe, expect, it } from "vitest";

import { derivedGroupId } from "./coord";
import { stepThatWorksTheGroups, taskGroupsOf } from "./group";
import { sampleDetail, samplePlan, sampleStep, sampleTask } from "@armada/screens/src/draft/sample";

function withTasks(ids: string[], step = sampleStep()) {
  return sampleDetail({
    steps: [step],
    work_plan: samplePlan(ids.map((id) => sampleTask({ id }))),
    job: { ...sampleDetail().job, current_step_id: step.step_id },
  });
}

describe("what the wire's silence about groups derives into", () => {
  it("is one group per task, in plan order, counted from one", () => {
    const groups = taskGroupsOf(withTasks(["T1", "T2", "T3"]));

    expect(groups).toHaveLength(3);
    expect(groups.map((group) => group.ordinal)).toEqual([1, 2, 3]);
    expect(groups.map((group) => group.id)).toEqual([
      derivedGroupId("T1"),
      derivedGroupId("T2"),
      derivedGroupId("T3"),
    ]);
  });

  it("holds exactly its one task and nothing concurrent", () => {
    const [group] = taskGroupsOf(withTasks(["T1"]));

    expect(group?.tasks.map((task) => task.id)).toEqual(["T1"]);
    expect(group?.concurrent).toBe(false);
  });

  it("is empty where no plan was recorded", () => {
    expect(taskGroupsOf(sampleDetail())).toEqual([]);
  });
});

describe("the scope and Checks a group takes", () => {
  it("takes its task's declared paths as its own", () => {
    const detail = withTasks([]);
    detail.work_plan = samplePlan([
      sampleTask({ id: "T1", scope: ["crates/ipc/src/work_plan.rs"] }),
    ]);

    expect(taskGroupsOf(detail)[0]?.scope).toEqual(["crates/ipc/src/work_plan.rs"]);
  });

  it("selects the step's declared Checks, by name", () => {
    const step = sampleStep({
      checks: [
        { kind: "manifest_check", name: "typecheck" },
        { kind: "manifest_check", name: "test" },
      ],
    });

    expect(taskGroupsOf(withTasks(["T1"], step))[0]?.checks_selected).toEqual([
      "typecheck",
      "test",
    ]);
  });

  it("leaves out a Check that is the step's own guard rather than a task's", () => {
    const step = sampleStep({ checks: [{ kind: "diff_nonempty" }, { kind: "every_manifest_check" }] });

    expect(taskGroupsOf(withTasks(["T1"], step))[0]?.checks_selected).toEqual([]);
  });

  it("takes only the Checks whose `when` reaches the task's paths", () => {
    const step = sampleStep({
      checks: [
        { kind: "manifest_check", name: "build", when: ["crates/**", "xtask/**", "Cargo.toml"] },
        { kind: "manifest_check", name: "typecheck", when: ["packages/**"] },
        { kind: "manifest_check", name: "readme", when: ["docs/*.md"] },
        { kind: "manifest_check", name: "always" },
      ],
    });
    const detail = withTasks([], step);
    detail.work_plan = samplePlan([
      sampleTask({ id: "T1", scope: ["xtask/src/main.rs"] }),
      sampleTask({ id: "T2", scope: ["packages/components/src/guides/index.ts"] }),
      sampleTask({ id: "T3", scope: ["docs/INDEX.md", "docs/concepts/a/b.md"] }),
      sampleTask({ id: "T4", scope: ["Cargo.toml.bak"] }),
    ]);

    expect(taskGroupsOf(detail).map((group) => group.checks_selected)).toEqual([
      ["build", "always"],
      ["typecheck", "always"],
      ["readme", "always"],
      ["always"],
    ]);
  });

  it("runs no cases at its boundary, because nothing resolves them", () => {
    expect(taskGroupsOf(withTasks(["T1"]))[0]?.cases_at_boundary).toEqual([]);
  });
});

describe("where a group is", () => {
  it("is checking while the gate is running the step's Checks", () => {
    const step = sampleStep({
      checking: { attempt: 1, checks: [] },
    });

    expect(taskGroupsOf(withTasks(["T1"], step))[0]?.state).toBe("checking");
  });

  it("reads a stopped step as failed and an advanced one as passed", () => {
    expect(taskGroupsOf(withTasks(["T1"], sampleStep({ state: "stopped" })))[0]?.state).toBe(
      "failed",
    );
    expect(taskGroupsOf(withTasks(["T1"], sampleStep({ state: "advanced" })))[0]?.state).toBe(
      "passed",
    );
  });

  it("reads a step state it does not know as pending, never as running", () => {
    expect(
      taskGroupsOf(withTasks(["T1"], sampleStep({ state: "not_started" })))[0]?.state,
    ).toBe("pending");
  });

  it("reads its own task while the step working the tasks runs", () => {
    const detail = withTasks([]);
    detail.work_plan = samplePlan([
      sampleTask({ id: "T1", state: "working" }),
      sampleTask({ id: "T2", state: "open" }),
    ]);

    expect(taskGroupsOf(detail).map((group) => group.state)).toEqual(["running", "pending"]);
  });

  it("takes nothing from the plan step it was written at", () => {
    const plan = sampleStep({
      step_id: "plan",
      ordinal: 1,
      state: "running",
      checks: [{ kind: "plan_recorded" }],
      last_verdict: { attempt: 1, named: "passed" },
    });
    const implement = sampleStep({
      state: "not_started",
      checks: [{ kind: "manifest_check", name: "test" }],
    });
    const detail = sampleDetail({
      steps: [plan, implement],
      work_plan: samplePlan([sampleTask({ id: "T1", state: "open" })]),
      job: { ...sampleDetail().job, current_step_id: "plan" },
    });

    const [group] = taskGroupsOf(detail);
    expect(group?.state).toBe("pending");
    expect(group?.checks_selected).toEqual(["test"]);
    expect(group?.verdict).toBeUndefined();
  });

  it("is worked at the step declaring a Drone per task, not merely the one after the plan", () => {
    const plan = sampleStep({ step_id: "plan", ordinal: 1, state: "advanced" });
    const read = sampleStep({ step_id: "read", ordinal: 2, state: "advanced" });
    const implement = sampleStep({ step_id: "implement", ordinal: 3, drone_per_task: true });
    const detail = sampleDetail({
      steps: [plan, read, implement],
      work_plan: samplePlan([sampleTask({ id: "T1", state: "working" })]),
      job: { ...sampleDetail().job, current_step_id: "implement" },
    });

    expect(stepThatWorksTheGroups(detail)).toBe("implement");
  });

  it("is passed where the task itself is done, whatever the step is doing", () => {
    const detail = withTasks([]);
    detail.work_plan = samplePlan([sampleTask({ id: "T1", state: "done" })]);

    expect(taskGroupsOf(detail)[0]?.state).toBe("passed");
  });
});

describe("retries, counted from the step's own runs", () => {
  it("is nought on a step nothing has entered", () => {
    expect(taskGroupsOf(withTasks(["T1"]))[0]?.retry_count).toBe(0);
  });

  it("is one fewer than the runs of the step holding it", () => {
    const step = sampleStep({
      attempts: [
        { attempt: 1, outcome: "refused", started_at: "2026-09-22T09:05:00Z" },
        { attempt: 2, outcome: "running", started_at: "2026-09-22T09:20:00Z" },
      ],
    });

    expect(taskGroupsOf(withTasks(["T1"], step))[0]?.retry_count).toBe(1);
  });
});

describe("a group Fleet served, its tasks run at once (23.10)", () => {
  it("is concurrent where a task names another beside it, and not otherwise", () => {
    const detail = sampleDetail({
      work_plan: samplePlan(
        [
          sampleTask({ id: "T1", group: "G1", concurrent_with: ["T2"] }),
          sampleTask({ id: "T2", group: "G1", concurrent_with: ["T1"] }),
          sampleTask({ id: "T3", group: "G2" }),
        ],
        {
          groups: [
            { id: "G1", tasks: ["T1", "T2"], state: "running" },
            { id: "G2", tasks: ["T3"], state: "pending" },
          ],
        },
      ),
    });

    expect(taskGroupsOf(detail).map((group) => group.concurrent)).toEqual([true, false]);
  });
});
