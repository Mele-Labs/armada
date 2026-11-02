// What the Jobs list draws on each row, chosen from its context menu: a right press on a row or on the list's empty room opens a View
// group of seven checkable fields, Status and Conflicts on to begin with. Status and Conflicts are places in the card's corner, Issues and
// Active PRs chips on the line under the title, Time plain text at the end of it, and Workflow and Now lines under those. A row without
// the fact draws nothing for it, and the choice is kept in `bridge.rowView` of settings.json.
// Through `App` on the flight deck's ship, where "Migration notes" is given an issue and a workflow so that every field has a row to fill it.

import { expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import type { Session } from "@armada/screens/src/draft/sessions";

import type { Scenario } from "./moment";
import { scenarioNamed } from "./scenario";
import { mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

const FILLED = "Migration notes";

/** The ship, with one Job that has an issue and follows a workflow. Its pull request and its last words are the ship's own. */
function withDetail(): Scenario {
  const base = scenarioNamed("cockpit-ship-flightdeck")!;
  const sessions = base.draft!.sessions!;
  const detailed = (one: Session): Session =>
    one.title !== FILLED
      ? one
      : ({
          ...one,
          attachments: [...one.attachments, { kind: "issue", number: "2301", title: "The flight deck", address: "https://git.example/armada/issues/2301", state: "open" }],
          work: { ...one.work, workflow: { name: "Ship it", steps: ["Plan", "Build", "Land"], at: 1 } },
        } as unknown as Session);
  return {
    ...base,
    name: "cockpit-row-view",
    draft: {
      ...base.draft,
      sessions: (control, more) => {
        const store = sessions(control, more);
        let from: readonly Session[] | undefined;
        let held: readonly Session[] = [];
        // Stable between changes, as a store's snapshot must be.
        return { ...store, get: () => {
          const now = store.get();
          if (now !== from) {
            from = now;
            held = now.map(detailed);
          }
          return held;
        } };
      },
    },
  };
}

const list = () => page.getByRole("region", { name: "Jobs", exact: true });
const row = (name: string) => list().getByRole("button", { name, exact: true });
const rows = () => [...document.querySelectorAll<HTMLElement>(".deck-list .deck-row")];
const of = (selector: string) => [...document.querySelectorAll<HTMLElement>(`.deck-list .deck-row ${selector}`)];
/** The state's place in the corner is the wide one, and the conflicts' the narrow one beside it. */
const statusPlaces = () => of(".armada-list-card__mark[data-wide]");
const conflictPlaces = () => of(".armada-list-card__mark:not([data-wide])");
const filled = () => rows().find((one) => one.getAttribute("aria-label") === FILLED)!;
const bare = () => rows().find((one) => one.getAttribute("aria-label") === "Cache key")!;
const field = (label: string) => page.getByRole("menuitem", { name: label, exact: true });
const FIELDS = ["Active PRs", "Issues", "Now", "Time", "Conflicts", "Status", "Workflow"];
/** The fields the menu shows ticked, which it marks as current. */
const ticked = () => FIELDS.filter((label) => field(label).query()?.getAttribute("aria-current") === "true");

async function deck() {
  const app = mount(withDetail());
  await onScreen();
  await expect.element(list()).toBeVisible();
  await expect.element(row(FILLED)).toBeVisible();
  return app;
}

/** The empty room of the list, pressed on the right as a person does: the press lands on the list and on no row. */
const pressEmptyRoom = () => document.querySelector(".deck-list__rows")!.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 120, clientY: 160 }));

test("a right press on a row opens a View group of the seven fields, Status and Conflicts ticked", async () => {
  await deck();
  await row("Cache key").click({ button: "right" });
  for (const label of FIELDS) await expect.element(field(label)).toBeVisible();
  expect(ticked()).toEqual(["Conflicts", "Status"]);
  // A Job is under the press, so its own acts are in the menu beside the View group.
  await expect.element(page.getByRole("menuitem", { name: "Auto Pilot" })).toBeVisible();
});

test("a right press on the list's empty room opens the View group alone", async () => {
  await deck();
  pressEmptyRoom();
  for (const label of FIELDS) await expect.element(field(label)).toBeVisible();
  expect(ticked()).toEqual(["Conflicts", "Status"]);
  expect(page.getByRole("menuitem").elements().map((one) => one.textContent)).toEqual(FIELDS);
});

test("by default a row draws its Status and Conflicts places and nothing else", async () => {
  await deck();
  expect(statusPlaces()).toHaveLength(rows().length);
  expect(conflictPlaces()).toHaveLength(rows().length);
  for (const other of [".armada-list-card__tag", ".armada-list-card__at", ".armada-list-card__lines"]) expect(of(other), other).toHaveLength(0);
});

test("each field switched on draws its part, and switching it off takes the part away", async () => {
  await deck();
  const part = { Issues: ".armada-list-card__chip", "Active PRs": ".armada-list-card__chip", Time: ".armada-list-card__at", Workflow: ".deck-row__flow", Now: ".deck-row__says", Conflicts: ".armada-list-card__mark:not([data-wide])", Status: ".armada-list-card__mark[data-wide]" } as const;
  for (const [label, selector] of Object.entries(part)) {
    const had = of(selector).length;
    pressEmptyRoom();
    await field(label).click();
    await expect.poll(() => of(selector).length, { message: label }).not.toBe(had);
    pressEmptyRoom();
    await field(label).click();
    await expect.poll(() => of(selector).length, { message: label }).toBe(had);
  }
});

test("a row's own fact stands on its card: its issue, its workflow's step, how long since its last turn", async () => {
  await deck();
  pressEmptyRoom();
  await field("Issues").click();
  pressEmptyRoom();
  await field("Workflow").click();
  pressEmptyRoom();
  await field("Time").click();

  const text = (selector: string) => filled().querySelector(selector)?.textContent;
  await expect.poll(() => text(".armada-list-card__chip")).toBe("#2301");
  expect(text(".deck-row__flow")).toBe("Build");
  expect(text(".armada-list-card__at")).toBeTruthy();
  // A row without the fact draws nothing for it.
  expect(bare().querySelector(".armada-list-card__chip")).toBeNull();
  expect(bare().querySelector(".deck-row__flow")).toBeNull();
});

test("a row with nothing under its title draws no tag line beside one that has", async () => {
  await deck();
  pressEmptyRoom();
  await field("Issues").click();
  await expect.poll(() => filled().querySelector(".armada-list-card__tag")).not.toBeNull();
  expect(bare().querySelector(".armada-list-card__tag")).toBeNull();
  expect(filled().getBoundingClientRect().height).toBeGreaterThan(bare().getBoundingClientRect().height);
});

test("a field no row fills draws nothing", async () => {
  // The ship as it is: none of its Jobs has an issue or a workflow.
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(row("Cache key")).toBeVisible();
  for (const label of ["Issues", "Workflow"]) {
    pressEmptyRoom();
    await field(label).click();
  }
  pressEmptyRoom();
  await expect.element(field("Workflow")).toBeVisible();
  expect(ticked()).toEqual(["Issues", "Conflicts", "Status", "Workflow"]);
  expect(of(".armada-list-card__tag")).toHaveLength(0);
  expect(of(".deck-row__flow")).toHaveLength(0);
});

test("Active PRs is a chip under the name, and Now a line under that", async () => {
  await deck();
  const box = (selector: string) => filled().querySelector(selector)?.getBoundingClientRect();

  pressEmptyRoom();
  await field("Active PRs").click();
  await expect.poll(() => box(".armada-list-card__tag")).toBeDefined();
  expect(filled().querySelector(".armada-list-card__tag")?.textContent).toContain("#1892");
  expect(box(".armada-list-card__tag")!.top).toBeGreaterThanOrEqual(box(".armada-list-card__title")!.bottom);

  pressEmptyRoom();
  await field("Now").click();
  await expect.poll(() => box(".deck-row__says")).toBeDefined();
  expect(filled().querySelector(".deck-row__says")?.textContent).not.toBe("");
  expect(box(".deck-row__says")!.top).toBeGreaterThanOrEqual(box(".armada-list-card__tag")!.bottom);
});

test("the choice is saved to the bridge.rowView setting, every field named", async () => {
  const app = await deck();
  const save = vi.spyOn(app.api, "saveSettings");
  pressEmptyRoom();
  await field("Time").click();
  await expect.poll(() => save.mock.calls.length).toBe(1);
  expect(save).toHaveBeenCalledWith({ "bridge.rowView": { pr: false, issue: false, now: false, time: true, conflicts: true, status: true, workflow: false } });
});

test("the choice saved is the one drawn: a window that reads it draws those fields and ticks them in the menu", async () => {
  const app = await deck();
  await app.api.saveSettings({ "bridge.rowView": { pr: false, issue: true, now: false, time: false, conflicts: false, status: false, workflow: false } });
  await expect.poll(() => filled().querySelector(".armada-list-card__chip")?.textContent).toBe("#2301");
  expect(statusPlaces()).toHaveLength(0);
  expect(conflictPlaces()).toHaveLength(0);
  pressEmptyRoom();
  await expect.element(field("Issues")).toBeVisible();
  expect(ticked()).toEqual(["Issues"]);
});
