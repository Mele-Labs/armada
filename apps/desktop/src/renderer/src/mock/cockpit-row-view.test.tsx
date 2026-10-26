// What the Jobs list draws on each row, chosen from its context menu: a right press on a row or on the list's empty room opens a View
// group of seven checkable fields, Status and Conflicts on to begin with. Each field draws a column, Now a second line and Active PRs a
// line of its own under it, a column no row fills draws no room, and the choice is kept in `bridge.rowView` of settings.json.
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
const cells = (field: string) => [...document.querySelectorAll<HTMLElement>(`.deck-list .deck-row__cell[data-field="${field}"]`)];
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

test("by default a row draws its Status and Conflicts columns and nothing else", async () => {
  await deck();
  expect(cells("status")).toHaveLength(rows().length);
  expect(cells("conflicts")).toHaveLength(rows().length);
  for (const other of ["issue", "time", "workflow"]) expect(cells(other), other).toHaveLength(0);
  expect(document.querySelector(".deck-row__says")).toBeNull();
  expect(document.querySelector(".deck-row__prs")).toBeNull();
});

test("each field switched on draws its column on every row, and switching it off takes the column away", async () => {
  await deck();
  const column = { Issues: "issue", Time: "time", Workflow: "workflow", Conflicts: "conflicts", Status: "status" } as const;
  for (const [label, name] of Object.entries(column)) {
    const had = cells(name).length;
    pressEmptyRoom();
    await field(label).click();
    await expect.poll(() => cells(name).length, { message: label }).toBe(had === 0 ? rows().length : 0);
  }
});

test("a row's own fact stands in its column: its issue, its workflow's step, how long since its last turn", async () => {
  await deck();
  pressEmptyRoom();
  await field("Issues").click();
  pressEmptyRoom();
  await field("Workflow").click();
  pressEmptyRoom();
  await field("Time").click();

  const filled = rows().find((one) => one.getAttribute("aria-label") === FILLED)!;
  const text = (selector: string) => filled.querySelector(selector)?.textContent;
  await expect.poll(() => text(".deck-row__issue")).toBe("#2301");
  expect(text(".deck-row__flow")).toBe("Build");
  expect(text('.deck-row__cell[data-field="time"] .deck-row__said')).toBeTruthy();
  // A row without the fact leaves its place empty, and nothing moves.
  const bare = rows().find((one) => one.getAttribute("aria-label") === "Cache key")!;
  expect(bare.querySelector('.deck-row__cell[data-field="issue"]')?.textContent).toBe("");
  expect(bare.querySelector('.deck-row__cell[data-field="workflow"]')?.textContent).toBe("");
});

test("a column a row fills draws its room", async () => {
  await deck();
  pressEmptyRoom();
  await field("Issues").click();
  await expect.poll(() => cells("issue")).toHaveLength(rows().length);
  expect(cells("issue")[0]!.getBoundingClientRect().width).toBeGreaterThan(0);
});

test("a column no row fills draws no room", async () => {
  // The ship as it is: none of its Jobs has an issue or a workflow.
  mount("cockpit-ship-flightdeck");
  await onScreen();
  await expect.element(row("Cache key")).toBeVisible();
  for (const label of ["Issues", "Workflow"]) {
    pressEmptyRoom();
    await field(label).click();
  }
  await expect.poll(() => cells("workflow")).toHaveLength(rows().length);
  for (const name of ["issue", "workflow"]) {
    expect(cells(name).every((one) => one.textContent === ""), name).toBe(true);
    expect(cells(name)[0]!.getBoundingClientRect().width, name).toBe(0);
  }
});

test("Now is a second line under the name, and Active PRs a line of its own under Now", async () => {
  await deck();
  const filled = () => rows().find((one) => one.getAttribute("aria-label") === FILLED)!;
  const top = (selector: string) => filled().querySelector(selector)?.getBoundingClientRect();

  pressEmptyRoom();
  await field("Active PRs").click();
  // Alone, the pull requests' line is the second line, with the pull request's number on it.
  await expect.poll(() => top(".deck-row__prs")).toBeDefined();
  expect(filled().querySelector(".deck-row__prs")?.textContent).toContain("#1892");
  expect(top(".deck-row__prs")!.top).toBeGreaterThanOrEqual(top(".deck-row__title")!.bottom);
  const alone = top(".deck-row__prs")!.top;

  pressEmptyRoom();
  await field("Now").click();
  await expect.poll(() => top(".deck-row__says")).toBeDefined();
  expect(filled().querySelector(".deck-row__says")?.textContent).not.toBe("");
  expect(top(".deck-row__says")!.top).toBeGreaterThanOrEqual(top(".deck-row__title")!.bottom);
  // The pull requests give Now the second line and stand under it.
  expect(top(".deck-row__prs")!.top).toBeGreaterThanOrEqual(top(".deck-row__says")!.bottom);
  expect(top(".deck-row__prs")!.top).toBeGreaterThan(alone);
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
  await expect.poll(() => cells("issue")).toHaveLength(rows().length);
  expect(cells("status")).toHaveLength(0);
  expect(cells("conflicts")).toHaveLength(0);
  pressEmptyRoom();
  await expect.element(field("Issues")).toBeVisible();
  expect(ticked()).toEqual(["Issues"]);
});
