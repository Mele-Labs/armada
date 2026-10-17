// A piloted Job on a real Fleet's path, through `App`: a Job a person closed on their word is marked, and
// the `/` list is what the agent named. What stands where main stands is `sessions-fleet.ts`, which answers
// the exits as Fleet does. This proves the screens draw it from what Fleet serves.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { completedSuccess } from "@armada/screens/src/fixtures/build/index";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard, onJob } from "./scenario";
import type { Scenario } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const STOPPED = "01JOBSTOPPED";

/** A Board with one escalated Job on it, over a Fleet that serves Sessions. */
function served(fleet: FakeSessionsFleet): Scenario {
  const stopped = job("escalated", { id: STOPPED, handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" });
  return fleet.scenario(onBoard([stopped], { repositories: [ARMADA], picked: ARMADA.root }));
}

/** `completedSuccess`, with the pilot a person ended it by. */
function closedBy(exit: string | undefined): Scenario {
  const fixture = completedSuccess();
  const piloted = exit === undefined ? undefined : { reason: "take_over", since: "2026-10-07T13:00:00.000Z", exit, ended_at: "2026-10-07T13:30:00.000Z" };
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
