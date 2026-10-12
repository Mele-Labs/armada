// A waiting item the agent gave no act, in the Needs you band, through `App`: its glyph is Reply, a press opens a box under
// the row, Escape keeps the draft, and a send reaches the Session and takes the item off the list.
// The walk `session-reply` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

test("Reply: a no-act item opens a box, keeps its draft, and a send answers it", async () => {
  mount("session-reply");
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Pin the store clock/ }));

  const section = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(section.getByRole("button", { name: /^Needs you/ }));
  await expect.element(section.getByRole("img", { name: "Reply", exact: true }).first()).toBeVisible();
  const row = section.getByRole("button", { name: "Open Which date should the clock pin to?" });
  await userEvent.click(row);
  const box = section.getByRole("textbox", { name: /^Reply to Which date/ });
  await expect.element(section.getByRole("button", { name: "Send" })).toBeDisabled();

  await userEvent.type(box, "Pin it to the first of March");
  await userEvent.keyboard("{Escape}");
  await expect.element(box).not.toBeInTheDocument();
  await userEvent.click(row);
  await expect.element(box).toHaveValue("Pin it to the first of March");

  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText(/Pin it to the first of March/).first()).toBeVisible();
  await expect.element(row).not.toBeInTheDocument();
});

test("Reply: an answer item with no choices and no question in the thread opens a box too", async () => {
  mount("session-reply");
  await onScreen();
  await userEvent.click(page.getByRole("button", { name: "Sessions", exact: true }));
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Pin the store clock/ }));

  const section = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(section.getByRole("button", { name: /^Needs you/ }));
  const row = section.getByRole("button", { name: "Open Name the helper eventually, or settle?" });
  await userEvent.click(row);
  await userEvent.type(section.getByRole("textbox", { name: /^Reply to Name the helper/ }), "Call it settle");
  await userEvent.keyboard("{Enter}");
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText(/Re: Name the helper eventually, or settle\?/)).toBeVisible();
  await expect.element(row).not.toBeInTheDocument();
});
