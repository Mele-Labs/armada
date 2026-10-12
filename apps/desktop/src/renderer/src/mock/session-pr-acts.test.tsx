// A press on a pull request in a Session's panel, through `App`: the button is pending until the
// forge answers, auto-merge then settles the Session's own Merge item, which tells the agent, and a
// Review settles as "Review dispatched". The walk `session-pr-acts` draws it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("PR acts: pending while underway, auto-merge settles the Merge item, Review dispatched", async () => {
  mount("session-pr-acts");
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Store migration spike/ }));

  const waiting = page.getByRole("region", { name: "Attachments" }).getByRole("region", { name: "Waiting on you" });
  await expect.element(waiting.getByRole("button", { name: "Open Merge #1850 once ci is green" })).toBeVisible();
  await userEvent.click(page.getByRole("region", { name: "Attachments" }).getByRole("button", { name: "Open Pull request #1850" }));

  const card = page.getByRole("dialog", { name: "Pull request #1850" });
  await userEvent.click(card.getByRole("button", { name: "Enable auto-merge" }));
  await expect.element(card.getByRole("button", { name: "Review", exact: true })).toBeDisabled();
  await expect.element(card.getByRole("button", { name: "Auto-merge on" })).toBeVisible();
  // What tells the agent: its own item quoted, then the answer, as Fleet words it (#2173).
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText(/^Re: .*#1850[\s\S]*Auto-merge is on for #1850/)).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Open Merge #1850 once ci is green" })).not.toBeInTheDocument();

  await userEvent.click(card.getByRole("button", { name: "Review", exact: true }));
  await expect.element(card.getByRole("button", { name: "Review dispatched" })).toBeDisabled();
});
