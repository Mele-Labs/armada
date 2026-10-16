// A write clash on the flight deck, through `App` on the ship scenario: both parties of a clash carry the mark on their list rows,
// on their drone, and on the head of a Job's pane, where a Session's pane names the Job in its Write clash bar instead. The mark names the
// other party and opens it on the deck, and nothing is marked
// where Fleet reports no clash. The ship holds a Job and a Session in one file, and a Session against a main that moved.
// Hand-run with the rest of `mock/`.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { s252CockpitShipFlightdeck as ship } from "./scenarios/cockpit-ship";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const JOB = "Shorten the reconnect wait";
const SESSION = "Fix the reconnect wait";
const AGAINST_MAIN = "Pin the store clock";
const QUIET = "Cache the manifest read between dispatches";
const FILE = "apps/desktop/src/renderer/src/cockpit/reconnect.ts";
const MAIN_SAID = "Conflicts with main · crates/store/tests/clock.rs";

const list = () => page.getByRole("region", { name: "Jobs and Sessions" });
const controls = () => page.getByRole("region", { name: "Flight controls" });
const pane = (kind: "Job" | "Session", title: string) => page.getByRole("region", { name: `${kind}: ${title}` });

/** The marks beside a row's button, the one named by its title. */
const rowMarks = (title: string) =>
  [...document.querySelectorAll(".deck-list .deck-row")]
    .filter((row) => row.getAttribute("aria-label") === title)
    .flatMap((row) => [...(row.parentElement?.querySelectorAll<HTMLElement>(".armada-clash-mark") ?? [])]);
const headMarks = (title: string) => [...document.querySelectorAll<HTMLElement>(`.deck-pane[aria-label$=": ${title}"] :is(.deck-pane__head, .armada-session-frame__head) .armada-clash-mark`)];
const droneMarks = (title: string) => [...document.querySelectorAll<HTMLElement>(`.bf-drone[data-label="${title}"] .bf-clash`)];
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
  await expect.poll(() => document.querySelector(".bf-drone") !== null).toBe(true);
  // The camera has eased to rest and the drones have settled.
  await new Promise((done) => window.setTimeout(done, 900));
  return app;
}

test("a row of each party to a clash carries the mark, and the mark names the other party and opens it", async () => {
  await deck();
  expect(said(rowMarks(JOB))).toEqual([`Conflicts with ${SESSION} · ${FILE}`]);
  expect(said(rowMarks(SESSION))).toEqual([`Conflicts with ${JOB} · ${FILE}`]);

  // Pressed on the Job's row it opens the Session on the deck, whose Write clash bar names the Job. The bar is a line of chips while it is
  // folded, and the chip for the Job opens the Job's pane.
  rowMarks(JOB)[0]!.click();
  await expect.element(pane("Session", SESSION)).toBeVisible();
  const bar = pane("Session", SESSION).getByRole("region", { name: "Write clash" });
  await expect.element(bar).toHaveTextContent(JOB);
  // The first is the chip: the closed list of clashes under the chips names the Job again.
  await bar.getByRole("button", { name: `Open ${JOB}` }).first().click();
  await expect.element(pane("Job", JOB)).toBeVisible();
  expect(said(headMarks(JOB))).toEqual([`Conflicts with ${SESSION} · ${FILE}`]);
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
  expect(said(droneMarks(JOB))).toEqual([`Conflicts with ${SESSION} · ${FILE}`]);
  expect(said(droneMarks(SESSION))).toEqual([`Conflicts with ${JOB} · ${FILE}`]);
  expect(said(droneMarks(AGAINST_MAIN))).toEqual([MAIN_SAID]);

  droneMarks(JOB)[0]!.click();
  await expect.element(pane("Session", SESSION)).toBeVisible();
});

test("no mark where no clash is reported", async () => {
  await deck();
  // Of the list's rows, the holders of the ship's clashes are marked and no other.
  const marked = [...document.querySelectorAll(".deck-list .deck-row")]
    .filter((row) => (row.parentElement?.querySelectorAll(".armada-clash-mark").length ?? 0) > 0)
    .map((row) => row.getAttribute("aria-label"));
  expect(marked.sort()).toEqual([AGAINST_MAIN, JOB, SESSION].sort());
  expect(rowMarks(QUIET)).toEqual([]);
  await list().getByRole("button", { name: QUIET, exact: true }).click();
  await expect.element(pane("Job", QUIET)).toBeVisible();
  expect(headMarks(QUIET)).toEqual([]);
});

test("a Fleet that reports none marks no row, pane head or drone", async () => {
  await map({ ...ship, draft: { ...ship.draft, clashes: [] } });
  expect(document.querySelectorAll(".armada-clash-mark, .bf-clash")).toHaveLength(0);
});
