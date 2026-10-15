// A waiting item that carries steps, through `App`: its press opens the steps panel in place of the link, the key
// reads Done while the box is empty and answers the item whatever was ticked, and corrections travel with the answer.
// The walk `needs-you-steps` draws it; this holds that the presses reach the same code.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toPlace, unmountAfterEach } from "./testing";

unmountAfterEach();

const TITLE = "Register the OAuth app";

async function openSteps() {
  mount("needs-you-steps");
  await onScreen();
  await toPlace("Sessions");
  await userEvent.click(page.getByRole("region", { name: "Sessions" }).getByRole("button", { name: /Add single sign-on/ }));
  const needs = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(needs.getByRole("button", { name: /^Needs you/ }));
  const row = needs.getByRole("button", { name: `Open ${TITLE}` });
  await userEvent.click(row);
  return { row, panel: page.getByRole("dialog", { name: TITLE }) };
}

test("Steps: Done with nothing ticked and nothing typed answers the item with Done", async () => {
  const { row, panel } = await openSteps();
  await expect.element(panel.getByRole("checkbox", { name: "Name it Armada" })).toBeVisible();
  await expect.element(panel.getByRole("button", { name: "Send" })).not.toBeInTheDocument();
  await userEvent.click(panel.getByRole("button", { name: "Done" }));
  await expect.element(page.getByRole("region", { name: "Thread" }).getByText("Done", { exact: true }).first()).toBeVisible();
  await expect.element(row).not.toBeInTheDocument();
});

test("Steps: a drop and a typed reply send the reply with the correction line", async () => {
  const { row, panel } = await openSteps();
  await expect.element(panel.getByRole("button", { name: "Done" })).toBeVisible();
  await userEvent.click(panel.getByRole("button", { name: "Drop step 4" }));
  await userEvent.type(panel.getByRole("textbox", { name: `Reply to ${TITLE}` }), "Registered it");
  await userEvent.click(panel.getByRole("button", { name: "Send" }));
  const thread = page.getByRole("region", { name: "Thread" });
  await expect.element(thread.getByText(/Registered it/).first()).toBeVisible();
  await expect.element(thread.getByText(/Step 4 dropped: "Upload a logo"/).first()).toBeVisible();
  await expect.element(row).not.toBeInTheDocument();
});
