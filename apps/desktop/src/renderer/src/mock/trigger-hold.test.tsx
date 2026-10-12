// The mock Fleet's Rerun and Skip, answered as Fleet answers them: a rerun that passes lets the Job go,
// a skip records the firing skipped by the owner, and each refuses where nothing holds the Job.

import { expect, test } from "vitest";
import { page, type Locator } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const refusedWith = (answer: unknown): string => JSON.stringify(answer);

// Pressed at the button itself, not by pointer: the canvas fits again as the Job's panels take their
// columns, and a pointer press lands on whatever stands over the button while it moves. Under a 12x
// CPU slowdown the press was taken by the Now panel's header and a Pull request node, not the button.
async function pressOnCanvas(button: Locator): Promise<void> {
  await expect.element(button).toBeEnabled();
  (button.element() as HTMLElement).click();
}

test("mock hold: Rerun passes the Command and lets the hold go: the firing is passed and the Job's alert is gone", async () => {
  mount("real/job-2-trigger-hold");
  await onScreen();
  await expect.element(page.getByRole("img", { name: "Alert" })).toBeVisible();
  await pressOnCanvas(page.getByRole("button", { name: "Rerun" }).first());
  await expect.element(page.getByRole("img", { name: "Alert" })).not.toBeInTheDocument();
  await expect.element(page.getByRole("button", { name: "Rerun" })).not.toBeInTheDocument();
});

test("mock hold: Skip records the firing skipped by the owner and lets the hold go", async () => {
  mount("real/job-2-trigger-hold-skipped");
  await onScreen();
  await pressOnCanvas(page.getByRole("button", { name: "Skip" }).first());
  await expect.element(page.getByRole("img", { name: /deploy_qa. was skipped by the owner/ }).first()).toBeVisible();
  await expect.element(page.getByRole("img", { name: "Alert" })).not.toBeInTheDocument();
});

test("mock hold: both acts refuse as Fleet does where nothing holds the Job, and where the body names the wrong things", async () => {
  mount("real/job-2-trigger-hold");
  await onScreen();
  const id = document.querySelector("[data-armada-open-job]")?.getAttribute("data-armada-open-job") ?? "";
  for (const act of [window.armada.rerunTrigger, window.armada.skipTrigger]) {
    expect(refusedWith(await act(id, { trigger: "nothing" }))).toContain("fleet.no_hold");
    expect(refusedWith(await act(id, {}))).toContain("fleet.no_hold_named");
    expect(refusedWith(await act(id, { trigger: "deploy_qa", addition: "a1" }))).toContain("fleet.no_hold_named");
  }
});

test("mock hold: a Trigger that is only being repaired holds nothing: it does not block, so both acts are refused", async () => {
  mount("real/job-2-trigger-repair");
  await onScreen();
  const id = document.querySelector("[data-armada-open-job]")?.getAttribute("data-armada-open-job") ?? "";
  expect(refusedWith(await window.armada.rerunTrigger(id, { trigger: "deploy_qa" }))).toContain("fleet.no_hold");
});

test("mock asks: the Board row rings, Run on the leaf passes the Command once and the bell goes", async () => {
  mount("real/job-2-trigger-asks");
  await onScreen();
  // The Job asks, so a call comes forward over the Dashboard; its Open Job opens the Job.
  await expect.element(page.getByRole("region", { name: /^Job:/ })).toBeVisible();
  await page.getByRole("button", { name: /^Open Job/ }).click();
  await pressOnCanvas(page.getByRole("button", { name: "Run", exact: true }));
  await expect.element(page.getByRole("button", { name: "Run", exact: true })).not.toBeInTheDocument();
  await expect.element(page.getByRole("img", { name: "Alert" })).not.toBeInTheDocument();
  const id = document.querySelector("[data-armada-open-job]")?.getAttribute("data-armada-open-job") ?? "";
  expect(refusedWith(await window.armada.rerunTrigger(id, { trigger: "wipe_qa" }))).toContain("fleet.no_hold");
});

test("mock asks: Skip records the firing skipped by the owner and the bell goes", async () => {
  mount("real/job-2-trigger-asks");
  await onScreen();
  await page.getByRole("button", { name: /^Open Job/ }).click();
  await pressOnCanvas(page.getByRole("button", { name: "Skip", exact: true }).first());
  await expect.element(page.getByRole("img", { name: /wipe_qa. was skipped by the owner/ }).first()).toBeVisible();
  await expect.element(page.getByRole("img", { name: "Alert" })).not.toBeInTheDocument();
});

test("mock asks: a step added to the Job on a destructive Command rings the row, and Run on its leaf passes it once", async () => {
  mount("real/job-2-added-step-asks");
  await onScreen();
  // The Job asks, so a call comes forward over the Dashboard; its Open Job opens the Job.
  await expect.element(page.getByRole("region", { name: /^Job:/ })).toBeVisible();
  await page.getByRole("button", { name: /^Open Job/ }).click();
  await page.getByRole("tab", { name: "Workflow" }).click();
  await pressOnCanvas(page.getByRole("button", { name: "Run", exact: true }));
  await expect.element(page.getByRole("button", { name: "Run", exact: true })).not.toBeInTheDocument();
  await expect.element(page.getByRole("img", { name: "Alert" })).not.toBeInTheDocument();
  const id = document.querySelector("[data-armada-open-job]")?.getAttribute("data-armada-open-job") ?? "";
  expect(refusedWith(await window.armada.rerunTrigger(id, { addition: "a1" }))).toContain("fleet.no_hold");
});

test("mock asks: Skip on an added step records it skipped by the owner and the bell goes", async () => {
  mount("real/job-2-added-step-asks");
  await onScreen();
  await page.getByRole("button", { name: /^Open Job/ }).click();
  await page.getByRole("tab", { name: "Workflow" }).click();
  await pressOnCanvas(page.getByRole("button", { name: "Skip", exact: true }).first());
  await expect.element(page.getByRole("img", { name: "Alert" })).not.toBeInTheDocument();
  const id = document.querySelector("[data-armada-open-job]")?.getAttribute("data-armada-open-job") ?? "";
  expect(refusedWith(await window.armada.skipTrigger(id, { addition: "a1" }))).toContain("fleet.no_hold");
});
