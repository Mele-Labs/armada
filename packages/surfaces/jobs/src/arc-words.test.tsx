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

import { mount, unmountAfterEach } from "@armada/desktop/mock";

unmountAfterEach();

// Core and Jobs only: the surface's own members, and the scenario answers the rest.
const SLICES = { slices: ["core", "jobs"] } as const;

/** A moment with one of the Job's destinations open, pressed the way a person presses it. */
async function at(moment: string, tab: string): Promise<void> {
  mount(moment, SLICES);
  await expect.element(page.getByRole("tab", { name: new RegExp(`^${tab}`) })).toBeVisible();
  await page.getByRole("tab", { name: new RegExp(`^${tab}`) }).click();
  await expect.element(page.getByRole("tabpanel", { name: tab })).toBeVisible();
}

/** The plan as a list. The board's reading is under this view since 28 Sep 2026. */
async function planList(moment: string): Promise<void> {
  await at(moment, "Plan");
  await page.getByRole("tab", { name: "List" }).click();
  await expect
    .element(page.getByRole("list", { name: "Groups, in the order they run" }))
    .toBeVisible();
}

/**
 * One group's card, by the name the card carries. **Never by its text**: a
 * card says `Group 4 writes these files too` where it overlaps group 4.
 */
const groupCard = (ordinal: number) =>
  page.getByRole("listitem", { name: `Group ${ordinal}`, exact: true });

/** One task's row, by the name the board gives it. */
const taskRow = (id: string) => page.getByRole("listitem", { name: new RegExp(`^${id} `) });

describe("the plan board", () => {
  test(
    "arc/group-failed: no band at the boundary claims anything about a case, and the task " +
      "that stopped says whose words the reason is",
    async () => {
      // On the Plan tab's List view since 28 Sep 2026, when the owner took the
      // groups off Workflow and the board's reading came here.
      await planList("arc/group-failed");

      // The band said Fleet serves no case yet, which is a fact about the
      // build rather than about this Job.
      await expect
        .element(groupCard(3).getByRole("region", { name: "Tests at this boundary" }))
        .not.toBeInTheDocument();
      await expect.element(groupCard(3)).not.toHaveTextContent("does not serve the cases");

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
    "arc/executing-concurrent: no headline says the look has not run, over figures that " +
      "have plainly been read",
    async () => {
      mount("arc/executing-concurrent", SLICES);
      await page.getByRole("tab", { name: "Pulse" }).click();
      await expect.element(page.getByRole("tabpanel", { name: "Pulse" })).toBeVisible();

      // It read "Nobody has asked whether this job is working." over a panel
      // of figures, which made the poll and the look read as one claim. Its
      // successor sat beside `Updated 4s ago` and read as a contradiction, so
      // the owner took the head's sentence out on 29 Sep.
      await expect.element(page.getByText(/Nobody has asked/)).not.toBeInTheDocument();
      await expect.element(page.getByText(/has not been looked at/)).not.toBeInTheDocument();

      // The figures are still the reading, and still qualified — by when, and
      // nothing more. The act beside them carries the owner's own word for it.
      await expect.element(page.getByText(/^Updated .* ago$/)).toBeVisible();
      await expect.element(page.getByRole("button", { name: "Refresh" })).toBeVisible();
      await expect
        .element(page.getByRole("button", { name: "Look now" }))
        .not.toBeInTheDocument();
    },
  );
});
