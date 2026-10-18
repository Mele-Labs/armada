// Esc closes the innermost thing open and no more. With a Job open on the Cockpit's stage, Esc in its head's More menu shuts
// the menu and leaves the Session; the next Esc is the deck going back. Held to the open menu and not to where focus is: a press
// on the menu's padding drops focus to the page, and the deck must still not hear that Esc. A pinned card is the same.
// Through `App` over the `session-head` scenario.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const session = () => page.getByRole("region", { name: /^Job: / });
const menu = () => page.getByRole("menu");

async function openMore() {
  mount("session-head");
  await onScreen();
  await toSession(/How does armada interface/);
  await userEvent.click(session().getByRole("button", { name: "More" }));
  await expect.element(menu()).toBeVisible();
}

test("Esc in the More menu shuts the menu and leaves the Job open, and the next Esc closes the Session", async () => {
  await openMore();
  expect(menu().element().contains(document.activeElement)).toBe(true);

  await userEvent.keyboard("{Escape}");
  await expect.element(menu()).not.toBeInTheDocument();
  await expect.element(session()).toBeVisible();

  await userEvent.keyboard("{Escape}");
  await expect.element(session()).not.toBeInTheDocument();
});

test("Esc shuts the menu alone when focus has left it, after a press on the menu's padding", async () => {
  await openMore();
  await menu().click({ position: { x: 2, y: 2 } });
  await expect.element(menu()).toBeVisible();
  expect(menu().element().contains(document.activeElement)).toBe(false);

  await userEvent.keyboard("{Escape}");
  await expect.element(menu()).not.toBeInTheDocument();
  await expect.element(session()).toBeVisible();

  await userEvent.keyboard("{Escape}");
  await expect.element(session()).not.toBeInTheDocument();
});

test("Esc closes a pinned card and leaves the Job open", async () => {
  mount("session-head");
  await onScreen();
  await toSession(/How does armada interface/);
  await userEvent.click(session().getByRole("button", { name: /^Context/ }));
  const card = page.getByRole("dialog", { name: "Context" });
  await expect.element(card).toBeVisible();

  await userEvent.keyboard("{Escape}");
  await expect.element(card).not.toBeInTheDocument();
  await expect.element(session()).toBeVisible();

  await userEvent.keyboard("{Escape}");
  await expect.element(session()).not.toBeInTheDocument();
});
