// A press on a pull request in a Session's panel, through `App`: the button is pending until the
// forge answers, auto-merge then settles the Session's own Merge item, which tells the agent. The walk `session-pr-acts` draws it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("PR acts: pending while underway, auto-merge settles the Merge item", async () => {
  mount("session-pr-acts");
  await onScreen();
  await toSession(/Store migration spike/);

  const waiting = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(waiting.getByRole("button", { name: /^Needs you/ }));
  await expect.element(waiting.getByRole("button", { name: "Open Merge #1850 once ci is green" })).toBeVisible();
  // Open, the panel's scrim covers the thread; fold it before pressing into Flight control.
  await userEvent.keyboard("{Escape}");
  await expect.element(waiting.getByRole("button", { name: /^Needs you/ })).toHaveAttribute("aria-expanded", "false");
  await userEvent.click(page.getByRole("region", { name: "Flight controls" }).getByRole("button", { name: "Artifacts", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Artifacts", exact: true }).getByRole("region", { name: "Pull requests", exact: true }).getByRole("button", { name: /^Pull request #1850 / }));

  const card = page.getByRole("dialog", { name: "Pull request #1850" });
  await userEvent.click(card.getByRole("button", { name: "Enable auto-merge" }));
  await expect.element(card.getByRole("button", { name: "Auto-merge on" })).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Thread" }).getByRole("region", { name: "You answered" }).filter({ hasText: "Auto-merge is on for #1850" }).getByRole("img", { name: "Answered: Merge #1850 once ci is green" })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Open Merge #1850 once ci is green" })).not.toBeInTheDocument();
});
