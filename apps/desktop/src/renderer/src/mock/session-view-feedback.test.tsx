// The owner's notes on the Cockpit's one Session view, approved on `?walk=sessionViewFeedback`: the Field and Progress stand in the right
// column (Ledger and Status gave way to Progress on 16 Oct 2026), and the Merge line frames the open Session's own pull requests and no other. Through `App` on `session-view-feedback`.
// The Field's fit is held by `cockpit/deck/battlefield-own-camera.test.tsx`.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const queue = () => page.getByRole("region", { name: "Merge queue" });

async function open(title: string) {
  await list().getByRole("button", { name: title, exact: true }).click();
  await expect.element(page.getByRole("region", { name: `Job: ${title}` })).toBeVisible();
}

const related = () => [...queue().element().querySelectorAll("[data-related]")].map((chip) => chip.getAttribute("aria-label") ?? "");

test("Progress is drawn under the Field in the open Session's column, and the Ledger and Status are not", async () => {
  mount("session-view-feedback");
  await onScreen();
  await open("Rotate the audit log");
  const field = page.getByRole("region", { name: "Field", exact: true }).element();
  const progress = page.getByRole("region", { name: "Progress", exact: true }).element();
  expect(field.compareDocumentPosition(progress) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(document.querySelector('section[aria-label="Ledger"], section[aria-label="Status"]')).toBeNull();
});

test("the Merge line frames the open Session's pull request, and the frame moves with the Session", async () => {
  mount("session-view-feedback");
  await onScreen();
  await open("Rotate the audit log");
  await expect.poll(related).toEqual([expect.stringMatching(/^#2388/)]);
  await open("Retry the slot lease");
  await expect.poll(related).toEqual([expect.stringMatching(/^#2390/)]);
});
