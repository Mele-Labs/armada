// A file of a Session's pull request opens as a diff in a panel to the left of the pull request's own,
// from a press on its row in the Files tab.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

const FORGE = "apps/desktop/src/main/forge.ts";

async function filesTab() {
  mount("session-pr-panel");
  await onScreen();
  await toSession(/Store migration spike/);
  await userEvent.click(page.getByRole("region", { name: "Attachments" }).getByRole("button", { name: /^Open Pull request #2209/ }));
  const pull = page.getByRole("dialog", { name: "Pull request #2209" });
  await userEvent.click(pull.getByRole("tab", { name: "Files" }));
  return { pull, row: pull.getByRole("button", { name: /main\/forge\.ts/ }), diff: page.getByRole("dialog", { name: FORGE }) };
}

test("a file's row opens its diff beside the panel, marks the row, and pressing it again closes it", async () => {
  const { row, diff } = await filesTab();
  await userEvent.click(row);
  await expect.element(diff.getByText(/openPullRequestsIn/).first()).toBeVisible();
  await expect.element(row).toHaveAttribute("aria-pressed", "true");
  const at = (name: string | RegExp) => page.getByRole("dialog", { name }).element().getBoundingClientRect();
  expect(at(FORGE).right).toBeLessThanOrEqual(at("Pull request #2209").left);
  await userEvent.click(row);
  await expect.element(diff).not.toBeInTheDocument();
  await expect.element(row).toHaveAttribute("aria-pressed", "false");
});

test("Esc closes the diff and leaves the pull request open", async () => {
  const { pull, row, diff } = await filesTab();
  await userEvent.click(row);
  await expect.element(diff).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.element(diff).not.toBeInTheDocument();
  await expect.element(pull).toBeVisible();
});
