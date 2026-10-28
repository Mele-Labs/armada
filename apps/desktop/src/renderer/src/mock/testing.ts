// What a browser test needs to drive `App` on a scenario: mount it, take it
// down after, and reach a surface the way a person does — by the title bar's tabs, the System strip or ⌘K.

import { forgetAllKept } from "@armada/components";
import { closeWalkWindow } from "@armada/screens/src/fixtures/walk-window";
import { afterEach, expect, onTestFinished } from "vitest";
import type { Mock } from "vitest";
import { commands, page, userEvent } from "vitest/browser";

import { forgetViewed } from "../cockpit/viewed";
import type { FakeOptions } from "./fake";
import { mountApp } from "./mount";
import type { Mounted } from "./mount";
import type { Scenario } from "./scenario";
import type { StudioKeeping } from "@armada/studios/fake";

let mounted: { app: Mounted; host: HTMLElement }[] = [];

/** What the next mount's viewer remembers, set by a test that opens on a pick it made earlier. */
let remembered: Readonly<Record<string, string>> = {};

/** The next mount starts from these remembered picks, by storage key. Forgotten again after the test. */
export function remembering(picks: Readonly<Record<string, string>>): void {
  remembered = picks;
}

/** Registers the teardown. Call once at the top of a test file. */
export function unmountAfterEach(): void {
  afterEach(() => {
    // The mock's own window a Session showed a page in outlives the app that opened it, and a dialog left
    // in the document stops every key the window listens for in the next test.
    closeWalkWindow();
    for (const one of mounted) {
      one.app.unmount();
      one.host.remove();
    }
    mounted = [];
    remembered = {};
    forgetAllKept();
    // Having been in a Session drops its calls and reads its reply, so a test must not leave him in one for the next.
    forgetViewed();
  });
}

/** Mount `App` on a scenario, in a host the app's stylesheet sizes as its window. */
export function mount(scenario: string | Scenario, options?: FakeOptions): Mounted {
  // Each mount is a fresh viewer: the ledger's kind filter is remembered in the browser, so a test that pressed it must not leak.
  localStorage.removeItem("armada.session-ledger.kinds.artifact");
  // So is the Dashboard's filter and its grid or map: a test that flipped either must not leak.
  localStorage.removeItem("armada.bridge.dashboard-tab");
  localStorage.removeItem("armada.bridge.cockpit-view");
  localStorage.removeItem("armada.bridge.cockpit-timeline-take");
  // And the flight deck's: the switch, its view and its filters.
  for (const key of ["deck-preview", "deck-system", "deck-view", "deck-job-view", "deck-only", "deck-filters", "deck-scope", "deck-order"]) localStorage.removeItem(`armada.bridge.${key}`);
  for (const [key, value] of Object.entries(remembered)) localStorage.setItem(key, value);
  const host = document.createElement("div");
  host.id = "root";
  document.body.append(host);
  const app = mountApp(scenario, host, undefined, options);
  mounted.push({ app, host });
  return app;
}

/** Every window mounted in this test is drawn, with its effects run. */
export async function onScreen(): Promise<void> {
  await Promise.all(mounted.map((one) => one.app.onScreen));
}

/**
 * Put Helm's dock up, and wait for it. **Bridge opens with it shut** since
 * #1583: the dock draws over the content rather than beside it, so leaving it
 * standing would cover whatever the window was opened to read. `⌘J` is the
 * way in that works at every width and from inside a field.
 *
 * **The window has to be up before the press, not after it.** `⌘J` is bound in
 * an effect, and `mount` returns before React has run one — a keystroke aimed
 * at a window that is not listening yet reaches `window`, matches nothing, and
 * is gone, with no locator to have waited on and nothing to retry it. Measured
 * 23 Sep 2026 on #1592, once in 25 full runs under load.
 */
export async function openHelm(): Promise<void> {
  await onScreen();
  await userEvent.keyboard("{Meta>}j{/Meta}");
  await expect.element(page.getByRole("complementary", { name: "Helm" })).toBeVisible();
}

/**
 * Two windows on one main, side by side, each labelled as a region so a test
 * can scope to it. Both talk to one fake, so a clone either sends lands in both.
 */
export function mountTwo(scenario: Scenario, labels: [string, string], options?: FakeOptions): [HTMLElement, HTMLElement] {
  const row = document.createElement("div");
  row.style.display = "flex";
  document.body.append(row);
  let api: Mounted["api"] | undefined;
  const hosts = labels.map((label) => {
    const host = document.createElement("section");
    host.setAttribute("aria-label", label);
    host.style.flex = "1";
    host.style.minWidth = "0";
    row.append(host);
    const app = mountApp(scenario, host, api, options);
    api = app.api;
    mounted.push({ app, host });
    return host;
  });
  // The row is not a window of its own: both of its windows are already tracked above.
  mounted.push({
    app: { api: api!, scenario, onScreen: Promise.resolve(), unmount: () => undefined },
    host: row,
  });
  return hosts as [HTMLElement, HTMLElement];
}

/**
 * A sheet or dialog that has finished entering: travelled in, or scaled up, by its own animation.
 * Visible holds from its first frame, while a sheet can still sit wholly past the window's
 * trailing edge with its travel not yet started: under a loaded full run a press aimed at it then
 * reached nothing, and the test read on as if it had landed — #1252. Its own animations only,
 * never its subtree's, where a running step's pulse never finishes. One that does not animate
 * returns at once.
 */
export async function entered(layer: ReturnType<typeof page.getByRole>): Promise<void> {
  await expect.element(layer).toBeVisible();
  await Promise.all(layer.element().getAnimations().map((one) => one.finished));
}

declare module "vitest/browser" {
  interface BrowserCommands {
    /** `vitest.config.ts`' own: motion on, or back to reduced. */
    motion: (on: boolean) => Promise<void>;
  }
}

/**
 * Motion on, for the rest of the test that awaits this. **This project runs with reduced motion
 * emulated**, so a panel is in place on its first frame (`vitest.config.ts`). A test whose claim
 * is what a person without the preference sees — a hold filling, a drawing moving, a control
 * named for its hold — awaits this first, and the test's end puts the preference back.
 */
export async function motion(): Promise<void> {
  await commands.motion(true);
  onTestFinished(() => commands.motion(false));
}

/** A place in the title bar: Cockpit, Jobs, Studios, Logbook or System. Pressing the one a person is in goes up to its list. A Session is no place: `toSession` reaches it. */
export async function toPlace(name: "Cockpit" | "Jobs" | "Studios" | "Logbook" | "System"): Promise<void> {
  await userEvent.click(page.getByRole("navigation", { name: "Places" }).getByRole("tab", { name }));
}

/** What the System strip's five pages were called, and the sidebar entry each opens now. A name that is an entry of its own is passed through. */
const SIDEBAR_ENTRY: Readonly<Record<string, string>> = { Kit: "Skills", Mods: "Theme", Settings: "Limits" };

/** An entry of the System sidebar, from anywhere: System, then the entry's button. Studios is a tab of its own, one press, and Retros is the Logbook's Retros tab. */
export async function toSystemPage(name: string): Promise<void> {
  if (name === "Studios") return toPlace(name);
  if (name === "Retros") {
    await toPlace("Logbook");
    return userEvent.click(page.getByRole("tab", { name: "Retros", exact: true }));
  }
  await toPlace("System");
  const entry = SIDEBAR_ENTRY[name] ?? name;
  await userEvent.click(page.getByRole("navigation", { name: "System" }).getByRole("button", { name: new RegExp(`^${entry}( |$)`) }));
}

/** A page no tab or strip place carries, by name from ⌘K. */
export async function toPageByName(name: string): Promise<void> {
  await userEvent.keyboard("{Meta>}k{/Meta}");
  await userEvent.keyboard(`${name}{Enter}`);
}

/**
 * A Session's view, from anywhere: the Cockpit, then the Session's row in the Work list, then the stage's own
 * `Session: <title>` region. The Sessions page is gone; this is the only way in. A title given as words is the row's whole
 * name, one given as a pattern is matched inside it. The list shows Active Sessions until its Filter menu is turned to another
 * scope, which a Session that ended or went quiet is found under, and which stays turned until it is chosen again.
 */
export async function toSession(title: RegExp | string, scope?: "Active" | "Quiet" | "Ended" | "All"): Promise<ReturnType<typeof page.getByRole>> {
  await toPlace("Cockpit");
  const work = page.getByRole("region", { name: "Jobs", exact: true });
  await expect.element(work).toBeVisible();
  if (scope !== undefined) {
    await userEvent.click(work.getByRole("button", { name: /^Filter/ }));
    await userEvent.click(page.getByRole("menuitem", { name: scope, exact: true }));
  }
  // The Session's row is the one keyed `session:<id>`, named by its title.
  const row = () =>
    [...document.querySelectorAll<HTMLElement>('section[aria-label="Jobs"] [data-row^="session:"]')].find((one) => {
      const name = one.getAttribute("aria-label") ?? "";
      return typeof title === "string" ? name === title : title.test(name);
    });
  await expect.poll(() => row() !== undefined).toBe(true);
  await userEvent.click(row()!);
  const view = page.getByRole("region", { name: /^Job: / });
  await expect.element(view).toBeVisible();
  return view;
}

/**
 * Resolves when `spy` has been called `times`, at once if it already has. **An awaited call, not a
 * poll**: a poll gives up after five seconds, which a loaded run can spend before the press is
 * even handled. Call it before the act that makes the call, or after — it wraps what the spy does.
 */
export function whenCalled(spy: Mock, times = 1): Promise<void> {
  if (spy.mock.calls.length >= times) return Promise.resolve();
  return new Promise((resolve) => {
    const through = spy.getMockImplementation();
    spy.mockImplementation(function (this: unknown, ...args: unknown[]) {
      const answer = through?.apply(this, args);
      if (spy.mock.calls.length >= times) resolve();
      return answer;
    });
  });
}

/**
 * Resolves once the fake Fleet's Studios or browsed addresses satisfy `held`, at once if they
 * already do. The same wait as `whenCalled`, on state the fake keeps rather than a function.
 */
export function whenFleet(fleet: Pick<StudioKeeping, "onChange">, held: () => boolean): Promise<void> {
  if (held()) return Promise.resolve();
  return new Promise((resolve) => {
    const stop = fleet.onChange(() => {
      if (!held()) return;
      stop();
      resolve();
    });
  });
}

/** Kept so a test reads as it was told: no call stands in front of the Cockpit's list, so there is nothing to put off. */
export async function putOffEveryCall(): Promise<void> {
  expect(document.querySelector(".deck-call")).toBeNull();
}
