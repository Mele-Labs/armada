// Linked Sessions on a real Fleet's path, through `App`: the plan comes off the Session's record, Hand to a Job on a group that has none
// is `hand_to_session` to Fleet, and the Session Fleet starts stands on that group in Progress, in the Artifacts layer and under its
// parent in Work, all read off the `detail.group` of the two link rows.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

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
  approved: true,
  scope: ["crates/store"],
  groups: [
    { id: "g1", agent: "rust-engineer", title: "Name the migrations", paths: ["crates/store"], tasks: [task("t1", "Give each migration a name")] },
    { id: "g2", agent: "rust-engineer", title: "Apply them in order", paths: ["crates/store"], tasks: [task("t2", "Sort by name before applying")] },
  ],
};

function fleet(): FakeSessionsFleet {
  return new FakeSessionsFleet([
    hosted(PARENT, { title: "Move the store to named migrations", plan, attachments: [held("started_to", KNOWN, { group: "g1" }, "spent")] }),
    hosted(KNOWN, { title: "Migrations get names", attachments: [held("started_by", PARENT, { group: "g1" }, "spent")] }),
  ]);
}

async function open(served: FakeSessionsFleet) {
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
  const plan = page.getByRole("region", { name: "Progress", exact: true });
  await expect.element(plan.getByRole("region", { name: "Name the migrations", exact: true }).getByRole("button", { name: "Open Migrations get names" })).toBeVisible();
  await page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Artifacts" }).click();
  await expect.element(page.getByRole("region", { name: "Artifacts", exact: true }).getByRole("button", { name: "Migrations get names" })).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(work.getByLabelText("Linked Job, handed from Name the migrations")).toBeVisible();
});

test("Hand to a Job on a group not handed is hand_to_session, and the Session it starts stands on the group", async () => {
  const served = fleet();
  await open(served);
  const plan = page.getByRole("region", { name: "Progress", exact: true });
  await plan.getByRole("button", { name: "Apply them in order", exact: true }).click();
  await userEvent.click(plan.getByRole("button", { name: "Hand to a Job" }));
  await expect.poll(() => served.calls.handed).toEqual([{ sessionId: PARENT, group: "g2" }]);
  await expect.element(plan.getByRole("region", { name: "Apply them in order", exact: true }).getByRole("button", { name: "Open Apply them in order" })).toBeVisible();
  await expect.element(plan.getByRole("button", { name: "Hand to a Job" })).not.toBeInTheDocument();
});
