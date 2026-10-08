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
    });

    it(`places ${fixture.job.handle}'s steps top to bottom, with even air between them and in order`, () => {
      const whole = wholeOf(fixture);
      const run = workflowRunOf({ whole, groups: taskGroupsOf(whole) });
      const spine = run.nodes.filter((node) => node.id.startsWith("step:"));
      // The spine runs down (owner, 29 Sep 2026): one column, one step under the last.
      const ys = spine.map((node) => node.position.y);
      expect([...ys].sort((a, b) => a - b)).toEqual(ys);
      expect(new Set(ys).size).toBe(ys.length);
      expect(spine.every((node) => node.position.x === 0)).toBe(true);
      // Evenly means the same air between two cards, whatever each draws: a
      // card leaves out a row it has nothing for (owner, 30 Sep 2026), so the
      // step to the next one is its rows' height plus one constant.
      // A running step's phase track is one row more, of its own height.
      const ROW = 28;
      const TRACK = 36;
      const rows = (card: (typeof spine)[number]["card"]) =>
        (card.needs?.length ?? 0) + Number(card.line !== undefined || card.bar !== undefined) + Number(card.gate !== undefined);
      const high = (card: (typeof spine)[number]["card"]) => rows(card) * ROW + (card.track === undefined ? 0 : TRACK);
      const air = ys.slice(1).map((y, at) => y - ys[at]! - high(spine[at]!.card));
      expect(new Set(air).size).toBeLessThanOrEqual(1);
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
      // (owner, 29 Sep 2026, `nm0h`), so the canvas is the steps
      // alone.
      expect(run.nodes.every((node) => node.id.startsWith("step:"))).toBe(true);
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
  // The owner's combination of 30 Sep 2026: what needs a person first, then
  // where the work has got to.
  it("draws the step at work's groups as its bar, with how long and its Drones beside it", () => {
    const { whole, groups } = arc("executingSequential");
    expect(groups.length).toBeGreaterThan(0);
    const run = workflowRunOf({ whole, groups, now: Date.parse(whole.job.started_at ?? whole.created_at) + 3_600_000 });
    const works = stepThatWorksTheGroups(whole)!;
    const card = run.nodes.find((node) => node.id === stepNodeId(works))!.card;
    expect(card.activity).toBe("running");
    expect(card.bar?.groups).toHaveLength(groups.length);
    // The count is the bar's tooltip, never drawn beside the segments.
    const done = groups.filter((group) => group.state === "passed" || group.state === "landed").length;
    expect(card.bar?.label).toBe(`${done} of ${groups.length} groups done`);
    expect(card.line).toMatch(/ · 1 Drone$/);
    expect(card.needs).toBeUndefined();
  });

  it("draws a step nothing has entered as its name alone, and a finished one as how long and its state", () => {
    const { whole, groups } = arc("executingSequential");
    const card = (state: string) => {
      const step = whole.steps.find((one) => one.state === state)!;
      return workflowRunOf({ whole, groups }).nodes.find((node) => node.id === stepNodeId(step.step_id))!.card;
    };
    expect(card("not_started").line).toBeUndefined();
    expect(card("not_started").bar).toBeUndefined();
    expect(card("advanced").line).toMatch(/ · advanced$/);
  });

  it("leads with a failed Check, and then says only how long", () => {
    const { whole, groups } = arc("groupFailed");
    const works = stepThatWorksTheGroups(whole)!;
    const card = workflowRunOf({ whole, groups }).nodes.find((node) => node.id === stepNodeId(works))!.card;
    expect(card.needs).toEqual([{ says: "screens_test failed", tone: "failed" }]);
    expect(card.bar?.groups).toContain("failed");
    expect(card.line).not.toMatch(/Drone|running/);
  });

  it("says a step is waiting on you, and places the next card by the rows each one draws", () => {
    const whole = KIND_FIXTURES.map(wholeOf).find((one) => one.steps.some((step) => step.state === "awaiting_human"))!;
    const run = workflowRunOf({ whole, groups: [] });
    const waiting = whole.steps.find((step) => step.state === "awaiting_human")!;
    const card = run.nodes.find((node) => node.id === stepNodeId(waiting.step_id))!.card;
    expect(card.needs?.[0]).toEqual({ says: "Waiting on you", tone: "waiting" });
    // A card with fewer rows under its name sits closer to the next one.
    const quiet = workflowRunOf({ whole: arc("executingSequential").whole, groups: [] });
    const gaps = quiet.nodes.slice(1).map((node, at) => node.position.y - quiet.nodes[at]!.position.y);
    expect(new Set(gaps).size).toBeGreaterThan(1);
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
    // The widest choice is the step and its neighbours, which on a three-step
    // run at its middle step is every step; the narrower one is the step alone.
    expect(widest.filter((id) => id.startsWith("step:")).length).toBeLessThanOrEqual(whole.steps.length);
    expect(run.opensOn[1]).toEqual([run.running]);
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
  });
});
