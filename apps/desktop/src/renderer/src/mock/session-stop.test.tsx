// A Stop sits beside the working mark of a hosted Session's thread, only while a turn runs. Pressing
// it calls `stopSession`, the Session goes idle, the Stop goes, and the message box takes the next
// message. Through `App` over a Fleet that serves Sessions.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const WORKING = "01SESSIONAAAAAAAAAAAAAAAAA";
const RESTING = "01SESSIONBBBBBBBBBBBBBBBBB";

function served(fleet: FakeSessionsFleet) {
  return fleet.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root }));
}

const thread = () => page.getByRole("region", { name: "Thread" });
const stop = () => thread().getByRole("button", { name: "Stop" });

test("Stop shows while the turn is working, ends it, and leaves the box ready", async () => {
  const fleet = new FakeSessionsFleet([
    hosted(WORKING, { title: "Backfill dry run", hosted: { turn: { state: "working" }, mode: "auto", running: true } }),
    hosted(RESTING, { title: "Resting one" }),
  ]);
  mount(served(fleet));
  await onScreen();

  // A Session at rest draws no Stop.
  await toSession("Resting one");
  await expect.element(thread()).toBeVisible();
  await expect.element(stop()).not.toBeInTheDocument();

  await toSession("Backfill dry run");
  await expect.element(thread().getByRole("img", { name: "Working" })).toBeVisible();
  await expect.element(stop()).toBeVisible();

  await userEvent.click(stop());
  await expect.poll(() => fleet.calls.stopped).toEqual([WORKING]);
  await expect.element(stop()).not.toBeInTheDocument();
  await expect.element(thread().getByRole("img", { name: "Working" })).not.toBeInTheDocument();

  // The box is usable: a message goes through and starts the next turn.
  await userEvent.click(page.getByRole("textbox", { name: "Message" }));
  await userEvent.keyboard("carry on{Enter}");
  await expect.poll(() => fleet.calls.sent.map((one) => one.text)).toEqual(["carry on"]);
  await expect.element(stop()).toBeVisible();
});
