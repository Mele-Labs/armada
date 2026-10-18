// A write clash on the flight deck, through `App` on the ship scenario: both parties of a clash carry the mark on their list rows,
// on their drone, and on the open Session's Bays key, which names the other. The mark names the
// other party and opens it on the deck, and nothing is marked
// where Fleet reports no clash. The ship holds two Sessions in one file, and a Session against a main that moved.
// Hand-run with the rest of `mock/`.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { s252CockpitShipFlightdeck as ship } from "./scenarios/cockpit-ship";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const ASKING = "Cache key";
const SESSION = "Fix the reconnect wait";
const AGAINST_MAIN = "Pin the store clock";
const QUIET = "Changelog script";
const FILE = "apps/desktop/src/renderer/src/cockpit/reconnect.ts";
const MAIN_SAID = "Conflicts with main · crates/store/tests/clock.rs";

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const controls = () => page.getByRole("region", { name: "Flight controls" });
const pane = (title: string) => page.getByRole("region", { name: `Job: ${title}` });

/** The marks beside a row's button, the one named by its title. */
const rowMarks = (title: string) =>
  [...document.querySelectorAll(".deck-list .deck-row")]
    .filter((row) => row.getAttribute("aria-label") === title)
    .flatMap((row) => [...(row.parentElement?.querySelectorAll<HTMLElement>(".armada-clash-mark") ?? [])]);
const headMarks = (title: string) => [...document.querySelectorAll<HTMLElement>(`.deck-pane[aria-label$=": ${title}"] :is(.deck-pane__head, .armada-session-frame__head) .armada-clash-mark`)];
const droneMarks = (title: string) => {
  const key = document.querySelector(`.bf [data-hit=drone][data-label="${title}"]`)?.getAttribute("data-key");
  return key === null || key === undefined ? [] : [...document.querySelectorAll<HTMLElement>(`.bf [data-hit=clash][data-key="${key}"]`)];
};
const said = (marks: HTMLElement[]) => marks.map((one) => one.getAttribute("aria-label"));

async function deck(scenario: Parameters<typeof mount>[0] = ship) {
  const app = mount(scenario);
  await onScreen();
  await expect.element(list()).toBeVisible();
  return app;
}

async function map(scenario: Parameters<typeof mount>[0] = ship) {
  const app = await deck(scenario);
  await controls().getByRole("button", { name: "Map" }).click();
  await expect.poll(() => document.querySelector(".bf [data-hit=drone]") !== null).toBe(true);
  // The camera has eased to rest and the drones have settled.
  await new Promise((done) => window.setTimeout(done, 900));
  return app;
}

test("a row of each party to a clash carries the mark, and the mark names the other party and opens it", async () => {
  await deck();
  expect(said(rowMarks(ASKING))).toEqual([`Conflicts with ${SESSION} · ${FILE}`]);
  expect(said(rowMarks(SESSION))).toEqual([`Conflicts with ${ASKING} · ${FILE}`]);

  // Pressed on one's row it opens the other on the deck, whose Bays key carries the glyph naming the first.
  rowMarks(ASKING)[0]!.click();
  await expect.element(pane(SESSION)).toBeVisible();
  await expect.element(controls().getByRole("img", { name: `Write clash with ${ASKING}` })).toBeVisible();
});

test("a marked row keeps the list's width, less the mark", async () => {
  await deck();
  const width = (title: string) => document.querySelector(`.deck-list .deck-row[aria-label="${title}"]`)!.getBoundingClientRect().width;
  // Only the mark and its gap come off: the mark's own wrapper does not take a share of the row.
  expect(width(QUIET) - width(SESSION)).toBeLessThan(32);
});

test("a clash against main marks the one party, as a picture with nothing to open", async () => {
  await deck();
  const [mark, ...rest] = rowMarks(AGAINST_MAIN);
  expect(rest).toEqual([]);
  expect(mark?.getAttribute("aria-label")).toBe(MAIN_SAID);
  expect(mark?.tagName).not.toBe("BUTTON");
});

test("a drone of each party carries it, and pressing it opens the other party", async () => {
  await map();
  expect(said(droneMarks(ASKING))).toEqual([`Conflicts with ${SESSION} · ${FILE}`]);
  expect(said(droneMarks(SESSION))).toEqual([`Conflicts with ${ASKING} · ${FILE}`]);
  expect(said(droneMarks(AGAINST_MAIN))).toEqual([MAIN_SAID]);

  droneMarks(ASKING)[0]!.click();
  await expect.element(pane(SESSION)).toBeVisible();
});

test("no mark where no clash is reported", async () => {
  await deck();
  // Of the list's rows, the holders of the ship's clashes are marked and no other.
  const marked = [...document.querySelectorAll(".deck-list .deck-row")]
    .filter((row) => (row.parentElement?.querySelectorAll(".armada-clash-mark").length ?? 0) > 0)
    .map((row) => row.getAttribute("aria-label"));
  expect(marked.sort()).toEqual([AGAINST_MAIN, ASKING, SESSION].sort());
  expect(rowMarks(QUIET)).toEqual([]);
  await list().getByRole("button", { name: QUIET, exact: true }).click();
  await expect.element(pane(QUIET)).toBeVisible();
  expect(headMarks(QUIET)).toEqual([]);
});

test("a Fleet that reports none marks no row, pane head or drone", async () => {
  await map({ ...ship, draft: { ...ship.draft, clashes: [] } });
  expect(document.querySelectorAll(".armada-clash-mark, .bf [data-hit=clash]")).toHaveLength(0);
});
