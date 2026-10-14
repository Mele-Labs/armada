// The Cockpit's one Active filter and the call that arrives in its tray, through `App` on the
// `cockpit-calls-arrive` mock Fleet: the filters are Active, Done and Bays, a call covers nothing and waits
// with its key printed on it until `w` or a press opens it, answering it brings the next after it, a stored
// Your move reads as Active, and an alert's press opens the call's card directly.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { settle } from "../cockpit/CallAlerts";
import { forgetDismissals } from "../cockpit/dismissed";
import { mount, onScreen, rememberingTheGrid, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const FILTER = "armada.bridge.dashboard-tab";
const SAVED = { ...settle };
beforeEach(() => {
  forgetDismissals();
  rememberingTheGrid();
  settle.ms = 0;
});
afterEach(() => Object.assign(settle, SAVED));

const call = (kind: RegExp) => page.getByRole("region", { name: kind });
const waiting = (kind: RegExp) => page.getByRole("button", { name: kind });

test("the filters are Active, Done and Bays, and Active is the one it opens on, with no dot on any tab", async () => {
  mount("cockpit-calls-arrive");
  await onScreen();
  await expect.element(page.getByRole("tab", { name: "Active" })).toBeVisible();
  expect([...(page.getByRole("tab", { name: "Active" }).element().parentElement?.querySelectorAll('[role="tab"]') ?? [])].map((one) => one.textContent)).toEqual(["Active", "Done", "Bays"]);
  expect(page.getByRole("tab", { name: "Your move" }).query()).toBeNull();
  await expect.element(page.getByRole("tab", { name: "Active" })).toHaveAttribute("aria-selected", "true");
  timePasses();
  timePasses();
  await expect.element(waiting(/^Plan question/)).toBeVisible();
  expect(document.querySelector('[role="tab"] [role="img"]')).toBeNull();
});

test("a filter stored as Your move reads as Active", async () => {
  mount("cockpit-calls-arrive");
  // `mount` forgets the viewer; the old choice is written as a visit before this one would have.
  localStorage.setItem(FILTER, "command-central");
  await onScreen();
  await expect.element(page.getByRole("tab", { name: "Active" })).toHaveAttribute("aria-selected", "true");
  await expect.element(page.getByRole("listbox", { name: "Tiles" })).toBeVisible();
});

test("a call covers nothing, waits in the tray with w on it, and w opens it; answering it clears the tray and the next arrives after", async () => {
  mount("cockpit-calls-arrive");
  await onScreen();
  const tile = page.getByRole("option", { name: /^Cache the manifest read/ });
  await expect.element(tile).toBeVisible();
  expect(document.querySelector(".armada-tray")).toBeNull();

  timePasses();
  timePasses();
  await expect.element(waiting(/^Plan question/)).toBeVisible();
  await expect.element(waiting(/^Plan question/)).toHaveTextContent("w");
  // The work is still there and pressable, and nothing is in front of it.
  expect(call(/^Plan question/).query()).toBeNull();
  expect(document.querySelector(".armada-callcard")).toBeNull();
  await expect.element(tile).toBeVisible();

  await userEvent.keyboard("w");
  await expect.element(call(/^Plan question/)).toBeVisible();
  expect(document.querySelector(".armada-tray")).toBeNull();
  await userEvent.keyboard("2");
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => call(/^Plan question/).query()).toBeNull();
  // Nothing opens by itself, and nothing else waits.
  expect(document.querySelector(".armada-callcard")).toBeNull();
  expect(waiting(/^Plan question/).query()).toBeNull();

  // The second arrives after, in the tray, and a press opens it; its first act is sent on its number.
  timePasses();
  await expect.element(waiting(/^Check failed/)).toBeVisible();
  expect(call(/^Check failed/).query()).toBeNull();
  await waiting(/^Check failed/).click();
  await expect.element(call(/^Check failed/)).toBeVisible();
});

test("the keys are the glass's until a card is open: j moves along the tiles with a call waiting", async () => {
  mount("cockpit-calls-arrive");
  await onScreen();
  timePasses();
  timePasses();
  await expect.element(waiting(/^Plan question/)).toBeVisible();
  const selected = () => document.querySelector('[role="option"][aria-selected="true"]')?.textContent;
  await userEvent.keyboard("j");
  const after = selected();
  await userEvent.keyboard("j");
  expect(selected()).not.toBe(after);
  expect(document.querySelector(".armada-callcard")).toBeNull();
});

test("an alert's press opens the call's card directly, from wherever he was", async () => {
  mount("cockpit-calls-arrive");
  await onScreen();
  await expect.element(page.getByRole("tab", { name: "Active" })).toBeVisible();
  await page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Jobs", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Jobs" })).toBeVisible();
  timePasses();
  timePasses();
  await page.getByRole("button", { name: /needs you/ }).click();
  await expect.element(call(/^Plan question/)).toBeVisible();
});
