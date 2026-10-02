// A Check's log opens from the Checks strip wherever it is drawn, through `App` and nothing else.
// The owner, 2 Oct 2026: *I wish in the little check component that I could click on the specific
// check to open a panel and see the live log of the check. This should work anywhere that little
// component is displayed.*
//
// Where it is drawn: a plan group's panel, the Plan's list, a task's Record row (those two are
// `record.test.tsx`'s and `arc-implement.test.tsx`'s), and each merge line row. A segment is a
// press while the strip is shut; a row is, once it is open.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const log = () => page.getByRole("dialog", { name: "Check log" });
const boundary = () => page.getByRole("region", { name: "Checks at this boundary" });

/** Group three's panel on Plan's graph, its strip shut. */
async function groupThree(): Promise<void> {
  mount("check-logs");
  await page.getByRole("tab", { name: /^Plan/ }).click();
  await page.getByRole("button", { name: /^Group 3, / }).click();
  await entered(page.getByRole("dialog", { name: /^Group 3/ }));
}

test("a running Check's segment on a plan group opens its log, which grows as the Check prints", async () => {
  await groupThree();
  await boundary().getByRole("button", { name: "screens_test, running" }).click();

  await expect.element(log().getByText("plan-board.test.ts")).toBeVisible();
  await expect.element(log().getByRole("img", { name: "Being written" })).toBeVisible();
  // Not there when it opened; written while it is open.
  await expect.element(log().getByText("running.test.tsx"), { timeout: 8_000 }).toBeVisible();
});

test("a Check that has not started is no button, and opens nothing", async () => {
  await groupThree();
  await expect.element(boundary().getByRole("button", { name: "screens_test, running" })).toBeVisible();
  expect(boundary().getByRole("button", { name: "desktop_test, not run" }).query()).toBeNull();
});

test("an ended Check's row opens its whole log, with no mark, and Esc puts the log away first", async () => {
  await groupThree();
  await boundary().getByRole("button", { name: "Checks", exact: true }).click();
  await boundary().getByRole("button", { name: "test, passed", exact: true }).click();

  await expect.element(log().getByText("4154 passed")).toBeVisible();
  expect(log().getByRole("img", { name: "Being written" }).query()).toBeNull();

  await userEvent.keyboard("{Escape}");
  await expect.poll(() => log().query()).toBeNull();
  // The group's panel the log was opened over is still there.
  await expect.element(page.getByRole("dialog", { name: /^Group 3/ })).toBeVisible();
});

test("a passed group's Check opens the log its run kept", async () => {
  mount("check-logs");
  await page.getByRole("tab", { name: /^Plan/ }).click();
  await page.getByRole("button", { name: /^Group 2, / }).click();
  await entered(page.getByRole("dialog", { name: /^Group 2/ }));
  await boundary().getByRole("button", { name: "screens_test, passed" }).click();

  await expect.element(log().getByText("1383 passed (1383)")).toBeVisible();
});

test("a merge line row's running Check opens its log, which grows as the runner writes it", async () => {
  mount("check-logs");
  await page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Merge line", exact: true }).click();
  const turn = page.getByRole("listitem", { name: /^worktree-agent-aef3c24792026e2c3/ });
  await turn.getByRole("button", { name: "screens_test, running" }).click();

  await expect.element(log()).toHaveTextContent("screens_test · worktree-agent-aef3c24792026e2c3");
  await expect.element(log().getByText("merge-line.test.ts")).toBeVisible();
  await expect.element(log().getByText("Row.test.ts"), { timeout: 8_000 }).toBeVisible();
});

test("a merge line Check that failed and was sent back opens its whole log", async () => {
  mount("check-logs");
  await page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Merge line", exact: true }).click();
  const sent = page.getByRole("listitem", { name: /^fleet\/pulse-log-rows/ });
  await sent.getByRole("button", { name: "desktop_test, failed" }).click();

  await expect.element(log().getByText("AssertionError: expected 2 to be 1")).toBeVisible();
  expect(log().getByRole("img", { name: "Being written" }).query()).toBeNull();
  // A Check still waiting in the turn has no log, so it is no button.
  const turn = page.getByRole("listitem", { name: /^worktree-agent-aef3c24792026e2c3/ });
  expect(turn.getByRole("button", { name: "desktop_test, not run" }).query()).toBeNull();
});
