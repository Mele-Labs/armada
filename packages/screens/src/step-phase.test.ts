// Which part of a running step it is in, read off what the wire carries
// (owner's annotation of 3 Oct 2026, `ouqa`: "I had no idea it was running
// checks").

import { describe, expect, it } from "vitest";

import type { JobDetail as JobWhole, StepDetail } from "@armada/protocol";
import type { DroneView } from "./draft/drone";
import { arcMoment } from "./fixtures/build/index";
import { groupChecking } from "./fixtures/build/arc-checking";
import { detailOf } from "./mine";
import { phaseOf } from "./step-phase";

/** The open Job's whole read at an arc moment, and its step at work. */
function at(moment: string): { whole: JobWhole; step: StepDetail } {
  const found = moment === "groupChecking" ? groupChecking() : arcMoment(moment);
  const one = found?.fixtures.find((fixture) => fixture.job.id === found.opens);
  if (one === undefined) throw new Error(`no open Job at arc/${moment}`);
  const whole = detailOf(one.watched, one.job.id);
  const step = whole?.steps.find((s) => s.step_id === whole.job.current_step_id);
  if (whole === null || whole === undefined || step === undefined) throw new Error(`no step at arc/${moment}`);
  return { whole, step };
}

const drone = (over: Partial<DroneView>): DroneView => ({ id: "d1", step: "implement", state: "running", ...over });

describe("the phase of a running step", () => {
  it("a Check in flight wins over a Drone that still reads running", () => {
    const { whole, step } = at("groupChecking");
    const phase = phaseOf(whole, step, "running", [drone({ step: step.step_id, task: "T4" })]);
    expect(phase).toEqual({ phase: "checks", label: "Checks running" });
  });

  it("a Drone on a task names the task", () => {
    const { whole, step } = at("executingSequential");
    const phase = phaseOf(whole, { ...step, checking: undefined }, "running", [
      drone({ step: step.step_id, task: "T4" }),
      drone({ id: "d0", step: step.step_id, task: "T3", state: "done" }),
    ]);
    expect(phase).toEqual({ phase: "drone", label: "Drone on T4" });
  });

  it("several Drones name their tasks", () => {
    const { whole, step } = at("executingSequential");
    const phase = phaseOf(whole, { ...step, checking: undefined }, "running", [
      drone({ step: step.step_id, task: "T4" }),
      drone({ id: "d2", step: step.step_id, task: "T5" }),
    ]);
    expect(phase).toEqual({ phase: "drone", label: "Drones on T4, T5" });
  });

  it("the Judge's call out is the Judge reading", () => {
    const { whole, step } = at("executingSequential");
    const judging = { look: "criterion", model: "sonnet", call: 1, of: 2, since: "2026-09-22T10:00:00Z", budget_ms: 120_000 };
    const phase = phaseOf(whole, { ...step, checking: undefined, judging }, "running", [
      drone({ step: step.step_id, task: "T4" }),
    ]);
    expect(phase).toEqual({ phase: "judge", label: "Judge reading" });
  });

  it("a Drone's question on the step is waiting on you", () => {
    const { whole, step } = at("executingSequential");
    const asking = { question: "Which one?", asked_at: "2026-09-22T10:00:00Z" } as JobWhole["asking"];
    const phase = phaseOf({ ...whole, asking }, step, "running", [drone({ step: step.step_id })]);
    expect(phase).toEqual({ phase: "waiting", label: "Waiting on you" });
  });

  it("a step that is not running has no phase", () => {
    const { whole, step } = at("groupChecking");
    expect(phaseOf(whole, step, "advanced", [])).toBeUndefined();
  });
});
