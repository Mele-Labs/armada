// What Overview's lead says, sentence by sentence.
//
// **A claim per state, read off the roster's own fixtures**, so a sentence
// that drifts fails here rather than being read off a screenshot. The voice is
// *The lead says the thing and stops*, 30 Sep 2026, in the decisions register.

import { describe, expect, it } from "vitest";

import { ARC_MOMENTS, FIXTURES, arcMoment } from "./fixtures/build/index";
import type { JobFixture } from "./fixtures/fixture";
import { leadOf } from "./lead";
import { detailOf } from "./mine";

/** The lead a fixture draws, through the same reading `JobDetail` does. */
function leadFor(one: JobFixture) {
  return leadOf(one.job, detailOf(one.watched, one.job.id), one.now);
}

/** One of the roster's fixtures, by the sentence it renders. */
function named(starts: string): JobFixture {
  const found = FIXTURES.find((one) => one.name.startsWith(starts));
  if (found === undefined) throw new Error(`no fixture named ${starts}`);
  return found;
}

/** A Job on some arc moment's Board, by the sentence it renders. */
function arcNamed(starts: string): JobFixture {
  const every = ARC_MOMENTS.flatMap((moment) => moment.fixtures);
  const found = every.find((one) => one.name.startsWith(starts));
  if (found === undefined) throw new Error(`no arc fixture named ${starts}`);
  return found;
}

/** The same Job with one step's Check answering differently. */
function withOutcome(one: JobFixture, stepId: string, outcome: string): JobFixture {
  if (one.watched.state !== "read") throw new Error(`${one.name} serves no detail`);
  const detail = one.watched.detail;
  return {
    ...one,
    watched: {
      ...one.watched,
      detail: {
        ...detail,
        steps: detail.steps.map((step) =>
          step.step_id === stepId
            ? { ...step, check_runs: step.check_runs.map((run) => ({ ...run, outcome })) }
            : step,
        ),
      },
    },
  };
}

/** The Job an arc moment is about — the one whose id the moment opens. */
function arcJobAt(moment: string): JobFixture {
  const found = arcMoment(moment);
  const one = found?.fixtures.find((fixture) => fixture.job.id === found.opens);
  if (one === undefined) throw new Error(`no open Job at arc/${moment}`);
  return one;
}

describe("the headline names the thing and stops", () => {
  it("a Drone waiting on a command names the command, not the step it is on", () => {
    const lead = leadFor(named("running — the drone is waiting for a person to allow"));
    expect(lead.said).toBe("A Drone wants to run a command");
    expect(lead.because).toBe(
      "pnpm add -D reselect@5.1.1 · Regression check and 2 more do not start until you answer",
    );
    // Aged by the lead now, because the box inside it no longer has a head.
    expect(lead.elapsed).toBe("1m 50s");
  });

  it("a human gate reads as a review, with what the gate found under it", () => {
    const lead = leadFor(named("awaiting_review — every Check passed"));
    expect(lead.said).toBe("Waiting for your review");
    expect(lead.because).toBe(
      "Both Checks passed and the Judge met both criteria · " +
        "Check the consumers still compile and 1 more do not start until you answer",
    );
    expect(lead.act).toBe("Review it");
  });

  it("a Check that failed names the Check, not the step it ran on", () => {
    const lead = leadFor(named("running — a Check failed and the Drone is retrying"));
    expect(lead.said).toBe("cargo_nextest failed");
    expect(lead.because).toBe("exit 101 — 3 of 2034 tests failed · Running again");
  });

  it("a running step is its label and how long it has been on it", () => {
    expect(leadFor(arcJobAt("executingSequential")).said).toBe("Implement · 1h 58m in");
  });

  it("a Judge refusal counts the criteria and quotes the first", () => {
    const lead = leadFor(named("escalated · evidence_suspect"));
    expect(lead.said).toBe("A Judge refused 1 of 2 criteria");
  });

  it("a Job Fleet stopped says why, in the registry's own verb and nothing more", () => {
    const lead = leadFor(named("escalated · gate_failure"));
    expect(lead.said).toBe("This Job stopped at the gate");
    expect(lead.opens).toEqual({ tab: "record" });
  });

  it("a Job that is over says so in a word", () => {
    expect(leadFor(named("completed_success")).said).toBe("Done");
    expect(leadFor(named("killed")).said).toBe("This Job stopped");
  });

  it("no headline ends in a full stop, anywhere on either roster", () => {
    const every = [...FIXTURES, ...ARC_MOMENTS.flatMap((moment) => moment.fixtures)];
    const ended = every.map(leadFor).filter((lead) => /[.!]$/.test(lead.said));
    expect(ended.map((lead) => lead.said)).toEqual([]);
  });
});

describe("the second line carries a fact or it is empty", () => {
  it("nothing behind a step is nothing, not a sentence saying so", () => {
    // `Land` is last, so nothing waits on the answer — and the clause is
    // absent rather than reading *Nothing in this workflow is behind it.*
    // What the line does carry is what the gates found, which is the claim
    // below.
    expect(leadFor(named("awaiting_review — the branch is pushed")).because).not.toContain(
      "do not start until you answer",
    );
  });

  it("a plan nobody has started is planned, not `0 of 8`", () => {
    expect(leadFor(arcJobAt("planned")).because).toBe("8 tasks planned, none started");
  });

  it("a plan part way through counts what is done", () => {
    expect(leadFor(arcJobAt("executingSequential")).because).toContain("4 of 8 tasks done");
  });

  it("a gate ahead is named, and no gate ahead says nothing", () => {
    expect(leadFor(arcJobAt("executingSequential")).because).toContain(
      "Review the change asks you, 2 steps away",
    );
    // Every step after `Fix` on the Bug workflow advances on its Checks, so
    // there is no *No step after this one stops for you* to read.
    expect(leadFor(named("running — mid-step on Fix")).because).toBe("");
  });

  it("a stopped Job drops the reassurance that nothing else is running", () => {
    for (const name of ["escalated · silent", "rejected", "killed"]) {
      expect(leadFor(named(name)).because).toBe("");
    }
  });
});

// **What is being signed off is the branch, not the step** — the owner, 30 Sep
// 2026, in `the-lead-counts-the-whole-jobs-evidence`. One claim per counting
// rule, because the counting is the part that can lie.
describe("what the gate found is the whole Job's evidence", () => {
  it("the delivering gate counts every step's Checks, not the step that is waiting", () => {
    // `Land` pushed a branch and verified nothing of its own, which is why the
    // step-local reading left this line empty on the Job most likely to be open.
    expect(leadFor(named("awaiting_review — the branch is pushed")).because).toBe(
      "All 3 Checks passed and the Judge met both criteria",
    );
  });

  it("a Check that failed and was retried is one Check that passed", () => {
    // `Fix` ran `cargo_build` twice. Four rows, three Checks, all of them
    // passed — attempt 1's failure is history and not a fourth Check.
    expect(leadFor(named("awaiting_review — a Check was retried")).because).toContain(
      "All 3 Checks passed",
    );
  });

  it("a criterion a person overruled is not one the Judge met", () => {
    // `regression_verify` advanced because somebody disagreed with the Judge.
    // The refusal is still on the wire, so the count cannot launder it.
    expect(leadFor(named("awaiting_review — a Check was retried")).because).toContain(
      "the Judge met 1 of 2 criteria",
    );
  });

  it("a skipped Check is counted neither as a pass nor as a failure", () => {
    // **Not a fixture**: `skipped` needs a Check declaring paths the step did
    // not touch, and this narrative's Manifest declares two Checks that cover
    // everything — so the outcome is moved here rather than a Manifest bent
    // around one row.
    const one = named("awaiting_review — the branch is pushed");
    const lead = leadFor(withOutcome(one, "consumers", "skipped"));
    expect(lead.because).toBe("Both Checks passed and the Judge met both criteria");
  });

  it("a Job at review whose steps found nothing draws no second line", () => {
    expect(leadFor(arcNamed("awaiting_review — a second Job holding")).because).toBe("");
  });
});

describe("the three states that were saying the wrong thing", () => {
  it("awaiting_repair is out of retries, not waiting to be approved", () => {
    const lead = leadFor(named("awaiting_repair"));
    expect(lead.said).toBe("Out of retries");
    expect(lead.because).toBe("cargo_nextest failed · exit 101");
    // Fleet does not serve the status, so there is nothing for a button to do.
    expect(lead.act).toBeUndefined();
  });

  it("awaiting_attestation names the criterion it owes, in the requester's words", () => {
    const lead = leadFor(named("awaiting_attestation"));
    expect(lead.said).toBe("A criterion needs your attestation");
    expect(lead.because).toBe("Every existing settings test still passes");
    expect(lead.act).toBeUndefined();
  });

  it("awaiting_approval opens the proposal, and is told apart by having no branch", () => {
    const lead = leadFor(named("awaiting_approval"));
    expect(lead.said).toBe("Waiting for your approval");
    expect(lead.opens).toEqual({ proposal: true });
    // And a Job that has run is never read as one waiting to be dispatched,
    // however few of its steps have started.
    const peer = arcMoment("dispatchTyping")?.fixtures.find(
      (one) => one.job.status === "awaiting_review",
    );
    expect(peer).toBeDefined();
    expect(leadFor(peer as JobFixture).said).toBe("Waiting for your review");
  });
});

describe("a dispatched request the proposer has not answered", () => {
  it("names the model reading it, and offers the one act", () => {
    const lead = leadFor(named("proposing"));
    expect(lead.said).toBe("A model is reading the request");
    expect(lead.act).toBe("Stop the proposer");
    // Answered in the lead's own region, so the act names no destination —
    // the wait region is what carries the control.
    expect(lead.opens).toBeUndefined();
  });

  it("says nothing on the second line, because the wait is under it", () => {
    // **The line that would otherwise restate the wait.** The wait region draws
    // the reach, the budget and the thinking estimate; a clause here repeating
    // any of them is the duplication the owner took out of this region.
    expect(leadFor(named("proposing")).because).toBe("");
  });

  it("is not the quiet line, which would say nothing needs you", () => {
    // What it said before this branch existed, over a model call spending money
    // on a Job with no step for `currentStep` to find.
    expect(leadFor(named("proposing")).said).not.toBe("Nothing needs you");
    // And no tone: nothing is waiting on a person and nothing has failed.
    expect(leadFor(named("proposing")).tone).toBeUndefined();
  });
});

// **Before this Job's own read has answered**, the lead stands in only where it
// would fall through to its quiet line — the owner, 1 Oct 2026. Every other
// branch it can reach with no read is proven by the Board's row alone.
describe("with no read yet, only the quiet line is a guess", () => {
  /** The fixture's Job, with its read not yet answered. */
  const unread = (one: JobFixture) => leadOf(one.job, null, one.now);

  it("a running Job with no read falls through to the quiet line, and says it did", () => {
    const lead = unread(named("running — this Job's own detail"));
    expect(lead.said).toBe("Nothing needs you");
    expect(lead.quiet).toBe(true);
  });

  it("a Job the row says is waiting on you is said at once, and is not quiet", () => {
    const lead = unread(named("awaiting_approval"));
    expect(lead.said).toBe("Waiting for your approval");
    expect(lead.quiet).toBeUndefined();
  });

  it("a Job the row says is over is said at once, and is not quiet", () => {
    const over = FIXTURES.filter((one) => one.job.status === "killed" || one.job.status === "rejected");
    expect(over.length).toBeGreaterThan(0);
    for (const one of over) expect(unread(one).quiet, one.name).toBeUndefined();
  });

  it("a read Job naming its running step is the same branch, and not quiet", () => {
    // Quiet is a headline with nothing to name, not the branch it came from.
    expect(leadFor(named("running — mid-step on Fix")).quiet).toBeUndefined();
  });
});
