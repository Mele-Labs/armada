// Autopilot's lane on the Cockpit, through `App` on the `dashboard-cockpit` mock Fleet with its own
// repository picked and the mock's Autopilot mid-run: the bar on every filter, Pause and Stop, the
// start form, each set-aside pick in Your move, its marks and sidings on the Bays metro, and the one
// bar's third kind. #2092.

import { beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { repository } from "@armada/screens/src/fixtures/build/base";

import { forgetDismissals } from "../cockpit/dismissed";
import type { Scenario } from "./moment";
import { scenarioNamed } from "./scenario";
import { mount, onScreen, unmountAfterEach } from "./testing";

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

const bar = () => page.getByRole("region", { name: "Autopilot" });

/** The switch, pressed where a person presses it: on its label, since the native box is clipped. */
const flip = () => bar().getByText("Autopilot", { exact: true }).click();

async function cockpit(): Promise<void> {
  localStorage.removeItem("armada.bridge.dashboard-tab");
  mount(onTheRepository());
  await onScreen();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
}

test("the bar is on every filter, with its caps used of their limit, and pauses and resumes", async () => {
  await cockpit();
  await expect.element(bar()).toHaveTextContent("work through the open issues that don't need design decisions");
  await expect.element(bar().getByText("1 of 2 running")).toBeVisible();
  await expect.element(bar().getByText("$4.12 of $30")).toBeVisible();
  await expect.element(bar().getByText("2 started · no cap")).toBeVisible();
  for (const name of ["Active", "Done", "Bays", "Your move"]) {
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
  await flip();
  const asked = bar().getByRole("group", { name: "Stop Autopilot" });
  await expect.element(asked).toHaveTextContent("Stop ends Autopilot. #401, picked and not started, is withdrawn.");
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

test("a pick it set aside is in Your move, and Dismiss takes it off", async () => {
  await cockpit();
  const tile = page.getByRole("option", { name: /#377 Should the Board group Jobs by repository\?/ });
  await expect.element(tile).toBeVisible();
  await tile.click();
  await page.getByRole("button", { name: "Dismiss" }).click();
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

test("the one bar's third kind is Autopilot: Tab steps to it, and the words open the start form read", async () => {
  await cockpit();
  await page.getByRole("tab", { name: "Active" }).click();
  await userEvent.keyboard("n");
  await userEvent.keyboard("Work the open bugs until 7am");
  const row = () => document.getElementById("armada-palette-row-dispatch")?.textContent ?? "";
  for (let tries = 0; tries < 3 && !row().startsWith("Start Autopilot"); tries += 1) await userEvent.keyboard("{Tab}");
  expect(row()).toBe("Start Autopilot with “Work the open bugs until 7am”⌘↵");
  // Off first, since this one is running and only one runs per repository.
  await userEvent.keyboard("{Meta>}{Enter}{/Meta}");
  await expect.element(bar().getByRole("alert")).toHaveTextContent("Autopilot is already on for this repository.");
  await expect.element(bar().getByRole("textbox", { name: /Tell Autopilot what to work on/ })).not.toBeInTheDocument();
});
