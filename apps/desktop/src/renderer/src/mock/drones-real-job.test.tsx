// The Drones tab on a Job shaped the way a real Fleet serves it.
//
// **Job 2's Drones tab was empty while its Drone wrote the plan** (owner, 1 Oct
// 2026). The tab listed only Drones a task names, and a Drone on a planning
// step works no task: Fleet names it on the Job's row, `assigned_drone`, and
// in `GET /drones` on the step. The mock's Jobs all had a Drone on a task, so
// the empty tab was never seen before a real Job reached it.

import { describe, expect, test } from "vitest";
import { page } from "vitest/browser";

import { featureOnItsPlan } from "./job-detail-fixtures";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** Each body row of the Drones table, as the text of its cells. */
function rows(): string[][] {
  const table = page.getByRole("region", { name: "Drones on this Job" }).last().element();
  return [...table.querySelectorAll("tbody tr")].map((row) =>
    [...row.querySelectorAll("td")].map((cell) => cell.textContent ?? ""),
  );
}

describe("the Drones of Job 2, as Fleet served them on its plan step", () => {
  test("the tab lists the Job's one Drone, running on the plan step", async () => {
    mount(onJob(featureOnItsPlan()));
    await page.getByRole("tab", { name: /^Drones/ }).last().click();
    await expect.element(page.getByRole("button", { name: "This Job's Drone" }).last()).toBeVisible();
    const [only, ...more] = rows();
    expect(more).toEqual([]);
    const [drone, where, state, , ranFor] = only ?? [];
    expect([drone, where]).toEqual(["This Job's Drone", "Plan the change"]);
    // A mark, named Running to a screen reader and on hover — never a word drawn.
    expect(state).toContain("Running");
    // Timed from the step's live run, which is when Fleet spawned it.
    expect(ranFor).not.toBe("");
  });

  // The list is Fleet's, `list_job_drones`, which names no task — so a task
  // naming the Drone adds no row of its own.
  test("a task that names the same Drone does not list it twice", async () => {
    mount(onJob(featureOnItsPlan({ T1: "working" })));
    await page.getByRole("tab", { name: /^Drones/ }).last().click();
    await expect.element(page.getByRole("button", { name: "This Job's Drone" }).last()).toBeVisible();
    expect(rows()).toHaveLength(1);
  });
});
