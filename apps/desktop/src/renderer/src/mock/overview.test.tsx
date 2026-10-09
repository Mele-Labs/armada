// Overview, through `App`. Moved here from the `Screens/Overview
// lists`, `Screens/Overview summary`, `Screens/Overview surface` and
// `Screens/Settings surface` stories, which stood each in a shell of their own
// rather than the one `App` draws — #1224.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { JobSummary, RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";
import { boardJobs, boardWorkflows } from "@armada/screens/src/fixtures/build/board";

import { onBoard } from "./scenario";
import type { Scenario } from "./scenario";
import { mount, openHelm, rows, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};

/** One Job in every state the Board names, and one status this build's registry does not know. */
const JOBS = (): JobSummary[] => [
  ...boardJobs(),
  job("not_a_status_the_registry_has", { id: "01M2C1TJ8G00UNKNOWNSTATUS00", handle: "11-unknown-status" }),
];

/** Two running Jobs, one with a plan and one without — the owner's report against the Running panel. */
const RUNNING_ONE_WITH_A_PLAN = (): JobSummary[] => [
  job("running", {
    id: "01M2C1TJ8G00RUNNINGWITHTASK",
    handle: "16-fix-801-unanswered-permission-ask-holds-dr",
    title: "Fix 801: unanswered permission ask holds Drone slot, 2nd dispatch",
    workflow_id: "implement",
    owner_manifest_id: "armada",
    current_step_id: "fix",
    started_at: new Date(Date.now() - 476_000).toISOString(),
    tasks: { done: 0, working: 0, open: 6, dropped: 0 },
  }),
  job("running", {
    id: "01M2C1TJ8G00RUNNINGNOPLAN00",
    handle: "17-job-details-view-shows-queued-while-fleet-r",
    title: 'Job details view shows "queued" while Fleet reports job as running',
    workflow_id: "plan",
    owner_manifest_id: "armada",
    current_step_id: "plan",
    started_at: new Date(Date.now() - 138_000).toISOString(),
  }),
];

/** Overview, where `App` opens, on these rows: the Dashboard's one panel, on its Your move filter. */
async function overview(scenario: Scenario): Promise<void> {
  mount(scenario);
  await expect.element(page.getByRole("tab", { name: "Your move" })).toBeVisible();
}

const onOverview = (jobs: JobSummary[], options: Parameters<typeof onBoard>[1] = {}) =>
  onBoard(jobs, { workflows: boardWorkflows(), ...options });

/** Fleet was connected and has gone: what Bridge holds is what it last received. */
function unreachable(scenario: Scenario): Scenario {
  const connection = scenario.state.connection;
  if (connection.state !== "connected") throw new Error("unreachable from a connected scenario");
  return {
    ...scenario,
    state: {
      ...scenario.state,
      connection: { state: "unreachable", fleet: connection.fleet, detail: "Fleet unreachable", sinceMs: Date.now() },
    },
  };
}

const picked = () => document.querySelector<HTMLElement>('[role="option"][aria-selected="true"]');

test("every filter draws its own: what needs you comes forward, what is live and what is over are tiles", async () => {
  await overview(onOverview(JOBS()));
  // Your move: what needs the owner is a call in front of the panel, whatever the filter.
  await expect.element(page.getByRole("region", { name: /^Job:/ })).toBeVisible();
  await page.getByRole("tab", { name: "Active" }).click();
  await expect.element(page.getByRole("listbox", { name: "Tiles" })).toBeVisible();
  // No badge to draw for it, so its tile's mark names the status.
  await expect.element(page.getByRole("img", { name: "not_a_status_the_registry_has" })).toBeVisible();
  await page.getByRole("tab", { name: "Done" }).click();
  await expect.element(page.getByRole("listbox", { name: "Tiles" })).toBeVisible();
});

test("no jobs: the cursor is in the dispatch bar, and typing opens the composer", async () => {
  mount(onOverview([]));
  const bar = page.getByRole("textbox", { name: "Request" });
  await expect.element(bar).toHaveFocus();
  await userEvent.keyboard("C");
  // On All, the composer opens on its one question first.
  await expect.element(page.getByText("Pick the repository this Job is for")).toBeVisible();
});

test("the filter picked is kept, and a filter draws no count", async () => {
  await overview(onOverview(JOBS()));
  await page.getByRole("tab", { name: "Done" }).click();
  await expect.element(page.getByRole("tab", { name: "Done" })).toHaveAttribute("aria-selected", "true");
  expect(localStorage.getItem("armada.bridge.dashboard-tab")).toBe("done");
  for (const name of ["Your move", "Active", "Done"]) {
    expect(page.getByRole("tab", { name }).element().textContent).not.toMatch(/\d/);
  }
});

test("Fleet unreachable: the rows held from before stay, Needs you included", async () => {
  // Where Fleet cannot be reached the Board's own lists draw, and the panel's filters do not.
  mount(unreachable(onOverview(JOBS())));
  await expect.element(page.getByRole("heading", { name: "Needs you" })).toBeVisible();
  expect(rows().length).toBeGreaterThan(0);
});

test("Fleet unreachable with nothing held says so flatly", async () => {
  mount(unreachable(onOverview([])));
  await expect.element(page.getByText("Fleet is not connected, so there is nothing to show.")).toBeVisible();
});

test("a running Job picked on Active, under Helm's dock, keeps its steps and its act", async () => {
  await overview(onOverview(RUNNING_ONE_WITH_A_PLAN(), { repositories: [ARMADA, STOREFRONT] }));
  await openHelm();
  await page.getByRole("tab", { name: "Active" }).click();
  await expect.element(page.getByRole("option", { name: /unanswered permission ask/ })).toBeVisible();
  await expect.element(page.getByRole("option", { name: /shows "queued"/ })).toBeVisible();
  await page.getByRole("option", { name: /unanswered permission ask/ }).click();
  await expect.element(page.getByRole("button", { name: /Redirect/ }).first()).toBeVisible();
});

test("j and k move the pick, x asks to kill, Enter opens", async () => {
  await overview(onOverview(RUNNING_ONE_WITH_A_PLAN()));
  await page.getByRole("tab", { name: "Active" }).click();
  await expect.element(page.getByRole("listbox", { name: "Tiles" })).toBeVisible();
  await expect.poll(picked).not.toBeNull();
  const first = picked()!;
  await userEvent.keyboard("j");
  await expect.poll(() => picked()).not.toBe(first);
  await userEvent.keyboard("k");
  await expect.poll(() => picked()).toBe(first);

  await userEvent.keyboard("x");
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();

  await userEvent.keyboard("{Enter}");
  await expect.poll(() => page.getByRole("tab", { name: "Active" }).query()).toBeNull();
});

test("the Job picked is Overview's cursor, and Helm's footer names it", async () => {
  await overview(onOverview(JOBS()));
  await openHelm();
  await expect.element(page.getByText(/^Cockpit · cursor on Job \d+$/)).toBeVisible();
});

test("n brings the cursor to the dispatch bar from the Dashboard", async () => {
  await overview(onOverview(JOBS()));
  await userEvent.keyboard("n");
  await expect.element(page.getByRole("textbox", { name: "Request" })).toHaveFocus();
});
