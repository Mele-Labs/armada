// The deck's two side columns resize by a handle on their inner edge: the centre takes the rest, the widths are kept on this
// machine, each column stops at its floor and the centre at its own, and a double click puts a column back.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const CENTRE_FLOOR = 480;
// Nothing is selected on mount, so the widths are kept under the idle layout's key.
const KEY = "armada.bridge.deck-columns.idle";
const left = () => page.getByRole("separator", { name: "Resize the left column" });
const right = () => page.getByRole("separator", { name: "Resize the right column" });
const width = (selector: string) => document.querySelector(selector)!.getBoundingClientRect().width;
const widths = () => ({ left: width(".deck-left"), centre: width(".deck-centre"), right: width(".deck-right") });
const now = (separator: ReturnType<typeof left>) => Number((separator.element() as HTMLElement).getAttribute("aria-valuenow"));
const bound = (separator: ReturnType<typeof left>, which: "min" | "max") => Number((separator.element() as HTMLElement).getAttribute(`aria-value${which}`));

async function deck() {
  await page.viewport(1440, 900);
  const app = mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(left()).toBeInTheDocument();
  return app;
}

beforeEach(() => localStorage.removeItem(KEY));
afterEach(() => localStorage.removeItem(KEY));

test("dragging the left handle widens the left column, the centre gives up the same width, and it is kept after a reload", async () => {
  const app = await deck();
  const before = widths();
  const handle = left().element() as HTMLElement;
  const box = handle.getBoundingClientRect();
  // It sits in the gap between the panels, over neither, and draws nothing at rest.
  const gap = { from: document.querySelector(".deck-left")!.getBoundingClientRect().right, to: document.querySelector(".deck-centre")!.getBoundingClientRect().left };
  expect(box.left).toBeGreaterThanOrEqual(gap.from);
  expect(box.right).toBeLessThanOrEqual(gap.to);
  expect(getComputedStyle(handle, "::before").backgroundColor).toBe("rgba(0, 0, 0, 0)");
  await userEvent.dragAndDrop(left(), left(), { sourcePosition: { x: box.width / 2, y: box.height / 2 }, targetPosition: { x: box.width / 2 + 60, y: box.height / 2 } });

  await expect.poll(() => Math.round(widths().left - before.left)).toBe(60);
  const after = widths();
  expect(Math.round(before.centre - after.centre)).toBe(60);
  expect(Math.round(after.right)).toBe(Math.round(before.right));
  expect(JSON.parse(localStorage.getItem(KEY) ?? "{}").left).toBe(Math.round(after.left));

  // A reload: the app comes down and is mounted again with the same storage.
  app.unmount();
  document.getElementById("root")?.remove();
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(left()).toBeInTheDocument();
  expect(Math.round(widths().left)).toBe(Math.round(after.left));
});

test("each column stops at its floor and the centre at its own", async () => {
  await deck();
  const press = async (separator: ReturnType<typeof left>, key: string, times: number) => {
    (separator.element() as HTMLElement).focus();
    for (let at = 0; at < times; at += 1) await userEvent.keyboard(key);
  };

  // Widening the left column: End takes it as far as the centre's floor allows, and no further.
  await press(left(), "{End}", 1);
  expect(now(left())).toBe(bound(left(), "max"));
  expect(Math.round(widths().left)).toBe(bound(left(), "max"));
  expect(Math.round(widths().centre)).toBe(CENTRE_FLOOR);
  // The right column cannot widen into a centre already at its floor.
  await press(right(), "{ArrowLeft}", 5);
  expect(Math.round(widths().right)).toBe(now(right()));
  expect(bound(right(), "max")).toBe(now(right()));
  expect(Math.round(widths().centre)).toBe(CENTRE_FLOOR);

  // Narrowing: arrows step down to the floor and no further.
  await press(left(), "{ArrowLeft}", 40);
  expect(now(left())).toBe(bound(left(), "min"));
  expect(Math.round(widths().left)).toBe(bound(left(), "min"));
  await press(right(), "{End}", 1);
  expect(Math.round(widths().right)).toBe(bound(right(), "min"));
  await press(right(), "{ArrowRight}", 3);
  expect(Math.round(widths().right)).toBe(bound(right(), "min"));
});

test("a double click resets a column, and Home does", async () => {
  await deck();
  const rest = widths();
  (left().element() as HTMLElement).focus();
  await userEvent.keyboard("{ArrowRight}{ArrowRight}");
  await expect.poll(() => Math.round(widths().left - rest.left)).toBe(32);
  await userEvent.dblClick(left());
  await expect.poll(() => Math.round(widths().left)).toBe(Math.round(rest.left));
  expect(JSON.parse(localStorage.getItem(KEY) ?? "{}").left).toBeUndefined();

  (right().element() as HTMLElement).focus();
  await userEvent.keyboard("{ArrowLeft}");
  await expect.poll(() => Math.round(widths().right - rest.right)).toBe(16);
  await userEvent.keyboard("{Home}");
  await expect.poll(() => Math.round(widths().right)).toBe(Math.round(rest.right));
});
