// Pilot from a Session on a real Fleet's path, through `App`: a person takes a stopped Job over from
// its row, the Session starts on its worktree, and one of three exits ends the pilot. What stands where
// main stands is `sessions-fleet.ts`, which answers the take over and the exits as Fleet does and
// publishes the Job and the Session changing. This proves the screens draw it from, and act on, what Fleet serves.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { completedSuccess } from "@armada/jobs/fixtures/build/index";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard, onJob } from "./scenario";
import type { Scenario } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, toJobPage, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();
// The Cockpit opens on its timeline; these pick a Job's tile, so they start from the grid.

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const STOPPED = "01JOBSTOPPED";

/** A Board with one escalated Job on it, over a Fleet that serves Sessions. */
function served(fleet: FakeSessionsFleet): Scenario {
  const stopped = job("escalated", { id: STOPPED, handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" });
  return fleet.scenario(onBoard([stopped], { repositories: [ARMADA], picked: ARMADA.root }));
}

/** The stopped Job's page, from its row in the Cockpit's list. */
async function toTheJob(): Promise<void> {
  await onScreen();
  await toJobPage(/^The retry loop/);
}

/** Pilot, confirmed with its default outcome, from the Job's act in its pane. */
async function pilotIt(): Promise<void> {
  await toTheJob();
  await userEvent.click(page.getByRole("button", { name: "Pilot", exact: true }));
  await userEvent.click(page.getByRole("dialog", { name: "Pilot this Job?" }).getByRole("button", { name: "Pilot", exact: true }));
}

const handoff = () => page.getByRole("region", { name: "Handed over: Job 52" });
const ledger = () => page.getByRole("region", { name: "Attachments" });

test("Pilot wired: a stopped Job is taken over from its pane, and the Session opens on a handoff of what Fleet knew", async () => {
  const fleet = new FakeSessionsFleet();
  mount(served(fleet));
  await pilotIt();

  await expect.poll(() => fleet.calls.piloted).toEqual([{ jobId: STOPPED, outcome: "take_over" }]);
  await expect.element(handoff()).toBeVisible();
  await expect.element(handoff().getByText("Verify the fix")).toBeVisible();
  await expect.element(handoff().getByText("No test covers the retry cap, retry.rs:41")).toBeVisible();
  await expect.element(handoff().getByText("written, not declared")).toBeVisible();
  // This Job's Drone said nothing before it stopped, so there is no account to draw, and no blank group for it.
  await expect.element(handoff().getByText("Trying to")).not.toBeInTheDocument();
  // The Cockpit's Ledger carries the branch and the bay as chips in its band, with no row of their own to say they were handed over.
  await expect.element(ledger().getByRole("img", { name: "Handed over with Job 52, not leased" })).not.toBeInTheDocument();
});

test("Pilot wired: Restart Step goes to Fleet as the outcome chosen", async () => {
  const fleet = new FakeSessionsFleet();
  mount(served(fleet));
  await toTheJob();
  await userEvent.click(page.getByRole("button", { name: "Pilot", exact: true }));
  const dialog = page.getByRole("dialog", { name: "Pilot this Job?" });
  await userEvent.click(dialog.getByText("Restart Step", { exact: true }));
  await userEvent.click(dialog.getByRole("button", { name: "Pilot", exact: true }));
  await expect.poll(() => fleet.calls.piloted).toEqual([{ jobId: STOPPED, outcome: "restart_step" }]);
});

test("Pilot wired: a take over Fleet refuses is said in the confirmation, which stays, and no Session opens", async () => {
  const fleet = new FakeSessionsFleet();
  fleet.refuses = { code: "fleet.already_piloted", message: "A person has already taken this Job over." };
  mount(served(fleet));
  await pilotIt();

  const dialog = page.getByRole("dialog", { name: "Pilot this Job?" });
  await expect.element(dialog.getByText("A person has already taken this Job over.")).toBeVisible();
  await expect.element(handoff()).not.toBeInTheDocument();
});

/** `completedSuccess`, with the pilot a person ended it by. */
function closedBy(exit: string | undefined): Scenario {
  const fixture = completedSuccess();
  const piloted = exit === undefined ? undefined : { reason: "take_over", session_id: "01SESSIONAAAAAAAAAAAAAAAAA", since: "2026-10-07T13:00:00.000Z", exit, ended_at: "2026-10-07T13:30:00.000Z" };
  const row = piloted === undefined ? fixture.job : { ...fixture.job, piloted };
  return onJob({ ...fixture, job: row, ...(fixture.watched.state === "read" ? { watched: { ...fixture.watched, detail: { ...fixture.watched.detail, job: row } } } : {}) });
}

test("Pilot wired: a Job a person attested is marked on its detail and its row", async () => {
  mount(closedBy("attested"));
  await onScreen();
  await expect.element(page.getByRole("img", { name: "Attested, not verified" }).first()).toBeVisible();
});

test("Pilot wired: a Job a person submitted, which then passed its gates, carries no attested mark", async () => {
  mount(closedBy("submitted"));
  await onScreen();
  await expect.element(page.getByRole("img", { name: "Attested, not verified" })).not.toBeInTheDocument();
});

test("Pilot wired: the `/` list is what the agent named, and the composer offers it", async () => {
  const fleet = new FakeSessionsFleet([hosted("01SESSIONAAAAAAAAAAAAAAAAA", { title: "Fix the flaky store test", hosted: { turn: { state: "idle" }, mode: "auto", running: false, commands: ["compact", "review"] } })]);
  mount(served(fleet));
  await onScreen();
  await toSession("Fix the flaky store test");
  await userEvent.fill(page.getByRole("textbox", { name: "Message" }), "/");
  await expect.element(page.getByRole("option", { name: "/compact" })).toBeVisible();
  await expect.element(page.getByRole("option", { name: "/review" })).toBeVisible();
});
