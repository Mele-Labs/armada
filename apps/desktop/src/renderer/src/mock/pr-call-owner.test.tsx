// A failing pull request a Session holds, through `App` on `pr-call-owner-working`: no call and no dot
// while that Session works on it, the call with the Session's thread once it is idle having been told
// nothing since the failure, and no call again once Poke has told it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { forgetDismissals } from "../cockpit/dismissed";
import { mount, onScreen, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const FILTER = "armada.bridge.dashboard-tab";
const call = page.getByRole("region", { name: /^Pull request: #1904/ });
const needsYou = () => page.getByRole("tab", { name: "Your move" }).getByRole("img", { name: "Needs you" });

test("no call while its Session works, the call with the Session's thread once it is idle, and none after a Poke", async () => {
  forgetDismissals();
  localStorage.removeItem(FILTER);
  mount("pr-call-owner-working");
  await onScreen();

  // At work: neither a card nor the dot on Your move.
  expect(call.query()).toBeNull();
  expect(needsYou().query()).toBeNull();

  // Idle, with nothing sent since the checks failed: the call, and the Session's own thread under it.
  timePasses();
  await expect.element(call).toBeVisible();
  await expect.element(call.getByRole("group", { name: /^Session Pin the Bridge clock/ })).toBeVisible();
  await expect.element(call.getByText("desktop_test is failing on a clock the Bridge tests also read.")).toBeVisible();

  // Poke tells it, so it is on the failure again and the call leaves.
  await userEvent.click(page.getByRole("button", { name: /Poke/ }));
  await expect.element(call).not.toBeInTheDocument();
  expect(needsYou().query()).toBeNull();
});
