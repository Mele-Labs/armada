// A Session selected on the Cockpit is drawn compact: Completed, Now and Next share the right column on one line, and there is no Ledger, Status, Plan
// or Incoming calls panel and no Field in the foot. A Session calling him pulses its row in Work, and its tooltip names the call. Through
// `App` on `one-session-view` and `compact-session-view`.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });

async function open(scenario: string, title: string) {
  mount(scenario);
  await onScreen();
  await expect.element(list()).toBeVisible();
  await list().getByRole("button", { name: title, exact: true }).click();
  await expect.element(page.getByRole("region", { name: `Job: ${title}` })).toBeVisible();
}

const panels = (name: string) => document.querySelectorAll(`section[aria-label="${name}"]`);
const calling = () => [...document.querySelectorAll<HTMLElement>("[data-row][data-calling]")].map((one) => one.getAttribute("aria-label"));

test("Completed, Now and Next share the right column on one line, in that order, and nothing else of the old column is drawn", async () => {
  await open("one-session-view", "Queue session for later");
  await expect.poll(() => panels("Now").length).toBe(1);
  expect(panels("Completed")).toHaveLength(1);
  expect(panels("Next")).toHaveLength(1);
  const [done, now, next] = ["Completed", "Now", "Next"].map((name) => page.getByRole("region", { name, exact: true }).element());
  expect(done!.compareDocumentPosition(now!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(now!.compareDocumentPosition(next!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(done!.parentElement).toBe(now!.parentElement);
  expect(done!.parentElement).toBe(next!.parentElement);
  expect(done!.parentElement!.classList.contains("sv-spine")).toBe(true);
  for (const gone of ["Blueprint", "Delivery", "Workflow", "Ledger", "Status", "Plan", "Incoming calls"]) expect(panels(gone), gone).toHaveLength(0);
  expect(document.querySelector(".sv-slot[data-place='foot']")).toBeNull();
});

test("the workflow heads the story as glyphs and Now carries the plan's groups", async () => {
  await open("compact-session-view", "Retry the slot lease");
  await expect.element(page.getByRole("list", { name: "Workflow" })).toBeVisible();
  await expect.element(page.getByRole("img", { name: "Build · current" })).toBeVisible();
  const now = page.getByRole("region", { name: "Now", exact: true });
  // The plan is approved, so a press on Whole plan opens every group with its tasks.
  await now.getByRole("button", { name: "Whole plan", exact: true }).click();
  await expect.element(now.getByRole("region", { name: "Attempt count on the wire", exact: true })).toBeVisible();
});

test("a Session calling him pulses its row and names the call, and the open Session's own row does not", async () => {
  await open("compact-session-view", "Retry the slot lease");
  await expect.poll(calling).toEqual(expect.arrayContaining(["Prune the stale branches"]));
  expect(calling()).not.toContain("Retry the slot lease");
  await userEvent.hover(list().getByRole("button", { name: "Prune the stale branches", exact: true }));
  await expect.element(page.getByText(/^Permission/)).toBeVisible();
});

test("the right column starts level with the open Job whichever of the handle's two stylesheets loads last", async () => {
  // The column handle is a `.armada-dock-handle` (components) placed by `.deck-handle` (deck.css), one class each: the order the bundle
  // writes them in decides which `position` holds, and a handle left in the flow pushes the column's first panel down by its gap.
  const later = document.createElement("style");
  later.textContent = ".armada-dock-handle { position: relative; margin-inline: 4px; }";
  document.head.append(later);
  try {
    await open("compact-session-view", "Retry the slot lease");
    await expect.poll(() => panels("Now").length).toBe(1);
    const top = (name: string) => page.getByRole("region", { name, exact: true }).element().getBoundingClientRect().top;
    expect(document.querySelector(".sv-spine")!.getBoundingClientRect().top).toBe(top("Job: Retry the slot lease"));
  } finally {
    later.remove();
  }
});
