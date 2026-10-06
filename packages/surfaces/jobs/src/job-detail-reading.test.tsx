// Job detail before this Job's own read has come back, through `App`: every
// region drawn from the read stands in with its shape, and none of them gives
// the empty answer that only a read could give. Then the read lands, and each
// draws what it holds — or, where it holds nothing, says so.

import { expect, test } from "vitest";
import { page } from "vitest/browser";
import { reading, running, unreadable, workingAPlan } from "./fixtures/build/index";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import { mount, onJob, unmountAfterEach } from "@armada/desktop/mock";
import type { FleetHandle } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Jobs only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "jobs"] } as const;

/**
 * App with the `reading` fixture open, and a way to land the read later — as
 * Fleet answering `GET /jobs/:job_id` does. Every `build/` fixture is the same
 * Job, so another fixture's read is this Job's.
 */
async function openedReading(): Promise<{ answer: (with_: JobFixture) => void }> {
  let fleet: FleetHandle | undefined;
  const fixture = reading();
  mount({
    ...onJob(fixture),
    behaves: (handle) => {
      fleet = handle;
      return {};
    },
  }, SLICES);
  // **Fleet's answer waits for the app's ask.** The Job being on screen does not mean the effect that
  // sends `watchJob` has run, and its reply is this fixture's unread read: an answer published first
  // is replaced by it and never comes back. The effect has not run yet when `mount` returns, so
  // wrapping here sees the call.
  const asked = new Promise<void>((resolve) => {
    const watchJob = window.armada.watchJob;
    window.armada.watchJob = async (jobId) => {
      await watchJob(jobId);
      if (jobId !== null) resolve();
    };
  });
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  await asked;
  return { answer: (with_) => fleet?.publish({ watched: with_.watched }) };
}

async function opened(fixture: JobFixture): Promise<void> {
  mount(onJob(fixture), SLICES);
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
}

const card = (name: string) => page.getByRole("region", { name, exact: true });
/**
 * Open a destination. **Found before it is pressed**: a press waits 3s for its
 * target, and on a loaded machine the first query after the Job opens took
 * longer than that, where `expect.element` waits as long as the run allows.
 */
async function toTab(name: string): Promise<void> {
  const one = page.getByRole("tab", { name, exact: true });
  await expect.element(one).toBeVisible();
  await one.click();
}
const panel = (name: string) => page.getByRole("tabpanel", { name });
const text = () => document.body.textContent ?? "";

test("Overview, unread: every card drawn from the read stands in, and none answers", async () => {
  await openedReading();
  for (const name of ["Brief", "Workflow", "Plan", "Pulse"]) {
    await expect.element(card(name).getByRole("status"), { message: name }).toBeVisible();
  }
  // The wrong answers each card gave before.
  expect(text()).not.toContain("No plan has been recorded.");
  expect(text()).not.toContain("Reading this job.");
  expect(text()).not.toContain("Reading this Job.");
  expect(text()).not.toContain("Checks running");
});

// The Plan and Brief cards folded into the canvas (the owner, 4 Oct 2026):
// the plan the read holds is its groups, drawn on the run.
test("Overview: the read lands and the canvas draws the plan it holds", async () => {
  const fleet = await openedReading();
  fleet.answer(workingAPlan());
  const run = page.getByRole("region", { name: "This Job's run" });
  await expect.element(run.getByRole("button", { name: /^Group 1, / })).toBeInTheDocument();
  expect(run.getByRole("status").elements()).toHaveLength(0);
});

test("Overview: a read that holds no plan draws nothing in the plan's place, once it has been read", async () => {
  const fleet = await openedReading();
  fleet.answer(running());
  const run = page.getByRole("region", { name: "This Job's run" });
  await expect.element(run).toBeVisible();
  expect(run.getByRole("status").elements()).toHaveLength(0);
  expect(run.getByRole("note").elements()).toHaveLength(0);
  expect(run.getByText("No plan has been recorded.").elements()).toHaveLength(0);
});

test("Overview: a refused read does not say there is no plan", async () => {
  await opened(unreadable());
  await expect.element(card("Plan").getByText("Fleet did not answer")).toBeVisible();
  expect(card("Plan").getByText("No plan has been recorded.").elements()).toHaveLength(0);
});

test("Workflow, unread: the run's frame stands in rather than a sentence", async () => {
  const fleet = await openedReading();
  await toTab("Workflow");
  await expect.element(panel("Workflow").getByRole("status", { name: "Reading the run" })).toBeVisible();
  expect(panel("Workflow").getByRole("note").elements()).toHaveLength(0);
  fleet.answer(running());
  await expect.element(panel("Workflow").getByRole("status")).not.toBeInTheDocument();
});

test("Plan, unread: neither the lead nor the plan says nothing was recorded", async () => {
  await openedReading();
  await toTab("Plan");
  await expect.element(panel("Plan").getByRole("status").first()).toBeVisible();
  expect(text()).not.toContain("Nothing was written down for this Job to be held to.");
  expect(text()).not.toContain("records no plan");
});

/**
 * `running`'s read with no process on the Job. `running` itself has one, and
 * the Drones tab lists a Job's own Drone since Job 2 drew it empty (1 Oct 2026).
 */
function runningWithNoDrone(): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  const { assigned_drone: _, ...job } = fixture.watched.detail.job;
  return { ...fixture, watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job } } };
}

test("Drones, unread: rows stand in, and none-ran is said only once it is read", async () => {
  const fleet = await openedReading();
  await toTab("Drones");
  await expect.element(panel("Drones").getByRole("status")).toBeVisible();
  expect(text()).not.toContain("No Drone has run on this Job yet.");
  fleet.answer(runningWithNoDrone());
  await expect.element(page.getByText("No Drone has run on this Job yet.")).toBeVisible();
});

test("Record, unread: rows stand in; a read Fleet refused is still told in words", async () => {
  await openedReading();
  await toTab("Record");
  await expect.element(panel("Record").getByRole("status")).toBeVisible();
  expect(text()).not.toContain("Fleet has not answered for this job");
});

/** Every stand-in on Overview that is not inside one of its cards: the lead's. */
const leadStandIns = () =>
  panel("Overview")
    .getByRole("status")
    .elements()
    .filter((one) => one.closest("section") === null);

test("Overview, unread: the lead stands in rather than saying nothing needs you", async () => {
  const fleet = await openedReading();
  await expect.poll(() => leadStandIns().length).toBe(1);
  expect(text()).not.toContain("Nothing needs you");
  // The read lands, and the lead names the step the Job is on.
  fleet.answer(running());
  await expect.element(panel("Overview").getByRole("heading", { level: 2, name: /^Fix/ })).toBeVisible();
  expect(leadStandIns()).toHaveLength(0);
});

test("Overview, unread: a lead the Board row already proves is said at once", async () => {
  // The same unread Job, which the row says was killed.
  const fixture = reading();
  await opened({ ...fixture, job: { ...fixture.job, status: "killed" } });
  await expect.element(panel("Overview").getByRole("heading", { level: 2, name: "This Job stopped" })).toBeVisible();
  expect(leadStandIns()).toHaveLength(0);
});

test("Pulse, unread: the Job's own figures stand in, and count once read", async () => {
  const fleet = await openedReading();
  await toTab("Pulse");
  await expect.element(panel("Pulse").getByRole("status")).toBeVisible();
  expect(text()).not.toContain("Checks running");
  expect(text()).not.toContain("Judges running");
  fleet.answer(running());
  await expect.element(panel("Pulse").getByText("Checks running")).toBeVisible();
  expect(panel("Pulse").getByRole("status").elements()).toHaveLength(0);
});

test("a refused read keeps its sentences and draws no skeleton", async () => {
  await opened(unreadable());
  await expect.element(card("Brief").getByText("Fleet did not answer")).toBeVisible();
  expect(page.getByRole("status").elements().filter((one) => one.getAttribute("aria-busy") !== null)).toHaveLength(0);
  await toTab("Record");
  await expect.element(page.getByText(/Fleet has not answered for this job/)).toBeVisible();
});
