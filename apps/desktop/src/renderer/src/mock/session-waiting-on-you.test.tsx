// A Session that waits on the person, through `App`: it is under Needs you with a hand, its ledger
// leads with Waiting on you, the page item opens the window and a choice answers the question.
// The walk `session-waiting-on-you` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Waiting on you: list hand, ledger section first, page item opens the window, a choice answers", async () => {
  mount("session-waiting-on-you");
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  const list = page.getByRole("region", { name: "Sessions" });
  await expect.element(list.getByText("Needs you")).toBeVisible();
  await expect.element(list.getByRole("img", { name: /^Waiting on you:/ })).toBeVisible();
  await userEvent.click(list.getByRole("button", { name: /Pin the store clock/ }));

  const ledger = page.getByRole("region", { name: "Attachments" });
  const section = ledger.getByRole("region", { name: "Waiting on you" });
  await expect.element(section).toBeVisible();
  await userEvent.click(section.getByRole("button", { name: "Open Look at the findings page" }));
  await expect.element(page.getByRole("dialog", { name: "Bridge's window on Pin the store clock" })).toBeVisible();
  await userEvent.click(page.getByRole("button", { name: "Close window" }));

  await userEvent.click(section.getByRole("button", { name: "Choice 2 Frozen" }));
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText("Going with Frozen.")).toBeVisible();
  await expect.element(ledger.getByRole("region", { name: "Waiting on you" }).getByRole("button", { name: "Open Which clock?" })).not.toBeInTheDocument();
});
