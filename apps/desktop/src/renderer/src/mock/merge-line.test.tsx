// The merge line draws wherever Fleet serves one: the owner, 2 Oct 2026. Where Fleet serves none,
// neither the palette row nor Overview's panel may draw. Where it serves one, the panel draws even
// with nobody in line, and on All each repository with a line has its own, named.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { scenarioNamed, type Scenario } from "./scenario";
import { scrollerOf } from "./scrolled";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
afterEach(async () => {
  await page.viewport(1440, 900);
});

/** The merge line's way in: its row in the palette, `⌘K`, since no tab or strip place carries the page. */
const row = () => page.getByRole("dialog", { name: "Command palette" }).getByRole("option", { name: /^Merge line/ });
/** Opens the palette, where the row is looked for. */
const palette = () => userEvent.keyboard("{Meta>}k{/Meta}");
const panel = (name = "Merge line") => page.getByRole("region", { name, exact: true });

test("with no line, neither the palette row nor the Overview panel draws", async () => {
  mount("default");
  await onScreen();

  await expect.element(page.getByRole("tab", { name: "Cockpit" })).toBeVisible();
  await palette();
  await expect.element(page.getByRole("dialog", { name: "Command palette" })).toBeVisible();
  expect(row().query()).toBeNull();
  expect(page.getByRole("region", { name: "Merge line" }).query()).toBeNull();
});

test("on All, the palette row, and on its page one named panel for each repository with a line", async () => {
  mount("merge-line");
  await onScreen();

  await palette();
  await expect.element(row()).toBeVisible();
  await row().click();
  await expect.element(panel("Merge line, armada").getByRole("list", { name: "Batch" })).toBeVisible();
  await expect.element(panel("Merge line, armada").getByRole("list", { name: "Sent back" })).toBeVisible();
  await expect.element(panel("Merge line, notes").getByRole("list", { name: "Recently landed" })).toBeVisible();
  expect(panel("Merge line, notes").getByRole("list", { name: "Sent back" }).query()).toBeNull();
  await expect.element(panel("Merge line, scratch").getByRole("img", { name: "Empty" })).toBeVisible();
  // The picture, and not one word under it.
  expect(panel("Merge line, scratch").element().querySelector(".armada-merge-line__empty")?.textContent).toBe("");
});

test("the rail surface draws the same panels", async () => {
  mount("merge-line");
  await onScreen();

  await palette();
  await row().click();
  await expect.element(panel("Merge line, notes").getByRole("list", { name: "Recently landed" })).toBeVisible();
  await expect.element(panel("Merge line, scratch").getByRole("img", { name: "Empty" })).toBeVisible();
});

test("a landed pull request wears its own badge for how it ended, and its number still opens it in the side panel", async () => {
  const opened: string[] = [];
  const watching: Scenario = {
    ...scenarioNamed("merge-line")!,
    behaves: () => ({
      openLink: async (address) => {
        opened.push(address);
        return { ok: true };
      },
    }),
  };
  mount(watching);
  await onScreen();
  await palette();
  await row().click();

  const armada = panel("Merge line, armada");
  const landed = armada.getByRole("listitem", { name: "studio/read-in-lands-in-a-zone, landed" });
  await expect.element(landed.getByText("Merged", { exact: true })).toBeVisible();
  await landed.getByRole("link", { name: "#1772" }).click();
  await expect.element(page.getByRole("dialog", { name: "Pull request #1772" })).toBeVisible();
  expect(opened).toEqual([]);

  // Nothing is known of a pull request still in line: its number, and no badge.
  const waiting = armada.getByRole("listitem", { name: /^fleet\/read-in-cluster-membership,/ });
  await expect.element(waiting.getByRole("link", { name: "#1770" })).toBeVisible();
  expect(waiting.getByText("Merged", { exact: true }).query()).toBeNull();

  const notes = panel("Merge line, notes");
  await expect.element(notes.getByText("Closed without merging", { exact: true })).toBeVisible();
});


test("each entry names who it came from: a Session by its title, nobody by nothing", async () => {
  mount("merge-line-owners");
  await onScreen();

  await palette();
  await row().click();
  const line = panel("Merge line");
  const entry = (branch: string) => line.getByRole("listitem", { name: new RegExp(`^${branch}`) });
  await expect.element(entry("fix/pin-store-clock").getByRole("button", { name: "Pull request #1861" })).toHaveTextContent("Pin the store clock");
  expect(entry("fix/61-order-store-migrations").element().querySelector(".armada-owner-chip")).toBeNull();
  const bare = entry("docs/typo-in-the-readme").element();
  expect(bare.querySelector(".armada-merge-line__owner")).toBeNull();
  expect(bare.querySelector(".armada-owner-chip")).toBeNull();

  await entry("fix/pin-store-clock").getByRole("button", { name: "Pull request #1861" }).click();
  await expect.element(page.getByRole("dialog", { name: "Owned by Pin the store clock" })).toBeVisible();
});

test("an open pull request wears its merge queue mark and place, and its owner", async () => {
  mount("merge-queue");
  await onScreen();

  await palette();
  await row().click();
  const line = panel("Merge line");
  const entry = (branch: string) => line.getByRole("listitem", { name: new RegExp(`^${branch}`) });
  await expect.element(entry("fix/61-order-store-migrations").getByRole("img", { name: "In the merge queue, running its checks" })).toBeVisible();
  await expect.element(entry("fix/61-order-store-migrations")).toHaveTextContent("#1");
  await expect.element(entry("fix/pin-store-clock").getByRole("img", { name: "In the merge queue, waiting its turn" })).toBeVisible();
  await expect.element(entry("docs/typo-in-the-readme").getByRole("img", { name: "Waiting for ci to join the merge queue" })).toBeVisible();
  await expect.element(entry("chore/bump-the-lockfile").getByRole("img", { name: "In the merge queue, cannot merge" })).toBeVisible();
  await expect.element(entry("fix/pin-store-clock").getByRole("button", { name: "Pull request #1861" })).toBeVisible();
  const order = [...line.element().querySelectorAll('ul[aria-label="Open pull requests"] > li')].map((li) => li.getAttribute("aria-label")?.split(",")[0]);
  expect(order).toEqual(["fix/61-order-store-migrations", "fix/pin-store-clock", "chore/bump-the-lockfile", "docs/typo-in-the-readme", "wip/not-in-the-queue"]);
  expect(entry("wip/not-in-the-queue").element().querySelector(".armada-merge-line__place")?.textContent).toBe("");
});

// The surface reads keys as the cockpit's glass does, the cursor being DOM focus: one Tab stop for
// every repository's tiles, `j` and `k` through them in reading order, the bare arrows across the
// board.
const nowhere = () => (document.activeElement as HTMLElement | null)?.blur();
const tileFocused = () => (document.activeElement as HTMLElement | null)?.closest("[data-merge-item]")?.getAttribute("aria-label") ?? null;

test("j and k walk the tiles in reading order, and Enter on a tile leaves the press alone", async () => {
  mount("merge-line-owners");
  await onScreen();

  await palette();
  await row().click();
  nowhere();
  const entry = (branch: string) => panel("Merge line").getByRole("listitem", { name: new RegExp(`^${branch},`) });
  await userEvent.keyboard("j");
  await expect.element(entry("fix/pin-store-clock")).toHaveFocus();
  await userEvent.keyboard("{Enter}");
  await expect.element(entry("fix/pin-store-clock")).toHaveFocus();
  await userEvent.keyboard("j");
  await expect.element(entry("fix/61-order-store-migrations")).toHaveFocus();
  await userEvent.keyboard("j");
  await userEvent.keyboard("k");
  await expect.element(entry("fix/61-order-store-migrations")).toHaveFocus();
});

test("Tab enters the tiles once and leaves without visiting another", async () => {
  mount("merge-line");
  await onScreen();

  await palette();
  await row().click();
  const armada = panel("Merge line, armada");
  armada.getByRole("button", { name: "Collapse Merge line, armada" }).element().focus();
  // Main's light at the horizon's end is the panel's one stop before its tiles.
  await userEvent.keyboard("{Tab}");
  await expect.element(armada.getByRole("img", { name: "main: not read yet" })).toHaveFocus();
  await userEvent.keyboard("{Tab}");
  await expect.element(armada.getByRole("listitem", { name: /^docs\/wire-lock-signed,/ })).toHaveFocus();
  const visited = new Set<string>([tileFocused()!]);
  for (let n = 0; n < 30; n++) {
    await userEvent.keyboard("{Tab}");
    const one = tileFocused();
    if (one !== null) visited.add(one);
  }
  expect([...visited]).toEqual([expect.stringMatching(/^docs\/wire-lock-signed,/)]);
});

test("bare arrows move across the board: right to what left the line, left and down along the line", async () => {
  mount("merge-line");
  await onScreen();

  await palette();
  await row().click();
  nowhere();
  const armada = panel("Merge line, armada");
  await userEvent.keyboard("j");
  await expect.element(armada.getByRole("listitem", { name: /^docs\/wire-lock-signed,/ })).toHaveFocus();
  await userEvent.keyboard("{ArrowRight}");
  await expect.element(armada.getByRole("list", { name: "Recently landed" }).getByRole("listitem", { name: /^studio\/read-in-lands-in-a-zone,/ })).toHaveFocus();
  await userEvent.keyboard("{ArrowLeft}");
  await expect.element(armada.getByRole("listitem", { name: /^docs\/wire-lock-signed,/ })).toHaveFocus();
  await userEvent.keyboard("{ArrowDown}");
  await expect.element(armada.getByRole("listitem", { name: /^worktree-agent-aef3c24792026e2c3,/ })).toHaveFocus();
});

// On its own page the band and the horizon stay put and only the tiles scroll: the one panel of a
// picked repository scrolls its glass, and on All the page scrolls, each band holding at its top.
const topOf = (element: Element) => Math.round(element.getBoundingClientRect().top);

test("on a picked repository, the band and the horizon stay in view while the glass scrolls", async () => {
  await page.viewport(1200, 600);
  mount("merge-line");
  await onScreen();
  await page.getByRole("combobox", { name: "Repository" }).click();
  await page.getByRole("option", { name: "armada", exact: true }).click();
  await palette();
  await row().click();

  const line = panel("Merge line");
  const heading = line.getByRole("heading", { name: "Merge line" }).element();
  const light = line.getByRole("img", { name: "main: not read yet" }).element();
  const tile = line.getByRole("listitem", { name: /^docs\/wire-lock-signed,/ }).element();
  const before = { heading: topOf(heading), light: topOf(light), tile: topOf(tile) };
  scrollerOf(tile)!.scrollTop = 400;
  await expect.poll(() => topOf(tile)).toBeLessThan(before.tile - 200);
  expect(topOf(heading)).toBe(before.heading);
  expect(topOf(light)).toBe(before.light);
  expect(before.heading).toBeGreaterThanOrEqual(0);
});

test("on All, a panel's band and horizon hold at the top of the page while its tiles pass under", async () => {
  await page.viewport(1200, 600);
  mount("merge-line");
  await onScreen();
  await palette();
  await row().click();

  const armada = panel("Merge line, armada");
  const heading = armada.getByRole("heading", { name: /^Merge line/ }).element();
  const light = armada.getByRole("img", { name: "main: not read yet" }).element();
  const tile = armada.getByRole("listitem", { name: /^docs\/wire-lock-signed,/ }).element();
  const before = { heading: topOf(heading), light: topOf(light), tile: topOf(tile) };
  scrollerOf(heading)!.scrollTop = 300;
  await expect.poll(() => topOf(tile)).toBeLessThan(before.tile - 200);
  // Within the panel's own top border, which scrolls away once the band holds.
  expect(Math.abs(topOf(heading) - before.heading)).toBeLessThanOrEqual(1);
  expect(Math.abs(topOf(light) - before.light)).toBeLessThanOrEqual(1);
});
