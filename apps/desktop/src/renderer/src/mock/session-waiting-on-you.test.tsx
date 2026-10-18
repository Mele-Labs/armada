// A Session that waits on the person, through `App`: its row in the Cockpit's list has a hand, the band at the
// foot of its thread opens to a list, the question first, a press on the page item opens the window,
// and the question's own row opens the card that answers it.
// The walk `session-waiting-on-you` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Waiting on you: row hand, the band opens, page item opens the window, the card answers", async () => {
  mount("session-waiting-on-you");
  await onScreen();
  const list = page.getByRole("region", { name: "Jobs", exact: true });
  await expect.element(list.getByRole("img", { name: /^Waiting on you/ })).toBeVisible();
  await toSession(/Pin the store clock/);

  await expect.element(page.getByRole("region", { name: "Attachments" }).getByRole("region", { name: "Waiting on you" })).not.toBeInTheDocument();
  const band = page.getByRole("region", { name: "Needs you" });
  // The question arrives with the panel open to the list, and is its first row.
  await expect.element(band.getByRole("button", { name: "Open Which clock?" })).toBeVisible();
  await userEvent.click(band.getByRole("button", { name: "Open Look at the findings page" }));
  await expect.element(page.getByRole("dialog", { name: "Bridge's window on Pin the store clock" })).toBeVisible();
  await userEvent.click(page.getByRole("button", { name: "Close window" }));

  await userEvent.click(band.getByRole("button", { name: "Open Which clock?" }));
  const card = page.getByRole("article", { name: "Waiting on you" });
  await userEvent.click(card.getByRole("radio", { name: "Frozen" }));
  await userEvent.click(card.getByRole("button", { name: "Answer" }));
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText("Going with Frozen.")).toBeVisible();
  await expect.element(card).not.toBeInTheDocument();
});
