// The Session composer's empty Model option names `ModelChoices.session`, the model a Session starts on,
// and not `ModelChoices.default`, the Drone and Job default.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "./scenario";
import { FakeSessionsFleet, hosted } from "./sessions-fleet";
import { mount, onScreen, toPlace, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

test("the composer's default model follows models.session when it differs from models.default", async () => {
  const served = new FakeSessionsFleet([hosted("01SESSIONAAAAAAAAAAAAAAAAA", { title: "First one" })]).scenario(
    onBoard([], { repositories: [ARMADA], picked: ARMADA.root }),
  );
  const models = { models: ["haiku", "sonnet", "opus"], default: "sonnet", session: "opus" };
  mount({ ...served, state: { ...served.state, holds: { ...served.state.holds, models } } });
  await onScreen();
  await toPlace("Sessions");
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: "First one" }));

  await expect.element(page.getByRole("option", { name: "opus · default" })).toBeInTheDocument();
  await expect.element(page.getByRole("option", { name: "sonnet · default" })).not.toBeInTheDocument();
});
