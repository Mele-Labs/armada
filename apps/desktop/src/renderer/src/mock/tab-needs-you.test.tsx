// A Custom tab's Mod Job held on a question. It has no row in the Jobs list; the Job whose panes hold the tab wears the needs-you mark and so does the tab; the tab's card holds the
// ask, and Allow once there clears it everywhere. Through `App` on `job-layouts`, whose fifth Job holds a Pull queue tab. The walk `tabNeedsYou` plays it.

import { beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetPanes } from "../job-panes/store";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(() => forgetPanes());

const HOLDER = "Design how a watch ends";
const jobs = () => page.getByRole("region", { name: "Jobs", exact: true });
const tabPane = () => page.getByRole("region", { name: "Pane: Pull queue", exact: true });

async function open() {
  mount("job-layouts");
  await onScreen();
  await toSession(HOLDER);
}

test("the Mod Job has no row, the Job holding its tab wears the mark, and so does the tab", async () => {
  await open();
  await expect.element(tabPane().getByRole("img", { name: "Waiting on you" })).toBeVisible();
  expect(jobs().getByRole("button", { name: "List the oldest first" }).query()).toBeNull();
  await expect.element(jobs().getByRole("button", { name: HOLDER }).getByRole("img", { name: "Pull queue waits on you" })).toBeVisible();
});

test("the card holds the ask with its keys, and Allow once clears it from the card, the tab and the Job's row", async () => {
  await open();
  const ask = page.getByRole("article", { name: "Waiting on you" });
  await expect.element(ask).toBeVisible();
  await expect.element(ask.getByRole("button", { name: /^Allow once/ })).toBeVisible();
  await expect.element(ask.getByRole("button", { name: /^Deny/ })).toBeVisible();
  // Under the ask, the thread tail, the box and Open Job.
  await expect.element(page.getByRole("textbox", { name: "Message the Mod Job" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Open Job" })).toBeVisible();

  await userEvent.click(ask.getByRole("button", { name: /^Allow once/ }));

  await expect.poll(() => page.getByRole("article", { name: "Waiting on you" }).query()).toBeNull();
  await expect.element(tabPane().getByRole("img", { name: "List the oldest first" })).toBeVisible();
  expect(tabPane().getByRole("img", { name: "Waiting on you" }).query()).toBeNull();
  expect(jobs().getByRole("button", { name: HOLDER }).getByRole("img", { name: "Pull queue waits on you" }).query()).toBeNull();
  expect(jobs().getByRole("button", { name: "List the oldest first" }).query()).toBeNull();
});
