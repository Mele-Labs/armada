import { describe, expect, it } from "vitest";

import type { JobDetail, JobSummary } from "@armada/protocol";

import { waveOf } from "./wave";

function row(over: Partial<JobSummary> & Pick<JobSummary, "id">): JobSummary {
  return {
    handle: `${over.id}-a-job`,
    title: `Job ${over.id}`,
    status: "running",
    workflow_id: "feature",
    owner_manifest_id: "m",
    origin: "manual",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-22T07:20:00Z",
    ...over,
  };
}

function parent(): JobDetail {
  return {
    job: row({ id: "parent", title: "Carry the error contract through every surface" }),
    created_at: "2026-09-22T07:10:00Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
  };
}

/** A row one pass of the parent's plan proposed, as Fleet serves it since 23.11. */
function proposed(id: string, pass: number, over: Partial<JobSummary> = {}): JobSummary {
  return row({ id, origin: "sub_dispatched", dispatched_by: "parent", dispatched_pass: pass, ...over });
}

describe("the wave a Job proposed", () => {
  it("is every row the Job's plan proposed, and nothing else", () => {
    const wave = waveOf(parent(), [
      proposed("a", 1),
      proposed("b", 1, { dispatched_by: "somebody-else" }),
      proposed("c", 1),
    ]);
    expect(wave?.jobs.map((one) => one.job)).toEqual(["a", "c"]);
  });

  // A landing order's members carry no pass, and they are the members band's
  // to draw: a wave is not a landing order.
  it("leaves out a row with no pass, which is a member of a landing order", () => {
    expect(
      waveOf(parent(), [row({ id: "a", origin: "sub_dispatched", dispatched_by: "parent" })]),
    ).toBeUndefined();
  });

  // Absent, never an empty graph: an empty one reads as Jobs that failed to
  // load, which is a different sentence from a Job that proposed none.
  it("is absent where the Job proposed nothing", () => {
    expect(waveOf(parent(), [row({ id: "a" })])).toBeUndefined();
  });

  it("carries each Job's status and where its pull request settled", () => {
    const wave = waveOf(parent(), [proposed("a", 1, { status: "completed_success", landed: "merged" })]);
    expect(wave?.jobs[0]).toMatchObject({ status: "completed_success", landed: "merged" });
  });

  // Since 23.14 each row carries what it waits on, so the graph has its order
  // off the Board alone.
  it("reads each Job's waiting order off its own row", () => {
    const wave = waveOf(parent(), [proposed("a", 1), proposed("b", 1, { waits_on: ["a"] })]);
    expect(wave?.jobs.map((one) => one.waits_on)).toEqual([[], ["a"]]);
  });

  // A Fleet before 23.14 sends none, and an invented order would be a graph
  // saying something Fleet never said.
  it("leaves the waiting order empty where the rows carry none", () => {
    const wave = waveOf(parent(), [proposed("a", 1), proposed("b", 1)]);
    expect(wave?.jobs.every((one) => one.waits_on.length === 0)).toBe(true);
  });

  // Each pass's line is the first sentence of the approach its plan recorded;
  // a pass with none is named alone rather than given a sentence standing in.
  it("names each pass with its plan's approach, and leaves one with none bare", () => {
    const wave = waveOf(
      {
        ...parent(),
        wave_rounds: [
          { pass: 2, approach: "Every surface that reads a refusal. The seam landed, so each can follow it." },
        ],
      },
      [proposed("a", 1), proposed("b", 2)],
    );
    expect(wave?.rounds).toEqual([
      { round: 1, live: false },
      {
        round: 2,
        says: "Every surface that reads a refusal",
        approach: "Every surface that reads a refusal. The seam landed, so each can follow it.",
        live: true,
      },
    ]);
  });

  it("reads one pass for each the rows were proposed on, the latest live", () => {
    const wave = waveOf(parent(), [proposed("a", 1), proposed("b", 2), proposed("c", 2)]);
    expect(wave?.rounds).toEqual([
      { round: 1, live: false },
      { round: 2, live: true },
    ]);
    expect(wave?.jobs.map((one) => one.round)).toEqual([1, 2, 2]);
  });
});
