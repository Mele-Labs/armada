// With a Job open, its state glyph and its name stand in the title bar's centre notch, not in the Job's head, which carries no title. Pressing the name renames the Job as
// before. Closing the Job empties the notch, and opening another puts that one's there. Through `App` on the flight deck's ship.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const notch = () => document.querySelector<HTMLElement>(".deck-places__job")!;
const pane = (name: string) => page.getByRole("region", { name: `Job: ${name}` });
/** The names in the notch: the title of each Job it holds. */
const named = () => [...notch().querySelectorAll(".armada-session-frame__title-text")].map((one) => one.textContent);

async function deck() {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
}

test("with a Job open its state glyph and name are in the title bar's notch, and the Job's head has no title", async () => {
  await deck();
  expect(named()).toEqual([]);
  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane("Pin the store clock")).toBeVisible();

  await expect.element(page.getByRole("button", { name: "Pin the store clock, rename" })).toBeVisible();
  expect(named()).toEqual(["Pin the store clock"]);
  expect(notch().querySelector('.armada-session-frame__state [role="img"]')).not.toBeNull();
  // The head is the Job's own, and carries neither.
  const head = document.querySelector(".armada-session-frame__head")!;
  expect(head.querySelector(".armada-session-frame__title")).toBeNull();
  expect(head.querySelector(".armada-session-frame__state")).toBeNull();
});

test("pressing the name renames the Job, in the notch, the list and the pane", async () => {
  await deck();
  await list().getByRole("button", { name: "Cache key", exact: true }).click();
  await expect.element(pane("Cache key")).toBeVisible();

  await page.getByRole("button", { name: "Cache key, rename" }).click();
  await page.getByRole("textbox", { name: "Job name" }).fill("Cache keys");
  await userEvent.keyboard("{Enter}");

  // Fleet took the new name, so the list, the pane and the notch all carry it.
  await expect.element(list().getByRole("button", { name: "Cache keys", exact: true })).toBeVisible();
  await expect.element(pane("Cache keys")).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Cache keys, rename" })).toBeVisible();
  expect(named()).toEqual(["Cache keys"]);
});

test("closing the Job empties the notch, and opening another shows that one", async () => {
  await deck();
  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(pane("Pin the store clock")).toBeVisible();
  expect(named()).toEqual(["Pin the store clock"]);

  await pane("Pin the store clock").getByRole("button", { name: "Back" }).click();
  await expect.element(pane("Pin the store clock")).not.toBeInTheDocument();
  await expect.poll(named).toEqual([]);
  expect(notch().childElementCount).toBe(0);

  await list().getByRole("button", { name: "Migration notes", exact: true }).click();
  await expect.element(pane("Migration notes")).toBeVisible();
  await expect.poll(named).toEqual(["Migration notes"]);
});
