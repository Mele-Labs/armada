// The Jobs page, a row of the rail's Work panel: every Job in the Sessions page's own layout, through `App`
// on the `fleet-timeline` mock Fleet. Its tabs, the keys that walk its tiles, the acts behind the ellipsis,
// the one-line Now on a running Job only, and the plan's bar only on a Job that has a plan.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { s207FleetTimeline } from "./scenarios/fleet-timeline";
import type { Scenario } from "./moment";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const tile = (title: string) => page.getByRole("button", { name: title, exact: true });
const RUNNING = "Debounce the Job Board's resize handler";
const QUEUED = "Store a pause marker on the Job";
const FAILED = "Move the store reads behind one trait";
const LANDED = "Fold the two notification routes into one";

/** The Jobs page, opened from the rail as a viewer who remembers nothing. */
async function jobs(scenario: Scenario = s207FleetTimeline): Promise<void> {
  mount(scenario);
  await onScreen();
  await page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Jobs", exact: true }).click();
  await expect.element(page.getByRole("region", { name: "Jobs" })).toBeVisible();
}

test("Jobs opens from the rail, marked while it is open, on Active", async () => {
  await jobs();

  await expect.element(page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Jobs", exact: true })).toHaveAttribute("aria-current", "page");
  await expect.element(page.getByRole("tab", { name: "Active" })).toHaveAttribute("aria-selected", "true");
  await expect.element(tile(RUNNING)).toBeVisible();
  await expect.element(tile(QUEUED)).toBeVisible();
  // What is over is not Active.
  expect(tile(FAILED).query()).toBeNull();
  expect(tile(LANDED).query()).toBeNull();
});

test("Done holds what is over, and All holds both", async () => {
  await jobs();

  await page.getByRole("tab", { name: "Done" }).click();
  await expect.element(tile(FAILED)).toBeVisible();
  await expect.element(tile(LANDED)).toBeVisible();
  expect(tile(RUNNING).query()).toBeNull();

  await page.getByRole("tab", { name: "All" }).click();
  await expect.element(tile(RUNNING)).toBeVisible();
  await expect.element(tile(LANDED)).toBeVisible();
});

test("a press anywhere on a tile opens the Job", async () => {
  await jobs();

  await tile(RUNNING).click();
  await expect.element(page.getByRole("tab", { name: "Overview" })).toBeVisible();
  // The rail still marks Jobs while its Job is open.
  await expect.element(page.getByRole("navigation", { name: "Work" }).getByRole("button", { name: "Jobs", exact: true })).toHaveAttribute("aria-current", "page");
});

test("j and k walk the tiles down and up, and Enter opens the one the cursor is on", async () => {
  await jobs();
  const opens = () => [...document.querySelectorAll<HTMLElement>("[data-job-id]")];
  const on = () => opens().indexOf(document.activeElement as HTMLElement);

  await userEvent.keyboard("j");
  await expect.poll(on).toBe(0);
  await userEvent.keyboard("j");
  await expect.poll(on).toBe(1);
  await userEvent.keyboard("k");
  await expect.poll(on).toBe(0);

  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("tab", { name: "Overview" })).toBeVisible();
});

test("the less used acts are behind the ellipsis, and Kill and Pause ask before they act", async () => {
  await jobs();
  const more = () => page.getByRole("button", { name: `More for ${RUNNING}`, exact: true });

  await more().click();
  await expect.element(page.getByRole("menuitem", { name: "Pause" })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "Kill" })).toBeVisible();
  await page.getByRole("menuitem", { name: "Kill" }).click();
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();

  await more().click();
  await page.getByRole("menuitem", { name: "Pause" }).click();
  await expect.element(page.getByRole("dialog")).toBeVisible();
});

test("x on the tile under the cursor asks to kill, as it does on the Board", async () => {
  await jobs();
  await userEvent.keyboard("j");
  await userEvent.keyboard("x");
  await expect.element(page.getByRole("dialog")).toBeVisible();
});

test("a finished Job offers Redispatch and Clear and no Kill, a running one Kill and no Redispatch", async () => {
  await jobs();

  await page.getByRole("tab", { name: "Done" }).click();
  await page.getByRole("button", { name: `More for ${FAILED}`, exact: true }).click();
  await expect.element(page.getByRole("menuitem", { name: "Redispatch as a new job" })).toBeVisible();
  await expect.element(page.getByRole("menuitem", { name: "Clear" })).toBeVisible();
  expect(page.getByRole("menuitem", { name: "Kill" }).query()).toBeNull();
});

test("the Now line is on a running Job and on no other", async () => {
  await jobs();

  const nows = () => [...document.querySelectorAll<HTMLElement>(".armada-jobs-page__now")];
  await expect.poll(() => nows().length).toBeGreaterThan(0);
  // Every Now line sits on a tile whose Job is running; the queued Job's tile has none.
  expect(nows().every((one) => one.closest<HTMLElement>(".armada-jobs-page__row")?.dataset.status === "running")).toBe(true);
  const queued = tile(QUEUED).element().closest(".armada-jobs-page__row")!;
  expect(queued.querySelector(".armada-jobs-page__now")).toBeNull();
  expect(nows().some((one) => one.textContent?.includes("Edit apps/desktop/src/renderer/src/resize-handler.ts"))).toBe(true);

  await page.getByRole("tab", { name: "Done" }).click();
  expect(document.querySelector(".armada-jobs-page__now")).toBeNull();
});

test("a Job that is not running draws no Now line even where a Drone's line is held for it", async () => {
  const queued = s207FleetTimeline.state.jobs.find((one) => one.title === QUEUED)!;
  const drone = { key: "d", of: "drone" as const, name: "Drone", tail: ["Edit crates/fleet/src/session.rs"], since: new Date().toISOString(), state: "running" as const };
  await jobs({ ...s207FleetTimeline, draft: { ...s207FleetTimeline.draft, now: { ...s207FleetTimeline.draft?.now, [queued.id]: { running: [drone] } } } });

  await expect.element(tile(QUEUED)).toBeVisible();
  expect(tile(QUEUED).element().closest(".armada-jobs-page__row")!.querySelector(".armada-jobs-page__now")).toBeNull();
});

test("a Fleet that serves no Now draws no Now line", async () => {
  await jobs({ ...s207FleetTimeline, draft: { ...s207FleetTimeline.draft, now: undefined } });

  await expect.element(tile(RUNNING)).toBeVisible();
  expect(document.querySelector(".armada-jobs-page__now")).toBeNull();
});

test("the plan's bar is on a live Job that has a plan and on none without, its figure only in the tooltip", async () => {
  const bare: Scenario = {
    ...s207FleetTimeline,
    state: {
      ...s207FleetTimeline.state,
      jobs: s207FleetTimeline.state.jobs.map((one) => {
        const { tasks: _gone, ...rest } = one;
        return one.title === "Cache the manifest read between dispatches" ? rest : one;
      }),
    },
  };
  await jobs(bare);

  const row = (title: string) => tile(title).element().closest<HTMLElement>(".armada-jobs-page__row")!;
  expect(row(RUNNING).querySelector('[role="img"][aria-label$="tasks done"]')).not.toBeNull();
  expect(row("Cache the manifest read between dispatches").querySelector('[role="img"][aria-label$="tasks done"]')).toBeNull();
  // The bar draws segments and no figure; the figure is its tooltip's.
  expect(row(RUNNING).querySelector('[role="img"][aria-label$="tasks done"]')!.textContent).toBe("");
});
