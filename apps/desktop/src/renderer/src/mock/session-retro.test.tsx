// A Session's retro, through `App`: the Retro press on a Sessions list row and in the Session's
// own header writes it, and the Retros page opens on it, headed by the Session's title and address.
// A Session elsewhere, which hears no retros, has no press. The walk (`walks/session-retro.ts`)
// shows how it reads.

import { beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { s205SessionRetro, s205SessionRetroRefused } from "./scenarios/session-retro";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

// The Retros page remembers which source it listed, and one test narrows it.
beforeEach(() => window.localStorage.clear());

const sessions = () => page.getByRole("region", { name: "Sessions" });
const row = () => sessions().getByRole("listitem", { name: "Fix the flaky store test" });
/** Sessions' own page: the Dashboard lists no Sessions apart from its tabs. */
const toSessions = () => userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
const sheet = () => page.getByRole("dialog", { name: "Retro" });

test("The Retro press on a row writes the retro and opens it on the Retros page, headed by the Session", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await toSessions();

  await userEvent.click(row().getByRole("button", { name: "Retro" }));
  await expect.element(row().getByRole("button", { name: "Writing the retro" })).toBeDisabled();

  await expect.element(sheet()).toBeVisible();
  await expect.element(sheet().getByText("Fix the flaky store test · s-01IDLECC")).toBeVisible();
  await expect.element(sheet().getByRole("listitem", { name: "A reset waited 40 minutes for an answer" })).toBeVisible();

  const item = sheet().getByRole("listitem", { name: "A reset waited 40 minutes for an answer" });
  await userEvent.click(item.getByRole("button", { name: "Create Job", exact: true }));
  await expect.element(item.getByRole("button", { name: "Proposed Job" })).toBeVisible();
});

test("The Retro press is also in the Session's own header", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await toSessions();

  await userEvent.click(row().getByRole("button", { name: "Fix the flaky store test" }));
  await userEvent.click(page.getByRole("button", { name: "Retro", exact: true }));
  await expect.element(sheet().getByText("Fix the flaky store test · s-01IDLECC")).toBeVisible();
});

test("A refused retro says why on the Sessions page and opens nothing", async () => {
  mount(s205SessionRetroRefused);
  await onScreen();
  await toSessions();

  await userEvent.click(row().getByRole("button", { name: "Retro" }));
  await expect.element(page.getByText("A retro is already being written for this Session")).toBeVisible();
  await expect.element(sheet()).not.toBeInTheDocument();
});

test("The Retros page narrows to Sessions' retros and to Jobs'", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await toSessions();

  await userEvent.click(row().getByRole("button", { name: "Retro" }));
  await userEvent.click(sheet().getByRole("button", { name: "Close" }));
  const list = page.getByRole("list", { name: "Retros" });
  await userEvent.click(page.getByRole("tab", { name: "Sessions", exact: true }));
  await expect.element(list.getByRole("button", { name: "Fix the flaky store test · s-01IDLECC" }).first()).toBeVisible();
  await expect.element(list.getByRole("button", { name: "Job 3" })).not.toBeInTheDocument();
  await userEvent.click(page.getByRole("tab", { name: "Jobs", exact: true }));
  await expect.element(list.getByRole("button", { name: "Job 3" }).first()).toBeVisible();
  await expect.element(list.getByRole("button", { name: "Fix the flaky store test · s-01IDLECC" })).not.toBeInTheDocument();
});

test("Closed, the retro's items stay on the Retros page beside the Jobs', each from its Session and as the agent's way", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await toSessions();

  await userEvent.click(row().getByRole("button", { name: "Retro" }));
  await expect.element(sheet()).toBeVisible();
  await userEvent.click(sheet().getByRole("button", { name: "Close" }));

  const list = page.getByRole("list", { name: "Retros" });
  await expect.element(list.getByRole("button", { name: "Fix the flaky store test · s-01IDLECC" }).first()).toBeVisible();
  await expect.element(list.getByRole("button", { name: "Job 3" }).first()).toBeVisible();
  await expect.element(list.getByRole("listitem", { name: "A restart of Fleet killed the subagents" }).getByText("Agent", { exact: true }).last()).toBeVisible();
});

test("A Session's read honours n: the numbered retro, the newest without it, and an unknown n refused", async () => {
  mount(s205SessionRetro);
  await onScreen();
  await toSessions();

  await userEvent.click(row().getByRole("button", { name: "Retro" }));
  await expect.element(sheet()).toBeVisible();
  const id = "01IDLECCCCCCCCCCCCCCCCCCCC";
  const first = await window.armada.readRetro({ kind: "session", id, n: 1 });
  expect(first.ok && first.retro.items?.[0]?.id).toBe(`${id}-r1-1`);
  const newest = await window.armada.readRetro({ kind: "session", id });
  expect(newest.ok && newest.retro.items?.[0]?.id).toBe(`${id}-r1-1`);
  expect((await window.armada.readRetro({ kind: "session", id, n: 9 })).ok).toBe(false);
});
