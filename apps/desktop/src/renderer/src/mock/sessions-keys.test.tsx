// j and k while a Session is open on the Cockpit, through `App`: neither does anything while the person is typing in a field.
// What j and k do on the Work list itself is `cockpit-list-by-repository.test.tsx`'s; the Sessions page's own list and its keys are gone.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/repository";

import { onBoard } from "./scenario";
import { FakeSessionsFleet, terminal } from "./sessions-fleet";
import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };

const opened = (title: string) => page.getByRole("button", { name: `${title}, rename` });

test("Sessions keys: a field keeps j and k", async () => {
  const fleet = new FakeSessionsFleet([
    terminal("S1", { title: "First one" }),
    terminal("S2", { title: "Second one" }),
    terminal("S3", { title: "Third one" }),
  ]);
  mount(fleet.scenario(onBoard({ repositories: [ARMADA], picked: ARMADA.root })));
  await onScreen();
  await toSession("First one");
  await expect.element(opened("First one")).toBeVisible();

  await userEvent.click(opened("First one"));
  await userEvent.keyboard("jj");
  await expect.element(page.getByRole("textbox", { name: "Job name" })).toHaveValue("First onejj");
  await userEvent.keyboard("{Escape}");
  await expect.element(opened("First one")).toBeVisible();
});
