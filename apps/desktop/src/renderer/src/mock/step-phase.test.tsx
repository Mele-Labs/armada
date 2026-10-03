// A running step's phase track — its parts in order, done, now and next — on
// its Workflow card and in its panel's header (owner, 3 Oct 2026: annotation
// `ouqa`, then "too easy to miss" of a mark on the pill, and this track drawn).

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

async function workflowOf(scenario: string): Promise<void> {
  mount(scenario);
  await onScreen();
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
}

/** Each part of the last track on screen, by its name on hover. */
function partsOf(track: Element): (string | null)[] {
  return [...track.querySelectorAll(".armada-step-track__part")].map((one) => one.getAttribute("aria-label"));
}

test("check-logs: the running card's track has the Checks now, and its panel the same", async () => {
  await workflowOf("check-logs");
  const card = page.getByRole("button", { name: "Implement, running, Checks running" }).last();
  await expect.element(card).toBeVisible();
  const track = (await card.element()).querySelector(".armada-step-track");
  expect(track, "no track on the card").not.toBeNull();
  expect(partsOf(track!)).toContain("Checks running");
  expect(partsOf(track!)[0]).toBe("Drones done");
  // The track is the card's one loop: no sweep beside it.
  expect((await card.element()).querySelector(".armada-wf-card__sweep")).toBeNull();
  // And the pill carries no mark of its own any more.
  expect(document.querySelector(".armada-wf-inspector__state .armada-step-track")).toBeNull();

  await card.click();
  const panel = page.getByRole("dialog", { name: "Implement" }).last();
  await expect.element(panel.getByRole("list", { name: "Phases" })).toBeVisible();
  await expect.element(panel.getByRole("img", { name: "Checks running" })).toBeVisible();
});

test("arc/executing-sequential: the running card's track has the Drones now, on their task", async () => {
  await workflowOf("arc/executing-sequential");
  await expect
    .element(page.getByRole("button", { name: /^Implement, running, Drones? on T\d+/ }).last())
    .toBeVisible();
});
