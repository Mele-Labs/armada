// j and k on the Sessions page, through `App`: each moves down or up the list and opens the Session it
// lands on, and neither does anything while the person is typing in a field.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";

import { onBoard } from "./scenario";
import { FakeSessionsFleet, terminal } from "./sessions-fleet";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const opened = (title: string) => page.getByRole("button", { name: `${title}, rename` });

test("Sessions keys: j opens the next Session down the list and k the one above, and a field keeps the keys", async () => {
  const fleet = new FakeSessionsFleet([
    terminal("S1", { title: "First one" }),
    terminal("S2", { title: "Second one" }),
    terminal("S3", { title: "Third one" }),
  ]);
  const stopped = job("escalated", { id: "01JOBSTOPPED", handle: "52-the-retry-loop", title: "The retry loop", branch: "fix/retry-loop", owner_manifest_id: "armada" });
  mount(fleet.scenario(onBoard([stopped], { repositories: [ARMADA], picked: ARMADA.root })));
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));

  await userEvent.keyboard("j");
  await expect.element(opened("First one")).toBeVisible();
  await userEvent.keyboard("j");
  await expect.element(opened("Second one")).toBeVisible();
  await userEvent.keyboard("k");
  await expect.element(opened("First one")).toBeVisible();

  await userEvent.click(opened("First one"));
  await userEvent.keyboard("jj");
  await expect.element(page.getByRole("textbox", { name: "Session name" })).toHaveValue("First onejj");
  await userEvent.keyboard("{Escape}");
  await expect.element(opened("First one")).toBeVisible();
});
