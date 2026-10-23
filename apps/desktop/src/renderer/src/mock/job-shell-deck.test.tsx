// The Job's shell docked in the Cockpit: its own panel between the Job and the merge queue, so nothing is drawn under the thread.
// Through `App` on `session-shell`. The walk `job-shell` plays it.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { s322SessionShell } from "./scenarios/session-shell";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";
import { timePasses } from "./time-passes";

unmountAfterEach();

const pane = () => page.getByRole("group", { name: "Terminal", exact: true });
const queue = () => page.getByRole("region", { name: "Merge queue" });
const terminal = () => page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Terminal", exact: true });

/** The shell's pane is under the Job's pane and over the merge queue, and not inside the Job's pane. */
async function placedBetween(view: Awaited<ReturnType<typeof toSession>>, printed: string | RegExp = "slot-1 %") {
  await expect.element(pane().getByText(printed)).toBeVisible();
  await expect.element(queue()).toBeVisible();
  expect(view.getByRole("group", { name: "Terminal", exact: true }).query()).toBeNull();
  const job = view.element().getBoundingClientRect();
  const shell = pane().element().getBoundingClientRect();
  const line = queue().element().getBoundingClientRect();
  expect(shell.top).toBeGreaterThanOrEqual(job.bottom);
  expect(shell.bottom).toBeLessThanOrEqual(line.top);
  // Full width: as wide as the Job's pane.
  expect(Math.abs(shell.width - job.width)).toBeLessThanOrEqual(2);
}

test("In the Cockpit, Terminal docks the shell as its own panel between the Job and the merge queue, outside the Job's pane", async () => {
  mount(s322SessionShell);
  await onScreen();
  const view = await toSession(/Deploy the store/);
  await terminal().click();
  await placedBetween(view);
  // The thread runs to the composer: no docked pane between them.
  const thread = view.getByRole("region", { name: "Thread" }).element().getBoundingClientRect();
  const box = view.getByRole("textbox", { name: "Message" }).element().getBoundingClientRect();
  expect(view.element().querySelector(".armada-terminal-dock")).toBeNull();
  expect(box.top - thread.bottom).toBeLessThan(box.height * 4);
});

test("In the Cockpit, the agent opening the Job's shell shows it in the deck's panel", async () => {
  mount(s322SessionShell);
  await onScreen();
  const view = await toSession(/Deploy the store/);
  timePasses();
  await placedBetween(view, /hmr update/);
});

test("In the Cockpit, the panel's height is a split kept with the deck's others, and the stage keeps its own when the panel goes", async () => {
  mount(s322SessionShell);
  await onScreen();
  const view = await toSession(/Deploy the store/);
  await terminal().click();
  await placedBetween(view);
  const split = page.getByRole("separator", { name: /^Resize Job: .* and Terminal$/ });
  await expect.element(split).toBeVisible();
  const before = pane().element().getBoundingClientRect().height;
  split.element().dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
  await expect.poll(() => pane().element().getBoundingClientRect().height).toBeGreaterThan(before);
  const kept = Object.entries(window.localStorage).filter(([key]) => key.startsWith("armada.bridge.deck-splits."));
  expect(kept.some(([, value]) => value.includes("deck-terminal"))).toBe(true);

  // Put away, the Job's pane takes the room back and the foot keeps its place.
  const foot = () => page.getByRole("region", { name: "Flight controls" }).element().getBoundingClientRect().top;
  const footAt = foot();
  await terminal().click();
  await expect.element(pane()).not.toBeInTheDocument();
  await expect.poll(() => queue().element().getBoundingClientRect().top - view.element().getBoundingClientRect().bottom).toBeLessThan(32);
  expect(Math.abs(foot() - footAt)).toBeLessThanOrEqual(2);
});
