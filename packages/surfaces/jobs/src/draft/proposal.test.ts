// Four gate states per step, and the line the ticks cannot turn off.

import { describe, expect, it } from "vitest";

import { gateReadingOf, gateViewOf, proposalViewOf, unmeantOf } from "./proposal";
import type { GateView } from "./proposal";
import { sampleDetail, sampleStep } from "@armada/screens/src/draft/sample";

describe("what gates one step", () => {
  it("reads auto as the Checks deciding and nobody looking", () => {
    const gate = gateViewOf(
      sampleStep({ advance_gate: "auto", checks: [{ kind: "manifest_check", name: "test" }] }),
    );

    expect(gate).toEqual({ step_id: "implement", checks: true, judge: false, you: false });
  });

  it("reads auto_if_judge_passes as the Judge looking", () => {
    expect(gateViewOf(sampleStep({ advance_gate: "auto_if_judge_passes" })).judge).toBe(true);
  });

  it("reads human_always as stopping for you", () => {
    expect(gateViewOf(sampleStep({ advance_gate: "human_always" })).you).toBe(true);
  });

  it("holds the repository's rule as a fourth state, not as three bits", () => {
    const merge = gateViewOf(sampleStep({ advance_gate: "manifest_rule:auto_merge" }));
    const review = gateViewOf(sampleStep({ advance_gate: "manifest_rule:review_gate" }));

    expect(merge.repository_decides).toBe("auto_merge");
    expect(review.repository_decides).toBe("review_gate");
  });

  it("says whether this Job overrode that rule, and only where one applies", () => {
    const repository = gateViewOf(
      sampleStep({ advance_gate: "manifest_rule:auto_merge", overridden: true }),
    );
    const ordinary = gateViewOf(sampleStep({ advance_gate: "auto", overridden: true }));

    expect(repository.overridden).toBe(true);
    expect(ordinary.overridden).toBeUndefined();
  });

  it("reads Fleet not knowing the workflow as stopping for you, never as ungated", () => {
    const gate = gateViewOf(sampleStep());

    expect(gate.you).toBe(true);
    expect(gate.checks).toBe(false);
  });

  it("says the Judge looks where the step declares a tier, whatever the gate", () => {
    const gate = gateViewOf(
      sampleStep({
        advance_gate: "auto",
        judge_checks: [{ criteria: 2, gaming_check: false }],
      }),
    );

    expect(gate.judge).toBe(true);
  });
});

describe("the line no tick turns off", () => {
  it("is always true, so a screen cannot draw a step as unwatched", () => {
    const view = proposalViewOf(sampleDetail(), 4);

    expect(view.fleet_always_looks).toBe(true);
  });
});

describe("what locks at approval", () => {
  it("reads a tier the map leaves out as Armada picking", () => {
    const detail = sampleDetail();
    detail.tiers = { difficult: "opus" };

    expect(proposalViewOf(detail, 4).tiers).toEqual({
      difficult: "opus",
      medium: null,
      easy: null,
    });
  });

  it("takes the machine's cap as given, and null before Fleet said", () => {
    expect(proposalViewOf(sampleDetail(), 4).machine_cap).toBe(4);
    expect(proposalViewOf(sampleDetail(), null).machine_cap).toBeNull();
  });

  it("caps this Job's Drones where Fleet kept a cap, and leaves it out where none", () => {
    const capped = sampleDetail();
    capped.drone_cap = 2;

    expect(proposalViewOf(capped, 4).drone_cap).toBe(2);
    expect(proposalViewOf(sampleDetail(), 4).drone_cap).toBeUndefined();
  });

  it("reads where it starts and how its pull request opens off the landing", () => {
    const landed = sampleDetail();
    landed.landing = { from_ref: "release/2.4", pr_mode: "draft" };

    expect(proposalViewOf(landed, 4)).toMatchObject({ from_ref: "release/2.4", pr_mode: "draft" });
    // Absent is the Manifest's base, which is not a branch name to print.
    expect(proposalViewOf(sampleDetail(), 4)).toMatchObject({ from_ref: null, pr_mode: "ready" });
  });

  it("carries no notes for the planner, which was dropped", () => {
    const view = proposalViewOf(sampleDetail(), 4);

    expect("notes_for_planner" in view).toBe(false);
  });

  it("dates the approval from approved_at, never from when the Job started", () => {
    const approved = sampleDetail();
    approved.approved_at = "2026-09-22T09:02:00Z";
    const started = sampleDetail();
    started.job.started_at = "2026-09-22T09:05:00Z";

    expect(proposalViewOf(approved, 4).approved_at).toBe("2026-09-22T09:02:00Z");
    expect(proposalViewOf(started, 4).approved_at).toBeUndefined();
  });

  it("gives one gate per step of the frozen workflow", () => {
    const detail = sampleDetail({
      steps: [sampleStep({ step_id: "plan" }), sampleStep({ step_id: "implement" })],
    });

    expect(proposalViewOf(detail, 4).gates.map((gate) => gate.step_id)).toEqual([
      "plan",
      "implement",
    ]);
  });
});


/** One gate, with the three boxes off unless the case turns one on. */
const boxes = (over: Partial<GateView> = {}): GateView => ({
  step_id: "implement",
  checks: false,
  judge: false,
  you: false,
  ...over,
});

describe("what a combination of the boxes is on the wire", () => {
  it("reads nothing ticked as auto, which stops nothing", () => {
    const reading = gateReadingOf(boxes());

    expect(reading.advance_gate).toBe("auto");
    expect(reading.does).toBe("Nothing stops it.");
  });

  it("reads Checks alone as auto with the Checks as the whole gate", () => {
    expect(gateReadingOf(boxes({ checks: true })).advance_gate).toBe("auto");
  });

  it("reads Checks and a Judge as auto_if_judge_passes", () => {
    expect(gateReadingOf(boxes({ checks: true, judge: true })).advance_gate).toBe(
      "auto_if_judge_passes",
    );
  });

  // The combination the issue is named after, and it is not meaningless: the
  // `feature` workflow's own plan step is a Judge and no Check, and with
  // nothing mechanical to fail, the Judge refusing is all that holds it.
  it("reads a Judge with no Checks as advancing unless the Judge refuses", () => {
    const reading = gateReadingOf(boxes({ judge: true }));

    expect(reading.advance_gate).toBe("auto_if_judge_passes");
    expect(reading.does).toBe("It advances unless the Judge refuses it.");
  });

  it("reads You as human_always, and says what ran before you read it", () => {
    const alone = gateReadingOf(boxes({ you: true }));
    const after = gateReadingOf(boxes({ you: true, checks: true, judge: true }));

    expect(alone.advance_gate).toBe("human_always");
    expect(alone.does).toContain("nothing run before you read it");
    expect(after.does).toContain("its Checks and the Judge");
  });

  it("reads a step the repository decides as the repository's, whatever is ticked", () => {
    const reading = gateReadingOf(
      boxes({ you: true, repository_decides: "review_gate", overridden: false }),
    );

    expect(reading.advance_gate).toBe("manifest_rule:review_gate");
    expect(reading.does).toContain("review_gate policy");
  });

  // What the deference resolves to is the thing the row could not say
  // (`rhxt`, 29 Sep): *the repository decides* leaves a reader unable to tell
  // whether a person will be asked or nobody will.
  it("says what the repository's word means, in the verb generated for it", () => {
    const gate = boxes({ repository_decides: "review_gate", overridden: false });

    expect(gateReadingOf(gate, { review_gate: "human_always" }).does).toContain(
      "a person answers",
    );
    expect(gateReadingOf(gate, { review_gate: "auto_if_judge_passes" }).does).toContain(
      "the checks decide, unless the Judge objects",
    );
    // The policy is read again at every gate, so the Job moves with it.
    expect(gateReadingOf(gate, { review_gate: "human_always" }).does).toContain(
      "for this Job as well",
    );
  });

  it("reads auto_merge's words off its own table", () => {
    const gate = boxes({ repository_decides: "auto_merge", overridden: false });

    expect(gateReadingOf(gate, { auto_merge: "checks-pass" }).does).toContain(
      "Fleet merges once the forge's checks pass",
    );
  });

  // Naming `human_always` because it is the documented default would be this
  // screen answering for a repository nothing has read.
  it("claims nothing about a policy no word was read for", () => {
    const gate = boxes({ repository_decides: "review_gate", overridden: false });

    expect(gateReadingOf(gate).does).toBe(
      "The repository's review_gate policy decides whether a person signs off.",
    );
    expect(gateReadingOf(gate, { auto_merge: "never" }).does).not.toContain("Today");
  });

  // Overriding hands the step back to the three boxes, which is the whole of
  // what "you can override it for this Job" means — #1548.
  it("reads an overridden step as its own boxes again", () => {
    const reading = gateReadingOf(
      boxes({ you: true, repository_decides: "review_gate", overridden: true }),
    );

    expect(reading.advance_gate).toBe("human_always");
  });

  // Fleet lays the override over the repository's word (23.8): `auto_merge`
  // with nobody ticked reads `checks-pass` (owner, 2 Oct 2026), and either way
  // the override outlives a change to the repository's rule.
  it("says what an override stands in for, and that it holds however the rule moves", () => {
    const merge = gateReadingOf(boxes({ repository_decides: "auto_merge", overridden: true }));
    const review = gateReadingOf(
      boxes({ judge: true, repository_decides: "review_gate", overridden: true }),
    );

    expect(merge.does).toBe(
      "Overridden for this Job: Fleet merges once the forge's checks pass, however the repository's auto_merge moves.",
    );
    expect(review.does).toContain("the checks decide, unless the Judge objects");
  });
});


describe("what a tick asks for that Fleet cannot do", () => {
  const declared = { checks: true, judge: true };

  it("is nothing where the step declares what the ticks ask for", () => {
    expect(unmeantOf(boxes({ checks: true, judge: true }), declared)).toBeUndefined();
    expect(unmeantOf(boxes({ you: true }), { checks: false, judge: false })).toBeUndefined();
  });

  // `mechanical_checks[]` and `judge_checks[]` are the workflow's and are
  // frozen at creation, so a tick moves the gate and never what runs at it.
  it("names a Check asked for on a step that declares none", () => {
    expect(unmeantOf(boxes({ checks: true }), { checks: false, judge: true })).toContain(
      "declares no Check",
    );
  });

  it("names a Judge asked for on a step that declares nothing to read", () => {
    expect(unmeantOf(boxes({ judge: true }), { checks: true, judge: false })).toContain(
      "nothing for a Judge to read",
    );
  });

  // Since 23.8 an override holds for the life of the Job (spike 022, answer
  // 4), so it is an answer and asks for nothing Fleet cannot do.
  it("reads an override as an ordinary gate", () => {
    expect(
      unmeantOf(boxes({ you: true, repository_decides: "review_gate", overridden: true }), declared),
    ).toBeUndefined();
  });
});
