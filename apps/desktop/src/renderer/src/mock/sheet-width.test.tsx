// Each kind of sheet remembers its own width (owner, 2 Oct 2026), through
// `App` on Plan: a task's panel and a group's panel, which shared one
// remembered width until every sheet took a kind. `everySheetResizes` walks it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { entered, mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const TASK = "Draw what is running, in four lists";

/** A panel opened from its card on Plan's graph, and its handle's width. */
async function opened(card: RegExp, title: string) {
  await page.getByRole("button", { name: card }).click();
  const panel = page.getByRole("dialog", { name: title });
  await entered(panel);
  const handle = page.getByRole("separator", { name: `Resize ${title}` });
  await expect.poll(() => Number(handle.element().getAttribute("aria-valuenow"))).toBeGreaterThan(0);
  return { panel, handle, width: () => Number(handle.element().getAttribute("aria-valuenow")) };
}

test("a task's panel dragged wider leaves a group's panel at its own width, and reopens as wide", async () => {
  mount("arc/executing-sequential");
  await onScreen();
  await page.getByRole("tab", { name: /^Plan/ }).click();

  const group = await opened(/^Group 3, /, "Group 3");
  const groupAtRest = group.width();
  await group.panel.getByRole("button", { name: "Close" }).click();

  const task = await opened(new RegExp(`^${TASK}, `), TASK);
  const taskAtRest = task.width();
  task.handle.element().focus();
  await userEvent.keyboard("{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}{ArrowLeft}");
  await expect.poll(task.width).toBe(taskAtRest + 80);
  await task.panel.getByRole("button", { name: "Close" }).click();

  const again = await opened(/^Group 3, /, "Group 3");
  expect(again.width()).toBe(groupAtRest);
  await again.panel.getByRole("button", { name: "Close" }).click();

  const reopened = await opened(new RegExp(`^${TASK}, `), TASK);
  expect(reopened.width()).toBe(taskAtRest + 80);
});
