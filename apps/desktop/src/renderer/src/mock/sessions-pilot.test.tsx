// Pilot from a Session on a real Fleet's path, through `App`: a person takes a stopped Job over from
// its row, the Session starts on its worktree, and one of three exits ends the pilot. What stands where
// main stands is `sessions-fleet.ts`, which answers the take over and the exits as Fleet does and
// publishes the Job and the Session changing. The walk (`walks/piloting.ts`) proves what the screens
// draw from fixtures; this proves they draw it from, and act on, what Fleet serves.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { completedSuccess } from "@armada/jobs/fixtures/build/index";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard, onJob } from "./scenario";
import type { Scenario } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, putOffEveryCall, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const STOPPED = "01JOBSTOPPED";

/** A Board with one escalated Job on it, over a Fleet that serves Sessions. */
function served(fleet: FakeSessionsFleet): Scenario {
  const stopped = job("escalated", { id: STOPPED, handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" });
  return fleet.scenario(onBoard([stopped], { repositories: [ARMADA], picked: ARMADA.root }));
}

/** The stopped Job's tile picked on Active, so its acts are in the pane beside the grid. */
async function toTheJob(): Promise<void> {
  await onScreen();
  // The Job asks, so its call is in front of the panel; put it off to reach the pane under it.
  await putOffEveryCall();
  await userEvent.click(page.getByRole("tab", { name: "Active" }));
  await userEvent.click(page.getByRole("option", { name: /^The retry loop/ }));
}

/** Pilot, confirmed with its default outcome, from the Job's act in its pane. */
async function pilotIt(): Promise<void> {
  await toTheJob();
  await userEvent.click(page.getByRole("button", { name: "Pilot", exact: true }));
  await userEvent.click(page.getByRole("dialog", { name: "Pilot this Job?" }).getByRole("button", { name: "Pilot", exact: true }));
}

const handoff = () => page.getByRole("region", { name: "Handed over: Job 52" });
const ledger = () => page.getByRole("region", { name: "Attachments" });
const exits = () => ledger().getByRole("group", { name: "Ways out of the pilot" });

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
  await expect.element(ledger().getByRole("img", { name: "Handed over with Job 52, not leased" })).toBeVisible();
  await expect.element(ledger().getByRole("img", { name: "Handed over with Job 52, not created here" })).toBeVisible();
  await expect.element(exits()).toBeVisible();
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

test("Pilot wired: the Board says which Session has the Job, and its card opens that Session", async () => {
  const fleet = new FakeSessionsFleet();
  mount(served(fleet));
  await pilotIt();
  await expect.element(handoff()).toBeVisible();

  await userEvent.click(page.getByRole("button", { name: "Cockpit", exact: true }));
  await userEvent.click(page.getByRole("tab", { name: "Active" }));
  await userEvent.click(page.getByRole("option", { name: "The retry loop, Job" }));
  await expect.element(page.getByRole("img", { name: "Piloted in The retry loop" })).toBeVisible();
  await userEvent.click(page.getByRole("button", { name: "Job 52" }));
  const card = page.getByRole("group", { name: "Owned by The retry loop" });
  await expect.element(card).toBeVisible();
  await userEvent.click(card.getByRole("button", { name: "Open Session" }));
  await expect.element(handoff()).toBeVisible();
});

test.each([
  ["Submit for verification", "submit"],
  ["Attest complete", "attest"],
  ["Close as superseded", "supersede"],
] as const)("Pilot wired: %s on the Session's ledger row goes to Fleet, and the row stops offering the exits", async (label, exit) => {
  const fleet = new FakeSessionsFleet();
  mount(served(fleet));
  await pilotIt();
  await expect.element(exits()).toBeVisible();

  await userEvent.click(exits().getByRole("button", { name: label }));
  await expect.poll(() => fleet.calls.exited).toEqual([{ jobId: STOPPED, exit }]);
  await expect.element(exits()).not.toBeInTheDocument();
  // The Session gave the slot and the branch back, so neither is drawn as its own any more.
  await expect.element(ledger().getByRole("img", { name: "Handed over with Job 52, not leased" })).not.toBeInTheDocument();
});

test("Pilot wired: an attestation is closed on a person's word, and the ledger says so apart from a pass", async () => {
  const fleet = new FakeSessionsFleet();
  mount(served(fleet));
  await pilotIt();
  await userEvent.click(exits().getByRole("button", { name: "Attest complete" }));
  // An ended Job is drawn under All.
  await userEvent.click(ledger().getByRole("region", { name: "Jobs" }).getByRole("radio", { name: "All" }));
  await expect.element(ledger().getByRole("img", { name: "Closed by your word, not verified" })).toBeVisible();
});

test("Pilot wired: an exit Fleet refuses is said under the exits, which stay", async () => {
  const fleet = new FakeSessionsFleet();
  mount(served(fleet));
  await pilotIt();
  await expect.element(exits()).toBeVisible();

  fleet.refuses = { code: "fleet.steps_not_advanced", message: "A step has not advanced. Submit it for verification, or close the Job as superseded." };
  await userEvent.click(exits().getByRole("button", { name: "Attest complete" }));
  await expect.element(ledger().getByText("A step has not advanced. Submit it for verification, or close the Job as superseded.")).toBeVisible();
  await expect.element(exits()).toBeVisible();
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
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("button", { name: "Fix the flaky store test" }));
  await userEvent.fill(page.getByRole("textbox", { name: "Message" }), "/");
  await expect.element(page.getByRole("option", { name: "/compact" })).toBeVisible();
  await expect.element(page.getByRole("option", { name: "/review" })).toBeVisible();
});
