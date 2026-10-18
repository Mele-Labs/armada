// Start Job on a retro item, through `App` (`docs/concepts/retro.md`, *Where Bridge draws it*):
// the press asks Fleet to start the Session and the app goes to the Cockpit, where a Session is viewed. Fleet sends the brief
// itself, so nothing is put in a message box. The mock Fleet mints no Session (the mock holds
// none), so only the move to the Cockpit is checked here.

import { beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { forgetAllKept } from "@armada/components";

import { s205SessionRetro } from "./scenarios/session-retro";
import { mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";

unmountAfterEach();

beforeEach(() => {
  window.localStorage.clear();
  forgetAllKept();
});

const toRetros = () => toSystemPage("Retros");

test("Start Job on an Armada item goes to the Cockpit and writes nothing in a message box", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await toRetros();

  const item = page.getByRole("listitem").filter({ has: page.getByRole("button", { name: "Start Job" }) }).first();
  await expect.element(item).toBeVisible();
  await userEvent.click(item.getByRole("button", { name: "Start Job" }));

  await expect.element(page.getByRole("region", { name: "Jobs", exact: true })).toBeVisible();
  expect(Object.keys(window.localStorage).filter((one) => one.startsWith("armada:kept:session:"))).toEqual([]);
});

test("a Kit item offers no Start Job", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await toRetros();

  const kit = page.getByRole("listitem", { name: "Walks opened in your browser" });
  await expect.element(kit).toBeVisible();
  expect(kit.getByRole("button", { name: "Start Job" }).elements()).toHaveLength(0);
});
