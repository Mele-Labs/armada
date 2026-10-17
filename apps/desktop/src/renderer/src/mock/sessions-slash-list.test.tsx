// The `/` list in a Session's box on a real Fleet's path, through `App`: it is what the agent named. What
// stands where main stands is `sessions-fleet.ts`. This proves the screens draw it from what Fleet serves.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

test("the `/` list is what the agent named, and the composer offers it", async () => {
  const fleet = new FakeSessionsFleet([hosted("01SESSIONAAAAAAAAAAAAAAAAA", { title: "Fix the flaky store test", hosted: { turn: { state: "idle" }, mode: "auto", running: false, commands: ["compact", "review"] } })]);
  mount(fleet.scenario(onBoard([], { repositories: [ARMADA], picked: ARMADA.root })));
  await onScreen();
  await toSession("Fix the flaky store test");
  await userEvent.fill(page.getByRole("textbox", { name: "Message" }), "/");
  await expect.element(page.getByRole("option", { name: "/compact" })).toBeVisible();
  await expect.element(page.getByRole("option", { name: "/review" })).toBeVisible();
});
