// Pulse's kill on a process row, through `App`, against a Fleet that does not
// serve the route yet.
//
// The owner, 29 Sep 2026: the control ships ahead of its route, and pressing it
// says so — `Not implemented`, with the issue that builds the route as a whole
// link in the debug info, so the paste is a brief an agent can start from.
// `docs/contracts/error-contract.md` has the rule; `packages/protocol/src/
// pending.ts` names the route. The mock answers as Fleet's router answers a
// path it has no route for: a bare 404.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { issueLink } from "@armada/protocol";

import { entered, motion, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * The issue that builds both kill routes, as the debug info must carry it.
 * **Through `issueLink`**, because the gate refuses the tracker's name as a
 * literal outside the adapters; what this pins is a whole link to #1647.
 */
const ISSUE = issueLink(1647);

test("a process held and confirmed says the kill is not built yet, and the debug info names the issue", async () => {
  // The kill is held, and under reduced motion there is no hold to make.
  await motion();
  const written: string[] = [];
  // `navigator.clipboard` is a getter, so the write is replaced rather than the object.
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: { writeText: (text: string) => (written.push(text), Promise.resolve()) },
  });
  mount("arc/executing-sequential");
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
  await dialog.getByRole("button", { name: "Kill process" }).click();

  await expect.element(page.getByText("Not implemented", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Copy debug info" }).click();
  await expect.poll(() => written).toHaveLength(1);
  const pasted = written[0]!;
  expect(pasted).toContain("bridge.not_implemented");
  expect(ISSUE).toMatch(/^https:\/\/\S+\/issues\/1647$/);
  expect(pasted).toContain(ISSUE);
  expect(pasted).toContain("POST /jobs/{job_id}/processes/{pid}/kill");
  expect(pasted).toContain("52118");
  // Not the disagreement it would read as without the pending entry.
  expect(pasted).not.toContain("bridge.command.unanswerable");
});
