// Where a step may be added to one Job: the places a `+` is offered match the ones Fleet accepts
// (`core_model::behind`), and the approval sends what moved and nothing else.

import { describe, expect, it } from "vitest";
import type { AddedStep, JobDetail } from "@armada/protocol";

import { addedTo, featureAtApproval, featureRunning } from "./fake";
import { addStepOf, additionsOf, ahead, atGate, blankAddition, chainAt, delivererOf, heldId, timelineOf } from "./added-reach";

const whole = (fixture: { watched: { state: string } }): JobDetail => {
  const read = fixture.watched as { state: "read"; detail: JobDetail };
  return read.detail;
};

const running = whole(featureRunning());
const gate = whole(featureAtApproval());

describe("where a place is still ahead", () => {
  it("is everywhere at the gate, since nothing has started", () => {
    expect(atGate(gate)).toBe(true);
    for (const step of gate.steps) {
      expect(ahead(gate, { when: "step_starts", step: step.step_id })).toBe(true);
      expect(ahead(gate, { when: "step_passes", step: step.step_id })).toBe(true);
    }
  });

  it("is the step the Job is on and the steps after it, once it runs", () => {
    expect(atGate(running)).toBe(false);
    const at = running.job.current_step_id!;
    // Behind: a step that has passed, and the start of one that has started.
    expect(ahead(running, { when: "step_passes", step: "plan" })).toBe(false);
    expect(ahead(running, { when: "step_starts", step: at })).toBe(false);
    // At and after.
    expect(ahead(running, { when: "step_passes", step: at })).toBe(true);
    expect(ahead(running, { when: "step_passes", step: "tests" })).toBe(true);
    expect(ahead(running, { when: "step_starts", step: "tests" })).toBe(true);
  });

  it("is nowhere on a Job that is over", () => {
    const over = { ...running, job: { ...running.job, status: "completed_success" } };
    expect(ahead(over, { when: "step_passes", step: "tests" })).toBe(false);
  });

  it("agrees with what the mock Fleet accepts, which is the rule Fleet has", () => {
    for (const step of running.steps) {
      for (const when of ["step_starts", "step_passes"] as const) {
        const taken = addedTo(running, { runs: { kind: "script", command: "fmt" }, when, step: step.step_id }, "now");
        expect("added" in taken).toBe(ahead(running, { when, step: step.step_id }));
      }
    }
  });
});

describe("the run's places", () => {
  it("hang pr_opened from the step that delivers, between its start and its pass", () => {
    const deliverer = delivererOf(gate)!;
    const line = timelineOf(gate).filter((gap) => gap.step === deliverer);
    expect(line.map((gap) => gap.when)).toEqual(["step_starts", "pr_opened", "step_passes"]);
    expect(timelineOf(gate).filter((gap) => gap.when === "pr_opened")).toHaveLength(1);
  });

  it("hold what was added there in the order it was added", () => {
    const one = (id: string, step: string): AddedStep => ({ ...blankAddition({ when: "step_passes", step }, "script"), id });
    const rows = [one("a1", "plan"), one("a2", "implement"), one("a3", "plan")];
    expect(chainAt(rows, { when: "step_passes", step: "plan" }).map((row) => row.id)).toEqual(["a1", "a3"]);
    expect(heldId([])).toBe("draft-1");
    expect(heldId([{ ...rows[0]!, id: "draft-4" }])).toBe("draft-5");
  });
});

describe("the approval's additions", () => {
  const added: AddedStep = {
    ...blankAddition({ when: "pr_opened", step: "handoff" }, "script"),
    id: "draft-1",
    runs: { kind: "script", command: "fmt" },
    repair: true,
  };

  it("send nothing where nothing moved, and the list where it did", () => {
    expect(additionsOf([], [])).toBeUndefined();
    expect(additionsOf([added], [added])).toBeUndefined();
    expect(additionsOf([added], [])).toEqual([
      { runs: { kind: "script", command: "fmt" }, when: "pr_opened", step: "handoff", repair: true },
    ]);
  });

  it("send [] to clear what was placed", () => {
    expect(additionsOf([], [added])).toEqual([]);
  });

  it("leave the switches out where off, as Fleet reads them", () => {
    expect(addStepOf({ ...added, repair: false })).toEqual({ runs: added.runs, when: "pr_opened", step: "handoff" });
  });
});
