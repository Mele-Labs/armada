// Autopilot's lane on the Cockpit, through `App` on the `dashboard-cockpit` mock Fleet with its own
// repository picked and the mock's Autopilot mid-run: the header control that opens its bar at the top,
// the bar on every filter, Pause and Stop, the start form, each set-aside pick in Active, its marks
// and sidings on the Bays metro, and the one bar's third kind. #2092. Then #2105's: the second
// repository's paused by its Job cap and raised, All with none picked asking for one, and a step the
// Judges sent back.

import { beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { forgetDismissals } from "../cockpit/dismissed";
import type { Scenario } from "./moment";
import { scenarioNamed } from "./scenario";
import { mount, onScreen, rememberingTheGrid, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(forgetDismissals);

/** The mock Autopilot's second started pick, whose Job has not had a Drone yet. */
const PICKED_JOB = "01JAUTOPILOTJOB00000000002";

/**
 * The Cockpit on its own repository, where the mock's Autopilot runs. Two of its Jobs are made
 * Autopilot's: the one in bay 01 by origin, and the one waiting for a bay as a pick not yet started.
 */
function onTheRepository(): Scenario {
  const base = scenarioNamed("dashboard-cockpit")!;
  const jobs = base.state.jobs.map((job) => {
    if (job.title === "Debounce the Job Board's resize handler") return { ...job, origin: "autopilot_picked" };
    if (job.title !== "Store a pause marker on the Job") return job;
    const { started_at: _never, ...rest } = { ...job, id: PICKED_JOB, origin: "autopilot_picked" };
    return rest;
  });
  return { ...base, name: "dashboard-cockpit-autopilot", state: { ...base.state, jobs, repository: repository().root } };
}

/** An issue's number as Bridge writes it, spelled so it never reads as a colour. */
const issue = (n: number) => `#${n}`;

const bar = () => page.getByRole("region", { name: "Autopilot" });

/** The control in the header band, which opens the bar at the top of the Cockpit and puts it away. */
const control = () => page.getByRole("button", { name: "Autopilot", exact: true });

/** The switch, pressed where a person presses it: on its label, since the native box is clipped. */
const flip = () => bar().getByText("Autopilot", { exact: true }).click();

/** The bar is put away until the control is pressed, so this opens it. */
async function cockpit(scenario: Scenario = onTheRepository()): Promise<void> {
  localStorage.removeItem("armada.bridge.dashboard-tab");
  mount(scenario);
  await onScreen();
  expect(bar().query()).toBeNull();
  await control().click();
  await expect.element(bar().getByRole("status")).toBeVisible();
}

test("the bar is on every filter, with its caps used of their limit, and pauses and resumes", async () => {
  await cockpit();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
  await expect.element(bar()).toHaveTextContent("work through the open issues that don't need design decisions");
  await expect.element(bar().getByText("1 of 2 running")).toBeVisible();
  await expect.element(bar().getByText("$4.12 of $30")).toBeVisible();
  await expect.element(bar().getByText("2 started · no cap")).toBeVisible();
  for (const name of ["Active", "Done", "Bays"]) {
    await page.getByRole("tab", { name }).click();
    await expect.element(bar().getByRole("button", { name: "Pause" })).toBeVisible();
  }
  await bar().getByRole("button", { name: "Pause" }).click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Paused · started Jobs finish");
  await bar().getByRole("button", { name: "Resume" }).click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
});

test("turning it off asks in the bar first, and turning it on reads a sentence before anything starts", async () => {
  await cockpit();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
  await flip();
  const asked = bar().getByRole("group", { name: "Stop Autopilot" });
  await expect.element(asked).toHaveTextContent(`Stop ends Autopilot. ${issue(401)}, picked and not started, is withdrawn.`);
  expect(page.getByRole("dialog").query()).toBeNull();
  await asked.getByRole("button", { name: "Stop Autopilot" }).click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Off");

  await flip();
  await bar().getByRole("textbox", { name: /Tell Autopilot what to work on/ }).fill("Fix the open bugs, two at a time, until 7am, $30 max.");
  expect(bar().getByRole("button", { name: "Start Autopilot" }).query()).toBeNull();
  await bar().getByRole("button", { name: "Read it" }).click();
  await expect.element(bar().getByRole("textbox", { name: "At once" })).toHaveValue("2");
  await expect.element(bar().getByRole("textbox", { name: "Spend, in dollars" })).toHaveValue("30");
  await expect.element(bar().getByText("not said")).toBeVisible();
  await bar().getByRole("textbox", { name: "Jobs" }).fill("4");
  await bar().getByRole("button", { name: "Start Autopilot" }).click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
  await expect.element(bar().getByText("0 of 4 started")).toBeVisible();
});

test("a pick it set aside is in Active, and Dismiss takes it off", async () => {
  rememberingTheGrid();
  await cockpit();
  const tile = page.getByRole("option", { name: `${issue(377)} Should the Board group Jobs by repository?` });
  await expect.element(tile).toBeVisible();
  await tile.click();
  await page.getByLabelText(`${issue(377)} Should the Board group Jobs by repository?`, { exact: true }).getByRole("button", { name: "Dismiss" }).click();
  await expect.poll(() => tile.query()).toBeNull();
});

test("on Bays: a Next up station before Waiting, its lines marked, its numbered list and the set-aside siding", async () => {
  await cockpit();
  await page.getByRole("tab", { name: "Bays" }).click();
  await expect.element(page.getByText("Next up", { exact: true })).toBeVisible();
  // A pick not started has left Waiting for a dashed stop at Next up.
  const picked = page.getByRole("list", { name: "Picked by Autopilot, not started" });
  await expect.element(picked.getByRole("button", { name: /Store a pause marker on the Job: Next up, picked/ })).toBeVisible();
  expect(page.getByRole("list", { name: "Waiting for a bay" }).query()).toBeNull();
  await expect.element(page.getByRole("button", { name: /^Bay 01: Debounce the Job Board/ }).getByLabelText("Picked by Autopilot")).toBeVisible();

  // Next up is a numbered list, and one it passes over says why on the right.
  const next = page.getByRole("region", { name: "Next up" });
  const rows = [...next.element().querySelectorAll("li")].map((li) => li.textContent);
  expect(rows).toEqual(["1#405Coupon field accepts an expired code", "2#409Order history pages past the last order", "–#398Checkout button stays disabled after a failed cardhas a pull request"]);

  const aside = page.getByRole("region", { name: "Set aside" });
  await expect.element(aside).toHaveTextContent("It asks how the Board should look, which is a design decision.");
  await aside.getByRole("button", { name: "Run it" }).click();
  await expect.element(aside).toHaveTextContent("Nothing set aside.");
});

test("the one bar's third kind is Autopilot: Tab steps to it, and the panel reads the words and starts it", async () => {
  await cockpit();
  await page.getByRole("tab", { name: "Active" }).click();
  await userEvent.keyboard("n");
  await userEvent.keyboard("Work the open bugs until 7am");
  const row = () => document.getElementById("armada-palette-row-dispatch")?.textContent ?? "";
  for (let tries = 0; tries < 3 && !row().startsWith("Start Autopilot"); tries += 1) await userEvent.keyboard("{Tab}");
  expect(row()).toBe("Start Autopilot with “Work the open bugs until 7am”⌘↵");
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  // The panel grows from the bar. Its act reads the words as settings, and nothing starts until the next press.
  const panel = page.getByRole("dialog", { name: "Dispatch" });
  await panel.getByRole("button", { name: "Read it" }).click();
  await expect.element(panel.getByRole("textbox", { name: "At once" })).toBeVisible();
  await panel.getByRole("button", { name: "Start Autopilot" }).click();
  await expect.element(panel).not.toBeInTheDocument();
});

/** The Cockpit on the second repository, whose Autopilot its Job cap paused. */
function onPocket(): Scenario {
  const base = scenarioNamed("dashboard-cockpit")!;
  return { ...base, name: "dashboard-cockpit-pocket", state: { ...base.state, repository: "/Users/user/pocket" } };
}

test("a cap paused it: the bar names the cap, Resume is refused while it holds, and raising it lets Resume through", async () => {
  await cockpit(onPocket());
  await expect.element(bar().getByRole("status")).toHaveTextContent("Paused · Job cap reached");
  await expect.element(bar().getByText("3 of 3 started")).toHaveTextContent("3 of 3 started · reached");

  await bar().getByRole("button", { name: "Resume" }).click();
  await expect.element(bar().getByRole("alert")).toHaveTextContent("Autopilot stays paused: the Job cap is reached. Raise it, then resume");
  await expect.element(bar().getByRole("status")).toHaveTextContent("Paused · Job cap reached");

  await bar().getByRole("button", { name: "Edit caps" }).click();
  await bar().getByRole("textbox", { name: "Jobs" }).fill("5");
  await bar().getByRole("button", { name: "Save caps" }).click();
  await expect.element(bar().getByText("3 of 5 started")).toBeVisible();
  expect(bar().getByRole("form", { name: "Edit caps" }).query()).toBeNull();
  await bar().getByRole("button", { name: "Resume" }).click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
});

test("on All with no repository picked the control asks for one, and the bar opens for the one picked", async () => {
  localStorage.removeItem("armada.bridge.dashboard-tab");
  mount(scenarioNamed("dashboard-cockpit")!);
  await onScreen();
  // Nothing stands at rest, and no line says where to start one.
  expect(bar().query()).toBeNull();
  expect(page.getByText("Pick a repository to start Autopilot").query()).toBeNull();
  expect(page.getByRole("switch", { name: "Autopilot" }).query()).toBeNull();

  // No repository is picked, so the press asks for one in the repository menu.
  await control().click();
  await expect.element(page.getByRole("searchbox", { name: "Find a repository" })).toBeVisible();
  expect(bar().query()).toBeNull();
  await page.getByRole("menuitem", { name: /armada/ }).click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
  // It is now a control that is pressed while the bar is open.
  await expect.element(control()).toHaveAttribute("aria-pressed", "true");
});

test("pressing the control again puts the bar away without stopping Autopilot", async () => {
  await cockpit();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
  await expect.element(control()).toHaveAttribute("aria-pressed", "true");

  await control().click();
  await expect.poll(() => bar().query()).toBeNull();
  await expect.element(control()).toHaveAttribute("aria-pressed", "false");
  // Still on: the control says so, and the bar comes back running where it was.
  await expect.element(control()).toHaveAttribute("data-autopilot", "running");
  await control().click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
  await expect.element(bar().getByText("1 of 2 running")).toBeVisible();
});

test("a step the Judges sent back to its Drone says so on its card", async () => {
  const base = scenarioNamed("dashboard-cockpit")!;
  const job = base.state.jobs.find((one) => one.title === "Debounce the Job Board's resize handler")!;
  mount({ ...base, name: "dashboard-cockpit-sent-back", opens: job.id });
  await onScreen();
  await page.getByRole("tab", { name: "Workflow" }).click();
  await expect.element(page.getByText(/Sent back by the Judges · 1 of 2/)).toBeVisible();
});
