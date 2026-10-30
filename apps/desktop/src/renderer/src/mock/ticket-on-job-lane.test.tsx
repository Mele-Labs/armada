// A Job that started other Jobs, over the mock Fleet's `ticket-on-job-lane`: the head of a lane for a Job that has lanes of its own opens them on a double-click or Enter, Back returns
// with focus on that head, Esc goes back one level wherever focus is and closes the Workflow only at the top, and a lane with no lanes to open has no button. The walk of the same
// name plays the opening with the press that Enter makes; this holds the keys and the focus it cannot reach.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";
import { button, deckRow, inside, role, walkThrough } from "./walk";

unmountAfterEach();

const PARENT = "Go through app feedback";

const lanes = () => page.getByRole("group", { name: "Workflow", exact: true });
const head = () => page.getByRole("button", { name: "Open Workflow: Fix cart rounding", exact: true });
const back = () => page.getByRole("button", { name: `Back to ${PARENT}`, exact: true });
const cart = () => page.getByRole("button", { name: "Fix cart rounding: PR open, ticket ARM-52, In review", exact: true });
const own = () => page.getByRole("button", { name: "ARM-52: In review, assigned to you", exact: true });

/** The parent's Workflow, from the Cockpit. */
async function onTheLanes() {
  mount("ticket-on-job-lane");
  await onScreen();
  await walkThrough([
    { press: deckRow(PARENT), say: "" },
    { press: inside(role("toolbar", "Views", { exact: true }), button("Workflow", { exact: true })), say: "" },
  ]);
  await expect.element(cart()).toBeVisible();
}

/** Opens Fix cart rounding's own Workflow by a double-click on its head, and waits for it to draw. */
async function opened() {
  await onTheLanes();
  await userEvent.dblClick(head());
  await expect.element(own()).toBeVisible();
}

test("a double-click on the head of a Job's lane opens that Job's Workflow under Back and its title, with focus on Back", async () => {
  await opened();
  await expect.element(page.getByRole("heading", { name: "Fix cart rounding" })).toBeVisible();
  await expect.element(back()).toHaveFocus();
  // The parent's lanes stay under it, out of reach: the canvas draws each node visible whatever holds it, so nothing of them is for a key or a reader.
  await expect.element(cart()).toBeInTheDocument();
  expect(cart().element().closest("[inert]")).not.toBeNull();
});

test("Enter on the head opens it too, and one press with a pointer does not", async () => {
  await onTheLanes();
  await userEvent.click(head());
  await expect.element(back()).not.toBeInTheDocument();
  head().element().focus();
  await userEvent.keyboard("{Enter}");
  await expect.element(own()).toBeVisible();
  await expect.element(back()).toHaveFocus();
});

test("Back returns to the parent's lanes as they were, with focus on the head that opened it", async () => {
  await opened();
  await userEvent.click(back());
  await expect.element(back()).not.toBeInTheDocument();
  await expect.element(cart()).toBeVisible();
  expect(cart().element().closest("[inert]")).toBeNull();
  await expect.element(head()).toHaveFocus();
});

test("Esc goes back one level wherever focus is while a Job's Workflow is open", async () => {
  await opened();
  // On the page, off the canvas and off Back.
  (document.activeElement as HTMLElement).blur();
  await expect.element(page.getByRole("button", { name: `Back to ${PARENT}` })).not.toHaveFocus();
  await userEvent.keyboard("{Escape}");
  await expect.element(back()).not.toBeInTheDocument();
  await expect.element(cart()).toBeVisible();
  // The Workflow itself is still up: the press went back a level and no further.
  await expect.element(lanes()).toBeVisible();
});

test("Esc goes back one level from a node of the opened Workflow too", async () => {
  await opened();
  own().element().focus();
  await userEvent.keyboard("{Escape}");
  await expect.element(back()).not.toBeInTheDocument();
  await expect.element(cart()).toBeVisible();
  await expect.element(lanes()).toBeVisible();
});

test("Esc at the top closes the Workflow", async () => {
  await onTheLanes();
  await userEvent.keyboard("{Escape}");
  await expect.element(lanes()).not.toBeInTheDocument();
});

test("a lane whose Job has no lanes to open has no button on its head", async () => {
  await onTheLanes();
  await expect.element(lanes().getByText("Tidy the PR panel landing", { exact: true })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Open Workflow: Tidy the PR panel landing" })).not.toBeInTheDocument();
});
