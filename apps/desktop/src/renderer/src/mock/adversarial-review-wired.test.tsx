// An adversarial review on a real Fleet's path, through `App`: the press on a pending plan reaches Fleet's route, the review
// Fleet keeps on the plan lands on the card by `session.changed`, each finding under the part it is about, and Send reaches
// Fleet's route too. `sessions-fleet.ts` stands where main stands; `adversarial-fleet.ts` answers as the daemon does.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { AdversarialFinding, RepositorySummary, SessionPlan } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { planCall } from "./plan-call";
import { onBoard } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const ID = "01SESSIONAAAAAAAAAAAAAAAAA";
const AT = "2026-10-25T09:00:00.000Z";

const PLAN: SessionPlan = {
  version: 1,
  summary: "The next free bay is warmed while the pool waits.",
  recorded_at: AT,
  approved: false,
  scope: ["crates/fleet/src/pool.rs"],
  groups: [
    {
      id: "w1",
      agent: "rust-engineer",
      title: "Warm a bay before it is leased",
      gives: "A bay is warm by the time a Job asks for it",
      paths: ["crates/fleet/src/pool.rs"],
      lands_in: { label: "PR 1" },
      tasks: [
        { id: "W1", text: "Find the next free bay", state: "open" },
        { id: "W2", text: "Warm it in the background", state: "open" },
      ],
    },
  ],
};

const FOUND: AdversarialFinding[] = [
  { id: "adv-1", about: { kind: "task", id: "W2" }, consequence: "A lease waits ten minutes when every slot is held", expected: "The warm skips a held slot", found: "It warms the slot the pool picked, held or not" },
  { id: "adv-2", about: { kind: "plan" }, consequence: "A warm that never finishes goes unnoticed", expected: "Each group names the check that proves it", found: "No task adds a test" },
];

test("Adversarial review wired: the press reaches Fleet, its findings land under their tasks, and Send settles one", async () => {
  const fleet = new FakeSessionsFleet([hosted(ID, { title: "Warm the next bay", plan: PLAN, waiting_for: [planCall(1, AT)] })]);
  fleet.adversary = { findings: FOUND, readsMs: 50 };
  mount(fleet.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root })));
  await onScreen();
  await toSession("Warm the next bay");
  const card = page.getByRole("article", { name: "Plan to approve" });

  await userEvent.click(card.getByRole("button", { name: "Adversarial review" }));
  const subject = { kind: "plan", session_id: ID, version: 1 };
  await expect.poll(() => fleet.calls.reviewed).toEqual([{ subject }]);

  const task = card.getByRole("list", { name: "Findings on Warm it in the background" });
  await expect.element(task.getByRole("group", { name: "Finding: A lease waits ten minutes when every slot is held" })).toBeVisible();
  await expect.element(card.getByRole("list", { name: "Findings on the plan" }).getByRole("group", { name: "Finding: A warm that never finishes goes unnoticed" })).toBeVisible();

  await userEvent.click(task.getByRole("button", { name: "Send to agent" }));
  await expect.poll(() => fleet.calls.findingsSent).toEqual([{ subject, finding_id: "adv-1" }]);
  await expect.element(card.getByRole("group", { name: "Finding sent: A lease waits ten minutes when every slot is held" })).toBeVisible();
});
