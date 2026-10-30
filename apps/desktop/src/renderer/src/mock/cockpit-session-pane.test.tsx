// A Job open on the flight deck's stage is the Sessions window's own panel, and its story (Done, Now and Open) takes the Bays' place in
// the right column, with its Field under the Jobs list. The claims: the stage carries the same head, the context figure and a rename that renames it; and while a
// Job is open the right column shows its story instead of the Bays, which return when it is closed. #2301.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const bays = () => page.getByRole("region", { name: "Bays" });
const right = () => document.querySelector(".deck-right")!;

async function deck() {
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(list()).toBeVisible();
}

test("a Job open in the middle area carries the context figure and a rename that renames it", async () => {
  await deck();
  await list().getByRole("button", { name: "Cache key", exact: true }).click();
  const pane = page.getByRole("region", { name: "Job: Cache key" });
  await expect.element(pane).toBeVisible();

  // The head the Sessions window draws: the context figure, and the title as the way to rename it.
  await expect.element(pane.getByLabelText("Context 27%")).toBeVisible();
  await pane.getByRole("button", { name: "Cache key, rename" }).click();
  const name = pane.getByRole("textbox", { name: "Job name" });
  await name.fill("Cache keys");
  await userEvent.keyboard("{Enter}");

  // Fleet took the new name, so the list, the pane and its label all carry it.
  await expect.element(list().getByRole("button", { name: "Cache keys", exact: true })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Job: Cache keys" })).toBeVisible();
});

test("while a Job is open the right column shows its Done, Now and Open instead of the Bays, with its Field under the list, and the Bays return when it is closed", async () => {
  await deck();
  await expect.element(bays()).toBeVisible();

  await list().getByRole("button", { name: "Pin the store clock", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Job: Pin the store clock" })).toBeVisible();
  await expect.poll(() => right().querySelector('[aria-label="Now"]') !== null).toBe(true);
  expect(["Done", "Now", "Open"].map((name) => right().querySelector(`section[aria-label="${name}"]`) !== null)).toEqual([true, true, true]);
  // The Field is the open Job's, under the Jobs list, and not in the column.
  expect(document.querySelector('.deck-left [aria-label="Field"]')).not.toBeNull();
  expect(right().querySelector('[aria-label="Field"]')).toBeNull();
  await expect.element(bays()).not.toBeInTheDocument();
  // The list stays where it was.
  await expect.element(list()).toBeVisible();

  // Closing the Session brings the Bays back.
  await page.getByRole("region", { name: "Job: Pin the store clock" }).getByRole("button", { name: "Back" }).click();
  await expect.element(bays()).toBeVisible();
  expect(right().querySelector('[aria-label="Now"]')).toBeNull();
});
