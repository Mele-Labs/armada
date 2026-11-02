// An instrument mod on the console through `App`, over the mock's Fleet: the real load path, real `instrument.js` text. What the walk `console-instrument-mods` shows, held.

import type { ModSummary } from "@armada/protocol";
import type { Session } from "@armada/screens/src/draft/sessions";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { forgetBays } from "../cockpit/deck/ConsoleBays";
import { ConsoleStage } from "../cockpit/deck/pixi/console-stage";
import { s263ConsoleInstrumentMods as base } from "./scenarios/console-instrument-mods";
import type { Scenario } from "./moment";
import { motion, mount, onScreen, unmountAfterEach } from "./testing";

unmountAfterEach();

type Tally = { paints: Record<string, number>; again: Record<string, () => void>; mounts: number };
declare global {
  interface Window {
    tally: Tally;
  }
}
beforeEach(() => {
  localStorage.clear();
  forgetBays();
  window.tally = { paints: {}, again: {}, mounts: 0 };
});
afterEach(() => vi.restoreAllMocks());

const controls = () => page.getByRole("region", { name: "Flight controls" });
const facet = (name: string) => controls().getByRole("button", { name, exact: true });
const seated = () => JSON.parse(localStorage.getItem("armada.console.bays") ?? "null") as string[] | null;
const said = (name: string) => facet(name).element().getAttribute("aria-description");
const bayEmpty = (n: number) => document.querySelector(`.console-bay[data-n="${n}"]`)?.hasAttribute("data-empty");

/** An instrument counting its paints in `window.tally`, as `name`. */
const counting = (name: string, moves: boolean) => `const paint = (draw, { w, h, ink }) => {
  window.tally.paints.${name} = (window.tally.paints.${name} ?? 0) + 1;
  draw.fillStyle = ink.run;
  draw.fillRect(0, 0, w, h);
};
export default function Counting({ host }) {
  const { createElement: el, useEffect, useState } = host.React;
  const [pressed, press] = useState(0);
  useEffect(() => {
    window.tally.mounts += 1;
    window.tally.again.${name} = () => host.face(paint);
    host.face(paint, { moves: ${moves} });
  }, []);
  return el("button", { type: "button", onClick: () => press(pressed + 1) }, "Pressed " + pressed);
}
`;
const THROWS_PAINTING = `const paint = () => { throw new Error("usage.month is undefined"); };
export default function Spend({ host }) {
  host.React.useEffect(() => host.face(paint), []);
  return null;
}
`;
const THROWS_RENDERING = `export default function Spend() { throw new Error("rows is not defined"); }\n`;
const NO_DEFAULT = "export const Spend = () => null;\n";

const row = (name: string, over: Partial<ModSummary> = {}): ModSummary => ({ name, kind: "instrument", enabled: true, valid: true, changed_at: "2026-11-02T10:00:00.000Z", ...over });

/** The scenario's window with these mods and these seats, and what changes what Fleet lists and checks, as a Job's edit does. */
function fleetOf(code: Record<string, string>, seats: readonly (string | null)[], over: { sessions?: readonly Session[] } = {}) {
  const rows = Object.keys(code).map((name) => row(name));
  const held = { code: { ...code }, publish: (_next: ModSummary[]): void => undefined, rows };
  const scenario: Scenario = {
    ...base,
    behaves: (fleet) => {
      held.publish = (next) => {
        held.rows = next;
        fleet.publish({ mods: { mods: next } });
      };
      const own = base.behaves!(fleet);
      held.publish(rows);
      return {
        ...own,
        validateMod: async (name: string) => {
          const text = held.code[name];
          return text === undefined ? null : { name, valid: true, problems: [], instrument: { code: text } };
        },
      };
    },
    draft: {
      ...base.draft!,
      sessions: (control, more = []) => {
        const store = base.draft!.sessions!(control, [...more, ...(over.sessions ?? [])]);
        localStorage.setItem("armada.console.bays", JSON.stringify(seats));
        forgetBays();
        return store;
      },
    },
  };
  return {
    scenario,
    /** The mod's file changed, and Fleet says so on its row. */
    edit: (name: string, text: string) => {
      held.code[name] = text;
      held.publish(held.rows.map((one) => (one.name === name ? { ...one, changed_at: new Date().toISOString() } : one)));
    },
    list: (next: ModSummary[]) => held.publish(next),
  };
}

async function deck(scenario: Scenario, first: string) {
  const app = mount(scenario);
  await onScreen();
  await expect.element(facet(first)).toBeInTheDocument();
  return app;
}

test("a still face paints once, a moving one keeps painting, and the still one paints again only when asked", async () => {
  // The browser project reduces motion, under which nothing moves at all.
  await motion();
  const { scenario } = fleetOf({ still: counting("still", false), moving: counting("moving", true) }, ["mod:still", "beacon", "mod:moving", "scope"]);
  await deck(scenario, "Still");
  await expect.poll(() => window.tally.paints.still ?? 0).toBeGreaterThan(0);
  await expect.poll(() => window.tally.paints.moving ?? 0).toBeGreaterThan(0);
  // A second effect under StrictMode may still be due, so the count is read once the moving face has had its ticks.
  const from = window.tally.paints.moving!;
  await expect.poll(() => window.tally.paints.moving!, { timeout: 15_000 }).toBeGreaterThan(from + 2);
  const settled = window.tally.paints.still!;
  const next = window.tally.paints.moving!;
  await expect.poll(() => window.tally.paints.moving!, { timeout: 15_000 }).toBeGreaterThan(next + 2);
  expect(window.tally.paints.still).toBe(settled);
  window.tally.again.still!();
  await expect.poll(() => window.tally.paints.still, { timeout: 15_000 }).toBe(settled + 1);
}, 60_000);

test("the component keeps its state across the panel opening and closing, and its return stands in the panel", async () => {
  const { scenario } = fleetOf({ held: counting("held", false) }, ["mod:held", "beacon", "trace", "scope"]);
  await deck(scenario, "Held");
  await expect.poll(() => window.tally.mounts).toBeGreaterThan(0);
  const mounts = window.tally.mounts;
  await facet("Held").click();
  const panel = page.getByRole("dialog", { name: "Held" });
  await panel.getByRole("button", { name: "Pressed 0" }).click();
  await expect.element(panel.getByRole("button", { name: "Pressed 1" })).toBeVisible();
  await facet("Held").click();
  await expect.poll(() => document.querySelector('[data-hud="instrument"]')).toBeNull();
  await facet("Held").click();
  await expect.element(panel.getByRole("button", { name: "Pressed 1" })).toBeVisible();
  expect(window.tally.mounts).toBe(mounts);
});

test("a reload on the row's changed_at is the new module, and the new module's face", async () => {
  const fleet = fleetOf({ held: counting("first", false) }, ["mod:held", "beacon", "trace", "scope"]);
  await deck(fleet.scenario, "Held");
  await expect.poll(() => window.tally.paints.first ?? 0).toBeGreaterThan(0);
  fleet.edit("held", counting("second", false));
  await expect.poll(() => window.tally.paints.second ?? 0, { timeout: 15_000 }).toBeGreaterThan(0);
  const first = window.tally.paints.first;
  await facet("Held").click();
  await expect.element(page.getByRole("dialog", { name: "Held" }).getByRole("button", { name: "Pressed 0" })).toBeVisible();
  expect(window.tally.paints.first).toBe(first);
});

test.each([
  ["as it paints", THROWS_PAINTING, "usage.month is undefined"],
  ["as it renders", THROWS_RENDERING, "rows is not defined"],
  ["as it loads", NO_DEFAULT, "instrument.js has no component as its default export"],
])("a mod that throws %s empties its socket with the reason, keeps its seat, and clears on the next good load", async (_how, text, why) => {
  // React says so on the console, as a tab's boundary lets it.
  vi.spyOn(console, "error").mockImplementation(() => undefined);
  const fleet = fleetOf({ spend: text }, ["beacon", "mod:spend", "trace", "scope"]);
  await deck(fleet.scenario, "Spend");
  await expect.poll(() => said("Spend"), { timeout: 15_000 }).toBe(`Spend did not draw: ${why}`);
  expect(seated()).toContain("mod:spend");
  fleet.edit("spend", counting("spend", false));
  await expect.poll(() => said("Spend")).toBeNull();
  await expect.poll(() => window.tally.paints.spend ?? 0, { timeout: 15_000 }).toBeGreaterThan(0);
});

test("switched off is an empty socket that says so, and it is back when switched on", async () => {
  const fleet = fleetOf({ held: counting("held", false) }, ["mod:held", "beacon", "trace", "scope"]);
  await deck(fleet.scenario, "Held");
  await expect.poll(() => window.tally.mounts).toBeGreaterThan(0);
  fleet.list([row("held", { enabled: false })]);
  await expect.poll(() => said("Held")).toBe("Held is switched off");
  expect(seated()).toContain("mod:held");
  fleet.list([row("held", { enabled: true })]);
  await expect.poll(() => said("Held")).toBeNull();
});

test("a mod Fleet no longer lists is an empty socket, and its seat waits for it", async () => {
  const fleet = fleetOf({ held: counting("held", false) }, ["mod:held", "beacon", "trace", "scope"]);
  await deck(fleet.scenario, "Held");
  fleet.list([]);
  await expect.poll(() => bayEmpty(1)).toBe(true);
  expect(seated()).toContain("mod:held");
});

test("a seat on a Job that ended with nothing built is an empty socket", async () => {
  const ended: Session = { id: "gone", title: "Mod New instrument", mod: true, modOfBay: true, dead: "ended", attachments: [], rows: [], turn: { state: "idle" }, standing: "idle" };
  const { scenario } = fleetOf({}, ["job:gone", "beacon", "trace", "scope"], { sessions: [ended] });
  await deck(scenario, "Calls");
  await expect.poll(() => bayEmpty(1)).toBe(true);
});

const make = async (words: string) => {
  await controls().getByRole("button", { name: "Edit", exact: true }).last().click();
  await page.getByRole("button", { name: /^Bay 4/ }).click();
  await page.getByRole("listbox", { name: "Instruments" }).getByRole("option", { name: "Make one", exact: true }).click();
  const sheet = page.getByRole("dialog", { name: "New instrument" });
  await sheet.getByRole("textbox", { name: "New instrument" }).fill(words);
  await sheet.getByRole("button", { name: "Send" }).click();
};

test("Make one starts a Mod Job aimed at an instrument, and the seat follows the row it makes", async () => {
  const start = vi.fn();
  const scenario: Scenario = {
    ...base,
    behaves: (fleet) => {
      const own = base.behaves!(fleet);
      return { ...own, startModSession: async (asked) => (start(asked), own.startModSession!(asked)) };
    },
  };
  await deck(scenario, "Reviews");
  await make("How long ci on main has taken");
  await expect.poll(() => start.mock.calls.length).toBe(1);
  expect(start).toHaveBeenCalledWith({ title: "Mod New instrument", said: "How long ci on main has taken", aim: "instrument" });
  await expect.poll(() => seated()?.[3], { timeout: 15_000 }).toBe("mod:main-build");
  await expect.element(page.getByRole("button", { name: "Bay 4, Main build" })).toBeInTheDocument();
});

test("while Fleet has listed no mod the seat is on the Job and the bay is New instrument", async () => {
  const scenario: Scenario = {
    ...base,
    behaves: (fleet) => ({ ...base.behaves!(fleet), startModSession: async () => ({ ok: true, value: { session_id: "mod-main-build", title: "Mod New instrument" } }) }),
  };
  await deck(scenario, "Reviews");
  await make("Anything");
  await expect.element(page.getByRole("button", { name: "Bay 4, New instrument" })).toBeInTheDocument();
  expect(seated()?.[3]).toBe("job:mod-main-build");
  await controls().getByRole("button", { name: "Done", exact: true }).click();
  await expect.element(facet("New instrument")).toBeInTheDocument();
});

test("a refused start is told, and takes no seat", async () => {
  const told: string[] = [];
  const heard = (event: Event) => told.push((event as CustomEvent<{ sentence: string }>).detail.sentence);
  window.addEventListener("armada:tell", heard);
  try {
    const scenario: Scenario = {
      ...base,
      behaves: (fleet) => ({ ...base.behaves!(fleet), startModSession: async () => ({ ok: false, outcome: { ok: false, status: 503, code: "fleet.unserved", message: "Mods are off" } as never }) }),
    };
    await deck(scenario, "Reviews");
    const before = seated();
    await make("Anything");
    await expect.poll(() => told.length).toBe(1);
    expect(told[0]).toMatch(/^Mod Job not started: /);
    await expect.element(page.getByRole("dialog", { name: "New instrument" })).toBeVisible();
    expect(seated()).toEqual(before);
  } finally {
    window.removeEventListener("armada:tell", heard);
  }
});

test("with no canvas the same face stands straight in the bay, and paints once if still", async () => {
  await motion();
  vi.spyOn(console, "error").mockImplementation(() => undefined);
  vi.spyOn(ConsoleStage.prototype as unknown as { init(): Promise<void> }, "init").mockRejectedValue(new Error("no hardware WebGL"));
  const { scenario } = fleetOf({ still: counting("still", false), moving: counting("moving", true) }, ["mod:still", "beacon", "mod:moving", "scope"]);
  await deck(scenario, "Still");
  await expect.poll(() => document.querySelectorAll(".console-instrument__face").length).toBe(2);
  const face = document.querySelector<HTMLCanvasElement>(".console-instrument__face")!;
  expect(face.clientWidth).toBeGreaterThan(0);
  expect(getComputedStyle(face).transform).toBe("none");
  await expect.poll(() => window.tally.paints.still ?? 0).toBeGreaterThan(0);
  const from = window.tally.paints.moving!;
  await expect.poll(() => window.tally.paints.moving!).toBeGreaterThan(from + 2);
  const settled = window.tally.paints.still!;
  const next = window.tally.paints.moving!;
  await expect.poll(() => window.tally.paints.moving!).toBeGreaterThan(next + 2);
  expect(window.tally.paints.still).toBe(settled);
});
