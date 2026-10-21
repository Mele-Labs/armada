// Session retros in Bridge (23.12, `docs/concepts/retro.md`): the Retros page lists what got in the
// way across Sessions, newest first, each item as words, a headline, what happened, a fix and its
// answers. A Session label opens its retro. Start Job agrees with an Armada item (see
// `retro-start-session.test.tsx`), Accept saves a Kit item, and Reject change discards.

import { beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import { entered, mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";

unmountAfterEach();

beforeEach(() => localStorage.removeItem("armada.bridge.lessons-tab"));

// Core and Lessons only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "lessons"] } as const;
// The Kit page is Manifest's.
const WITH_KIT = { slices: ["core", "lessons", "manifest"] } as const;

const RESET = /A reset waited 40 minutes for an answer/;
const COMMIT = /A commit message was read as a redirect/;
const WALKS = /Walks opened in your browser/;
const GREP = /grep had to be allowed on a check log/;
const SESSION = "Retire guides 8 and 20, add validation that";

const card = (title: RegExp) => page.getByRole("listitem").filter({ hasText: title });

/** The Retros surface, and the headlines its cards draw. */
async function lessons(): Promise<() => string[]> {
  await toSystemPage("Retros");
  await expect.element(page.getByRole("list", { name: "Retros" })).toBeVisible();
  return () => [...document.querySelectorAll(".armada-lesson")].map((one) => one.getAttribute("aria-label") ?? "");
}

/** The tabs that narrow the list, not the title bar's places or the System strip. */
const lessonTabs = () => page.getByRole("tablist").filter({ has: page.getByRole("tab", { name: "Manifest", exact: true }) });

test("Retros is a page under System, and lists every Session's items newest first", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();

  const titles = await lessons();
  // The places bar marks where the window is.
  await expect.element(page.getByRole("navigation", { name: "Places" }).getByRole("tab", { name: "Retros" })).toHaveAttribute("aria-selected", "true");
  const from = [...document.querySelectorAll(".armada-lesson__from")].map((one) => one.textContent?.split(" · ")[0]);
  // The newer Session's five items, then the older one's: Fleet's order, newest retro first.
  expect(from).toEqual([...Array<string>(5).fill(SESSION), "Pin the Bridge store clock"]);
  expect(titles()).toHaveLength(6);
});

test("an item says whose way and where in words, and an old item has its statement alone", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  await lessons();

  const one = card(RESET);
  const words = [...one.element().querySelectorAll(".armada-lesson__word")].map((word) => word.textContent);
  expect(words).toEqual(["Agent", "Armada"]);
  await expect.element(one.getByRole("heading")).toBeVisible();
  await expect.element(one.getByText(/Tell you when an ask has waited ten minutes/)).toBeVisible();
  // The marks stay, named by their tooltips.
  expect(one.getByRole("img").elements().map((mark) => mark.getAttribute("aria-label"))).toEqual([
    "Agent",
    "Lands in Armada",
  ]);
  const old = card(/waited in the dock/);
  expect(old.getByRole("heading").elements()).toHaveLength(0);
  expect(old.getByRole("button", { name: /^(Start Job|Accept|Reject change)$/ }).elements()).toHaveLength(0);
});

test("a Session label opens its retro, each item with the record rows it cites behind Evidence", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  await lessons();

  await card(RESET).getByRole("button", { name: new RegExp(`^${SESSION}`) }).click();
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await entered(sheet);
  await expect.element(sheet.getByText(new RegExp(`^${SESSION}`))).toBeVisible();
  expect(sheet.getByText(/Allow git reset --hard origin\/main\?/).elements()).toHaveLength(0);
  await sheet.getByRole("listitem").filter({ hasText: RESET }).getByRole("button", { name: "Evidence" }).click();
  await expect.element(sheet.getByText(/Allow git reset --hard origin\/main\?/)).toBeVisible();
});

test("the retro lessons tabs narrow the list to where each fix lands, with All first and no counts", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  const titles = await lessons();

  const tabs = lessonTabs().getByRole("tab").elements();
  expect(tabs.map((one) => one.textContent)).toEqual(["All", "Armada", "Kit", "Manifest"]);
  expect(tabs[0]?.getAttribute("aria-selected")).toBe("true");
  expect(titles()).toHaveLength(6);

  await lessonTabs().getByRole("tab", { name: "Kit", exact: true }).click();
  await expect.poll(() => titles()).toEqual(["Walks opened in your browser"]);
  await lessonTabs().getByRole("tab", { name: "Manifest", exact: true }).click();
  await expect.poll(() => titles()).toEqual([]);
  await lessonTabs().getByRole("tab", { name: "Armada", exact: true }).click();
  // The older item predates where a fix lands, so it is under All alone.
  await expect.poll(() => titles()).toHaveLength(4);
});

test("the retro lessons tab is remembered for the viewer", async () => {
  localStorage.setItem("armada.bridge.lessons-tab", "kit");
  mount("retro/lessons", SLICES);
  await onScreen();
  const titles = await lessons();

  await expect.element(lessonTabs().getByRole("tab", { name: "Kit", exact: true })).toHaveAttribute("aria-selected", "true");
  await expect.poll(() => titles()).toHaveLength(1);
  await lessonTabs().getByRole("tab", { name: "Manifest", exact: true }).click();
  expect(localStorage.getItem("armada.bridge.lessons-tab")).toBe("manifest");
});

test("Accept on a Kit item saves it under Accepted, and Reject change on an Armada item discards it", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  const titles = await lessons();

  // An Armada item is agreed by starting a Session, so it has no Accept.
  expect(card(COMMIT).getByRole("button", { name: "Accept", exact: true }).elements()).toHaveLength(0);
  await card(WALKS).getByRole("button", { name: "Accept", exact: true }).click();
  await expect.poll(() => titles().some((one) => WALKS.test(one))).toBe(false);
  await card(COMMIT).getByRole("button", { name: "Reject change", exact: true }).click();
  await expect.poll(() => titles().some((one) => COMMIT.test(one))).toBe(false);

  await page.getByRole("tab", { name: "Accepted", exact: true }).click();
  await expect.poll(() => titles()).toEqual(["Walks opened in your browser"]);
  expect(card(WALKS).getByRole("button", { name: /^(Start Job|Accept|Update Kit|Reject change)$/ }).elements()).toHaveLength(0);
  // Read again, the saved item is no longer open either: four items are left.
  await page.getByRole("tab", { name: "Open", exact: true }).click();
  await expect.poll(() => titles()).toHaveLength(4);
});

test("Update Kit puts the command on the Kit page's allowlist as a retro item's, and Remove takes it off", async () => {
  mount("retro/kit-command", WITH_KIT);
  await onScreen();
  await lessons();

  await card(GREP).getByRole("button", { name: "Update Kit", exact: true }).click();
  // A Kit item that updated Kit stays, reading Updated Kit, until the list is read again.
  await expect.element(card(GREP).getByText("Updated Kit", { exact: true })).toBeVisible();

  await toSystemPage("Allowlist");
  const allowlist = page.getByRole("region", { name: "Allowlist", exact: true });
  const row = allowlist.getByRole("button", { name: /^grep/ });
  await expect.element(row).toBeVisible();
  await expect.element(row.getByText("Retro item", { exact: true })).toBeVisible();
  await row.click();
  await page.getByRole("region", { name: "Detail", exact: true }).getByRole("button", { name: "Remove grep" }).click();
  await expect.poll(() => allowlist.getByRole("button", { name: /^grep/ }).elements().length).toBe(0);
});
