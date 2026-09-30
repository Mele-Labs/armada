// The run's placement and its edges, off the eight shipped workflows and the
// arc's own moments. Arithmetic, so it is unit-tested here rather than in a
// story paying a browser's price.
//
// **What the plan is made of moved to `plan-canvas.test.ts`** with the code it
// tests (owner, 25 Sep 2026), and on 29 Sep the one Plan node left the canvas
// too (`nm0h`): what is left here is the run, its steps top to bottom.

import { describe, expect, it } from "vitest";

import { ARC_MOMENTS } from "./fixtures/build/arc";
import { KIND_FIXTURES, KIND_NAMES } from "./fixtures/build/kinds";
import { taskGroupsOf } from "./draft/group";
import { stepNodeId, stepThatWorksTheGroups, workflowRunOf } from "./workflow-canvas";

/** The Job whole, from a fixture's own `GET /jobs/:job_id` answer. */
function wholeOf(fixture: (typeof KIND_FIXTURES)[number]) {
  const watched = fixture.watched;
  if (watched.state !== "read") throw new Error(`${fixture.job.handle} has no detail`);
  return watched.detail;
}

describe("every shipped workflow draws", () => {
  it("covers each of the eight workflow files and no ninth", () => {
    expect(KIND_FIXTURES).toHaveLength(KIND_NAMES.length);
    expect(KIND_NAMES).not.toContain("verify-and-ship");
  });

  for (const fixture of KIND_FIXTURES) {
    it(`draws ${fixture.job.handle} on the steps its own file declares`, () => {
      const whole = wholeOf(fixture);
      const run = workflowRunOf({ whole, groups: taskGroupsOf(whole) });
      const steps = whole.steps.map((step) => stepNodeId(step.step_id));
      // Every step is a node, in the workflow's own order, and nothing else is.
      expect(run.nodes.filter((node) => node.id.startsWith("step:")).map((node) => node.id)).toEqual(steps);
      // The same run, stacked, holds the same nodes in the same order.
      expect(run.rows.map((row) => row.id)).toEqual(expect.arrayContaining(steps));
    });

    it(`places ${fixture.job.handle}'s steps top to bottom, evenly and in order`, () => {
      const whole = wholeOf(fixture);
      const run = workflowRunOf({ whole, groups: taskGroupsOf(whole) });
      const spine = run.nodes.filter((node) => node.id.startsWith("step:"));
      // The spine runs down (owner, 29 Sep 2026): one column, one step under the last.
      const ys = spine.map((node) => node.position.y);
      expect([...ys].sort((a, b) => a - b)).toEqual(ys);
      expect(new Set(ys).size).toBe(ys.length);
      expect(spine.every((node) => node.position.x === 0)).toBe(true);
      const gaps = ys.slice(1).map((y, at) => y - ys[at]!);
      expect(new Set(gaps).size).toBeLessThanOrEqual(1);
    });

    it(`joins ${fixture.job.handle}'s steps one to the next, and no further`, () => {
      const whole = wholeOf(fixture);
      const run = workflowRunOf({ whole, groups: taskGroupsOf(whole) });
      const spine = run.edges.filter((edge) => edge.source.startsWith("step:") && edge.target.startsWith("step:"));
      // One edge per pair, and one more for each step that sends the run back.
      // **`epic.json` is the one shipped file that does** — `verdict_routing`
      // on `roll_up`, capped at five passes — and the returning edge is what
      // the wave's iteration count is drawn from (`#1544`).
      const loops = whole.steps.filter(
        (step) => step.pass !== undefined && step.verdict_routing_target !== undefined,
      ).length;
      expect(spine).toHaveLength(Math.max(0, whole.steps.length - 1) + loops);
      expect(spine.filter((edge) => edge.kind === "returns")).toHaveLength(loops);
    });

    it(`draws nothing of the plan on ${fixture.job.handle}: no Plan node, no group, no task`, () => {
      const whole = wholeOf(fixture);
      const run = workflowRunOf({ whole, groups: taskGroupsOf(whole) });
      // The step that makes or works the plan links to it from its own panel
      // (owner, 29 Sep 2026, `nm0h`), so the canvas and the stacked run are
      // the steps alone.
      expect(run.nodes.every((node) => node.id.startsWith("step:"))).toBe(true);
      expect(run.rows.every((row) => row.id.startsWith("step:"))).toBe(true);
      expect(run.edges.map((edge) => edge.kind)).not.toContain("made");
    });
  }
});

/** The arc moment by name, with the Job it opens. */
function arc(name: string) {
  const moment = ARC_MOMENTS.find((one) => one.name === name);
  const opened = moment?.fixtures.find((one) => one.job.id === moment.opens);
  const watched = opened?.watched;
  if (watched?.state !== "read") throw new Error(`the ${name} moment has no detail`);
  return { whole: watched.detail, groups: moment?.draft.groups ?? [] };
}

/**
 * The density case the note names: twelve steps and four groups of three. No
 * shipped workflow is that long, so it is built here rather than photographed.
 */
function dense() {
  const { whole, groups } = arc("groupFailed");
  const one = whole.steps[0]!;
  const steps = Array.from({ length: 12 }, (_, at) => ({
    ...one,
    step_id: `s${at + 1}`,
    label: `Step ${at + 1}`,
    ordinal: at + 1,
  }));
  const spare = groups.flatMap((group) => group.tasks)[0]!;
  const four = groups.slice(0, 4).map((group, at) => ({
    ...group,
    tasks: [0, 1, 2].map((k) => ({ ...spare, id: `D${at * 3 + k + 1}`, group: group.id })),
  }));
  return {
    whole: {
      ...whole,
      steps,
      job: { ...whole.job, current_step_id: "s7" },
      work_plan: { ...whole.work_plan!, recorded_by: { by: "step" as const, step_id: "s1", attempt: 1 } },
    },
    groups: four,
  };
}

describe("twelve steps and a plan of four groups", () => {
  it("draws twelve steps and nothing else, each somewhere of its own", () => {
    const { whole, groups } = dense();
    const run = workflowRunOf({ whole, groups });
    // Twelve steps — not twelve plus a Plan node (until 29 Sep 2026), nor
    // twelve plus four groups plus their twelve tasks (until 25 Sep).
    expect(run.nodes).toHaveLength(12);
    const where = run.nodes.map((node) => `${node.position.x},${node.position.y}`);
    expect(new Set(where).size).toBe(where.length);
  });

  it("opens on the step you are on and its neighbours, with no plan hanging off them", () => {
    const { whole, groups } = dense();
    const run = workflowRunOf({ whole, groups });
    // `s7` neither wrote the plan nor works it, so what opens is three steps.
    expect(run.opensOn[0]).toEqual(["s6", "s7", "s8"].map(stepNodeId));
    expect(run.opensOn[run.opensOn.length - 1]).toEqual([stepNodeId("s7")]);
  });
});

describe("a step's card", () => {
  it("says how long and where it has got to on one line, and the step at work counts its groups", () => {
    const { whole, groups } = arc("executingSequential");
    expect(groups.length).toBeGreaterThan(0);
    const run = workflowRunOf({ whole, groups, now: Date.parse(whole.job.started_at ?? whole.created_at) + 3_600_000 });
    const works = stepThatWorksTheGroups(whole)!;
    const card = run.nodes.find((node) => node.id === stepNodeId(works))!.card;
    // The board's `55m · 4 groups`: running, so the groups stand in for the word.
    expect(card.activity).toBe("running");
    expect(card.line).toMatch(new RegExp(`${groups.length} groups$`));
    // A step nothing has entered says so and nothing else.
    const idle = whole.steps.find((step) => step.state === "not_started")!;
    expect(run.nodes.find((node) => node.id === stepNodeId(idle.step_id))!.card.line).toBe("not started");
  });

  it("draws the gate as a chip only where a person answers", () => {
    const { whole, groups } = arc("executingSequential");
    const asking = {
      ...whole,
      steps: whole.steps.map((step, at) => (at === whole.steps.length - 1 ? { ...step, advance_gate: "human_always" } : step)),
    };
    const run = workflowRunOf({ whole: asking, groups });
    const gates = run.nodes.map((node) => node.card.gate).filter((gate) => gate !== undefined);
    expect(gates).toEqual(["will ask you"]);
  });

  it("draws no second edge, whatever the groups have got to", () => {
    const { whole, groups } = arc("groupFailed");
    expect(groups.some((group) => group.state !== "pending")).toBe(true);
    expect(groups.some((group) => group.state === "pending")).toBe(true);
    const run = workflowRunOf({ whole, groups });
    expect(run.edges.map((edge) => edge.kind).every((kind) => kind === "leads" || kind === "returns")).toBe(true);
  });

  it("names the step the Job is on, and narrows onto it rather than shrinking", () => {
    const { whole, groups } = arc("executingSequential");
    const run = workflowRunOf({ whole, groups });
    expect(run.running).toBe(stepNodeId(whole.job.current_step_id ?? ""));
    const widest = run.opensOn[0]!;
    // Narrow opens on where you are, which is never the whole run.
    expect(widest.filter((id) => id.startsWith("step:")).length).toBeLessThan(whole.steps.length);
    // Each choice is narrower than the one before it, and each holds the step
    // a person is on.
    for (const [at, choice] of run.opensOn.entries()) {
      expect(choice).toContain(run.running);
      const before = run.opensOn[at - 1];
      if (before !== undefined) expect(before.length).toBeGreaterThan(choice.length);
    }
  });

  it("marks the card a person has open and no other", () => {
    const { whole, groups } = arc("groupFailed");
    const open = stepNodeId(whole.steps[0]!.step_id);
    const run = workflowRunOf({ whole, groups, selected: open });
    expect(run.nodes.filter((node) => node.card.selected === true).map((node) => node.id)).toEqual([open]);
  });
});

describe("a step that loops draws a returning edge", () => {
  it("returns to the step it names, dashed, with its cap", () => {
    const whole = wholeOf(KIND_FIXTURES[1]!);
    const looping = {
      ...whole,
      steps: whole.steps.map((step, at) =>
        at === 1 ? { ...step, verdict_routing_target: whole.steps[0]!.step_id, pass: { number: 1, of: 5 } } : step,
      ),
    };
    const run = workflowRunOf({ whole: looping, groups: [] });
    const returning = run.edges.filter((edge) => edge.kind === "returns");
    expect(returning).toHaveLength(1);
    expect(returning[0]?.target).toBe(stepNodeId(whole.steps[0]!.step_id));
    expect(returning[0]?.label).toBe("up to 5 passes");
    // Stacked has nothing to arc over, so the row says where it goes back to.
    expect(run.rows[1]?.returns?.toName).toBe(whole.steps[0]!.label);
  });
});
