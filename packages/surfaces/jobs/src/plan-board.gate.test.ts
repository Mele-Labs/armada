// A group's boundary and a handed-in task, read off a Job as Fleet serves it.
//
// **Fleet never writes a group `checking`** (`GroupState` in
// `crates/core-model/src/job/plan_group.rs`: *drawn from the step's own
// `checking`*), so a served group stays `running` while its gate runs and the
// boundary read every Check `not run` beside a step panel that showed six
// passed (owner, 5 Oct 2026). These claims are made against the served shape.

import { describe, expect, test } from "vitest";
import type { CheckRun, CheckUnderway, DeclaredCheck, StepDetail } from "@armada/protocol";

import { taskGroupsOf } from "./draft/group";
import { sampleDetail, samplePlan, sampleStep, sampleTask } from "@armada/screens/src/draft/sample";
import { boundaryOf, groupCardOf, verdictSaid } from "./plan-board";

const NAMES = ["build", "test", "typecheck", "storybook", "desktop_test", "screens_test", "components_test"];
const WAITING = "desktop_test";
const declared = (name: string): DeclaredCheck => ({ kind: "manifest_check", name, run: name });

const ran = (name: string, over: Partial<CheckRun> = {}): CheckRun => ({
  attempt: 1,
  name,
  outcome: "passed",
  ...over,
});

/** The gate mid-run: every Check answered but `desktop_test`, which has not started. */
function gate(row: (name: string) => CheckUnderway = defaultRow): NonNullable<StepDetail["checking"]> {
  return { attempt: 1, checks: NAMES.map(row) };
}

function defaultRow(name: string): CheckUnderway {
  return name === WAITING
    ? { name }
    : { name, started_at: "2026-10-05T09:00:00Z", ran: ran(name) };
}

/** One group of tasks in the given states, served as Fleet serves it. */
function job(states: readonly string[], checking?: StepDetail["checking"], state = "running") {
  const step = sampleStep({ checks: NAMES.map(declared), ...(checking === undefined ? {} : { checking }) });
  const ids = states.map((_, at) => `T${at + 1}`);
  return sampleDetail({
    steps: [step],
    work_plan: samplePlan(
      states.map((one, at) => sampleTask({ id: ids[at]!, group: "G1", state: one as never })),
      { groups: [{ id: "G1", tasks: ids, state }] },
    ),
  });
}

function boundary(detail: ReturnType<typeof job>) {
  const [group] = taskGroupsOf(detail);
  return { group: group!, read: boundaryOf(group!, [], detail, detail.steps[0]) };
}

const reads = (detail: ReturnType<typeof job>) => boundary(detail).read.checks.map((one) => one.reads);

describe("a group whose step is running its Checks", () => {
  const all = ["handed_in", "handed_in", "handed_in", "handed_in"];

  test("reads each Check off the gate, though Fleet still calls the group running", () => {
    const detail = job(all, gate());
    expect(boundary(detail).group.state).toBe("running");
    expect(boundary(detail).read.checks.map((one) => [one.name, one.reads])).toEqual(
      NAMES.map((name) => [name, name === WAITING ? "not run" : "passed"]),
    );
  });

  test("says the boundary is running now", () => {
    expect(boundary(job(all, gate())).read.verdictSays).toBe("running now");
  });

  test("a Check that has started reads running", () => {
    const live = job(all, gate((name) => (name === "screens_test" ? { name, started_at: "2026-10-05T09:01:00Z" } : defaultRow(name))));
    expect(reads(live)).toContain("running");
  });

  test("a gate whose finished rows name another group is not this group's", () => {
    const other = job(
      all,
      gate((name) => ({ name, started_at: "2026-10-05T09:00:00Z", ran: ran(name, { group: "G0", group_attempt: 1 }) })),
    );
    expect(new Set(reads(other))).toEqual(new Set(["not run"]));
  });

  test("a group Fleet has not reached does not take the running group's gate", () => {
    const later = job(["open"], gate(), "pending");
    expect(new Set(reads(later))).toEqual(new Set(["not run"]));
    expect(boundary(later).read.verdictSays).toBeUndefined();
  });

  test("verdictSaid takes the step the gate is on", () => {
    const detail = job(all, gate());
    const { group } = boundary(detail);
    expect(verdictSaid(group, [])).toBeUndefined();
    expect(verdictSaid(group, [], detail.steps[0])).toBe("running now");
  });
});

describe("a group with tasks still to come in", () => {
  test("its Checks read not run and the boundary says what they wait on", () => {
    const { read } = boundary(job(["handed_in", "handed_in", "handed_in", "working"]));
    expect(new Set(read.checks.map((one) => one.reads))).toEqual(new Set(["not run"]));
    expect(read.verdictSays).toBe("Waiting on T4");
    expect(read.verdictNamed).toBeUndefined();
  });

  test("names every task it waits on, open or working", () => {
    expect(boundary(job(["handed_in", "open", "working"])).read.verdictSays).toBe("Waiting on T2, T3");
  });

  test("says nothing of waiting once every task is in and the gate has not begun", () => {
    expect(boundary(job(["handed_in", "handed_in"])).read.verdictSays).toBeUndefined();
  });

  test("a group that has not started says nothing, as before", () => {
    expect(boundary(job(["open", "open"], undefined, "pending")).read.verdictSays).toBeUndefined();
  });
});

describe("a handed-in task on the group's row", () => {
  const detail = job(["done", "handed_in", "working", "open"]);
  const [group] = taskGroupsOf(detail);
  const card = groupCardOf(group!, [], new Map(), detail, detail.steps[0]);
  const task = (id: string) => card.tasks.find((one) => one.id === id)!;

  test("says Submitted · awaiting checks", () => {
    expect(task("T2").statusSays).toBe("Submitted · awaiting checks");
  });

  test("keeps its own mark, which is not a working task's", () => {
    expect(task("T2").mark).toBe("handed_in");
    expect(task("T3").mark).toBe("working");
  });

  test("no other state says it", () => {
    expect(["T1", "T3", "T4"].map((id) => task(id).statusSays)).toEqual([undefined, undefined, undefined]);
  });

  test("the group itself stays running while a task is out", () => {
    expect(card.state).toBe("running");
  });
});
