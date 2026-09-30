// What the Settings destination is, measured off the DOM through `App`.
//
// **Geometry and ancestry, which is why these are here and not in a story.**
// Whether a card is glass is decided by what it was rendered inside, and
// whether the board reaches the trailing edge is decided by the window — a
// story drawing one composition in a sized `div` can see neither. The defect
// it is written against: one left-aligned column a little past half the width
// of a 1600px window, with nothing on it a card (the owner, 29 Sep 2026).

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

// The project's own size, back after every test: the viewport outlives an
// unmount.
const RESTING = { width: 1440, height: 900 };
afterEach(async () => {
  await page.viewport(RESTING.width, RESTING.height);
});

/** A spacing token, read rather than retyped. */
function space(step: number): number {
  const value = parseFloat(
    getComputedStyle(document.documentElement).getPropertyValue(`--space-${step}`),
  );
  if (!Number.isFinite(value)) throw new Error(`--space-${step} is not declared`);
  return value;
}

/** One element's box, or a failure naming what was not on screen. */
function boxOf(selector: string): DOMRect {
  const found = document.querySelector(selector);
  if (found === null) throw new Error(`nothing matched ${selector}`);
  return found.getBoundingClientRect();
}

/** Every card on the destination, in the order the board lays them out. */
function cards(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>(".armada-settings-tab .armada-destination-card")];
}

/** Open a Job's Settings and wait for the board to be drawn. */
async function settings(scenario: string): Promise<void> {
  mount(scenario);
  await onScreen();
  await page.getByRole("tab", { name: /^Settings/ }).click();
}

test("every section is a card, and the canvas under it is what makes it glass", async () => {
  await settings("job/running");
  await expect.poll(() => cards().length).toBeGreaterThanOrEqual(3);

  for (const card of cards()) {
    // The recipe is taken, never copied — `glass.css`, and the class is how.
    expect(card.classList.contains("armada-glass")).toBe(true);
    const drawn = getComputedStyle(card);
    expect(parseFloat(drawn.borderTopWidth)).toBeGreaterThan(0);
    expect(drawn.boxShadow).not.toBe("none");
    // The gradient sits on a layer behind the content rather than on the card.
    expect(getComputedStyle(card, "::before").backgroundImage).toContain("gradient");
  }
});

test("the board reaches the destination's trailing edge, two cards to a row", async () => {
  await page.viewport(1600, 1000);
  await settings("job/running");
  await expect.poll(() => cards().length).toBeGreaterThanOrEqual(3);

  const [first, second] = cards().map((card) => card.getBoundingClientRect());
  // Side by side is two facts: the second starts after the first ends, and
  // the two share a band of the screen.
  expect(second!.left).toBeGreaterThanOrEqual(first!.right);
  expect(second!.top).toBeLessThan(first!.bottom);
  // And the row spends the whole destination. Half a window of nothing is
  // what the single column left.
  expect(second!.right).toBeGreaterThanOrEqual(boxOf(".armada-settings-tab").right - 1);
});

test("a cap sits in a box of its own, with Raise against the figure it raises", async () => {
  await settings("job/running");
  await expect.poll(() => document.querySelectorAll('[data-ceiling="cost"]').length).toBe(1);

  const raising = document.querySelector('[data-ceiling="cost"]')!;
  const row = raising.closest(".armada-proposal__field")!;
  const label = row.querySelector(".armada-proposal__field-label")!.getBoundingClientRect();
  const value = row.querySelector(".armada-proposal__field-value")!.getBoundingClientRect();
  const raise = raising.querySelector("button")!.getBoundingClientRect();

  // The label carries no box, and the value does.
  expect(getComputedStyle(row.querySelector(".armada-proposal__field-label")!).backgroundColor).toBe(
    "rgba(0, 0, 0, 0)",
  );
  expect(value.left).toBeGreaterThan(label.right);
  // The box holds the figure rather than the row: stretched across the column
  // it reads as an empty field, and the act beside it reads as the label's.
  expect(value.width).toBeLessThan(row.getBoundingClientRect().width / 2);
  // Against the value, not out at the row's trailing edge, which is where the
  // owner read it floating off the label.
  expect(raise.left - value.right).toBeLessThanOrEqual(space(2) + 1);
});

test("what froze at approval is one card, with the instant in its head", async () => {
  await settings("arc/approved-frozen");
  const card = page.getByRole("region", { name: "Frozen at approval" });
  await expect.element(card).toBeVisible();

  // The three regions that were left homeless when Overview stopped drawing
  // the proposal, and nothing else.
  for (const region of ["Workflow", "How it lands", "Model per tier"]) {
    await expect.element(card.getByRole("heading", { name: region })).toBeVisible();
  }
  expect(document.querySelector(".armada-settings-tab__frozen-at")?.textContent).toContain("2026");
});

test("a Job that is over says why, and draws no card of controls", async () => {
  await settings("job/completedSuccess");
  await expect
    .element(page.getByText("This job has finished, so there is nothing left for a change to reach."))
    .toBeVisible();
  expect(cards()).toHaveLength(0);
});
