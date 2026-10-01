// What the inspector reads for the step or group a person has open.

import { describe, expect, it } from "vitest";

import { ARC_MOMENTS } from "./fixtures/build/arc";
import { KIND_FIXTURES } from "./fixtures/build/kinds";
import { taskGroupsOf } from "./draft/group";
import { groupNodeId } from "./plan-canvas";
import { stepNodeId, stepThatWorksTheGroups } from "./workflow-canvas";
import { workflowReadingOf } from "./workflow-inspector";

const executing = ARC_MOMENTS.find((moment) => moment.name === "executingSequential")!;
const opened = executing.fixtures.find((one) => one.job.id === executing.opens)!;
const watched = opened.watched;
if (watched.state !== "read") throw new Error("the executing moment has no detail");
const whole = watched.detail;
const groups = taskGroupsOf(whole);
const groupsUnder = stepThatWorksTheGroups(whole);

describe("nothing open", () => {
  it("reads nothing for no selection, and nothing for a node this Job has not", () => {
    expect(workflowReadingOf({ whole, groups, selected: null })).toBeUndefined();
    expect(workflowReadingOf({ whole, groups, selected: "step:not-a-step" })).toBeUndefined();
  });
});

describe("a step", () => {
  const reading = workflowReadingOf({
    whole,
    groups,
    groupsUnder,
    selected: stepNodeId(whole.steps[1]!.step_id),
  })!;

  it("names the step, where it sits and its state, and the plan as a card of its groups", () => {
    expect(reading.kind).toBe("step");
    expect(reading.name).toBe(whole.steps[1]!.label);
    expect(reading.eyebrow).toBe(`Step ${whole.steps[1]!.ordinal + 1}`);
    expect(reading.state?.activity).toBe("running");
    // The step that works the plan links to it and lists none of its tasks
    // (owner, 28 and 29 Sep 2026, `25i2`, `nm0h`, `dco5`).
    expect(reading.plan?.groups.map((group) => group.name)).toEqual(groups.map((group) => `Group ${group.ordinal}`));
    expect(reading.plan?.groups.map((group) => group.tasks)).toEqual(
      groups.map((group) => `${group.tasks.length} ${group.tasks.length === 1 ? "task" : "tasks"}`),
    );
    expect(reading.tasks ?? []).toEqual([]);
  });

  it("hands the plan card the way to Plan, where it is given one", () => {
    let opened = 0;
    const linked = workflowReadingOf({
      whole,
      groups,
      groupsUnder,
      selected: stepNodeId(whole.steps[1]!.step_id),
      onOpenPlan: () => (opened += 1),
    })!;
    linked.plan?.onOpen?.();
    expect(opened).toBe(1);
  });

  it("draws every Check the step declares, once each, whichever glob selected it", () => {
    const declared = (whole.steps[1]!.checks ?? []).map((check) => check.name ?? check.kind);
    // `implement` declares `test` twice — once for Rust and once for Bridge —
    // and one command is one row.
    expect(new Set(declared).size).toBeLessThan(declared.length);
    expect(reading.checks?.map((check) => check.name)).toEqual([...new Set(declared)]);
  });

  // It said *Fleet does not serve the cases a boundary owes yet* until 28 Sep,
  // and the owner asked what was missing and how it gets fixed (`frpl`).
  it("says what is missing where no case runs, and links the issue that builds it", () => {
    expect(reading.tests).toEqual([]);
    expect(reading.testsAbsent?.says).toContain("COVERS");
    expect(reading.testsAbsent?.issue?.href).toMatch(/\/issues\/1274$/);
    expect(JSON.stringify(reading)).not.toContain("does not serve the cases");
  });

  it("reads a Check the gate is running as running, and one waiting its turn as waiting", () => {
    const step = whole.steps[1]!;
    const [first, second] = [...new Set((step.checks ?? []).map((check) => check.name ?? check.kind))];
    const started = "2026-09-22T10:00:00Z";
    const live = {
      ...whole,
      steps: whole.steps.map((one, at) =>
        at === 1
          ? { ...one, checking: { attempt: 1, checks: [{ name: first!, started_at: started }, { name: second! }] } }
          : one,
      ),
    };
    const checks = workflowReadingOf({
      whole: live,
      groups,
      groupsUnder,
      selected: stepNodeId(step.step_id),
      now: Date.parse(started) + 42_000,
    })!.checks!;
    expect(checks.find((check) => check.name === first)).toMatchObject({ live: "running", outcome: "42s" });
    expect(checks.find((check) => check.name === second)).toMatchObject({ live: "waiting", outcome: "waiting" });
  });
});

describe("a group", () => {
  const group = groups[0]!;
  const reading = workflowReadingOf({ whole, groups, groupsUnder, selected: groupNodeId(group.id) })!;

  it("holds the tasks that group holds and nothing beside them", () => {
    expect(reading.kind).toBe("group");
    expect(reading.tasks?.map((task) => task.id)).toEqual(group.tasks.map((task) => task.id));
  });

  it("draws only the Checks selected at this group's boundary", () => {
    for (const check of reading.checks ?? []) {
      expect(group.checks_selected).toContain(check.name);
    }
  });

  it("names the Drone a redirect from here would reach", () => {
    // One Drone per Job today, so a group's redirect reaches the Job's own.
    const expected = whole.job.assigned_drone === undefined ? 0 : 1;
    expect(reading.drones.length).toBeGreaterThanOrEqual(expected);
  });
});

describe("a Job with no plan", () => {
  it("draws no plan and no task list, rather than an empty one", () => {
    const bare = KIND_FIXTURES.map((fixture) => fixture.watched)
      .filter((one) => one.state === "read")
      .map((one) => one.detail)
      .find((detail) => detail.work_plan === undefined);
    if (bare === undefined) return;
    const reading = workflowReadingOf({
      whole: bare,
      groups: [],
      selected: stepNodeId(bare.steps[0]!.step_id),
    })!;
    expect(reading.plan).toBeUndefined();
    expect(reading.tasks ?? []).toEqual([]);
  });
});
