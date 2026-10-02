// What a Job is being approved for, through `App`, **on a Fleet that serves no
// draft** — every real one until #1545. The owner approved his Job 1 from
// Overview's lead on 1 Oct 2026 with the request, the workflow's name and the
// title in front of him and nothing else; the proposer had picked `refactor`
// for a visible change, and the Judge refused the plan for it. What counts as
// done, what the workflow promises and how each step gates now read under the
// lead, while it waits.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { REFACTOR_FOR_REQUESTS, refactorAtApproval, withRow } from "./job-detail-fixtures";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const approving = () => page.getByRole("region", { name: "What you are approving" });

test("Job 1 at its gate reads what counts as done and how each step gates, under the lead", async () => {
  mount(onJob(refactorAtApproval()));
  await expect.element(page.getByRole("heading", { name: "Waiting for your approval" })).toBeVisible();

  const done = approving().getByRole("region", { name: "Done when" });
  await expect.element(done.getByText("Guide 8 is removed from the catalogue")).toBeVisible();
  await expect
    .element(done.getByText("A validation rule prevents guides without drawn pieces"))
    .toBeVisible();

  // Who decides each step, read off its frozen `advance_gate`: Checks and the
  // Judge on the two that do the work, a person on the one that hands it over.
  const workflow = approving().getByRole("region", { name: "Workflow" });
  await expect.element(workflow.getByText("refactor", { exact: true })).toBeVisible();
  const judged = "Checks · JudgeIts Checks have to pass and the Judge has to decline to refuse them.";
  await expect
    .element(workflow.getByRole("listitem", { name: "Scope the refactor" }))
    .toHaveTextContent(judged);
  await expect
    .element(workflow.getByRole("listitem", { name: "Restructure" }))
    .toHaveTextContent(judged);
  await expect
    .element(workflow.getByRole("listitem", { name: "Review the change" }))
    .toHaveTextContent(/^Review the changeYouIt holds at awaiting review for you to answer/);
  // The registry's word for the status, never its id (`#1748` row 16).
  await expect.element(workflow.getByText(/awaiting_review/)).not.toBeInTheDocument();

  // Read, never moved: editing waits for #1545.
  expect(approving().getByRole("checkbox").all()).toHaveLength(0);
  expect(approving().getByRole("textbox").all()).toHaveLength(0);
  expect(approving().getByRole("combobox").all()).toHaveLength(0);

  // Between the lead and the Brief.
  const lead = document.querySelector(".armada-lead");
  const region = document.querySelector('[aria-label="What you are approving"]');
  const brief = document.querySelector('[aria-label="Brief"]');
  expect(lead?.nextElementSibling).toBe(region);
  expect(region?.compareDocumentPosition(brief as Node)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
});

test("once the Job is running nothing draws: the approval is behind it", async () => {
  mount(
    onJob(
      withRow(refactorAtApproval(), {
        status: "running",
        branch: "armada/1-retire-guide-8-and-add-guide-validation-ru",
        started_at: "2026-10-01T18:33:36.691Z",
      }),
    ),
  );
  await expect.element(page.getByRole("region", { name: "Brief" })).toBeVisible();
  expect(approving().all()).toHaveLength(0);
});

test("the workflow's promise reads under its name, from this repository's row of the list", async () => {
  mount(onJob(refactorAtApproval()));
  const workflow = approving().getByRole("region", { name: "Workflow" });
  await expect.element(workflow.getByText(REFACTOR_FOR_REQUESTS)).toBeVisible();
  expect(workflow.getByText("Another repository's refactor", { exact: false }).all()).toHaveLength(0);
});

test("a workflow that declares no promise draws nothing under its name", async () => {
  const fixture = refactorAtApproval();
  mount(
    onJob({
      ...fixture,
      workflows: fixture.workflows.map(({ for_requests: _none, ...row }) => row),
    }),
  );
  const workflow = approving().getByRole("region", { name: "Workflow" });
  await expect.element(workflow.getByText("refactor", { exact: true })).toBeVisible();
  expect(document.querySelector(".armada-proposal__workflow-promise")).toBeNull();
});
