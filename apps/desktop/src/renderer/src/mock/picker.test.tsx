// The mock's own scenario picker, through `mountPicker` — the mount the page
// uses, so what is asserted here is the card the owner drags.
//
// **The picker is not part of the app**, so nothing in `testing.ts` reaches it:
// `mount` puts up `App` and the picker is a second root beside it on the page.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import "../styles/index.css";
import { mountPicker } from "./Picker";
import { clampSpot, forgetSpot, writeCollapsed, writeSpot } from "./picker-place";

const SCENARIO = "arc/executing-concurrent";

/** One `--space-4`, the step the grip takes per arrow press. */
const STEP = 16;

let unmount: (() => void) | null = null;
let host: HTMLElement | null = null;

/** The picker on a page of its own, as `main.tsx` mounts it beside the app. */
function show(): void {
  host = document.createElement("div");
  host.id = "picker";
  document.body.append(host);
  unmount = mountPicker(SCENARIO, host);
}

/** Down and up again, which is what a reload of the mock does to it. */
function remount(): void {
  take();
  show();
}

function take(): void {
  unmount?.();
  host?.remove();
  unmount = null;
  host = null;
}

beforeEach(() => {
  // Nothing remembered from the test before it, and nothing from the page.
  forgetSpot();
  writeCollapsed(false);
});

afterEach(() => {
  take();
  forgetSpot();
  writeCollapsed(false);
});

// Not the whole name: the grip's own label carries the scenario on the end of
// it, so that it contains the text a person can see on the button.
const grip = () => page.getByRole("button", { name: "Move the scenario picker" });
const card = (): DOMRect => grip().element().closest(".armada-mock-picker")!.getBoundingClientRect();

/**
 * One drag of the grip, in pointer events dispatched by hand — `sketch-pen.test.tsx`'s
 * reason: the browser helper drags between two locators, and where a card ends up
 * is a position rather than another element.
 */
function dragBy(dx: number, dy: number): void {
  const handle = grip().element();
  const from = handle.getBoundingClientRect();
  const x = from.left + from.width / 2;
  const y = from.top + from.height / 2;
  const event = (kind: string, atX: number, atY: number) =>
    new PointerEvent(kind, { clientX: atX, clientY: atY, bubbles: true, button: 0, isPrimary: true });

  handle.dispatchEvent(event("pointerdown", x, y));
  for (const part of [0.25, 0.5, 0.75, 1]) {
    handle.dispatchEvent(event("pointermove", x + dx * part, y + dy * part));
  }
  handle.dispatchEvent(event("pointerup", x + dx, y + dy));
}

test("dragged by its grip, the picker stays where it was left — across the reload a scenario causes", async () => {
  show();
  await expect.element(grip()).toBeVisible();
  const rested = card();

  // Polled, not read: a press dispatched by hand schedules React's commit
  // rather than flushing it, so the box right after the lift is the old one.
  dragBy(-500, -400);
  await expect.poll(() => card().left).toBeCloseTo(rested.left - 500, 0);
  expect(card().top).toBeCloseTo(rested.top - 400, 0);
  const moved = card();

  // Choosing a scenario reloads the page, which is this: the card comes back
  // where he put it rather than back in the corner.
  remount();
  await expect.element(grip()).toBeVisible();
  expect(card().left).toBeCloseTo(moved.left, 0);
  expect(card().top).toBeCloseTo(moved.top, 0);
});

test("the arrow keys move it too, so a drag is not the only way — and Home returns it to the corner", async () => {
  show();
  await expect.element(grip()).toBeVisible();
  const rested = card();

  // Focused and pressed for real — the grip is a button, so a keyboard has it.
  grip().element().focus();
  expect(document.activeElement).toBe(grip().element());
  await userEvent.keyboard("{ArrowUp}{ArrowLeft}");
  await expect.poll(() => card().top).toBeCloseTo(rested.top - STEP, 0);
  expect(card().left).toBeCloseTo(rested.left - STEP, 0);

  // Home is the way back for a card left somewhere awkward, with no drag in it.
  await userEvent.keyboard("{Home}");
  await expect.poll(() => card().left).toBeCloseTo(rested.left, 0);
  expect(card().top).toBeCloseTo(rested.top, 0);
});

test("minimized, it still says which scenario is on, and one press brings the picker back", async () => {
  show();
  await expect.element(page.getByRole("combobox", { name: "Mock scenario" })).toBeVisible();
  const open = card();

  await page.getByRole("button", { name: "Minimize" }).click();

  // Small enough to ignore, and it never has to be opened to know where he is.
  expect(page.getByRole("combobox", { name: "Mock scenario" }).query()).toBeNull();
  await expect.element(grip()).toHaveTextContent(SCENARIO);
  expect(card().height).toBeLessThan(open.height);

  // Collapsed across the reload too, and one press is the way back. The grip
  // is waited on first: a select that has not drawn yet is absent for the
  // wrong reason, and this assertion would pass on an empty page.
  remount();
  await expect.element(grip()).toHaveTextContent(SCENARIO);
  expect(page.getByRole("combobox", { name: "Mock scenario" }).query()).toBeNull();

  await page.getByRole("button", { name: "Expand" }).click();
  await expect.element(page.getByRole("combobox", { name: "Mock scenario" })).toBeVisible();
});

test("a spot remembered outside this window comes back inside it, rather than trapping the card off screen", async () => {
  // What a narrower window, or a wilder drag than this window allows, leaves
  // behind. Written straight to storage: the clamp on the way out is what stops
  // a drag reaching here, and this is the read-back half.
  writeSpot({ x: 9000, y: 9000 });
  show();
  await expect.element(grip()).toBeVisible();

  const at = card();
  expect(at.left).toBeGreaterThanOrEqual(0);
  expect(at.top).toBeGreaterThanOrEqual(0);
  expect(at.right).toBeLessThanOrEqual(window.innerWidth);
  expect(at.bottom).toBeLessThanOrEqual(window.innerHeight);

  // And the recovery is remembered, or the next reload reads 9000 back again.
  remount();
  await expect.element(grip()).toBeVisible();
  expect(card().left).toBeCloseTo(at.left, 0);

  // A card wider or taller than its window has no spot inside both edges. It
  // goes to the leading one, where the grip is, because that is the half that
  // moves it again.
  expect(clampSpot({ x: 500, y: 500 }, { width: 400, height: 300 }, { width: 200, height: 100 })).toEqual({
    x: 0,
    y: 0,
  });
});

test("a drag cannot leave the card outside the window it was dragged in", async () => {
  show();
  await expect.element(grip()).toBeVisible();

  dragBy(4000, 4000);
  await expect.poll(() => card().right).toBeCloseTo(window.innerWidth, 0);
  expect(card().bottom).toBeCloseTo(window.innerHeight, 0);

  dragBy(-4000, -4000);
  await expect.poll(() => card().left).toBeCloseTo(0, 0);
  expect(card().top).toBeCloseTo(0, 0);
});
