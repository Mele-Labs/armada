// What the mock's Fleet refuses and echoes at the gate (23.20–23.25).

import { expect, it } from "vitest";
import { sampleDetail, sampleStep } from "@armada/screens/src/draft/sample";

import { approvedAs, landingRefusal, sentBack } from "./approval-fleet";

const atGate = () => {
  const detail = sampleDetail({ steps: [sampleStep({ step_id: "plan" })] });
  detail.job.status = "awaiting_approval";
  return detail;
};

it("refuses a note that is blank, and a proposal past its gate", () => {
  expect(sentBack(atGate(), { note: "  " })).toMatchObject({ why: "refused", error: { code: "fleet.unacceptable_proposal" } });
  const past = atGate();
  past.job.status = "queued";
  expect(sentBack(past, { note: "again" })).toMatchObject({ error: { code: "fleet.proposal_frozen" } });
});

it("returns the Job at its gate with the landing it was sent", () => {
  const back = sentBack(atGate(), { note: "smaller", landing: { local: true } });
  expect(back).toMatchObject({ job: { status: "awaiting_approval" }, landing: { local: true, pr_mode: "ready" } });
});

it("refuses local with auto-merge, and a harness it does not run", () => {
  expect(landingRefusal({ local: true, auto_merge: true })).toMatchObject({ error: { code: "fleet.unacceptable_proposal" } });
  expect(sentBack(atGate(), { note: "x", tuning: [{ step_id: "plan", harness: "other" }] })).toMatchObject({ why: "refused" });
});

it("freezes the delivery the approval chose", () => {
  const approved = approvedAs(atGate(), { landing: { auto_merge: true } }, "2026-10-05T09:00:00Z");
  expect(approved.landing).toMatchObject({ auto_merge: true, pr_mode: "ready" });
});
