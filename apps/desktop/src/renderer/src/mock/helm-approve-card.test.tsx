// Helm's approval ask in the dock, through `App` on the `helm-approve-card` moment: what each card reads and what
// its presses do. The look is the walk's; this holds the five things the card says and the one call Approve makes.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, openHelm, unmountAfterEach } from "./testing";
import { s238HelmApproveCard } from "./scenarios/helm-approve-card";

unmountAfterEach();

const helm = () => page.getByRole("complementary", { name: "Helm" });
const cards = () => helm().getByRole("region", { name: "Approval asked for" });
const GATE = /81-retire-the-legacy-poke-path/;
const gateCard = () => cards().filter({ hasText: GATE });

test("a Job at its approval gate is a card with Approve and Not now", async () => {
  mount("helm-approve-card");
  await openHelm();
  await expect.element(gateCard().getByRole("button", { name: "Approve" })).toBeVisible();
  await expect.element(gateCard().getByRole("button", { name: "Not now" })).toBeVisible();
});

test("Approve presses the Job's gate once, with its id, and the card reads Approved.", async () => {
  const app = mount("helm-approve-card");
  const approved = vi.spyOn(app.api, "approveDispatch");
  await openHelm();
  const gate = s238HelmApproveCard.state.jobs.find((one) => GATE.test(one.handle))!;

  await userEvent.click(gateCard().getByRole("button", { name: "Approve" }));

  await expect.element(gateCard().getByText("Approved.")).toBeVisible();
  expect(approved).toHaveBeenCalledTimes(1);
  expect(approved.mock.calls[0]![0]).toBe(gate.id);
  await expect.element(gateCard().getByRole("button")).not.toBeInTheDocument();
});

test("Not now reads Dismissed. and presses nothing", async () => {
  const app = mount("helm-approve-card");
  const approved = vi.spyOn(app.api, "approveDispatch");
  await openHelm();

  await userEvent.click(gateCard().getByRole("button", { name: "Not now" }));

  await expect.element(gateCard().getByText("Dismissed.")).toBeVisible();
  expect(approved).not.toHaveBeenCalled();
});

test("a Job that is no longer at its gate reads No longer awaiting approval., with nothing to press", async () => {
  mount("helm-approve-card");
  await openHelm();
  const moved = s238HelmApproveCard.state.jobs[0]!;
  expect(moved.status).not.toBe("awaiting_approval");
  const card = cards().filter({ hasText: moved.handle });
  await expect.element(card.getByText("No longer awaiting approval.")).toBeVisible();
  await expect.element(card.getByRole("button")).not.toBeInTheDocument();
});

test("an ask naming an id the Board has never heard of reads There is no Job with that id.", async () => {
  const sessions = s238HelmApproveCard.draft!.sessions!;
  mount({
    ...s238HelmApproveCard,
    name: "helm-approve-card-unknown",
    draft: {
      sessions: (control) => {
        const store = sessions(control);
        store.edit("helm-armada", (one) => ({
          ...one,
          rows: [...one.rows, { id: "a6", at: "13:59:30", kind: "tool", text: "mcp__armada-fleet__ask_person_to_approve no-such-job" }],
        }));
        return store;
      },
    },
  });
  await openHelm();
  const card = cards().filter({ hasText: "no-such-job" });
  await expect.element(card.getByText("There is no Job with that id.")).toBeVisible();
  await expect.element(card.getByRole("button")).not.toBeInTheDocument();
});
