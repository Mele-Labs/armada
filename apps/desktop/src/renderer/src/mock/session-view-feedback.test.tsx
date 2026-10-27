// The owner's notes on the Cockpit's one Session view, approved on `?walk=sessionViewFeedback`: the Job's story (Completed, Now and Next) stands in the right
// column and the Field under the Jobs list (Ledger and Status gave way to the story), and the Merge line frames the open Session's own pull requests and no other. Through `App` on `session-view-feedback`.
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

test("Completed, Now and Next are drawn in that order in the open Session's column, the Field is under the Jobs list, and the Ledger and Status are not", async () => {
  mount("session-view-feedback");
  await onScreen();
  await open("Rotate the audit log");
  const [done, now, next] = ["Completed", "Now", "Next"].map((name) => page.getByRole("region", { name, exact: true }).element());
  expect(done!.compareDocumentPosition(now!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(now!.compareDocumentPosition(next!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(done!.parentElement).toBe(now!.parentElement);
  expect(document.querySelector('.deck-left [aria-label="Field"]')).not.toBeNull();
  expect(document.querySelector('.deck-right [aria-label="Field"]')).toBeNull();
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
