// Job retros in Bridge (23.12, `docs/concepts/retro.md`): the Retros page
// lists what got in the way across Jobs, newest first, each item as words, a
// headline, what happened, a fix and two answers. A Job label opens its retro,
// and the same retro opens from the Job's own Record. Agree proposes a Job or
// saves a Kit item; Disagree discards.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { entered, mount, onScreen, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Jobs only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "jobs"] } as const;
// The Kit page is Manifest's.
const WITH_KIT = { slices: ["core", "jobs", "manifest"] } as const;

const STALE_MAIN = /blamed the Drone for Fleet's own mistake/;
const GREP = /had to wait for grep to be allowed/;
const DOCS = /A docs edit ran every Rust test/;

const card = (title: RegExp) => page.getByRole("listitem").filter({ hasText: title });

/** The Retros surface, and the headlines its cards draw. */
async function lessons(): Promise<() => string[]> {
  await page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Retros", exact: true }).click();
  await expect.element(page.getByRole("list", { name: "Retros" })).toBeVisible();
  return () => [...document.querySelectorAll(".armada-lesson")].map((one) => one.getAttribute("aria-label") ?? "");
}

test("Retros is a rail row in Work, and lists every Job's items newest first", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();

  const titles = await lessons();
  // The rail marks where the window is.
  expect(document.querySelector('[aria-current="page"]')?.textContent).toMatch(/Retros/);
  const from = [...document.querySelectorAll(".armada-lesson__from")].map((one) => one.textContent);
  // Job 3's four items, then Job 2's one: Fleet's order, newest retro first.
  expect(from).toEqual([...Array<string>(4).fill("Job 3"), "Job 2"]);
  expect(titles()).toHaveLength(5);
});

test("an item says whose way and where in words, and an old item has its statement alone", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  await lessons();

  const one = card(STALE_MAIN);
  const words = [...one.element().querySelectorAll(".armada-lesson__word")].map((word) => word.textContent);
  expect(words).toEqual(["Fleet", "Armada"]);
  await expect.element(one.getByRole("heading")).toBeVisible();
  await expect.element(one.getByText(/Compare against origin\/main/)).toBeVisible();
  // The marks stay, named by their tooltips.
  expect(one.getByRole("img").elements().map((mark) => mark.getAttribute("aria-label"))).toEqual([
    "Fleet",
    "Lands in Armada",
  ]);
  const old = card(/waited in the dock/);
  expect(old.getByRole("heading").elements()).toHaveLength(0);
  expect(old.getByRole("button", { name: /^(Create Job|Accept|Reject change)$/ }).elements()).toHaveLength(0);
});

test("a Job label opens its retro, each item with the record rows it cites behind Evidence, and the owner's notes", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  await lessons();

  await card(STALE_MAIN).getByRole("button", { name: "Job 3" }).click();
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await entered(sheet);
  await expect.element(sheet.getByText("Job 3")).toBeVisible();
  expect(sheet.getByText(/4e1c2a9 on main/).elements()).toHaveLength(0);
  await sheet.getByRole("listitem").filter({ hasText: STALE_MAIN }).getByRole("button", { name: "Evidence" }).click();
  await expect.element(sheet.getByText(/4e1c2a9 on main/)).toBeVisible();
  await expect.element(sheet.getByRole("region", { name: "Notes" })).toBeVisible();
});

test("a Job's own retro opens from its Record, with the same items and answers", async () => {
  mount("retro/job-3", SLICES);
  await onScreen();

  await page.getByRole("tab", { name: /^Record/ }).last().click();
  await page.getByRole("button", { name: "Retro", exact: true }).click();
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await entered(sheet);
  await expect.element(sheet.getByText(/Browser tests timed out under the gate/)).toBeVisible();
  // Where each item's fix lands, beside whose way it got in.
  expect(sheet.getByRole("img", { name: "Lands in Kit" }).elements()).toHaveLength(1);
  expect(sheet.getByRole("img", { name: "Lands in Armada" }).elements()).toHaveLength(2);
  expect(sheet.getByRole("img", { name: "Lands in the Manifest" }).elements()).toHaveLength(1);
  expect(sheet.getByRole("button", { name: "Create Job", exact: true }).elements()).toHaveLength(3);
  // Job 3's grep item carries a change, so Fleet's Accept is Update Kit here.
  expect(sheet.getByRole("button", { name: "Update Kit", exact: true }).elements()).toHaveLength(1);
  expect(sheet.getByRole("button", { name: "Accept", exact: true }).elements()).toHaveLength(0);
});

test("the retro lessons tabs narrow the list to where each fix lands, with All first and no counts", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  const titles = await lessons();

  const tabs = page.getByRole("tablist").first().getByRole("tab").elements();
  expect(tabs.map((one) => one.textContent)).toEqual(["All", "Armada", "Kit", "Manifest"]);
  expect(tabs[0]?.getAttribute("aria-selected")).toBe("true");
  expect(titles()).toHaveLength(5);

  await page.getByRole("tab", { name: "Kit", exact: true }).click();
  await expect.poll(() => titles()).toEqual([expect.stringMatching(/grep to be allowed/)]);
  await page.getByRole("tab", { name: "Manifest", exact: true }).click();
  await expect.poll(() => titles()).toEqual(["A docs edit ran every Rust test"]);
  await page.getByRole("tab", { name: "Armada", exact: true }).click();
  // Job 2's item predates where a fix lands, so it is under All alone.
  await expect.poll(() => titles()).toHaveLength(2);
});

test("the retro lessons tab is remembered for the viewer", async () => {
  localStorage.setItem("armada.bridge.lessons-tab", "kit");
  mount("retro/lessons", SLICES);
  await onScreen();
  const titles = await lessons();

  await expect.element(page.getByRole("tab", { name: "Kit", exact: true })).toHaveAttribute("aria-selected", "true");
  await expect.poll(() => titles()).toHaveLength(1);
  await page.getByRole("tab", { name: "Manifest", exact: true }).click();
  expect(localStorage.getItem("armada.bridge.lessons-tab")).toBe("manifest");
});

test("Agree on an Armada item proposes a Job and keeps a link to it, Agree on a Kit item saves it under Accepted, and Disagree discards", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  const titles = await lessons();

  await card(STALE_MAIN).getByRole("button", { name: "Create Job", exact: true }).click();
  await expect.element(card(STALE_MAIN).getByText("Agreed")).toBeVisible();
  await expect.element(card(STALE_MAIN).getByRole("button", { name: "Proposed Job" })).toBeVisible();

  await card(GREP).getByRole("button", { name: "Update Kit", exact: true }).click();
  // A Kit item that updated Kit stays, reading Updated Kit, until the list is read again.
  await expect.element(card(GREP).getByText("Updated Kit", { exact: true })).toBeVisible();
  await card(DOCS).getByRole("button", { name: "Reject change", exact: true }).click();
  await expect.poll(() => titles().some((one) => /Rust test/.test(one))).toBe(false);

  await page.getByRole("tab", { name: "Accepted", exact: true }).click();
  await expect.poll(() => titles()).toEqual([expect.stringMatching(/grep to be allowed/)]);
  expect(card(GREP).getByRole("button", { name: /^(Create Job|Accept|Update Kit|Reject change)$/ }).elements()).toHaveLength(0);
  // What Fleet applied is read back with the saved item.
  await expect.element(card(GREP).getByText("Updated Kit", { exact: true })).toBeVisible();
  await expect.element(card(GREP).getByText("grep", { exact: true })).toBeVisible();
  // Read again, the agreed item is no longer open either: two items are left.
  await page.getByRole("tab", { name: "Open", exact: true }).click();
  await expect.poll(() => titles()).toHaveLength(2);
});

test("Update Kit puts the command on the Kit page's allowlist as a retro item's, and Remove takes it off", async () => {
  mount("retro/lessons", WITH_KIT);
  await onScreen();
  await lessons();

  await card(GREP).getByRole("button", { name: "Update Kit", exact: true }).click();
  await expect.element(card(GREP).getByText("Updated Kit", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "Kit", exact: true }).click();
  const allowlist = page.getByRole("region", { name: "Allowlist" });
  const row = allowlist.getByRole("listitem").filter({ hasText: "grep" });
  await expect.element(row).toBeVisible();
  await expect.element(row.getByText("Retro item", { exact: true })).toBeVisible();
  await row.getByRole("button", { name: "Remove grep" }).click();
  await expect.poll(() => allowlist.getByRole("listitem").elements().length).toBe(0);
});

test("a Job whose retro is not written says so, and nothing more", async () => {
  mount("judge/refusal-agreed", SLICES);
  await onScreen();

  await page.getByRole("tab", { name: /^Record/ }).last().click();
  await page.getByRole("button", { name: "Retro", exact: true }).click();
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await entered(sheet);
  await expect.element(sheet.getByText("Not written yet")).toBeVisible();
});
