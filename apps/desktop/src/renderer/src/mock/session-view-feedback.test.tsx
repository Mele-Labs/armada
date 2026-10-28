// The owner's notes on the Cockpit's one Session view, approved on `?walk=sessionViewFeedback`: the Job's story (the Brief, then a zone for each step of its workflow) stands in the right
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

test("the Brief and the workflow's steps are drawn in that order in the open Session's column, the Field is under the Jobs list, and the Ledger and Status are not", async () => {
  mount("session-view-feedback");
  await onScreen();
  await open("Rotate the audit log");
  const zones = [...document.querySelector(".sv-spine")!.children].filter((one) => one.matches("section.sv-panel")).map((one) => one.getAttribute("aria-label"));
  expect(zones[0]).toBe("Brief");
  expect(zones.length).toBeGreaterThan(2);
  expect(zones).not.toContain("Completed");
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

// The owner, 28 Oct 2026: the open Session's pull requests were framed in the line but drew too little to see. A related chip stands on a ground, in an edge and a weight a plain one does not.
test("the open Session's pull request chip is drawn apart from the others", async () => {
  mount("session-view-feedback");
  await onScreen();
  await open("Rotate the audit log");
  await expect.poll(related).toHaveLength(1);
  const look = (chip: Element) => {
    const style = getComputedStyle(chip);
    return { ground: style.backgroundColor, edge: style.borderTopColor, weight: style.fontWeight };
  };
  const chips = [...queue().element().querySelectorAll("button.deck-chip")];
  const mine = look(chips.find((chip) => chip.hasAttribute("data-related"))!);
  for (const other of chips.filter((chip) => !chip.hasAttribute("data-related"))) {
    const plain = look(other);
    expect(mine.ground).not.toBe(plain.ground);
    expect(mine.edge).not.toBe(plain.edge);
    expect(mine.weight).not.toBe(plain.weight);
  }
});
