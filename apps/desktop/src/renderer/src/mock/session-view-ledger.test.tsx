// A Session selected on the Cockpit has one Ledger panel, and it lists what the Session holds beside its branch and bay, which are chips
// in its band: a page it showed is a row. A Session holding nothing more is its band, folded. Through `App` on `one-session-view`,
// the Cockpit path, where the Session's own panels are drawn into the deck.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs and Sessions" });

async function open(title: string) {
  mount("one-session-view");
  await onScreen();
  await expect.element(list()).toBeVisible();
  await list().getByRole("button", { name: title, exact: true }).click();
  await expect.element(page.getByRole("region", { name: `Session: ${title}` })).toBeVisible();
}

const ledgers = () => [...document.querySelectorAll('section[aria-label="Ledger"]')] as HTMLElement[];

test("a Session holding a page it showed lists it in its one Ledger, with its branch and bay as chips", async () => {
  await open("Queue session for later");
  await expect.poll(() => ledgers().length).toBe(1);
  const ledger = page.getByRole("region", { name: "Ledger" });
  await expect.element(ledger.getByRole("button", { name: /Queue for later walk/ })).toBeVisible();
  await expect.element(ledger.getByLabelText("Bay 9")).toBeVisible();
  expect(ledgers()[0]!.hasAttribute("data-collapsed")).toBe(false);
  // The ledger the Sessions window draws beside a thread is not drawn a second time in the deck's column.
  expect(document.querySelectorAll(".armada-session-ledger")).toHaveLength(1);
});

test("a Session whose ledger holds nothing beside its branch and bay has one Ledger panel, folded to its band", async () => {
  await open("Rotate the audit log");
  await expect.poll(() => ledgers().length).toBe(1);
  expect(ledgers()[0]!.hasAttribute("data-collapsed")).toBe(true);
  expect(ledgers()[0]!.querySelector(".armada-session-ledger")).toBeNull();
  expect(document.querySelectorAll(".armada-session-ledger")).toHaveLength(0);
});

test("a Session with no work is drawn as it was, in the deck's one Ledger panel and not beside its thread", async () => {
  await open("Name the store migrations");
  await expect.poll(() => ledgers().length).toBe(1);
  expect(document.querySelectorAll(".armada-session-ledger")).toHaveLength(1);
});
