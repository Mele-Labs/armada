// The owner's notes on the Cockpit's one Session view, approved on `?walk=sessionViewFeedback`: the Ledger stands above Status in the
// right column, and the Merge line frames the open Session's own pull requests and no other. Through `App` on `session-view-feedback`.
// The Field's fit is held by `cockpit/deck/battlefield-own-camera.test.tsx`.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs and Sessions" });
const queue = () => page.getByRole("region", { name: "Merge queue" });

async function open(title: string) {
  await list().getByRole("button", { name: title, exact: true }).click();
  await expect.element(page.getByRole("region", { name: `Session: ${title}` })).toBeVisible();
}

const related = () => [...queue().element().querySelectorAll("[data-related]")].map((chip) => chip.getAttribute("aria-label") ?? "");

test("the Ledger is drawn above Status in the open Session's column", async () => {
  mount("session-view-feedback");
  await onScreen();
  await open("Rotate the audit log");
  const ledger = page.getByRole("region", { name: "Ledger", exact: true }).element();
  const status = page.getByRole("region", { name: "Status", exact: true }).element();
  expect(ledger.compareDocumentPosition(status) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
});

test("the Merge line frames the open Session's pull request, and the frame moves with the Session", async () => {
  mount("session-view-feedback");
  await onScreen();
  await open("Rotate the audit log");
  await expect.poll(related).toEqual([expect.stringMatching(/^#2388/)]);
  await open("Retry the slot lease");
  await expect.poll(related).toEqual([expect.stringMatching(/^#2390/)]);
});
