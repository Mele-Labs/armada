// The phase track of a running step: its parts in order, done, now and next,
// read off what the wire carries (owner's annotation of 3 Oct 2026, `ouqa`,
// and his walk of the first answer: a track, not a mark on the pill).

import { describe, expect, it } from "vitest";

import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";
import type { DroneView } from "./draft/drone";
import { arcMoment } from "./fixtures/build/index";
import { groupChecking } from "./fixtures/build/arc-checking";
import { awaitingRepairChecksAgain } from "./fixtures/build/waiting";
import { detailOf } from "./mine";
import { trackOf, type TrackPart } from "./step-phase";

/** The open Job's whole read at an arc moment, and its step at work, declaring a Judge. */
function at(moment: string): { whole: JobWhole; step: StepDetail } {
  const found = moment === "groupChecking" ? groupChecking() : arcMoment(moment);
  const one = found?.fixtures.find((fixture) => fixture.job.id === found.opens);
  if (one === undefined) throw new Error(`no open Job at arc/${moment}`);
  const whole = detailOf(one.watched, one.job.id);
  const step = whole?.steps.find((s) => s.step_id === whole.job.current_step_id);
  if (whole === null || whole === undefined || step === undefined) throw new Error(`no step at arc/${moment}`);
  return { whole, step: { ...step, judge_checks: [{ criteria: 2, gaming_check: false }] } };
}

const drone = (over: Partial<DroneView>): DroneView => ({ id: "d1", step: "implement", state: "running", ...over });
const RESTED = "2026-09-22T10:40:00Z";

/** Each part as `name:state`, which is the whole of what the track draws. */
const states = (parts: readonly TrackPart[] | undefined) => parts?.map((part) => `${part.name}:${part.state}`);
const now = (parts: readonly TrackPart[] | undefined) => parts?.find((part) => part.state === "now")?.label;

describe("the phase track of a running step", () => {
  it("a Check in flight is the part now, over a Drone that still reads running", () => {
    const { whole, step } = at("groupChecking");
    const track = trackOf(whole, step, "running", [drone({ step: step.step_id, task: "T4" })]);
    expect(states(track)).toEqual(["Drones:done", "Checks:now", "Judge:next"]);
    expect(now(track)).toBe("Checks running");
  });

  it("a Drone at rest is not working: Drones are done while the Checks run", () => {
    const { whole, step } = at("groupChecking");
    const track = trackOf(whole, step, "running", [drone({ step: step.step_id, task: "T4", at_rest_since: RESTED })]);
    expect(states(track)).toEqual(["Drones:done", "Checks:now", "Judge:next"]);
  });

  it("a Drone at work is the part now, named by its task", () => {
    const { whole, step } = at("executingSequential");
    const track = trackOf(whole, { ...step, checking: undefined }, "running", [
      drone({ step: step.step_id, task: "T4" }),
      drone({ id: "d0", step: step.step_id, task: "T3", state: "done" }),
    ]);
    expect(states(track)).toEqual(["Drones:now", "Checks:next", "Judge:next"]);
    expect(now(track)).toBe("Drone on T4");
  });

  it("several Drones at work name their tasks; one at rest is left out", () => {
    const { whole, step } = at("executingSequential");
    const track = trackOf(whole, { ...step, checking: undefined }, "running", [
      drone({ step: step.step_id, task: "T4" }),
      drone({ id: "d2", step: step.step_id, task: "T5" }),
      drone({ id: "d3", step: step.step_id, task: "T6", at_rest_since: RESTED }),
    ]);
    expect(now(track)).toBe("Drones on T4, T5");
  });

  it("the Judge's call out is the part now, after the Checks", () => {
    const { whole, step } = at("executingSequential");
    const judging = { look: "criterion", model: "sonnet", call: 1, of: 2, since: RESTED, budget_ms: 120_000 };
    const track = trackOf(whole, { ...step, checking: undefined, judging }, "running", [
      drone({ step: step.step_id, task: "T4", at_rest_since: RESTED }),
    ]);
    expect(states(track)).toEqual(["Drones:done", "Checks:done", "Judge:now"]);
    expect(now(track)).toBe("Judge reading");
  });

  it("a Drone's question holds the step after the Drones, as you", () => {
    const { whole, step } = at("executingSequential");
    const asking = { question: "Which one?", asked_at: RESTED } as JobWhole["asking"];
    const track = trackOf({ ...whole, asking }, { ...step, checking: undefined }, "running", [
      drone({ step: step.step_id }),
    ]);
    expect(states(track)).toEqual(["Drones:done", "You:now", "Checks:next", "Judge:next"]);
    expect(now(track)).toBe("Waiting on you");
  });

  it("a person's gate holds the step at its end", () => {
    const { whole, step } = at("executingSequential");
    const track = trackOf(whole, { ...step, checking: undefined, state: "awaiting_human" }, "awaiting_human", []);
    expect(states(track)).toEqual(["Drones:done", "Checks:done", "Judge:done", "You:now"]);
  });

  it("only the parts the step declares", () => {
    const { whole, step } = at("executingSequential");
    const bare = { ...step, checking: undefined, checks: [], judge_checks: [] };
    expect(states(trackOf(whole, bare, "running", [drone({ step: step.step_id })]))).toEqual(["Drones:now"]);
  });

  it("a stopped step whose Checks are running again draws them as the part now", () => {
    // The owner's Job 3, 4 Oct 2026: Run Checks again, and nothing on the
    // stopped step said its Checks were running.
    const one = awaitingRepairChecksAgain();
    const whole = detailOf(one.watched, one.job.id)!;
    const step = whole.steps.find((s) => s.state === "stopped")!;
    const track = trackOf(whole, step, "stopped", []);
    expect(states(track)).toEqual(["Drones:done", "Checks:now"]);
    expect(now(track)).toBe("Checks running");
  });

  it("a stopped step with no Check live has no track", () => {
    const one = awaitingRepairChecksAgain();
    const whole = detailOf(one.watched, one.job.id)!;
    const step = whole.steps.find((s) => s.state === "stopped")!;
    expect(trackOf(whole, { ...step, checking: undefined }, "stopped", [])).toBeUndefined();
  });

  it("a step that is not running has no track", () => {
    const { whole, step } = at("groupChecking");
    expect(trackOf(whole, step, "advanced", [])).toBeUndefined();
  });
});
