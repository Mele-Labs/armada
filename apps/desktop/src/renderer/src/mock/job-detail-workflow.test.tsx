// The Workflow tab through `App`: which arrangement it opens in, that the
// toggle is remembered, and what a step opens into. `#1539`.
//
// **Canvas by default, at every width** (owner, 21 and 22 Sep, #1530). The
// revised Narrow board drops the toggle; the toggle stays, so the narrow claim
// below is the one that would have been lost.

import { afterEach, beforeEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { executingSequential } from "@armada/jobs/fixtures/build/arc";
import { GUIDES, RETIRED_GUIDE_NUMBERS } from "@armada/components";

import { entered, motion, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const RESTING = { width: 1440, height: 900 };
afterEach(async () => {
  await page.viewport(RESTING.width, RESTING.height);
});

// The arrangement is remembered per viewer, so one test's press would otherwise
// be the next test's default.
beforeEach(() => window.localStorage.removeItem("armada.bridge.workflow-view"));

/**
 * The feature Job mid-implement, with its Workflow tab open.
 *
 * **Every locator is the newest window's.** The relaunch claim below mounts a
 * second `App` beside the first, which is what a relaunch is from the
 * arrangement's point of view — it is this viewer's and not this Job's.
 */
async function workflow(): Promise<void> {
  mount("arc/executing-sequential");
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
}

/** The steps the arc's Job froze, by the labels its own workflow file declares. */
const STEPS = executingSequential().fixtures[0]!.workflows[0]!.steps.map((step) => step.label);

const card = (label: string) => page.getByRole("button", { name: new RegExp(`^${label}, `) }).last();

test("the Workflow tab opens on the canvas, with every step of the frozen workflow", async () => {
  await workflow();
  await expect.element(page.getByRole("tab", { name: "Canvas", selected: true }).last()).toBeVisible();
  for (const label of STEPS) await expect.element(card(label)).toBeVisible();
});

test("the canvas is still what opens at the narrowest window Bridge lays out for", async () => {
  await page.viewport(768, 900);
  await workflow();
  await expect.element(page.getByRole("tab", { name: "Canvas", selected: true }).last()).toBeVisible();
  await expect.element(page.getByRole("tab", { name: "Stacked" }).last()).toBeVisible();
});

test("Stacked draws the same steps as a list, and the choice survives a relaunch", async () => {
  await workflow();
  await page.getByRole("tab", { name: "Stacked" }).last().click();
  const run = page.getByRole("list", { name: /as its workflow's run$/ }).last();
  await expect.element(run).toBeVisible();
  for (const label of STEPS) await expect.element(card(label)).toBeVisible();

  // A second window, as a relaunch is: the arrangement is this viewer's and
  // not this Job's, so it is what opens.
  await workflow();
  await expect.element(page.getByRole("tab", { name: "Stacked", selected: true }).first()).toBeVisible();
});

// # The step opens in the app's own panel, and nothing is open until a press
//
// The owner's notes: *nothing is open until a press* (25 Sep); *all of our
// panels open to the full height of the app. This one should be no different*
// (29 Sep); and every job-detail panel dims what is under it (30 Sep). So a
// step opens in the floating `Sheet` Record, Drones and Plan use, over the
// work area, and the canvas under it keeps its size.

/** The canvas's frame, by the box it draws in. */
function canvasBox(): DOMRect {
  const frame = document.querySelectorAll<HTMLElement>(".armada-workflow-tab__canvas");
  const last = frame[frame.length - 1];
  if (last === undefined) throw new Error("the canvas is not drawn");
  return last.getBoundingClientRect();
}

/** The step's panel, by the name its head gives it. */
const panel = (label: string) => page.getByRole("dialog", { name: label }).last();

test("nothing is open until a press, and the canvas spans the width of the tab", async () => {
  await page.viewport(2000, 900);
  await workflow();
  // The run is drawn, so the measurement below is of a laid-out canvas.
  await expect.element(card("Implement")).toBeVisible();
  expect(await page.getByRole("dialog").elements()).toHaveLength(0);
  // The canvas reaches the trailing edge of the tab it is drawn in.
  const tab = document.querySelectorAll<HTMLElement>(".armada-workflow-tab");
  const frame = tab[tab.length - 1]!.getBoundingClientRect();
  expect(canvasBox().right).toBeCloseTo(frame.right, 0);
});

test("a press opens the step in the app's own panel, over the work area, and the canvas does not reflow", async () => {
  await page.viewport(2000, 900);
  await workflow();
  await expect.element(card("Implement")).toBeVisible();
  const shut = canvasBox();

  await card("Plan the change").click();
  const opened = panel("Plan the change");
  await expect.element(opened).toBeVisible();
  // The panel every job-detail destination opens: modal, over a dimmed work area.
  await expect.element(opened).toHaveAttribute("aria-modal", "true");
  // Over the canvas, not beside it: bit-for-bit, so a tolerance cannot admit
  // a column's width. `job-detail-width.test.tsx` carries the reasoning.
  const open = canvasBox();
  expect(open.width).toBe(shut.width);
  expect(open.left).toBe(shut.left);
  // The app's full height under the title row, as Helm's dock runs.
  const box = opened.element().getBoundingClientRect();
  expect(box.bottom).toBeGreaterThan(window.innerHeight * 0.9);
});

test("Close takes the panel off, and the canvas is back with nothing open", async () => {
  await page.viewport(2000, 900);
  await workflow();
  await expect.element(card("Implement")).toBeVisible();
  const shut = canvasBox();

  await card("Plan the change").click();
  await expect.element(panel("Plan the change")).toBeVisible();
  await page.getByRole("button", { name: "Close" }).last().click();

  expect(await page.getByRole("dialog").elements()).toHaveLength(0);
  expect(canvasBox().width).toBe(shut.width);
});

test("the panel's head says where the step sits and its state, and its regions scroll under it", async () => {
  // 1512 × 817 is the window the owner's notes were written in.
  await page.viewport(1512, 817);
  await workflow();
  await card("Implement").click();
  const opened = panel("Implement");
  await expect.element(opened).toBeVisible();
  await expect.element(opened).toHaveTextContent(/Step 2/);
  await expect.element(opened).toHaveTextContent(/running/);
  // Bounded by the window, not by how much this step has to say.
  expect(opened.element().getBoundingClientRect().height).toBeLessThanOrEqual(window.innerHeight);
});

// The tests band said *Fleet does not serve the cases a boundary owes yet*
// until 28 Sep. The owner asked what was missing and how it gets fixed
// (`frpl`), so it says both, and links the issue that builds it.
test("a step opens with its plan, its Drones, the tests it lacks and why, and its Checks", async () => {
  // The stop is named for its hold, which reduced motion does not offer.
  await motion();
  await workflow();
  await card("Implement").click();
  const opened = panel("Implement");
  await expect.element(opened).toBeVisible();
  // The plan as a card that opens Plan, never its task list (`25i2`, `nm0h`, `dco5`).
  await expect.element(opened.getByRole("button", { name: "Open the plan" })).toBeVisible();
  // Every Drone that worked the step, running or not.
  await expect.element(opened.getByRole("region", { name: "Drones" })).toHaveTextContent(/Drone on T5/);
  await expect.element(opened.getByRole("region", { name: "Drones" })).toHaveTextContent(/Drone on T1/);
  // What is missing, and the issue that builds it.
  const tests = opened.getByRole("region", { name: "Tests at this step" });
  await expect.element(tests).toHaveTextContent(/COVERS/);
  await expect.element(tests.getByRole("link", { name: "See #1274." })).toHaveAttribute(
    "href",
    expect.stringMatching(/\/issues\/1274$/),
  );
  await expect.element(opened.getByRole("region", { name: "Checks at this step" })).toBeVisible();
  await expect.element(page.getByText(/does not serve the cases/)).not.toBeInTheDocument();
  // The redirect box went (`losq`): nobody knows which Drone it would reach.
  await expect.element(opened.getByRole("region", { name: "Redirect a Drone" })).not.toBeInTheDocument();
  // The stop names the step, not a drone: a step can have several running.
  await expect.element(opened.getByRole("button", { name: /Hold to stop this step/ })).toBeInTheDocument();
});

/**
 * The moment with three groups worked and one not — four groups and eight
 * tasks. `arc/group-failed` is the note's own scenario.
 */
async function failed(): Promise<void> {
  mount("arc/group-failed");
  await page.getByRole("tab", { name: /^Workflow/ }).last().click();
}

// # The canvas draws the run and nothing of the plan
//
// The owner's notes: the plan's graph went to Plan on 25 Sep, and on 29 Sep
// the one Plan node left too (`nm0h`) — the step that makes or works the plan
// says so in its own panel, with a card that opens Plan.

test("Workflow draws the steps and nothing of the plan: no Plan node, no group, no task", async () => {
  await page.viewport(2000, 900);
  await failed();
  await expect.element(card("Implement")).toBeVisible();
  expect(await page.getByRole("button", { name: /^Plan, / }).elements()).toHaveLength(0);
  expect(await page.getByLabelText(/ made Plan$/).elements()).toHaveLength(0);
  expect(await page.getByRole("button", { name: /^Group \d, / }).elements()).toHaveLength(0);
  expect(await page.getByLabelText(/^Implement worked /).elements()).toHaveLength(0);
});

test("the plan card in a step's panel lands on the Plan tab", async () => {
  await page.viewport(2000, 900);
  await failed();
  await card("Implement").click();
  await panel("Implement").getByRole("button", { name: "Open the plan" }).click();
  await expect.element(page.getByRole("tab", { name: /^Plan/, selected: true }).last()).toBeVisible();
  await expect.element(page.getByRole("tabpanel", { name: "Plan" }).last()).toBeVisible();
});

test("Stacked says what the graph says, in words", async () => {
  await failed();
  await page.getByRole("tab", { name: "Stacked" }).last().click();
  await expect.element(card("Plan the change")).toBeVisible();
  await expect.element(card("Implement")).toBeVisible();
  // No Plan row, as the canvas draws no Plan node.
  expect(await page.getByRole("button", { name: /^Plan, / }).elements()).toHaveLength(0);
  expect(await page.getByRole("button", { name: /^Group \d, / }).elements()).toHaveLength(0);
});

test("no board stands under the run, so the groups are the Plan tab's alone", async () => {
  await page.viewport(2000, 900);
  await failed();
  // The run first: an absence passes against a window that has not drawn yet,
  // so a negative stands behind a positive.
  await expect.element(card("Implement")).toBeVisible();
  expect(page.getByRole("heading", { name: "Groups", exact: true }).elements()).toHaveLength(0);
  expect(
    page.getByRole("list", { name: "The groups of this step, in the order they run" }).elements(),
  ).toHaveLength(0);
  expect(page.getByRole("listitem", { name: /^T\d /}).elements()).toHaveLength(0);
});

test("the canvas head carries no mark, because the guide that hung there is retired", async () => {
  await workflow();
  // Guide 11 explained how a group gets its second edge. The plan's graph
  // moved to the Plan tab on 25 September 2026, so it was retired rather than
  // rewritten, and 11 is a number nothing may take again.
  expect(RETIRED_GUIDE_NUMBERS).toContain(11);
  expect(GUIDES.map((guide) => guide.number)).not.toContain(11);
  expect(page.getByRole("button", { name: /^Open guide 11,/ }).elements()).toHaveLength(0);

  await page.getByRole("tab", { name: "Stacked" }).last().click();
  await expect.element(page.getByRole("list", { name: /as its workflow's run$/ }).last()).toBeVisible();
});

// # A Drone opens where Drones are read, with a way back to the step
//
// The owner's decision of 29 Sep 2026: a panel that sends you to another
// destination leaves a way back, rather than opening a second panel beside it.
// The step's Drones are presses into the Drones tab, and Back — or Close —
// returns to the step with its panel open.

test("a Drone pressed in a step's panel opens in Drones, and Back returns to the step", async () => {
  await workflow();
  await card("Implement").click();
  await entered(panel("Implement"));
  await panel("Implement").getByRole("button", { name: /Drone on T1/ }).click();

  await expect.element(page.getByRole("tab", { name: /^Drones/, selected: true }).last()).toBeVisible();
  const drone = page.getByRole("dialog", { name: "Drone on T1" }).last();
  await entered(drone);
  const back = drone.getByRole("button", { name: /Back to Implement/ });
  await expect.element(back).toBeVisible();

  await back.click();
  await expect.element(page.getByRole("tab", { name: /^Workflow/, selected: true }).last()).toBeVisible();
  await expect.element(panel("Implement")).toBeVisible();
});

test("a step reached from a Drone's panel carries the way back to that Drone", async () => {
  mount("arc/executing-sequential");
  await page.getByRole("tab", { name: /^Drones/ }).last().click();
  await page.getByRole("row", { name: /Drone on T1/ }).last().click();
  const drone = page.getByRole("dialog", { name: "Drone on T1" }).last();
  await entered(drone);
  await drone.getByRole("button", { name: "Implement", exact: true }).click();

  await expect.element(page.getByRole("tab", { name: /^Workflow/, selected: true }).last()).toBeVisible();
  const opened = panel("Implement");
  await entered(opened);
  await expect.element(opened.getByRole("button", { name: /Back to Drone on T1/ })).toBeVisible();
  // Close goes back too, after a jump: the step was reached, not opened.
  await opened.getByRole("button", { name: "Close" }).click();
  await expect.element(page.getByRole("tab", { name: /^Drones/, selected: true }).last()).toBeVisible();
});
