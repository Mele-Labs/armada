// Linked Sessions on a real Fleet's path, through `App`: the plan comes off the Session's record, and the Session Fleet says was
// handed a group stands on that group in Now, in the Artifacts layer and under its parent in Work, all read off the `detail.group`
// of the two link rows. A group with none offers no hand off, since only the agent hands one.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { resetFolds } from "../session-view/story";
import { onBoard } from "./scenario";
import { FakeSessionsFleet, held, hosted } from "./sessions-fleet";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const AT = "2026-10-16T14:00:00Z";
const PARENT = "01PARENTAAAAAAAAAAAAAAAAAA";
const KNOWN = "01CHILDAAAAAAAAAAAAAAAAAAA";

const task = (id: string, text: string) => ({ id, text, state: "open" as const, seen: false });
const plan = {
  version: 1,
  recorded_at: AT,
  summary: "",
  approved: true,
  scope: ["crates/store"],
  groups: [
    { id: "g1", agent: "rust-engineer", title: "Name the migrations", paths: ["crates/store"], lands_in: { label: "PR 1" }, tasks: [task("t1", "Give each migration a name")] },
    { id: "g2", agent: "rust-engineer", title: "Apply them in order", paths: ["crates/store"], lands_in: { label: "PR 1" }, tasks: [task("t2", "Sort by name before applying")] },
  ],
};

function fleet(): FakeSessionsFleet {
  return new FakeSessionsFleet([
    hosted(PARENT, { title: "Move the store to named migrations", plan, attachments: [held("started_to", KNOWN, { group: "g1" }, "spent")] }),
    hosted(KNOWN, { title: "Migrations get names", attachments: [held("started_by", PARENT, { group: "g1" }, "spent")] }),
  ]);
}

async function open(served: FakeSessionsFleet) {
  // Whole plan is kept per Job, in the window's storage and in memory, so the test before this one would leave it on.
  resetFolds(PARENT);
  mount(served.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root })));
  await onScreen();
  const work = page.getByRole("region", { name: "Jobs", exact: true });
  await expect.element(work).toBeVisible();
  await work.getByRole("button", { name: "Move the store to named migrations", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Job: Move the store to named migrations" })).toBeVisible();
  return work;
}

test("a Session Fleet says was handed a group stands on that group, in the Ledger and under its parent in Work", async () => {
  const work = await open(fleet());
  const plan = page.getByRole("region", { name: "Now", exact: true });
  await plan.getByRole("button", { name: "Whole plan", exact: true }).click();
  await expect.element(plan.getByRole("region", { name: "Name the migrations", exact: true }).getByRole("button", { name: "Open Migrations get names" })).toBeVisible();
  await page.getByRole("region", { name: /^Job: / }).getByRole("button", { name: "Artifacts", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Artifacts", exact: true }).getByRole("button", { name: "Migrations get names" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(work.getByLabelText("Linked Job, handed from Name the migrations")).toBeVisible();
});

test("a group not handed opens with no Hand to a Job, and nothing is handed to Fleet", async () => {
  const served = fleet();
  await open(served);
  const plan = page.getByRole("region", { name: "Now", exact: true });
  await plan.getByRole("button", { name: "Whole plan", exact: true }).click();
  await plan.getByRole("button", { name: "Apply them in order", exact: true }).click();
  await expect.element(plan.getByRole("button", { name: "Hand to a Job" })).not.toBeInTheDocument();
  expect(served.calls.handed).toEqual([]);
});
