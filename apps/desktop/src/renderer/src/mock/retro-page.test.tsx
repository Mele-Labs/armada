// Session retros in Bridge (23.12, `docs/concepts/retro.md`): the Logbook's Retros tab lists what got in
// the way across Sessions, newest first, a line each, and the open item's card shows words, a headline, what happened, a fix
// and its answers. A Session label opens its retro. Start Job agrees with an Armada item (see
// `retro-start-session.test.tsx`), Accept saves a Kit item, and Reject change discards.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { entered, mount, onScreen, toSystemPage, unmountAfterEach } from "./testing";

unmountAfterEach();

// Core and Lessons only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "lessons"] } as const;
// The Kit page is Manifest's.
const WITH_KIT = { slices: ["core", "lessons", "manifest"] } as const;

const RESET = /A reset waited 40 minutes for an answer/;
const COMMIT = /A commit message was read as a redirect/;
const WALKS = /Walks opened in your browser/;
const GREP = /grep had to be allowed on a check log/;
const SESSION = "Retire guides 8 and 20, add validation that";

const retros = () => page.getByRole("grid", { name: "Retros" });
const ROWS = 'table[aria-label="Retros"] tbody tr';
/** The open item's card, drawn under its own head beside the log of items. */
const open = () => page.getByRole("region", { name: "Retro item" }).getByRole("listitem");
const row = (title: RegExp) => retros().getByRole("row").filter({ hasText: title });
/** The item a row names, opened: a press on its line in the log, and its card is `open()`. */
async function item(title: RegExp) {
  await row(title).click();
  await expect.element(open().filter({ hasText: title })).toBeVisible();
  return open();
}

/** The Logbook's Retros tab, and the titles of the items its log lists, in order. */
async function lessons(): Promise<() => string[]> {
  await toSystemPage("Retros");
  await expect.element(retros()).toBeVisible();
  return () => [...document.querySelectorAll(ROWS)].map((one) => one.querySelectorAll("td")[1]?.textContent ?? "");
}

test("the Retros tab is on the Logbook, and lists every Session's items newest first", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();

  const titles = await lessons();
  // The places bar marks where the window is.
  await expect.element(page.getByRole("navigation", { name: "Places" }).getByRole("tab", { name: "Logbook" })).toHaveAttribute("aria-selected", "true");
  const from = [...document.querySelectorAll(ROWS)].map((one) => one.querySelectorAll("td")[2]?.textContent?.split(" · ")[0]);
  // The newer Session's five items, then the older one's: Fleet's order, newest retro first.
  expect(from).toEqual([...Array<string>(5).fill(SESSION), "Pin the Bridge store clock"]);
  expect(titles()).toHaveLength(6);
});

test("an item says whose way and where in words, and an old item has its statement alone", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  await lessons();

  const one = await item(RESET);
  const words = [...one.element().querySelectorAll(".armada-lesson__word")].map((word) => word.textContent);
  expect(words).toEqual(["Agent", "Armada"]);
  await expect.element(one.getByRole("heading")).toBeVisible();
  await expect.element(one.getByText(/Tell you when an ask has waited ten minutes/)).toBeVisible();
  // The marks stay, named by their tooltips.
  expect(page.getByRole("region", { name: "Retro item" }).getByRole("img").elements().map((mark) => mark.getAttribute("aria-label"))).toEqual([
    "Agent",
    "Lands in Armada",
  ]);
  const old = await item(/waited in the dock/);
  expect(old.getByRole("heading").elements()).toHaveLength(0);
  expect(old.getByRole("button", { name: /^(Start Job|Accept|Reject change)$/ }).elements()).toHaveLength(0);
});

test("the Job button opens its retro, each item with the record rows it cites behind Evidence", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  await lessons();

  // The Job's button is in the open item's Record.
  await item(RESET);
  await page.getByRole("region", { name: "Retro item" }).getByRole("table", { name: "Record" }).getByRole("button", { name: new RegExp(`^${SESSION}`) }).click();
  const sheet = page.getByRole("dialog", { name: "Retro" });
  await entered(sheet);
  await expect.element(sheet.getByText(new RegExp(`^${SESSION}`))).toBeVisible();
  expect(sheet.getByText(/Allow git reset --hard origin\/main\?/).elements()).toHaveLength(0);
  await sheet.getByRole("listitem").filter({ hasText: RESET }).getByRole("button", { name: "Evidence" }).click();
  await expect.element(sheet.getByText(/Allow git reset --hard origin\/main\?/)).toBeVisible();
});

test("Accept on a Kit item saves it under Accepted, and Reject change on an Armada item discards it", async () => {
  mount("retro/lessons", SLICES);
  await onScreen();
  const titles = await lessons();

  // An Armada item is agreed by starting a Session, so it has no Accept.
  expect((await item(COMMIT)).getByRole("button", { name: "Accept", exact: true }).elements()).toHaveLength(0);
  await (await item(WALKS)).getByRole("button", { name: "Accept", exact: true }).click();
  await expect.poll(() => titles().some((one) => WALKS.test(one))).toBe(false);
  await (await item(COMMIT)).getByRole("button", { name: "Reject change", exact: true }).click();
  await expect.poll(() => titles().some((one) => COMMIT.test(one))).toBe(false);

  await page.getByRole("tab", { name: "Accepted", exact: true }).click();
  await expect.poll(() => titles()).toEqual(["Walks opened in your browser"]);
  expect((await item(WALKS)).getByRole("button", { name: /^(Start Job|Accept|Update Kit|Reject change)$/ }).elements()).toHaveLength(0);
  // Read again, the saved item is no longer open either: four items are left.
  await page.getByRole("tab", { name: "Open", exact: true }).click();
  await expect.poll(() => titles()).toHaveLength(4);
});

test("Update Kit puts the command on the Kit page's allowlist as a retro item's, and Remove takes it off", async () => {
  mount("retro/kit-command", WITH_KIT);
  await onScreen();
  await lessons();

  await (await item(GREP)).getByRole("button", { name: "Update Kit", exact: true }).click();
  // A Kit item that updated Kit stays, reading Updated Kit, until the list is read again.
  await expect.element(open().filter({ hasText: GREP }).getByText("Updated Kit", { exact: true })).toBeVisible();

  await toSystemPage("Allowlist");
  const allowlist = page.getByRole("region", { name: "Allowlist", exact: true });
  const row = allowlist.getByRole("button", { name: /^grep/ });
  await expect.element(row).toBeVisible();
  await expect.element(row.getByText("Retro item", { exact: true })).toBeVisible();
  await row.click();
  await page.getByRole("region", { name: "Detail", exact: true }).getByRole("button", { name: "Remove grep" }).click();
  await expect.poll(() => allowlist.getByRole("button", { name: /^grep/ }).elements().length).toBe(0);
});
