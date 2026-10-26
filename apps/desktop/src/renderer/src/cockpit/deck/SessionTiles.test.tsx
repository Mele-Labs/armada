// The Flight control of an open Job: one row of keys, Auto land and Pause each with a lamp that is on while the switch is, a tile Fleet does not offer
// aria-disabled and pressing nothing. Browser: a rendering and a press are what is read. The Job is drawn with no file of the owner's, so the tiles are the built-in ones.

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeAll, expect, test, vi } from "vitest";
import type { FlightControls } from "@armada/protocol";
import type { Session, SessionsDraft } from "@armada/screens/src/draft/sessions";

import type { BridgeState } from "../../../../shared/bridge";
import { NOTHING_YET } from "../../../../shared/bridge";
import { SessionTiles } from "./SessionTiles";
import "../../styles/index.css";
import "./deck.css";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const NONE: FlightControls = { version: 1, job: [], deck: [] };

let base: unknown;
beforeAll(() => {
  const state = { ...NOTHING_YET, connection: { state: "connected" } } as unknown as BridgeState;
  base = (window as unknown as { armada: unknown }).armada = {
    subscribeChanges: () => () => {},
    syncState: async () => ({ full: state, seq: 1 }),
    getFlightControls: async () => ({ ok: true, value: { what: "read", controls: NONE } }),
    saveFlightControls: async (controls: FlightControls) => ({ ok: true, value: { what: "read", controls } }),
    getRepositoryFlightControls: async (repository: string) => ({ ok: true, value: { repository, place: "this_mac", read: { what: "read", controls: NONE } } }),
  };
});

const roots: { root: Root; host: HTMLElement }[] = [];
afterEach(() => {
  for (const { root, host } of roots.splice(0)) {
    act(() => root.unmount());
    host.remove();
  }
  (window as unknown as { armada: unknown }).armada = base;
});

const flush = () => act(async () => void (await new Promise((done) => setTimeout(done, 0))));

async function mount(node: React.ReactNode) {
  const host = document.createElement("div");
  // The foot gives the panel its height and `--tile`; a bare host has neither.
  host.style.cssText = "--tile: calc(var(--space-12) + var(--space-4)); display: flex; flex-direction: column; height: calc(var(--space-12) * 3 + var(--space-4))";
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(node));
  roots.push({ root, host });
  await flush();
  await flush();
  return host;
}

const REPOSITORY = { root: "/code/armada", name: "armada" };
const jobOf = (more: object = {}): Session => ({ id: "s1", rows: [], attachments: [], turn: { state: "idle" }, ...more }) as unknown as Session;
/** What Fleet offers: Auto land, Pause and Release, with something held to release. Anything else is not offered. */
const draftOf = (more: object = {}) => ({ commands: [], get: () => [], send: vi.fn(async () => true), autoLand: vi.fn(async () => true), pause: vi.fn(async () => true), resume: vi.fn(async () => true), ...more }) as unknown as SessionsDraft & { autoLand: ReturnType<typeof vi.fn>; pause: ReturnType<typeof vi.fn>; resume: ReturnType<typeof vi.fn> };

const tiles = (host: HTMLElement) => [...host.querySelectorAll<HTMLElement>(".deck-flight .deck-tile")];
const tile = (host: HTMLElement, name: string) => host.querySelector<HTMLElement>(`.deck-flight button[aria-label="${name}"]`)!;
const lamp = (button: HTMLElement) => button.querySelector<HTMLElement>(".deck-tile__state");

test("Flight control is one row of tiles: Auto land, Pause, Judge, Report, Release, Terminal, Requests", async () => {
  const host = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} />);
  expect(tiles(host).map((one) => one.getAttribute("aria-label"))).toEqual(["Auto land", "Pause", "Judge", "Report issue with job", "Release everything the Job holds", "Terminal", "Requests"]);
  // One row: every tile stands at the same height, each to the right of the one before.
  const boxes = tiles(host).map((one) => one.getBoundingClientRect());
  expect(new Set(boxes.map((box) => Math.round(box.top))).size).toBe(1);
  boxes.slice(1).forEach((box, at) => expect(box.left).toBeGreaterThanOrEqual(boxes[at]!.right - 1));
});

test("only Auto land and Pause carry a lamp", async () => {
  const host = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} />);
  expect(tiles(host).filter((one) => lamp(one) !== null).map((one) => one.getAttribute("aria-label"))).toEqual(["Auto land", "Pause"]);
});

test("Auto land's lamp is off and the tile not pressed while the switch is off", async () => {
  const host = await mount(<SessionTiles session={jobOf()} draft={draftOf()} repository={REPOSITORY} />);
  const auto = tile(host, "Auto land");
  expect(lamp(auto)!.hasAttribute("data-on")).toBe(false);
  expect(lamp(auto)!.getAttribute("aria-label")).toBe("Auto land off");
  expect(auto.getAttribute("aria-pressed")).toBe("false");
});

test("Auto land's lamp is on and the tile pressed while the switch is on", async () => {
  const host = await mount(<SessionTiles session={jobOf({ autoLand: true })} draft={draftOf()} repository={REPOSITORY} />);
  const auto = tile(host, "Auto land");
  expect(lamp(auto)!.hasAttribute("data-on")).toBe(true);
  expect(lamp(auto)!.getAttribute("aria-label")).toBe("Auto land on");
  expect(auto.getAttribute("aria-pressed")).toBe("true");
});

test("pressing Auto land turns the flag on through Fleet", async () => {
  const draft = draftOf();
  const host = await mount(<SessionTiles session={jobOf()} draft={draft} repository={REPOSITORY} />);
  await act(async () => tile(host, "Auto land").click());
  expect(draft.autoLand.mock.calls).toEqual([["s1", true]]);
});

test("pressing Auto land while it is on turns the flag off", async () => {
  const draft = draftOf();
  const host = await mount(<SessionTiles session={jobOf({ autoLand: true })} draft={draft} repository={REPOSITORY} />);
  await act(async () => tile(host, "Auto land").click());
  expect(draft.autoLand.mock.calls).toEqual([["s1", false]]);
});

test("Pause's lamp is off while the Job runs, and pressing the tile pauses it", async () => {
  const draft = draftOf();
  const host = await mount(<SessionTiles session={jobOf()} draft={draft} repository={REPOSITORY} />);
  const pause = tile(host, "Pause");
  expect(lamp(pause)!.hasAttribute("data-on")).toBe(false);
  expect(lamp(pause)!.getAttribute("aria-label")).toBe("Not paused");
  expect(pause.getAttribute("aria-pressed")).toBe("false");
  await act(async () => pause.click());
  expect(draft.pause.mock.calls).toEqual([["s1"]]);
});

test("Pause's lamp is on while the Job is paused, the tile reads Resume, and pressing it resumes the Job", async () => {
  const draft = draftOf();
  const host = await mount(<SessionTiles session={jobOf({ paused: true })} draft={draft} repository={REPOSITORY} />);
  const resume = tile(host, "Resume");
  expect(resume).not.toBeNull();
  expect(lamp(resume)!.hasAttribute("data-on")).toBe(true);
  expect(lamp(resume)!.getAttribute("aria-label")).toBe("Paused");
  await act(async () => resume.click());
  expect(draft.resume.mock.calls).toEqual([["s1"]]);
});

// Paused, the key is seated (`data-on`) and its name says Resume, so it is not also announced as pressed: "Resume, pressed" would say the opposite of what a press does.
test("Pause's key is seated while the Job is paused, and its name, not aria-pressed, carries the state", async () => {
  const host = await mount(<SessionTiles session={jobOf({ paused: true })} draft={draftOf()} repository={REPOSITORY} />);
  const resume = tile(host, "Resume");
  expect(resume.hasAttribute("data-on")).toBe(true);
  expect(resume.getAttribute("aria-pressed")).toBe("false");
});

test("a tile Fleet does not offer is aria-disabled and a press does nothing", async () => {
  const draft = draftOf({ pause: undefined, autoLand: undefined });
  const host = await mount(<SessionTiles session={jobOf()} draft={draft} repository={REPOSITORY} />);
  for (const name of ["Auto land", "Pause", "Judge", "Release everything the Job holds", "Terminal"]) expect(tile(host, name).getAttribute("aria-disabled"), name).toBe("true");
  // What is offered is not: Report, with its repository known, and Requests.
  for (const name of ["Report issue with job", "Requests"]) expect(tile(host, name).hasAttribute("aria-disabled"), name).toBe(false);
  await act(async () => tile(host, "Auto land").click());
  await act(async () => tile(host, "Pause").click());
  expect(draft.send).not.toHaveBeenCalled();
});
