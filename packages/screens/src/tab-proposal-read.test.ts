// What the classifying screen reads off a draft, and what a person's change
// puts back on it.

import { describe, expect, it } from "vitest";

import type { CriterionView } from "./draft/criterion";
import type { LandingRule } from "./draft/landing";
import type { GateView, ProposalView } from "./draft/proposal";
import { sampleDetail, sampleStep } from "./draft/sample";
import type { WorkflowSummary } from "@armada/protocol";
import {
  completeChoices,
  criteriaAdded,
  criteriaRowsOf,
  criteriaWith,
  criteriaWithout,
  frozenAtOf,
  gateRowsOf,
  gatesWith,
  landingValueOf,
  landingWith,
  proposalEditsOf,
  proposalOnWorkflow,
  workflowChoicesOf,
} from "./tab-proposal-read";

const GATES: GateView[] = [
  { step_id: "plan", checks: false, judge: true, you: false },
  { step_id: "handoff", checks: false, judge: false, you: false, repository_decides: "review_gate" },
];

const PROPOSAL: ProposalView = {
  status: "awaiting_approval",
  title: "Show what is running in the Drones stat",
  workflow_id: "feature",
  gates: GATES,
  fleet_always_looks: true,
  tiers: { difficult: "opus", medium: "sonnet", easy: null },
  drone_cap: 2,
  machine_cap: 4,
  from_ref: "main",
  pr_mode: "ready",
};

const LANDING: LandingRule = {
  target: "main",
  from_ref: "main",
  prs: "job",
  branching: "job",
  pr_mode: "ready",
  complete_when: "pr_merged",
  land_together: [],
};

describe("what the screen opens on", () => {
  it("is nothing at all where the Job is at no proposal", () => {
    expect(proposalEditsOf(undefined)).toBeUndefined();
    expect(proposalEditsOf({})).toBeUndefined();
  });

  // A Manifest that has not been read names no base, and `null` is how
  // `amending.ts` already spells that — never the default branch by another name.
  it("lands in nothing where the moment carries no landing rule", () => {
    const edits = proposalEditsOf({ proposal: PROPOSAL });

    expect(edits?.landing.target).toBeNull();
    expect(edits?.landing.from_ref).toBe("main");
    expect(edits?.criteria).toEqual([]);
  });
});

describe("one row per step", () => {
  it("takes the step's own label off the Job, and its id where Fleet has no record", () => {
    const whole = sampleDetail({
      steps: [sampleStep({ step_id: "plan", label: "Plan the change" })],
    });
    const rows = gateRowsOf(GATES, whole);

    expect(rows[0]?.label).toBe("Plan the change");
    expect(rows[1]?.label).toBe("handoff");
  });

  it("carries the reading of the combination onto the row", () => {
    const rows = gateRowsOf(GATES, null);

    expect(rows[0]?.advanceGate).toBe("auto_if_judge_passes");
    expect(rows[1]?.repositoryDecides).toBe("review_gate");
  });

  // What the step declares is what a tick cannot move, so the warning is read
  // against the frozen step rather than against the boxes alone.
  it("says where a box asks for something the step declares nothing for", () => {
    const declares = sampleDetail({
      steps: [sampleStep({ step_id: "plan", judge_checks: [{ criteria: 2, gaming_check: false }] })],
    });

    expect(gateRowsOf(GATES, declares)[0]?.unmeant).toBeUndefined();
    expect(gateRowsOf(gatesWith(GATES, "plan", { checks: true }), declares)[0]?.unmeant).toContain(
      "declares no Check",
    );
  });

  it("moves one box on one step and leaves the others alone", () => {
    const moved = gatesWith(GATES, "plan", { checks: true });

    expect(moved[0]).toEqual({ ...GATES[0], checks: true });
    expect(moved[1]).toBe(GATES[1]);
  });
});

describe("how it lands", () => {
  it("draws a ref the Manifest does not name as empty rather than as a branch", () => {
    expect(landingValueOf({ ...LANDING, target: null }).target).toBe("");
  });

  it("puts an emptied field back as null, which is the Manifest naming none", () => {
    const back = landingWith(LANDING, { ...landingValueOf(LANDING), target: "" });

    expect(back.target).toBeNull();
    expect(back.from_ref).toBe("main");
  });

  // `COMPLETE_WHEN_SERVED` is read rather than restated, so an answer Fleet
  // learns to observe stops being flagged without this file being touched.
  it("says which answers nothing on a Job's record can tell yet", () => {
    const choices = completeChoices();

    expect(choices.map((one) => one.value)).toContain("pr_merged");
    expect(choices.find((one) => one.value === "pr_merged")?.served).toBe(false);
    expect(choices.find((one) => one.value === "delivered")?.served).toBe(true);
  });
});

describe("what the Job is held to", () => {
  const criteria: CriterionView[] = [
    {
      criterion_id: "a1",
      text: "The stat reads one running",
      verified_by: "check",
      origin: { origin: "issue", ref: "armada/1162" },
      origin_moved_at: "2026-09-22T10:02:00Z",
    },
    {
      text: "Pressing it lists the Drone",
      verified_by: "judge",
      origin: { origin: "prompt" },
    },
  ];

  it("says where each line's words came from, apart from what will decide it", () => {
    const rows = criteriaRowsOf(criteria);

    expect(rows[0]?.origin).toBe("From issue");
    expect(rows[0]?.issue?.ref).toBe("armada/1162");
    expect(rows[0]?.decidedBy).toBe("A Check will decide it");
    expect(rows[1]?.origin).toBe("From your prompt");
    expect(rows[1]?.issue).toBeUndefined();
  });

  // The Job keeps the words it froze and says the issue has moved since — the
  // instant is the issue's own edit, never the freeze.
  it("carries the issue having moved on the line it moved under", () => {
    const rows = criteriaRowsOf(criteria);

    expect(rows[0]?.movedSince).not.toBeUndefined();
    expect(rows[1]?.movedSince).toBeUndefined();
  });

  it("rewords one line and leaves the rest as they were", () => {
    const moved = criteriaWith(criteria, 1, "Pressing it lists the Drone and its step");

    expect(moved[0]).toBe(criteria[0]);
    expect(moved[1]?.text).toBe("Pressing it lists the Drone and its step");
  });

  // The order is the brief's and a citation names a criterion's place in it,
  // so a line inserted above the others renumbers every citation written.
  it("appends a new line at the foot, never above what is already there", () => {
    const grown = criteriaAdded(criteria);

    expect(grown).toHaveLength(3);
    expect(grown[0]).toBe(criteria[0]);
    expect(grown[2]?.text).toBe("");
    expect(grown[2]?.origin).toEqual({ origin: "person" });
  });

  it("takes one line off and carries the rest through untouched", () => {
    const shrunk = criteriaWithout(criteria, 0);

    expect(shrunk).toHaveLength(1);
    expect(shrunk[0]).toBe(criteria[1]);
  });

  it("leaves nothing behind when the last line goes", () => {
    expect(criteriaWithout(criteriaWithout(criteria, 0), 0)).toEqual([]);
  });
});

describe("picking another workflow", () => {
  const workflow = (id: string, steps: string[]): WorkflowSummary => ({
    id,
    name: id,
    version: 1,
    manifest_id: "01M",
    steps: steps.map((step_id) => ({
      step_id,
      label: step_id,
      checks: [],
      judge_checks: [],
      advance_gate: "auto_if_judge_passes",
      delivers: false,
    })),
  });
  const held = [workflow("feature", ["plan", "handoff"]), workflow("bug", ["reproduce"])];

  // A gate belongs to a step, so ticks moved on the old workflow name steps
  // the new one may not have. Carrying them across by position would put a
  // tick meant for `handoff` on whatever runs second.
  it("rebuilds every gate from the steps the new workflow declares", () => {
    const moved = proposalOnWorkflow(PROPOSAL, held, "bug");

    expect(moved.workflow_id).toBe("bug");
    expect(moved.gates.map((gate) => gate.step_id)).toEqual(["reproduce"]);
  });

  // There are no steps to rebuild from, and emptying the list would draw a
  // workflow with no steps at all.
  it("leaves the gates alone on a workflow this Fleet has no record of", () => {
    const moved = proposalOnWorkflow(PROPOSAL, held, "carried");

    expect(moved.workflow_id).toBe("carried");
    expect(moved.gates).toBe(PROPOSAL.gates);
  });

  // A workflow belongs to a Manifest, and a Job cannot run one declared for
  // another repository — offering them is a picker mostly of refusals.
  it("offers this Job's repository's workflows and no others", () => {
    const elsewhere: WorkflowSummary = { ...workflow("review", ["look"]), manifest_id: "02M" };
    const choices = workflowChoicesOf([...held, elsewhere], "01M");

    expect(choices.map((one) => one.id)).toEqual(["feature", "bug"]);
    expect(choices[0]?.steps).toBe(2);
  });
});

describe("when it froze", () => {
  it("is nothing at all while nobody has approved it", () => {
    expect(frozenAtOf(PROPOSAL)).toBeUndefined();
  });

  // A frozen proposal whose instant cannot be read is still frozen: drawing it
  // as editable would offer controls over values the Job is already running on.
  it("falls back to the instant itself where it cannot be written out", () => {
    expect(frozenAtOf({ ...PROPOSAL, approved_at: "not an instant" })).toBe("not an instant");
    expect(frozenAtOf({ ...PROPOSAL, approved_at: "2026-09-22T09:14:00Z" })).toContain("2026");
  });
});
