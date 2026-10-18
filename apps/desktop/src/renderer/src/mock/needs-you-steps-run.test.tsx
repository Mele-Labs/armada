// A run item that carries steps, through `App`: its row in the Needs you list reads the item's words, and its steps panel keeps the
// command with a key that copies it. A step's backticked command gets its own key, and the item's command
// is left off the top where a step names it. The walk `needs-you-steps-run` draws it.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const DEPLOY = "Deploy the store Worker once, from this machine";
const BY_STEP = "Deploy the store Worker, a command a step";
const COMMAND = "cd ~/Development/armada/apps/store && pnpm exec wrangler login && pnpm exec wrangler deploy";

async function open(title: string) {
  mount("needs-you-steps-run");
  await onScreen();
  await toSession(/Central store/);
  const needs = page.getByRole("region", { name: "Needs you" });
  await userEvent.click(needs.getByRole("button", { name: /^Needs you/ }));
  const row = needs.getByRole("button", { name: `Open ${title}` });
  await expect.element(row).toHaveTextContent(title);
  await userEvent.click(row);
  // An item whose act is its steps does it from the list row.
  return page.getByRole("dialog", { name: title });
}

test("Steps: a run item's row reads its words, and its panel copies the command", async () => {
  const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
  const panel = await open(DEPLOY);
  await expect.element(panel.getByRole("heading", { name: DEPLOY })).toBeVisible();
  await expect.element(panel.getByText(COMMAND, { exact: true })).toBeVisible();
  await userEvent.click(panel.getByRole("button", { name: "Copy command" }));
  expect(writeText).toHaveBeenCalledWith(COMMAND);
  await expect.element(panel.getByRole("checkbox", { name: "Copy the workers.dev address it prints" })).toBeVisible();
});

test("Steps: each backticked command has its own key, and the top leaves out one a step names", async () => {
  const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
  const panel = await open(BY_STEP);
  await expect.element(panel.getByRole("button", { name: "Copy step 1 command" })).toBeInTheDocument();
  await expect.element(panel.getByRole("button", { name: "Copy step 4 command" })).toBeInTheDocument();
  await expect.element(panel.getByRole("button", { name: "Copy step 3 command" })).not.toBeInTheDocument();
  await expect.element(panel.getByRole("button", { name: "Copy command" })).not.toBeInTheDocument();
  await userEvent.click(panel.getByRole("button", { name: "Copy step 2 command" }));
  expect(writeText).toHaveBeenCalledWith("pnpm exec wrangler deploy");
});
