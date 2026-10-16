// How old the merge queue's reading is, and the forge going quiet, on main's light: the tooltip of the plain chip and the card of a
// red main's light say the same two things, and the dot is dimmed while the forge is quiet. The moments are `merge-line-freshness`'s.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { scenarioNamed, type Scenario } from "./scenario";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const RED = {
  state: "red" as const,
  commit: "a".repeat(40),
  read_at: new Date().toISOString(),
  red_since: new Date().toISOString(),
  failed: [{ name: "desktop_test", check: "desktop_test" }],
};

/** The scenario at its first moment, or after the forge went quiet; `red` has main fail the same way in both. */
function standing(quiet: boolean, red = false): Scenario {
  const base = scenarioNamed("merge-line-freshness")!;
  const state = quiet ? { ...base.state, ...base.later![0]! } : base.state;
  if (!red) return { ...base, state, later: [] };
  const [line] = state.mergeLines!.lines;
  return { ...base, state: { ...state, mergeLines: { lines: [{ ...line!, hub: { ...line!.hub!, main: RED } }] } }, later: [] };
}

async function deck(scenario: Scenario) {
  mount(scenario);
  await onScreen();
  await expect.element(page.getByRole("region", { name: "Merge queue" })).toBeVisible();
}

const chip = () => document.querySelector<HTMLElement>(".deck-queue [data-main]")!;
const bubble = () => chip().querySelector(":scope > .armada-tooltip__bubble")!.textContent;

test("main's chip adds how long ago Fleet read the forge, and shows no glyph while the forge answers", async () => {
  await deck(standing(false));
  expect(bubble()).toMatch(/^Main is green\. Read \d+s ago$/);
  expect(chip().querySelector(".deck-chip__quiet")).toBeNull();
  expect(chip().querySelector(".deck-chip__dot")!.hasAttribute("data-stale")).toBe(false);
});

test("a quiet forge marks the chip with a glyph naming the time and dims the dot, and the age stays in the tooltip", async () => {
  await deck(standing(true));
  const glyph = page.getByRole("img", { name: /^GitHub not answering since \d\d:\d\d$/ });
  await expect.element(glyph).toBeVisible();
  expect(chip().querySelector(".deck-chip__dot")!.hasAttribute("data-stale")).toBe(true);
  expect(bubble()).toMatch(/^Main is green\. Read 6m ago/);
});

test("a red main's card says the same, since the light's own tooltip stands down while it is open", async () => {
  await deck(standing(true, true));
  const light = page.getByRole("button", { name: /^Main is red in/ });
  await expect.element(light).toBeVisible();
  expect(document.querySelector(".deck-queue [data-main] .deck-chip__dot")!.hasAttribute("data-stale")).toBe(true);
  await light.hover();
  const card = page.getByRole("dialog", { name: /^Main is red in/ });
  await expect.element(card.getByText(/^Read 6m ago$/)).toBeVisible();
  await expect.element(card.getByText(/^GitHub not answering since \d\d:\d\d$/)).toBeVisible();
});
