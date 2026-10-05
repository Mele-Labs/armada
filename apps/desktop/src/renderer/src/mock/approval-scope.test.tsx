// What a Job is being approved for, through `App`, **on a Fleet that serves no
// draft**. The owner approved his Job 1 from Overview's lead on 1 Oct 2026 with
// the request, the workflow's name and the title in front of him and nothing
// else; the proposer had picked `refactor` for a visible change, and the Judge
// refused the plan for it. What counts as done, what the workflow promises and
// how each step gates now read under the lead, while it waits — and since
// 23.8 each is a person's to change before the press. Since the approval
// canvas (prototype, 4 Oct 2026) each is on the node it belongs to, read by
// opening that node.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { REFACTOR_FOR_REQUESTS, refactorAtApproval, withRow } from "./job-detail-fixtures";
import { proposalFromAnIssue } from "./proposal-from-an-issue";
import { onJob } from "./scenario";
import { mount, openNode, unmountAfterEach } from "./testing";

unmountAfterEach();

const approving = () => page.getByRole("region", { name: "What you are approving" });

test("Job 1 at its gate reads what counts as done and how each step gates, under the lead", async () => {
  mount(onJob(refactorAtApproval()));
  await expect.element(page.getByRole("heading", { name: "Waiting for your approval" })).toBeVisible();

  const brief = await openNode("Brief");
  const done = brief.getByRole("region", { name: "Done when" });
  await expect.element(done.getByRole("textbox", { name: "Criterion 1" })).toHaveValue(
    "Guide 8 is removed from the catalogue",
  );
  await expect
    .element(done.getByRole("textbox", { name: "Criterion 2" }))
    .toHaveValue("A validation rule prevents guides without drawn pieces");
  // Yours to change until the press: `approve_dispatch` carries it since 23.8.
  await expect.element(brief.getByRole("textbox", { name: "Title", exact: true })).toHaveValue(
    "Retire guide 8 and add guide validation rule",
  );

  // The Work lane's head is the workflow picker.
  await expect.element(approving().getByRole("combobox", { name: "Workflow", exact: true })).toHaveValue("refactor");

  // Who decides each step, read off its frozen `advance_gate`: Checks and the
  // Judge on the two that do the work, each its own gate node in run order.
  for (const [at, step] of ["Scope the refactor", "Restructure"].entries()) {
    const gate = await openNode("Checks", at);
    await expect.element(gate.getByRole("checkbox", { name: `Checks on ${step}` })).toBeChecked();
    await expect.element(gate.getByRole("checkbox", { name: `Judge on ${step}` })).toBeChecked();
    await expect.element(gate.getByRole("checkbox", { name: `You on ${step}` })).not.toBeChecked();
    if (at === 0) {
      await expect
        .element(gate.getByRole("listitem", { name: "Scope the refactor" }))
        .toHaveTextContent("Its Checks have to pass and the Judge has to decline to refuse them.");
    }
  }
  // A person on the one that hands it over, whose gate is on its own card.
  const review = await openNode("Review the change");
  await expect.element(review.getByRole("checkbox", { name: "You on Review the change" })).toBeChecked();
  await expect
    .element(review.getByRole("listitem", { name: "Review the change" }))
    .toHaveTextContent("It reads needs review until you answer");
  // The registry's word for the status, never its id (`#1748` row 16).
  await expect.element(review.getByText(/awaiting_review/)).not.toBeInTheDocument();

  // No branch list read for this Job, so the field takes a typed name.
  const land = await openNode("Land");
  await expect.element(land.getByRole("textbox", { name: "Lands in" })).toBeVisible();

  // Straight under the lead.
  const lead = document.querySelector(".armada-lead");
  const region = document.querySelector('[aria-label="What you are approving"]');
  expect(lead?.nextElementSibling).toBe(region);
});

// The owner, 3 Oct 2026: the panel holds the request, editable, so the Brief
// is not drawn beside it; after the press the Brief reads the approved words —
// on the canvas's Brief node, since the canvas became the whole Overview.
test("the Brief is not drawn at the gate, and reads the request as approved after the press", async () => {
  mount(onJob(proposalFromAnIssue()));
  const asked = (await openNode("Brief")).getByRole("textbox", { name: "What was asked" });
  await expect.element(asked).toBeVisible();
  expect(page.getByRole("region", { name: "Brief", exact: true }).all()).toHaveLength(0);

  await asked.fill("Retire guide 8 and refuse a guide with no drawn pieces");
  await page.getByRole("button", { name: "Approve dispatch" }).last().click();

  // Past the press the canvas reads the run, and its Brief node carries the approved words.
  await expect.element(page.getByRole("region", { name: "This Job's run" })).toBeVisible();
  expect(approving().all()).toHaveLength(0);
  const brief = await openNode("Brief");
  await expect.element(brief).toHaveTextContent("Retire guide 8 and refuse a guide with no drawn pieces");
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
  // The canvas reads the run, read-only: the approval is behind it.
  await expect.element(page.getByRole("region", { name: "This Job's run" })).toBeVisible();
  expect(approving().all()).toHaveLength(0);
});

test("the workflow's promise is its picker's description, from this repository's row of the list", async () => {
  mount(onJob(refactorAtApproval()));
  const picker = approving().getByRole("combobox", { name: "Workflow", exact: true });
  await expect.element(picker).toHaveAccessibleDescription(REFACTOR_FOR_REQUESTS);
  expect(page.getByText("Another repository's refactor", { exact: false }).all()).toHaveLength(0);
});

test("a workflow that declares no promise draws nothing under its name", async () => {
  const fixture = refactorAtApproval();
  mount(
    onJob({
      ...fixture,
      workflows: fixture.workflows.map(({ for_requests: _none, ...row }) => row),
    }),
  );
  const picker = approving().getByRole("combobox", { name: "Workflow", exact: true });
  await expect.element(picker).toHaveValue("refactor");
  await expect.element(picker).not.toHaveAccessibleDescription(REFACTOR_FOR_REQUESTS);
});

// Spike 022, slice 4: what a person changes under the lead is what the Job
// runs. The press sends what moved, and the Job read back carries it.
test("what a person moved under the lead is what the press sends, and what the Job carries", async () => {
  const app = mount(onJob(proposalFromAnIssue()));
  const approveDispatch = vi.spyOn(app.api, "approveDispatch");
  const brief = await openNode("Brief");
  await brief.getByRole("textbox", { name: "Title", exact: true }).fill("Retire guide 8");
  await brief.getByRole("textbox", { name: "Criterion 2" }).fill("A rule refuses a guide with no pieces");
  await (await openNode("Checks", 1)).getByRole("checkbox", { name: "You on Restructure" }).click();
  await (await openNode("Land")).getByRole("combobox", { name: "Lands in" }).fill("release/2026-10");
  await page.getByRole("button", { name: "Approve dispatch" }).last().click();

  await expect.poll(() => approveDispatch.mock.calls.length).toBe(1);
  expect(approveDispatch.mock.calls[0]![1]).toEqual({
    title: "Retire guide 8",
    gates: [{ step_id: "implement", checks: true, judge: true, you: true }],
    criteria: [
      { criterion_id: "c1", text: "Guide 8 is removed from the catalogue", source: "judge" },
      { criterion_id: "c2", text: "A rule refuses a guide with no pieces", source: "judge" },
      { criterion_id: "c3", text: "The guide catalogue still opens on guide 1", source: "judge" },
    ],
    landing: { target: "release/2026-10", branching: "job", pr_mode: "ready", complete_when: "delivered" },
  });
  await expect.element(page.getByRole("heading", { name: "Retire guide 8", exact: true })).toBeVisible();
  expect(approving().all()).toHaveLength(0);
});
