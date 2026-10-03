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
  await expect.element(sheet.getByText(/fixed 15 s timeouts ran out under load/)).toBeVisible();
  // Where each item's fix lands, beside whose way it got in.
  expect(sheet.getByRole("img", { name: "Lands in Kit" }).elements()).toHaveLength(1);
  expect(sheet.getByRole("img", { name: "Lands in the Manifest" }).elements()).toHaveLength(2);
});

/** The Lessons surface, and the statements its list draws. */
async function lessons(): Promise<() => string[]> {
  await page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Lessons", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Lessons" })).toBeVisible();
  return () =>
    [...document.querySelectorAll(".armada-lessons__statement")].map((one) => one.textContent ?? "");
}

test("the retro lessons tabs narrow the list to where each fix lands, with All first and no counts", async () => {
  mount("retro/lessons");
  await onScreen();
  const statements = await lessons();

  const tabs = page.getByRole("tablist").last().getByRole("tab").elements();
  expect(tabs.map((one) => one.textContent)).toEqual(["All", "Armada", "Kit", "Manifest"]);
  expect(tabs[0]?.getAttribute("aria-selected")).toBe("true");
  expect(statements()).toHaveLength(7);

  await page.getByRole("tab", { name: "Kit", exact: true }).click();
  await expect.poll(() => statements()).toEqual([expect.stringMatching(/^T4 had to ask you to allow/)]);
  await page.getByRole("tab", { name: "Manifest", exact: true }).click();
  await expect.poll(() => statements()).toEqual([
    expect.stringMatching(/fixed 15 s timeouts/),
    "A docs edit set off every Rust test.",
  ]);
  await page.getByRole("tab", { name: "Armada", exact: true }).click();
  // Job 2's item predates where a fix lands, so it is under All alone.
  await expect.poll(() => statements()).toHaveLength(3);
  expect(statements().some((one) => one.startsWith("The Judge's question waited"))).toBe(false);
  // Each row says where its fix lands, beside whose way.
  const row = page.getByRole("row", { name: JOB_3_FIRST }).element();
  expect([...row.querySelectorAll('[role="img"]')].map((one) => one.getAttribute("aria-label"))).toEqual([
    "Fleet",
    "Lands in Armada",
  ]);
});

test("the retro lessons tab is remembered for the viewer", async () => {
  localStorage.setItem("armada.bridge.lessons-tab", "kit");
  mount("retro/lessons");
  await onScreen();
  const statements = await lessons();

  await expect.element(page.getByRole("tab", { name: "Kit", exact: true })).toHaveAttribute("aria-selected", "true");
  await expect.poll(() => statements()).toHaveLength(1);
  await page.getByRole("tab", { name: "Manifest", exact: true }).click();
  expect(localStorage.getItem("armada.bridge.lessons-tab")).toBe("manifest");
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
