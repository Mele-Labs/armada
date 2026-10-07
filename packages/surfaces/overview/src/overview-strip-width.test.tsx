// Overview's summary tiles, measured through `App` at the width the owner hit.
//
// **A geometry test, so it lives here and not in a story.** Which row a tile
// lands on is decided by the whole window, and a story drawing one composition
// in a sized `div` cannot see that — `job-detail-width.test.tsx` is the
// precedent, and names `Layer.test.tsx` as its own.
//
// The note: *On my 14" laptop this is a 2 x 2 grid, but that seems unnecessary.
// We should be able to span these in a single row. The labels and values only
// take up like 10% of the width of the cards now.*

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

/** The owner's own window: a 14" MacBook Pro is 1512 points wide. */
const LAPTOP = { width: 1512, height: 982 };

/** The project's own size, back after every test. The viewport outlives an unmount. */
const RESTING = { width: 1440, height: 900 };
afterEach(async () => {
  await page.viewport(RESTING.width, RESTING.height);
});

/** `--layout-breakpoint`, read rather than retyped — the one collapse point this uses. */
function breakpoint(): number {
  const value = parseFloat(
    getComputedStyle(document.documentElement).getPropertyValue("--layout-breakpoint"),
  );
  if (!Number.isFinite(value)) throw new Error("--layout-breakpoint is not declared");
  return value;
}

/** Every tile's box, in the order the strip draws them. */
function tiles(): DOMRect[] {
  const found = document.querySelectorAll<HTMLElement>(".armada-overview-summary__item");
  if (found.length === 0) throw new Error("the summary strip is not drawn");
  return [...found].map((one) => one.getBoundingClientRect());
}

/**
 * How many rows the tiles sit on. **Tops are compared with a tolerance**: a
 * `DOMRect` is a float, and two boxes on one row can differ in the last place.
 */
function rowsOf(boxes: readonly DOMRect[]): number {
  const tops: number[] = [];
  for (const box of boxes) {
    if (!tops.some((top) => Math.abs(top - box.top) < 1)) tops.push(box.top);
  }
  return tops.length;
}

async function overview(width: number, height: number): Promise<void> {
  await page.viewport(width, height);
  mount("kinds", { slices: ["core", "overview"] });
  await expect.element(page.getByRole("navigation", { name: "What is on Overview" })).toBeVisible();
}

test("the summary tiles span one row on a 14-inch laptop", async () => {
  await overview(LAPTOP.width, LAPTOP.height);

  const boxes = tiles();
  expect(boxes.length).toBeGreaterThan(1);
  expect(rowsOf(boxes)).toBe(1);
  // The complaint was the height a second row spent: every tile is shorter than
  // it is wide there, which is what says the band is a strip and not a grid.
  for (const box of boxes) expect(box.height).toBeLessThan(box.width);
});

test("below the collapse point the tiles go back to two by two", async () => {
  await overview(breakpoint() - 1, LAPTOP.height);

  const boxes = tiles();
  expect(rowsOf(boxes)).toBe(Math.ceil(boxes.length / 2));
});
