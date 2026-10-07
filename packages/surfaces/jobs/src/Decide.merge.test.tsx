// Merge follows the forge's checks: passed is Merge, running is Enable auto-merge and sends the
// auto-merge press, failed is off with the names, and a pull request already asked for reads
// Auto-merge on and cannot be asked again.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { JobSummary, PullRequestChecks, PullRequestState } from "@armada/protocol";
import { Decide } from "./Decide";
import { mount, unmount } from "@armada/screens/src/mounted";

afterEach(unmount);

const JOB: JobSummary = {
  id: "01M130Y1380016YK5S0JXBXDQ5",
  handle: "12-a-job",
  title: "Coalesce concurrent token refreshes",
  status: "awaiting_review",
  workflow_id: "bug",
  owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
  origin: "dispatched",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  created_at: "2026-08-31T09:00:00Z",
  branch: "armada/01M130Y1380016YK5S0JXBXDQ5",
};

const READING: PullRequestState = {
  manifest_id: JOB.owner_manifest_id,
  number: 533,
  state: "open",
  auto_merge: true,
  checks: { state: "pending" },
  title: "Coalesce",
  branch: "armada/x",
  address: "https://git.example/armada/armada/pull/533",
};

function gate(checks: PullRequestChecks, forgeReading?: PullRequestState) {
  const sent = { merged: [] as string[], auto: [] as string[] };
  mount(
    <Decide
      onNeedMaterial={() => {}}
      onNeedRemarks={() => {}}
      job={JOB}
      evidence={{ state: "none" }}
      remarks={{ state: "none" }}
      stale={false}
      deciding={false}
      pullRequest="https://git.example/armada/armada/pull/533"
      checks={checks}
      forgeReading={forgeReading}
      onMerge={(id) => sent.merged.push(id)}
      onAutoMerge={(id) => sent.auto.push(id)}
      onApprove={() => {}}
      onRequestChanges={() => {}}
      onReject={() => {}}
      onTakeUpRemarks={() => {}}
      onOpenRemarkLink={() => {}}
    />,
  );
  return sent;
}

test("passed checks leave Merge as it was", async () => {
  const sent = gate({ kind: "all_passed", checks: 4, finished: 4 });
  await userEvent.click(page.getByRole("button", { name: "Merge pull request" }));
  await userEvent.click(page.getByRole("dialog").getByRole("button", { name: "Merge pull request" }));
  expect(sent.merged).toEqual([JOB.id]);
  expect(sent.auto).toEqual([]);
});

test("running checks make it Enable auto-merge, which sends the auto-merge press", async () => {
  const sent = gate({ kind: "still_waiting", checks: 4, finished: 2 });
  await userEvent.click(page.getByRole("button", { name: "Enable auto-merge" }));
  await userEvent.click(page.getByRole("dialog").getByRole("button", { name: "Enable auto-merge" }));
  expect(sent.auto).toEqual([JOB.id]);
  expect(sent.merged).toEqual([]);
});

test("failed checks turn Merge off and name the failing ones", async () => {
  gate({ kind: "some_failed", checks: 4, finished: 4, failed: ["ci / test", "ci / lint"] });
  await expect.element(page.getByRole("button", { name: "Merge pull request" })).toBeDisabled();
  await expect.element(page.getByText("ci / test, ci / lint failed")).toBeVisible();
});

test("auto-merge already asked for reads Auto-merge on and cannot be pressed again", async () => {
  gate({ kind: "still_waiting", checks: 4, finished: 2 }, READING);
  await expect.element(page.getByRole("button", { name: "Auto-merge on" })).toBeDisabled();
});
