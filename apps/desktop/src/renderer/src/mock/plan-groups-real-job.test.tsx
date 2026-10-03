// The Plan tab's groups on a Job shaped the way a real Fleet serves it.
//
// **Job 2 drew four groups running while it was still on "Plan the change"**
// (owner, 1 Oct 2026), and each group's Checks listed `plan_recorded · not
// run`. Fleet serves no groups, so each task stands in for one — and each took
// the state and the Checks of the step the Job was on, which was the step
// writing the tasks. The mock had shown it as a Job mid-implement, so neither
// was ever seen before a real Job reached it.

import { beforeEach, describe, expect, test } from "vitest";
import { page } from "vitest/browser";

import { featureOnItsPlan } from "./job-detail-fixtures";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

beforeEach(() => window.localStorage.removeItem("armada.bridge.plan-view"));

const ORDINALS = [1, 2, 3, 4];

/** The graph's group card, which carries its state word in its name. */
const node = (ordinal: number) => page.getByRole("button", { name: new RegExp(`^Group ${ordinal}, `) }).last();

/** The list's group, whose head draws the same state as a mark. */
const row = (ordinal: number) =>
  page.getByRole("list", { name: "Groups, in the order they run" }).last().getByRole("listitem", { name: `Group ${ordinal}`, exact: true });

/** Which groups read running, on the graph and then on the list. */
async function runningGroups(): Promise<{ graph: number[]; list: number[] }> {
  await page.getByRole("tab", { name: "Graph" }).last().click();
  for (const ordinal of ORDINALS) await expect.element(node(ordinal)).toBeVisible();
  const graph = ORDINALS.filter((ordinal) => node(ordinal).element().getAttribute("aria-label")?.endsWith(", running"));
  await page.getByRole("tab", { name: "List" }).last().click();
  for (const ordinal of ORDINALS) await expect.element(row(ordinal)).toBeVisible();
  const list = ORDINALS.filter((ordinal) => row(ordinal).getByRole("img", { name: "Running", exact: true }).query() !== null);
  return { graph, list };
}

describe("Job 2, as Fleet served it on its plan step", () => {
  test("no group reads running while the Job is still writing the plan", async () => {
    await page.viewport(2000, 900);
    mount(onJob(featureOnItsPlan()));
    await page.getByRole("tab", { name: /^Plan/ }).last().click();
    expect(await runningGroups()).toEqual({ graph: [], list: [] });
  });

  test("a task under way runs its own group and no other", async () => {
    await page.viewport(2000, 900);
    mount(onJob(featureOnItsPlan({ T1: "working" })));
    await page.getByRole("tab", { name: /^Plan/ }).last().click();
    expect(await runningGroups()).toEqual({ graph: [1], list: [1] });
  });

  test("a group's Checks are the ones the working step runs on its task's paths, never the plan step's", async () => {
    await page.viewport(2000, 900);
    mount(onJob(featureOnItsPlan()));
    await page.getByRole("tab", { name: /^Plan/ }).last().click();
    await page.getByRole("tab", { name: "Graph" }).last().click();
    await node(3).click();
    const sheet = page.getByRole("dialog", { name: "Group 3" }).last();
    // The Checks bar is folded until pressed.
    await sheet.getByRole("button", { name: /^Checks/ }).click();
    const checks = sheet.getByRole("region", { name: "Checks at this boundary" });
    await expect.element(checks).toBeVisible();
    const names = checks
      .getByRole("listitem")
      .elements()
      .map((one) => one.textContent ?? "");
    // Each row is the Check's name and then what it reads, with no space between.
    expect(names).toEqual(["buildnot run", "testnot run", "formatnot run"]);
  });
});
