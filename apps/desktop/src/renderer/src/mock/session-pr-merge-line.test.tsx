// A Session's pull request panel through `App`: a pull request in the merge queue shows the line
// it is in, in order with itself current, and one in no queue shows none. The walk
// `session-pr-merge-line` draws it.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { mount, onScreen, toSession, unmountAfterEach } from "./testing";

unmountAfterEach();

test("PR panel: the merge line it is in, itself current; none for one in no queue", async () => {
  mount("session-merge-line");
  await onScreen();
  await toSession(/Store migration spike/);
  const ledger = page.getByRole("region", { name: "Attachments" });

  await userEvent.click(ledger.getByRole("button", { name: "Open Pull request #1849" }));
  const first = page.getByRole("dialog", { name: "Pull request #1849" });
  await expect.element(first).toBeVisible();
  await expect.element(page.getByRole("region", { name: "Merge line" })).not.toBeInTheDocument();
  await userEvent.click(first.getByRole("button", { name: "Close" }));

  await userEvent.click(ledger.getByRole("button", { name: "Open Pull request #1850" }));
  await expect.element(page.getByRole("dialog", { name: "Pull request #1850" })).toBeVisible();
  // The Cockpit draws its own merge queue track too; the panel's is the plate beside its dialog, drawn over the app and not in the Session's view.
  const queue = page.elementLocator(document.querySelector<HTMLElement>(".armada-sheet__aside")!).getByRole("list", { name: "Merge queue" });
  await expect.element(queue).toBeVisible();
  await expect.element(queue.getByRole("listitem").nth(2)).toHaveAttribute("aria-current", "true");
  await expect.element(queue.getByRole("listitem").nth(2)).toHaveAccessibleName(/^Record each migration once/);
});
