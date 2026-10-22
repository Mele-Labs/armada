// A Session selected on the Cockpit is drawn compact: the Field and Progress share the right column, and there is no Ledger, Status, Plan
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

test("the Field and Progress share the right column, in that order, and nothing else of the old column is drawn", async () => {
  await open("one-session-view", "Queue session for later");
  await expect.poll(() => panels("Progress").length).toBe(1);
  expect(panels("Field")).toHaveLength(1);
  const field = page.getByRole("region", { name: "Field", exact: true }).element();
  const progress = page.getByRole("region", { name: "Progress", exact: true }).element();
  expect(field.compareDocumentPosition(progress) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  expect(field.parentElement).toBe(progress.parentElement);
  for (const gone of ["Ledger", "Status", "Plan", "Incoming calls"]) expect(panels(gone), gone).toHaveLength(0);
  expect(document.querySelector(".sv-slot[data-place='foot']")).toBeNull();
});

test("Progress carries the workflow as glyphs and the plan's groups under them", async () => {
  await open("compact-session-view", "Retry the slot lease");
  const progress = page.getByRole("region", { name: "Progress", exact: true });
  await expect.element(progress.getByRole("list", { name: "Workflow" })).toBeVisible();
  await expect.element(progress.getByRole("img", { name: "Build · current" })).toBeVisible();
  await expect.element(progress.getByRole("region", { name: "Attempt count on the wire", exact: true })).toBeVisible();
});

test("a Session calling him pulses its row and names the call, and the open Session's own row does not", async () => {
  await open("compact-session-view", "Retry the slot lease");
  await expect.poll(calling).toEqual(expect.arrayContaining(["Prune the stale branches"]));
  expect(calling()).not.toContain("Retry the slot lease");
  await userEvent.hover(list().getByRole("button", { name: "Prune the stale branches", exact: true }));
  await expect.element(page.getByText(/^Permission/)).toBeVisible();
});
