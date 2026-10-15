// The mock's own claim, through `App` and nothing else: on `every-state` the
// Jobs page lists every Job the scenario holds, and each row opens that Job.
//
// **A Job is a test, not a turn of a loop.** Opening every Job inside one test
// spent one 5000ms budget on the whole page — 4628ms of it on an idle machine
// — so on a busy one the budget ran out mid-press and Playwright reported a
// click with 36ms, then one with `-8ms`, left to it. That reads as the app
// ignoring a press rather than as a test out of time, which is what it was
// (#1339). `test.for` gives each Job its own budget, and the roster the loop
// opened with is a test of its own. The Jobs page is where every Job is listed since
// the Cockpit's list keeps to what is not over (#2301).

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { titleOf } from "@armada/screens";

import { scenarioNamed } from "./scenario";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * Every Job `every-state` holds, read while the file is collected so each one
 * is its own test. The same scenario `mount` will hand each test back.
 */
const JOBS = scenarioNamed("every-state")!.state.jobs;

/** The Jobs page on All, where nothing is filtered out. */
async function jobsPage(): Promise<void> {
  await onScreen();
  await userEvent.click(page.getByRole("navigation", { name: "Places" }).getByRole("tab", { name: "Jobs" }));
  await expect.element(page.getByRole("region", { name: "Jobs" })).toBeVisible();
  await userEvent.click(page.getByRole("tab", { name: "All", exact: true }));
}

const tile = (title: string) => page.getByRole("region", { name: "Jobs" }).getByRole("button", { name: title, exact: true });

test("every-state lists a row per Job", async () => {
  mount("every-state");
  await jobsPage();
  for (const job of JOBS) await expect.element(tile(titleOf(job)).first()).toBeInTheDocument();
});

test.for(JOBS)("$handle's row opens its own detail", async (job) => {
  const { scenario } = mount("every-state");
  await jobsPage();
  await userEvent.click(tile(titleOf(job)).first());
  // The list is gone and the header names this Job by its handle.
  await expect.element(page.getByRole("button", { name: job.handle })).toBeVisible();
  // Where the scenario holds the Job's detail, it is that detail drawn: its first step, by label.
  const watched = scenario.reads[job.id]?.watched;
  const step = watched?.state === "read" ? watched.detail.steps[0]?.label : undefined;
  if (step !== undefined) {
    await expect.element(page.getByText(step, { exact: true }).first()).toBeVisible();
  }
  // Escape puts the list back, which the loop proved on every Job it went on from.
  await userEvent.keyboard("{Escape}");
  await expect.element(page.getByRole("region", { name: "Jobs" })).toBeVisible();
});

test("fleet-not-running draws what Bridge draws with no Fleet", async () => {
  mount("fleet-not-running");
  await expect.element(page.getByText("Fleet is not running").first()).toBeVisible();
});

test("a job scenario opens its Job on start", async () => {
  const { scenario } = mount("job/review");
  const job = scenario.state.jobs[0]!;
  await expect.element(page.getByRole("button", { name: job.handle })).toBeVisible();
});
