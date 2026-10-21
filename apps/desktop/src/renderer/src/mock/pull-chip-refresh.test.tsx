// A right press on a pull request chip, in the Cockpit's merge line or in Progress, offers Refresh: Fleet reads the pull request
// from the forge now. Success shows only as the chip changing, by Fleet's events; a refusal is a toast. The walk `pull-chip-refresh`
// shows how it reads.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { configFleet, configScenario } from "./scenarios/pull-chip-refresh";
import { CONFIG_PR } from "./sessions/pull-chip-refresh";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

async function open() {
  const fleet = configFleet();
  mount(configScenario(fleet));
  await onScreen();
  return fleet;
}

const chip = () => page.getByRole("region", { name: "Merge queue" }).getByRole("button", { name: /^#2290 / });

function rightPress(target: Element): void {
  const box = target.getBoundingClientRect();
  target.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, composed: true, view: window, clientX: box.x + 4, clientY: box.y + 4, button: 2 }));
}

const toast = (headline: string) => page.getByText(headline, { exact: true }).first();

test("a right press on the merge line chip offers Refresh under the pull request's number, and nothing else", async () => {
  await open();
  await expect.element(chip()).toBeVisible();
  rightPress(chip().element());
  await expect.element(page.getByRole("menu").getByText("Pull request #2290", { exact: true })).toBeVisible();
  await expect.element(page.getByRole("menuitem")).toHaveLength(1);
  await expect.element(page.getByRole("menuitem", { name: "Refresh" })).toBeVisible();
});

test("Refresh on the merge line chip asks Fleet once, by the repository's Manifest, and the chip reads failed from Fleet's events", async () => {
  const fleet = await open();
  await expect.element(chip()).toHaveAccessibleName(/ci running/);
  rightPress(chip().element());
  await userEvent.click(page.getByRole("menuitem", { name: "Refresh" }));
  expect(fleet.calls.refreshed).toEqual([{ manifestId: "armada", number: CONFIG_PR }]);
  await expect.element(chip()).toHaveAccessibleName(/ci failed/);
});

test("Shift+F10 on the focused chip opens the same menu", async () => {
  await open();
  chip().element().focus();
  await userEvent.keyboard("{Shift>}{F10}{/Shift}");
  await expect.element(page.getByRole("menuitem", { name: "Refresh" })).toBeVisible();
});

test("a refused Refresh is a toast in Fleet's words, and the chip stays as it was", async () => {
  const fleet = await open();
  fleet.refusesRefresh = { code: "fleet.pull_request_unreadable", message: "The forge did not answer." };
  rightPress(chip().element());
  await userEvent.click(page.getByRole("menuitem", { name: "Refresh" }));
  await expect.element(toast("The forge did not answer.")).toBeVisible();
  await expect.element(chip()).toHaveAccessibleName(/ci running/);
});

const progressChip = () => page.getByRole("region", { name: "Progress" }).getByRole("button", { name: /^Pull request #2290 / });

test("Refresh on the pull request in Progress asks Fleet once, and its Checks mark reads failed from Fleet's events", async () => {
  const fleet = await open();
  await toSession("Who sees your config");
  await expect.element(progressChip().getByRole("img", { name: /^Checks running/ })).toBeVisible();
  rightPress(progressChip().element());
  await expect.element(page.getByRole("menu").getByText("Pull request #2290", { exact: true })).toBeVisible();
  await userEvent.click(page.getByRole("menuitem", { name: "Refresh" }));
  expect(fleet.calls.refreshed).toEqual([{ manifestId: "armada", number: CONFIG_PR }]);
  await expect.element(progressChip().getByRole("img", { name: "Checks failed: ci / test" })).toBeVisible();
});

test("a refused Refresh in Progress is a toast too", async () => {
  const fleet = await open();
  await toSession("Who sees your config");
  fleet.refusesRefresh = { code: "fleet.pull_request_unreadable", message: "The forge did not answer." };
  rightPress(progressChip().element());
  await userEvent.click(page.getByRole("menuitem", { name: "Refresh" }));
  await expect.element(toast("The forge did not answer.")).toBeVisible();
});
