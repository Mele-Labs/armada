// A Session's retro, through `App`: the Retro press on a Sessions list row and in the Session's
// own header writes it, and the Retros page opens on it, headed by the Session's title and address.
// A Session elsewhere, which hears no retros, has no press. The walk (`walks/session-retro.ts`)
// shows how it reads.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { s205SessionRetro } from "./scenarios/session-retro";
import { s204SessionListViews } from "./scenarios/session-list-views";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const sessions = () => page.getByRole("region", { name: "Sessions" });
const row = () => sessions().getByRole("listitem", { name: "Fix the flaky store test" });
const sheet = () => page.getByRole("dialog", { name: "Retro" });

test("The Retro press on a row writes the retro and opens it on the Retros page, headed by the Session", async () => {
  mount(s205SessionRetro);
  await onScreen();

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

  await userEvent.click(row().getByRole("button", { name: "Fix the flaky store test" }));
  await userEvent.click(page.getByRole("button", { name: "Retro", exact: true }));
  await expect.element(sheet().getByText("Fix the flaky store test · s-01IDLECC")).toBeVisible();
});

test("A Session that hears no retros has no press", async () => {
  mount(s204SessionListViews);
  await onScreen();

  await expect.element(page.getByRole("button", { name: "Retro", exact: true })).not.toBeInTheDocument();
});

test("Closed, the retro's items stay on the Retros page beside the Jobs', each from its Session and as the agent's way", async () => {
  mount(s205SessionRetro);
  await onScreen();

  await userEvent.click(row().getByRole("button", { name: "Retro" }));
  await expect.element(sheet()).toBeVisible();
  await userEvent.click(sheet().getByRole("button", { name: "Close" }));

  const list = page.getByRole("list", { name: "Retros" });
  await expect.element(list.getByRole("button", { name: "Fix the flaky store test · s-01IDLECC" }).first()).toBeVisible();
  await expect.element(list.getByRole("button", { name: "Job 3" }).first()).toBeVisible();
  await expect.element(list.getByRole("listitem", { name: "A restart of Fleet killed the subagents" }).getByText("Agent", { exact: true }).last()).toBeVisible();
});
