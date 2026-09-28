// What the arc's screens say in words, where the owner has ruled on a
// sentence. `arc.test.tsx` is the rest of the arc, and this is a sibling of it
// because that file is at the length the gate refuses.
//
// **The rule these claims hold to is his.** The app shows facts, and an
// explanation is something he chooses to see — so a sentence that would still
// be true of a Job that had never run is not on the screen, and a sentence
// that apologises for what Fleet does not serve yet is not on it either.
// Every claim below names a sentence he cut on 28 September 2026 and, beside
// it, the fact that had to survive the cut.

import { expect, test, describe } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** A moment with one of the Job's destinations open, pressed the way a person presses it. */
async function at(moment: string, tab: string): Promise<void> {
  mount(moment);
  await expect.element(page.getByRole("tab", { name: new RegExp(`^${tab}`) })).toBeVisible();
  await page.getByRole("tab", { name: new RegExp(`^${tab}`) }).click();
  await expect.element(page.getByRole("tabpanel", { name: tab })).toBeVisible();
}

/** One group of the running board, by the heading it carries. */
const runGroup = (ordinal: number) =>
  page
    .getByRole("list", { name: "The groups of this step, in the order they run" })
    .getByRole("listitem")
    .filter({ hasText: new RegExp(`Group ${ordinal}`) })
    .first();

/** One task's row, by the name the board gives it. */
const taskRow = (id: string) => page.getByRole("listitem", { name: new RegExp(`^${id} `) });

describe("the implement board", () => {
  test(
    "arc/group-failed: no band at the boundary claims anything about a case, and the task " +
      "that stopped says whose words the reason is",
    async () => {
      await at("arc/group-failed", "Workflow");

      // The band said Fleet serves no case yet, which is a fact about the
      // build rather than about this Job.
      await expect
        .element(runGroup(3).getByRole("region", { name: "Tests at this boundary" }))
        .not.toBeInTheDocument();
      await expect.element(runGroup(3)).not.toHaveTextContent("does not serve the cases");

      // The reason the task stopped stays. It is the Drone's own words, and
      // the label in front of it is what says so — a bare sentence under a
      // row read as a note somebody had left.
      await expect.element(taskRow("T6")).toHaveTextContent("Why it stopped");
      await expect
        .element(taskRow("T6"))
        .toHaveTextContent("The row's press opened the Board rather than the Job");
    },
  );
});

describe("Pulse", () => {
  test(
    "arc/executing-concurrent: the headline says the look has not run, over figures that " +
      "have plainly been read",
    async () => {
      mount("arc/executing-concurrent");
      await page.getByRole("tab", { name: "Pulse" }).click();
      await expect.element(page.getByRole("tabpanel", { name: "Pulse" })).toBeVisible();

      // It read "Nobody has asked whether this job is working." over a panel
      // of figures, which made the poll and the look read as one claim.
      await expect.element(page.getByText(/Nobody has asked/)).not.toBeInTheDocument();
      await expect.element(page.getByText("This job has not been looked at.")).toBeVisible();

      // The figures under it are still the reading, and still qualified. The
      // act above them carries the owner's own word for it.
      await expect
        .element(page.getByText(/^Read .* ago\. Taken again every 10s while open\.$/))
        .toBeVisible();
      await expect.element(page.getByRole("button", { name: "Refresh" })).toBeVisible();
      await expect
        .element(page.getByRole("button", { name: "Look now" }))
        .not.toBeInTheDocument();
    },
  );
});
