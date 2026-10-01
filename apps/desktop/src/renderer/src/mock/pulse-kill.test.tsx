// Pulse's kill on a process row, through `App`, against a Fleet that serves
// the route (#1647).
//
// The owner, 29 Sep 2026: the row's kill is held, and then **also** confirmed
// in a dialog naming the process. What crosses is the Job and the pid, and
// nothing else: Fleet reads the Job's tree again at the act and decides
// whether the pid is in it, so Bridge names and never grants.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { motion, entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

test("a process held and confirmed is sent to Fleet by its pid, and nothing fails", async () => {
  // The kill is held, and under reduced motion there is no hold to make.
  await motion();
  const app = mount("arc/executing-sequential");
  const sent: [string, number][] = [];
  const answer = app.api.killProcess;
  app.api.killProcess = (jobId, pid) => (sent.push([jobId, pid]), answer(jobId, pid));
  await page.getByRole("tab", { name: "Pulse" }).click();
  await expect.element(page.getByRole("tabpanel", { name: "Pulse" })).toBeVisible();

  // The row's kill, held on the keyboard for the whole of `--duration-hold`. A
  // press let go at once kills nothing, which is the hold's own story.
  const row = page.getByRole("region", { name: "Processes" }).getByRole("listitem").filter({ hasText: "52118" });
  const kill = row.getByRole("button", { name: "Hold to kill" });
  (kill.element() as HTMLElement).focus();
  await userEvent.keyboard("{Enter>}");
  await new Promise((resolve) => setTimeout(resolve, 1200));
  await userEvent.keyboard("{/Enter}");

  // Held, and still asked: the owner wanted both.
  const dialog = page.getByRole("dialog", { name: /pid 52118\?$/ });
  await entered(dialog);
  expect(sent, "nothing is sent before the dialog is answered").toHaveLength(0);
  await dialog.getByRole("button", { name: "Kill process" }).click();

  await expect.poll(() => sent).toHaveLength(1);
  expect(sent[0]?.[1]).toBe(52118);
  await expect.element(page.getByText("Not implemented", { exact: true })).not.toBeInTheDocument();
});
