// What a press asks the trackpad to play, through `App` — the one place the
// tap can be read without a trackpad under it.
//
// **Answers, not controls.** The tap plays where Fleet's answer arrives, so
// what is pinned here is that an act still taps when the control that sent it
// is gone by the time the answer lands. #1326.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { Outcome } from "@armada/protocol";
import { reviewAtDelivery } from "@armada/screens/src/fixtures/build/index";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { BridgeApi } from "../../../shared/api";
import type { Scenario } from "./scenario";
import { onJob } from "./scenario";
import { entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };

async function opened(fixture: JobFixture, behaves?: Scenario["behaves"]): Promise<BridgeApi> {
  const app = mount({ ...onJob(fixture), behaves });
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  return app.api;
}


/** The confirming dialog an act goes through, once it has finished scaling up — #1323. */
async function confirming() {
  const dialog = page.getByRole("dialog");
  await entered(dialog);
  return dialog;
}

test("an accepted act taps once, in alignment", async () => {
  const api = await opened(reviewAtDelivery());
  const tap = vi.spyOn(api, "tap");
  await page.getByRole("button", { name: /^Merge/ }).first().click();
  await (await confirming()).getByRole("button", { name: /^Merge/ }).click();
  await expect.poll(() => tap.mock.calls.length).toBe(1);
  expect(tap).toHaveBeenCalledWith("alignment");
});

test("a refused act plays the level change, from the same place", async () => {
  const api = await opened(reviewAtDelivery(), () => ({
    mergePullRequest: async () => NOT_CONNECTED,
  }));
  const tap = vi.spyOn(api, "tap");
  await page.getByRole("button", { name: /^Merge/ }).first().click();
  await (await confirming()).getByRole("button", { name: /^Merge/ }).click();
  await expect.poll(() => tap.mock.calls.length).toBe(1);
  expect(tap).toHaveBeenCalledWith("level_change");
});

// **A refused plan drop tapped too**, from the screen holding its own answer
// rather than from a toast. `Drop…` went with the Plan region on 29 Sep 2026
// and `PlanWell` has no renderer, so there is no drop to refuse; the claim
// belongs to the Plan destination when its rows carry the act again.
