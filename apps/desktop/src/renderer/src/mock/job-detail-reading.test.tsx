// Job detail before this Job's own read has come back, through `App`: every
// region drawn from the read stands in with its shape, and none of them gives
// the empty answer that only a read could give. Then the read lands, and each
// draws what it holds — or, where it holds nothing, says so.

import { expect, test } from "vitest";
import { page } from "vitest/browser";
import { reading, running, unreadable, workingAPlan } from "@armada/screens/src/fixtures/build/index";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { FleetHandle } from "./scenario";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

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
  });
  await expect.element(page.getByRole("button", { name: fixture.job.handle })).toBeVisible();
  return { answer: (with_) => fleet?.publish({ watched: with_.watched }) };
}

async function opened(fixture: JobFixture): Promise<void> {
  mount(onJob(fixture));
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

test("Overview: the read lands and the Plan card draws the plan it holds", async () => {
  const fleet = await openedReading();
  fleet.answer(workingAPlan());
  await expect.element(card("Plan").getByText("Working now")).toBeVisible();
  expect(card("Plan").getByRole("status").elements()).toHaveLength(0);
  expect(card("Brief").getByRole("status").elements()).toHaveLength(0);
});

test("Overview: a read that holds no plan says so, once it has been read", async () => {
  const fleet = await openedReading();
  fleet.answer(running());
  await expect.element(card("Plan").getByRole("note")).toBeVisible();
  expect(card("Plan").getByRole("status").elements()).toHaveLength(0);
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
