// A Session that waits on the person, through `App`: it is under Needs you with a hand, the band at the
// foot of its thread opens to the items, the page item opens the window, and the question is answered
// on its own card and is not in the band.
// The walk `session-waiting-on-you` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Waiting on you: list hand, the band opens, page item opens the window, the card answers", async () => {
  mount("session-waiting-on-you");
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  const list = page.getByRole("region", { name: "Sessions" });
  await expect.element(list.getByText("Needs you")).toBeVisible();
  await expect.element(list.getByRole("img", { name: /^Waiting on you:/ })).toBeVisible();
  await userEvent.click(list.getByRole("button", { name: /Pin the store clock/ }));

  await expect.element(page.getByRole("region", { name: "Attachments" }).getByRole("region", { name: "Waiting on you" })).not.toBeInTheDocument();
  const band = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(band.getByRole("button", { name: /^Needs you/ }));
  await expect.element(band.getByRole("button", { name: "Open Which clock?" })).not.toBeInTheDocument();
  await userEvent.click(band.getByRole("button", { name: "Open Look at the findings page" }));
  await expect.element(page.getByRole("dialog", { name: "Bridge's window on Pin the store clock" })).toBeVisible();
  await userEvent.click(page.getByRole("button", { name: "Close window" }));

  const card = page.getByRole("article", { name: "Waiting on you" });
  await userEvent.click(card.getByRole("radio", { name: "Frozen" }));
  await userEvent.click(card.getByRole("button", { name: "Answer" }));
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText("Going with Frozen.")).toBeVisible();
  await expect.element(card).not.toBeInTheDocument();
});
