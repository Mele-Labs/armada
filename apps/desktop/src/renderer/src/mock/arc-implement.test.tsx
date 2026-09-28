// What a person sees of a plan that is running, through `App` and nothing else.
//
// **Nine claims that were parked and are not any more.** They were made against
// a second board under the Workflow canvas until the owner took the groups off
// it (28 Sep 2026), and `#1619` returned them to `test.todo` because a claim
// with no surface is a todo. He decided the same day that the board's reading
// moves under the Plan tab's List view, and every sentence below is the one it
// was parked with, asserted there.
//
// **A sibling of `arc.test.tsx` for length**, on `arc-record.test.tsx`'s own
// precedent: that file is at its ceiling, and nine bodies would take it over.

import { expect, test, describe } from "vitest";
import { page } from "vitest/browser";

import { GUIDE_GROUP_ORDER } from "@armada/components";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * A moment, with the plan open as a list. **The tab is pressed the way a person
 * presses it**, and so is the view — a test that reached the list any other way
 * would pass over a toggle that had stopped working.
 */
async function planList(moment: string) {
  mount(moment);
  await expect.element(page.getByRole("tab", { name: /^Plan/ })).toBeVisible();
  await page.getByRole("tab", { name: /^Plan/ }).click();
  await page.getByRole("tab", { name: "List" }).click();
  await expect
    .element(page.getByRole("list", { name: "Groups, in the order they run" }))
    .toBeVisible();
}

/** One group's card, by the heading it carries. Case-sensitive: the clash warning names the same groups in lower case. */
const groupCard = (ordinal: number) =>
  page
    .getByRole("list", { name: "Groups, in the order they run" })
    .getByRole("listitem")
    .filter({ hasText: new RegExp(`Group ${ordinal}`) })
    .first();

/** One task's row, by the name the board gives it. */
const taskRow = (id: string) => page.getByRole("listitem", { name: new RegExp(`^${id} `) });

/** The Checks region inside one group's card — the boundary's own reading. */
const boundaryOf = (ordinal: number) =>
  groupCard(ordinal).getByRole("region", { name: "Checks at this boundary" });

describe("implement", () => {
  test(
    "arc/executing-sequential: groups one and two read passed with the commit each left, " +
      "group three is working, and group four has not started",
    async () => {
      await planList("arc/executing-sequential");

      await expect.element(groupCard(1)).toHaveTextContent("passed");
      await expect.element(groupCard(1)).toHaveTextContent("4c1b9d2");
      await expect.element(groupCard(2)).toHaveTextContent("passed");
      await expect.element(groupCard(2)).toHaveTextContent("7a2f0c5");
      await expect.element(groupCard(3)).toHaveTextContent("working");
      await expect.element(groupCard(4)).toHaveTextContent("not started");
    },
  );

  test(
    "arc/executing-sequential: the working task shows its turns and no cost, because its " +
      "agent has not stopped",
    async () => {
      await planList("arc/executing-sequential");

      await expect.element(taskRow("T5")).toHaveTextContent("14 turns");
      await expect.element(taskRow("T5")).not.toHaveTextContent("$");
    },
  );

  test(
    "arc/executing-sequential: every finished task shows what it cost, including the ones in " +
      "a group that has already passed",
    async () => {
      // Nothing is opened. The plan's List view draws every group whole — a fold
      // was the second board's, where eight tasks and four boundaries at once
      // were the wall of rows it replaced, and the plan here is what a person
      // came to read.
      await planList("arc/executing-sequential");

      await expect.element(taskRow("T1")).toHaveTextContent("34 turns · ~$2.40");
      await expect.element(taskRow("T2")).toHaveTextContent("12 turns · ~$0.64");
      await expect.element(taskRow("T3")).toHaveTextContent("8 turns · ~$0.26");
      await expect.element(taskRow("T4")).toHaveTextContent("9 turns · ~$0.31");
    },
  );

  test(
    "arc/executing-concurrent: T5 and T6 are drawn as having run at the same time, each with " +
      "its own agent, and group three reads joining",
    async () => {
      await planList("arc/executing-concurrent");

      await expect.element(groupCard(3)).toHaveTextContent("joining its work");
      await expect.element(groupCard(3)).toHaveTextContent("2 tasks, at the same time");
      // The Job's cap rode on that line until 28 Sep, and read as one number twice.
      await expect.element(groupCard(3)).not.toHaveTextContent("Drones at once");
      await expect.element(taskRow("T5")).toHaveTextContent("its own agent");
      await expect.element(taskRow("T5")).toHaveTextContent("runs beside T6");
      await expect.element(taskRow("T6")).toHaveTextContent("its own agent");
      await expect.element(taskRow("T6")).toHaveTextContent("runs beside T5");
    },
  );

  test(
    "arc/executing-concurrent: both tasks show a cost the moment their own agent stopped, " +
      "before their group has been checked",
    async () => {
      await planList("arc/executing-concurrent");

      await expect.element(taskRow("T5")).toHaveTextContent("27 turns · ~$1.90");
      await expect.element(taskRow("T6")).toHaveTextContent("15 turns · ~$0.72");
      // The boundary has not run, and the two costs are on screen anyway.
      await expect.element(boundaryOf(3)).toHaveTextContent("7 checks will run at this boundary");
      await expect.element(boundaryOf(3)).toHaveTextContent("not run");
    },
  );

  test(
    "arc/group-failed: group three reads failed with the one Check that failed named, and " +
      "says this is its second run",
    async () => {
      await planList("arc/group-failed");

      await expect.element(groupCard(3)).toHaveTextContent("failed at its checks");
      await expect.element(boundaryOf(3)).toHaveTextContent("screens_test failed");
      await expect.element(boundaryOf(3)).toHaveTextContent("second run");
      // **What the next Drone is given is the Check's own output, not a summary**
      // — the reading the owner kept this board for, and the only place in the
      // app it is drawn.
      await expect.element(boundaryOf(3)).toHaveTextContent("What the next Drone is told");
      await expect.element(boundaryOf(3)).toHaveTextContent("1 of 1384 failed");
      // The ordering rule came off this boundary on 28 Sep — guide 4's now, and
      // true of a step that never ran.
      await expect.element(boundaryOf(3)).not.toHaveTextContent("No task of group 4 starts");
      // And that is where the `?` hangs: over the list named for the order.
      await expect
        .element(
          page.getByRole("button", {
            name: `Open guide ${GUIDE_GROUP_ORDER.number}, ${GUIDE_GROUP_ORDER.title}`,
          }),
        )
        .toBeVisible();
    },
  );

  test(
    "arc/group-failed: the six Checks that passed are drawn beside the one that did not, " +
      "rather than the group reading red with nothing said",
    async () => {
      await planList("arc/group-failed");

      const checks = boundaryOf(3).getByRole("listitem");
      // Every Check the boundary declares has a row, and exactly one is red.
      await expect.poll(() => checks.all().length).toBe(7);
      const reads = () => checks.all().map((one) => one.element().getAttribute("data-reads"));
      expect(reads().filter((one) => one === "failed")).toHaveLength(1);
      expect(reads().filter((one) => one === "passed")).toHaveLength(6);
      await expect.element(checks.filter({ hasText: "typecheck" }).first()).toHaveTextContent("passed");
    },
  );

  test(
    "arc/done-touched: T6 still reads done and carries a flag saying a later task edited the " +
      "file it had finished, and T7 is named as the task that did",
    async () => {
      await planList("arc/done-touched");

      await expect.element(taskRow("T6")).toHaveTextContent("touched later · T7");
      await expect.element(taskRow("T6").getByText("Done")).toBeInTheDocument();
      await expect.element(groupCard(3)).toHaveTextContent("passed");
      // And the task that did it is still working, with turns and no cost.
      await expect.element(taskRow("T7")).toHaveTextContent("6 turns");
      await expect.element(taskRow("T7")).not.toHaveTextContent("$");
    },
  );

  test(
    "arc/executing-sequential: a task opens into what its Drone was told, what it may touch, " +
      "what it runs beside, and a redirect addressed to that task's own Drone",
    async () => {
      await planList("arc/executing-sequential");
      await taskRow("T5").getByRole("button").first().click();

      const sheet = page.getByRole("dialog").first();
      await expect.element(sheet).toBeVisible();
      await expect.element(sheet).toHaveTextContent("T5");
      // Where its own agent has got to, which is turns while it is still working.
      await expect.element(sheet).toHaveTextContent("Its agent is working");
      await expect.element(sheet).toHaveTextContent("14 turns");
      // What it was told, what it may touch, and what it runs beside.
      await expect.element(sheet).toHaveTextContent("The panel lists Drones");
      await expect.element(sheet).toHaveTextContent("Running.tsx");
      await expect.element(sheet).toHaveTextContent("T6");
      // **The redirect, addressed to this task's own Drone** — the second
      // reading the owner kept the board for, and the only way to do it
      // anywhere in the app.
      const redirect = sheet.getByRole("region", { name: "Redirect" });
      await expect.element(redirect).toHaveTextContent("Reaches Drone on T5");
      await expect.element(redirect.getByRole("textbox").first()).toBeVisible();
    },
  );
});
