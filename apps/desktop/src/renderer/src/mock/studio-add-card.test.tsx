// The add field opens beside the rail, through `App` — the owner's note of 1 Oct
// 2026: pressing Note opened a fixed panel in the canvas's far corner, and he
// asked for a card right next to the vertical toolbar, already taking what he
// types.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mountApp, type Mounted } from "./mount";
import { everyKind, studying } from "./studio-fleet";

let mounted: { app: Mounted; host: HTMLElement } | undefined;

afterEach(() => {
  mounted?.app.unmount();
  mounted?.host.remove();
  mounted = undefined;
});

const card = () => page.getByRole("dialog", { name: "Add a node" });
const node = (name: RegExp) => page.getByRole("group", { name });

/** A Studio with nodes on it, continued so it can be written to. */
async function openEditable(): Promise<void> {
  const fleet = studying([everyKind("01JOBEVERYKIND0000000000000")]);
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  mounted = { app: mountApp(fleet.scenario, host), host };
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
  await page.getByRole("cell", { name: "Every kind of node and edge", exact: true }).click();
  await page.getByRole("button", { name: "Continue" }).click();
  await expect.element(page.getByRole("button", { name: "Add a Note", exact: true })).toBeVisible();
}

test("pressing Note opens the field beside the rail, already focused, and not in the corner", async () => {
  await openEditable();
  const press = page.getByRole("button", { name: "Add a Note", exact: true });
  await press.click();

  await expect.element(card()).toBeVisible();
  const field = card().getByLabelText("Note", { exact: true });
  await expect.element(field).toHaveFocus();

  // Beside the button pressed: to its right, starting level with it. The aside
  // is the top-right panel he refused, so the card is not drawn there either —
  // there is no role that names a corner, so this one fact is read off markup.
  const button = press.element().getBoundingClientRect();
  const drawn = card().element().getBoundingClientRect();
  expect(drawn.left).toBeGreaterThanOrEqual(button.right);
  expect(drawn.left - button.right).toBeLessThan(button.width);
  expect(Math.abs(drawn.top - button.top)).toBeLessThan(button.height);
  const aside = document.querySelector(".armada-graph-canvas__aside");
  expect(aside?.contains(card().element()) ?? false).toBe(false);

  // Adding closes it, and the note lands on the board.
  await userEvent.fill(field, "The legend wraps on a narrow window");
  await card().getByRole("button", { name: "Add note" }).click();
  await expect.element(node(/^Note: The legend wraps on a narrow window/)).toBeVisible();
  expect(card().query()).toBeNull();
});

test("a press outside keeps what is typed, closes a blank card, and Esc always closes", async () => {
  await openEditable();
  const canvas = () => document.querySelector<HTMLElement>(".react-flow__pane")!;
  const outside = () => canvas().dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 }));

  await page.getByRole("button", { name: "Add a Sketch", exact: true }).click();
  await userEvent.fill(card().getByLabelText("Sketch", { exact: true }), "Board -> Detail");
  outside();
  await expect.element(card()).toBeVisible();

  await userEvent.keyboard("{Escape}");
  await expect.poll(() => card().query()).toBeNull();

  await userEvent.keyboard("N");
  await expect.element(card().getByLabelText("Note", { exact: true })).toHaveFocus();
  outside();
  await expect.poll(() => card().query()).toBeNull();
});

test("a Link pasted in the card offers its line and both choices, and Cancel closes it", async () => {
  await openEditable();
  await page.getByRole("button", { name: "Add a Link", exact: true }).click();
  const field = card().getByLabelText("Link", { exact: true });
  await expect.element(field).toHaveFocus();
  await userEvent.fill(field, "docs/contracts/design-system.md");
  await expect.element(card().getByRole("group", { name: "What to do with this address" })).toBeVisible();
  await expect.element(card().getByRole("button", { name: "Keep the link" })).toBeEnabled();
  await expect.element(card().getByRole("button", { name: "Read it in" })).toBeVisible();
  await card().getByRole("button", { name: "Cancel" }).click();
  await expect.poll(() => card().query()).toBeNull();
});
