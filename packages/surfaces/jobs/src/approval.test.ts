// What the approval press sends (`approve_dispatch`, 23.8): what a person
// moved since Fleet served the proposal, and nothing else.

import { describe, expect, it } from "vitest";
import type { Criterion, JobDetail } from "@armada/protocol";

import { sampleDetail, sampleStep } from "@armada/screens/src/draft/sample";
import { approvalOf, landingValueOf, landingWith, proposalEditsOfWhole } from "./tab-proposal-read";
import type { ProposalEdits } from "./tab-proposal-read";

const ISSUE = { kind: "issue", ref: "armada#1162", url: "https://example.invalid/armada/issues/1162" };

function atGate(over: Partial<JobDetail> = {}): JobDetail {
  const detail = sampleDetail({
    steps: [
      sampleStep({ step_id: "plan", advance_gate: "auto_if_judge_passes" }),
      sampleStep({ step_id: "handoff", advance_gate: "manifest_rule:auto_merge" }),
    ],
    acceptance_criteria: [
      { criterion_id: "c1", text: "Guide 8 is gone", source: "judge", origin: ISSUE },
      { criterion_id: "c2", text: "The catalogue opens", source: "check" },
    ] satisfies Criterion[],
    ...over,
  });
  detail.job.status = "awaiting_approval";
  return detail;
}

const read = (detail: JobDetail = atGate()): ProposalEdits => proposalEditsOfWhole(detail, 4);

describe("the approval body", () => {
  it("is nothing where nothing moved, which approves the proposal as it stands", () => {
    expect(approvalOf(read(), read(), [])).toBeUndefined();
  });

  it("carries a new title and only that", () => {
    const before = read();
    const edits = { ...before, proposal: { ...before.proposal, title: "  A better title " } };

    expect(approvalOf(edits, before, [])).toEqual({ title: "A better title" });
  });

  it("sends only the gate a person moved, as all four of its answers", () => {
    const before = read();
    const gates = before.proposal.gates.map((gate) => (gate.step_id === "plan" ? { ...gate, you: true } : gate));

    expect(approvalOf({ ...before, proposal: { ...before.proposal, gates } }, before, [])).toEqual({
      gates: [{ step_id: "plan", checks: false, judge: true, you: true }],
    });
  });

  it("sends an override on a step that defers, and nothing for the step beside it", () => {
    const before = read();
    const gates = before.proposal.gates.map((gate) =>
      gate.step_id === "handoff" ? { ...gate, overridden: true } : gate,
    );

    expect(approvalOf({ ...before, proposal: { ...before.proposal, gates } }, before, [])?.gates).toEqual([
      { step_id: "handoff", checks: false, judge: false, you: false, overridden: true },
    ]);
  });

  // #1641's criteria, not lines: a line with an id keeps its origin.
  it("sends the criteria whole once one is reworded, each with its id and how it is answered", () => {
    const before = read();
    const criteria = before.criteria.map((one, at) => (at === 0 ? { ...one, text: "Guide 8 is retired" } : one));

    expect(approvalOf({ ...before, criteria }, before, [])).toEqual({
      criteria: [
        { criterion_id: "c1", text: "Guide 8 is retired", source: "judge" },
        { criterion_id: "c2", text: "The catalogue opens", source: "check" },
      ],
    });
  });

  it("drops a line left blank rather than sending a criterion with no words", () => {
    const before = read();
    const criteria = [...before.criteria, { text: "  ", verified_by: "judge" as const, origin: { origin: "person" as const } }];

    expect(approvalOf({ ...before, criteria }, before, [])).toBeUndefined();
  });

  // A cap left out is the machine's, so a cap the Job holds goes every time.
  it("sends the Drone cap whenever there is one", () => {
    const before = read(atGate({ drone_cap: 2 }));

    expect(approvalOf(before, before, [])).toEqual({ drone_cap: 2 });
  });

  it("sends a tier map with a tier left out as Armada picking", () => {
    const before = read();
    const edits = { ...before, proposal: { ...before.proposal, tiers: { ...before.proposal.tiers, difficult: "opus" } } };

    expect(approvalOf(edits, before, [])).toEqual({ tiers: { difficult: "opus" } });
  });

  it("sends where it lands, leaving the base it was drawn with as the base", () => {
    const before = read();
    const drawn = landingValueOf(before.landing, "main");
    const landing = landingWith(before.landing, { ...drawn, target: "release/2026-10" }, "main");

    expect(drawn).toMatchObject({ target: "main", from: "main" });
    expect(approvalOf({ ...before, landing }, before, [])).toEqual({
      landing: { target: "release/2026-10", branching: "job", pr_mode: "ready", complete_when: "delivered" },
    });
  });

  it("sends a base nobody has cut with the repository's base as its start point", () => {
    const before = read();
    const branches = [{ name: "main", base: true }, { name: "release/2026-09", base: false }];
    const drawn = landingValueOf(before.landing, "main");
    const cut = landingWith(before.landing, { ...drawn, from: "release/2026-10" }, "main");
    const held = landingWith(before.landing, { ...drawn, from: "release/2026-09" }, "main");

    expect(approvalOf({ ...before, landing: cut }, before, [], branches)?.landing).toMatchObject({
      from_ref: "release/2026-10",
      start_point: "main",
    });
    expect(approvalOf({ ...before, landing: held }, before, [], branches)?.landing).not.toHaveProperty("start_point");
  });

  it("sends local, or auto-merge, and never both", () => {
    const before = read();
    const local = { ...before.landing, local: true, auto_merge: true };
    const merges = { ...before.landing, auto_merge: true };

    expect(approvalOf({ ...before, landing: local }, before, [])?.landing).toMatchObject({ local: true });
    expect(approvalOf({ ...before, landing: local }, before, [])?.landing).not.toHaveProperty("auto_merge");
    expect(approvalOf({ ...before, landing: merges }, before, [])?.landing).toMatchObject({ auto_merge: true });
  });

  it("sends what a step was tuned to, and only what moved off the step as declared", () => {
    const before = read();
    const tuning = {
      steps: {
        plan: { model: "opus", effort: "high" as const, harness: "codex", context: " Mind the schema ", judges: 3, checks_off: ["build"] },
        handoff: { model: null, effort: null, harness: null, context: "", judges: 1, checks_off: [] },
      },
    };

    expect(approvalOf({ ...before, tuning }, before, [])).toEqual({
      tuning: [{ step_id: "plan", model: "opus", effort: "high", harness: "codex", context: " Mind the schema ", judges: 3, checks_off: ["build"] }],
    });
  });
});

describe("a real Job's proposal", () => {
  it("reads its criteria with where they came from and when the issue moved", () => {
    const detail = atGate();
    detail.acceptance_criteria[0]!.origin_moved_at = "2026-10-02T15:41:09Z";
    const [first] = read(detail).criteria;

    expect(first?.origin).toEqual({ origin: "issue", ref: "armada#1162", url: ISSUE.url });
    expect(first?.origin_moved_at).toBe("2026-10-02T15:41:09Z");
  });

  it("lands where Fleet froze it, and in the base where it froze nothing", () => {
    expect(read(atGate({ landing: { target: "release/2026-10", pr_mode: "draft" } })).landing).toMatchObject({
      target: "release/2026-10",
      from_ref: null,
      pr_mode: "draft",
    });
    expect(read().landing).toMatchObject({ target: null, from_ref: null, pr_mode: "ready" });
  });
});
