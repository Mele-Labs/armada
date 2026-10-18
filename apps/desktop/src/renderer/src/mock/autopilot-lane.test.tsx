// Autopilot's lane on the Cockpit, through `App` on the `dashboard-cockpit` mock Fleet with its own
// repository picked and the mock's Autopilot mid-run: the switch in the deck's foot that opens its lane on the
// stage, Pause and Stop, the start form, its Next up and Set aside sidings, and the one bar's second kind. #2092.
// Then #2105's: the second repository's paused by its Session cap and raised, All with none picked asking for
// one. The Cockpit is the flight deck (#2301).

import { beforeEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { repository } from "@armada/screens/src/fixtures/repository";
import type { Session } from "@armada/screens/src/draft/sessions";

import { forgetDismissals } from "../cockpit/dismissed";
import type { Scenario } from "./moment";
import { scenarioNamed } from "./scenario";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();
beforeEach(forgetDismissals);

/** The Session the mock Autopilot's second started pick opened, which has not run yet. */
const PICKED_SESSION = "01JAUTOPILOTSESSION000000002";

/**
 * The Cockpit on its own repository, where the mock's Autopilot runs. Its second pick's Session is
 * added to the window's Sessions, idle and never run, as a pick not yet started.
 */
function onTheRepository(): Scenario {
  const base = scenarioNamed("dashboard-cockpit")!;
  const picked: Session = { id: PICKED_SESSION, title: "Checkout total rounds the tax twice", turn: { state: "idle" }, rows: [], attachments: [] };
  const sessions = base.draft!.sessions!;
  return {
    ...base,
    name: "dashboard-cockpit-autopilot",
    state: { ...base.state, repository: repository().root },
    draft: {
      ...base.draft,
      sessions: (control) => {
        const store = sessions(control);
        let from: readonly Session[] | undefined;
        let held: readonly Session[] = [];
        // Stable between changes, as a store's snapshot must be.
        return { ...store, get: () => {
          const now = store.get();
          if (now !== from) {
            from = now;
            held = [...now, picked];
          }
          return held;
        } };
      },
    },
  };
}

/** An issue's number as Bridge writes it, spelled so it never reads as a colour. */
const issue = (n: number) => `#${n}`;

/** The lane on the stage, and the bar inside it. The foot's own card is a region named Autopilot too, so the bar is found in the lane. */
const lane = () => page.getByRole("region", { name: "Autopilot lane" });
const bar = () => lane().getByRole("region", { name: "Autopilot", exact: true });

/** The switch in the deck's foot, which opens the lane on the stage and puts it away. */
const foot = () => page.getByRole("region", { name: "Autopilot", exact: true }).filter({ has: page.getByRole("progressbar") });
// The switch's input is hidden under its label, so the label is what is pressed.
const control = () => foot().getByText("Autopilot", { exact: true });

/** How the foot's card says Autopilot stands: its bar's name, which stays while the lane is put away. */
const standing = () => page.getByRole("progressbar", { name: /^Autopilot, / });

/** The switch, pressed where a person presses it: on its label, since the native box is clipped. */
const flip = () => bar().getByText("Autopilot", { exact: true }).click();

/** The lane is put away until the switch is pressed, so this opens it. */
async function cockpit(scenario: Scenario = onTheRepository()): Promise<void> {
  localStorage.removeItem("armada.bridge.dashboard-tab");
  mount(scenario);
  await onScreen();
  expect(bar().query()).toBeNull();
  await control().click();
  await expect.element(bar().getByRole("status")).toBeVisible();
}

test("the bar shows its caps used of their limit, and pauses and resumes", async () => {
  await cockpit();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
  await expect.element(bar()).toHaveTextContent("work through the open issues that don't need design decisions");
  await expect.element(bar().getByText("1 of 2 running")).toBeVisible();
  await expect.element(bar().getByText("$4.12 of $30")).toBeVisible();
  await expect.element(bar().getByText("2 started · no cap")).toBeVisible();
  await expect.element(bar().getByRole("button", { name: "Pause" })).toBeVisible();
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
  await bar().getByRole("textbox", { name: "Sessions" }).fill("4");
  await bar().getByRole("button", { name: "Start Autopilot" }).click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
  await expect.element(bar().getByText("0 of 4 started")).toBeVisible();
});

test("a pick it set aside is in the lane's Set aside, and Dismiss takes it off", async () => {
  await cockpit();
  const aside = lane().getByRole("region", { name: "Set aside" });
  await expect.element(aside).toHaveTextContent(`${issue(377)}`);
  await aside.getByRole("button", { name: "Dismiss" }).first().click();
  await expect.element(aside).not.toHaveTextContent(`${issue(377)}`);
});

test("the lane draws Next up as a numbered list and the set-aside siding", async () => {
  await cockpit();
  await expect.element(lane().getByText("Next up", { exact: false }).first()).toBeVisible();

  // Next up is a numbered list, and one it passes over says why on the right.
  const next = lane().getByRole("region", { name: "Next up" });
  const rows = [...next.element().querySelectorAll("li")].map((li) => li.textContent);
  expect(rows).toEqual(["1#405Coupon field accepts an expired code", "2#409Order history pages past the last order", "–#398Checkout button stays disabled after a failed cardhas a pull request"]);

  const aside = lane().getByRole("region", { name: "Set aside" });
  await expect.element(aside).toHaveTextContent("It asks how the Board should look, which is a design decision.");
  await aside.getByRole("button", { name: "Run it" }).click();
  await expect.element(aside).toHaveTextContent("Nothing set aside.");
});

test("the Dispatch sheet's second kind is Autopilot: Tab steps to it, and the sheet reads the words and starts it", async () => {
  await cockpit();
  // The switch just pressed holds focus, and a bare key does nothing while a control does.
  (document.activeElement as HTMLElement).blur();
  await userEvent.keyboard("n");
  const panel = page.getByRole("region", { name: "Dispatch", exact: true });
  const field = panel.getByRole("textbox", { name: "Request" });
  await expect.element(field).toHaveFocus();
  await userEvent.keyboard("Work the open bugs until 7am");
  for (let tries = 0; tries < 3; tries += 1) {
    if ((await panel.getByRole("tab", { name: "Autopilot" }).element().getAttribute("aria-selected")) === "true") break;
    await userEvent.keyboard("{Tab}");
  }
  await expect.element(panel.getByRole("tab", { name: "Autopilot" })).toHaveAttribute("aria-selected", "true");
  // Its act reads the words as settings, and nothing starts until the next press.
  await panel.getByRole("button", { name: "Read it" }).click();
  await expect.element(panel.getByRole("textbox", { name: "At once" })).toBeVisible();
  await panel.getByRole("button", { name: "Start Autopilot" }).click();
  await expect.element(panel).not.toBeInTheDocument();
});

test("in a short window the Dispatch sheet stays inside it and its Autopilot form scrolls to the start act", async () => {
  await page.viewport(1100, 460);
  await cockpit();
  (document.activeElement as HTMLElement).blur();
  await userEvent.keyboard("n");
  const panel = page.getByRole("region", { name: "Dispatch", exact: true });
  await userEvent.keyboard("Work the open bugs until 7am");
  for (let tries = 0; tries < 3; tries += 1) {
    if ((await panel.getByRole("tab", { name: "Autopilot" }).element().getAttribute("aria-selected")) === "true") break;
    await userEvent.keyboard("{Tab}");
  }
  await panel.getByRole("button", { name: "Read it" }).click();
  const start = panel.getByRole("button", { name: "Start Autopilot" });
  await expect.element(start).toBeInTheDocument();
  const frame = panel.element().closest<HTMLElement>(".armada-dispatch-panel__frame")!;
  await expect.poll(() => frame.getBoundingClientRect().bottom).toBeLessThanOrEqual(window.innerHeight);
  start.element().scrollIntoView();
  await expect.element(start).toBeVisible();
  await start.click();
  await expect.element(panel).not.toBeInTheDocument();
});

/** The Cockpit on the second repository, whose Autopilot its Session cap paused. */
function onPocket(): Scenario {
  const base = scenarioNamed("dashboard-cockpit")!;
  return { ...base, name: "dashboard-cockpit-pocket", state: { ...base.state, repository: "/Users/user/pocket" } };
}

test("a cap paused it: the bar names the cap, Resume is refused while it holds, and raising it lets Resume through", async () => {
  await cockpit(onPocket());
  await expect.element(bar().getByRole("status")).toHaveTextContent("Paused · Job cap reached");
  await expect.element(bar().getByText("3 of 3 started")).toHaveTextContent("3 of 3 started · reached");

  await bar().getByRole("button", { name: "Resume" }).click();
  await expect.element(bar().getByRole("alert")).toHaveTextContent("Autopilot stays paused: the Session cap is reached. Raise it, then resume");
  await expect.element(bar().getByRole("status")).toHaveTextContent("Paused · Job cap reached");

  await bar().getByRole("button", { name: "Edit caps" }).click();
  await bar().getByRole("textbox", { name: "Sessions" }).fill("5");
  await bar().getByRole("button", { name: "Save caps" }).click();
  await expect.element(bar().getByText("3 of 5 started")).toBeVisible();
  expect(bar().getByRole("form", { name: "Edit caps" }).query()).toBeNull();
  await bar().getByRole("button", { name: "Resume" }).click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
});

test("on All with no repository picked the lane asks for one, and the bar opens for the one picked", async () => {
  localStorage.removeItem("armada.bridge.dashboard-tab");
  mount(scenarioNamed("dashboard-cockpit")!);
  await onScreen();
  // Nothing stands at rest, and no line says where to start one.
  expect(bar().query()).toBeNull();
  expect(page.getByText("Pick a repository to start Autopilot").query()).toBeNull();

  // No repository is picked, so the press brings up the lane with a picker in place of the bar.
  await control().click();
  await expect.element(lane()).toBeVisible();
  expect(bar().query()).toBeNull();
  await lane().getByRole("button", { name: "Repository" }).click();
  await expect.element(page.getByRole("searchbox", { name: "Find a repository" })).toBeVisible();
  await page.getByRole("menuitem", { name: /armada/ }).click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
});

test("pressing the switch again puts the lane away without stopping Autopilot", async () => {
  await cockpit();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
  await expect.element(standing()).toHaveAttribute("aria-label", "Autopilot, running");

  await control().click();
  await expect.poll(() => lane().query()).toBeNull();
  // Still on: the foot's card says so, and the bar comes back running where it was.
  await expect.element(standing()).toHaveAttribute("aria-label", "Autopilot, running");
  await control().click();
  await expect.element(bar().getByRole("status")).toHaveTextContent("Running");
  await expect.element(bar().getByText("1 of 2 running")).toBeVisible();
});
