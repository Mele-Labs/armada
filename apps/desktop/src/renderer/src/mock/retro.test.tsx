// Job retros in Bridge (23.12, `docs/concepts/retro.md`): the Lessons page
// lists what got in the way across Jobs, newest first, and a row opens its
// Job's retro; the same retro opens from the Job's own Record. Nothing on
// either acts.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { entered, mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const JOB_3_FIRST = /out_of_bounds on armada\.yml/;

test("Lessons is a rail row in Work, and lists every Job's retro items newest first", async () => {
  mount("retro/lessons");
  await onScreen();

  await page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Lessons", exact: true }).click();
  const list = page.getByRole("region", { name: "Lessons" });
  await expect.element(list).toBeVisible();
  // The rail marks where the window is.
  expect(document.querySelector('[aria-current="page"]')?.textContent).toMatch(/Lessons/);
  const rows = list.getByRole("row").elements();
  // Job 3's six items, then Job 2's one: Fleet's order, newest retro first.
  expect(rows.map((row) => row.querySelector(".armada-lessons__job")?.textContent)).toEqual([
    ...Array<string>(6).fill("Job 3"),
    "Job 2",
  ]);
  // Whose way is a mark its tooltip names, never a word in the row.
  expect(rows[0]?.querySelector('[role="img"]')?.getAttribute("aria-label")).toBe("Fleet");
});

test("a row opens its Job's retro, each item with the record rows it cites, and the owner's notes", async () => {
  mount("retro/lessons");
  await onScreen();

  await page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Lessons", exact: true }).click();
  await page.getByRole("row", { name: JOB_3_FIRST }).click();
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await entered(sheet);
  await expect.element(sheet.getByText("Job 3")).toBeVisible();
  await expect.element(sheet.getByText("armada.yml changed; the line came in with 4e1c2a9 on main")).toBeVisible();
  await expect.element(sheet.getByText("helm via http").first()).toBeVisible();
  await expect.element(sheet.getByRole("region", { name: "Notes" })).toBeVisible();
  // Nothing on a retro acts: its close is its only control.
  expect(sheet.getByRole("button").elements().map((one) => one.textContent)).toEqual([
    expect.stringContaining("Close"),
  ]);
});

test("a Job's own retro opens from its Record", async () => {
  mount("retro/job-3");
  await onScreen();

  await page.getByRole("tab", { name: /^Record/ }).last().click();
  await page.getByRole("button", { name: "Retro", exact: true }).click();
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await entered(sheet);
  await expect.element(sheet.getByText(/without ever seeing screens_test pass/)).toBeVisible();
});

test("a Job whose retro is not written says so, and nothing more", async () => {
  mount("judge/refusal-agreed");
  await onScreen();

  await page.getByRole("tab", { name: /^Record/ }).last().click();
  await page.getByRole("button", { name: "Retro", exact: true }).click();
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await entered(sheet);
  await expect.element(sheet.getByText("Not written yet")).toBeVisible();
});
