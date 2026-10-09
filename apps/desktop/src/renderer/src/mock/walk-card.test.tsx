// The walk's card: it folds to a pill on `.` and unfolds on it, Next still reachable, and a header drag keeps its spot.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountWalk } from "./WalkPlayer";
import { text, walk } from "./walk";

let host: HTMLElement | undefined;
afterEach(() => {
  host?.remove();
  window.sessionStorage.removeItem("armada.mock.walk-card");
});

function play(): void {
  document.body.innerHTML = '<p id="target">Hello</p>';
  host = document.createElement("div");
  document.body.append(host);
  mountWalk("probe", walk("none", [{ look: text("Hello"), say: "Look here" }, { look: text("Hello"), say: "And here" }]), false, host);
}

test(". folds the card to a pill with the step and Next, and unfolds it again", async () => {
  play();
  await expect.element(page.getByText("Look here")).toBeVisible();
  await userEvent.keyboard(".");
  await expect.element(page.getByText("Step 1 of 2")).toBeVisible();
  expect(document.body.textContent).not.toContain("Look here");
  await expect.element(page.getByRole("button", { name: "Next" })).toBeEnabled();
  await userEvent.keyboard(".");
  await expect.element(page.getByText("Look here")).toBeVisible();
});

test("a drag by the header moves the card and the spot is kept for the session", async () => {
  play();
  await expect.element(page.getByText("Look here")).toBeVisible();
  const card = document.querySelector<HTMLElement>("[data-walk-state]")!;
  const head = card.querySelector<HTMLElement>(".armada-mock-walk__head")!;
  const was = card.getBoundingClientRect();
  const grab = head.getBoundingClientRect();
  const at = { x: grab.left + 20, y: grab.top + 10 };
  // Within the window, since a card is kept inside it.
  const by = { x: -Math.floor(was.left / 2), y: -Math.floor(was.top / 2) };
  head.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, clientX: at.x, clientY: at.y }));
  window.dispatchEvent(new PointerEvent("pointermove", { clientX: at.x + by.x, clientY: at.y + by.y }));
  window.dispatchEvent(new PointerEvent("pointerup", { clientX: at.x + by.x, clientY: at.y + by.y }));
  await expect.poll(() => Math.round(card.getBoundingClientRect().left)).toBe(Math.round(was.left + by.x));
  expect(JSON.parse(window.sessionStorage.getItem("armada.mock.walk-card")!)).toEqual({ x: Math.round(was.left + by.x), y: Math.round(was.top + by.y) });
});
