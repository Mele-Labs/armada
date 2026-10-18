// A note on the whole window: the owner reporting the state of the app rather than one thing on it.
// Taken from the bar or with ⌥⌘S, its picture is the window, and it carries the page, the open
// Session and Doctor's read, so an agent can debug the state without the owner there to ask.

import { StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import { requestOf, type Annotation } from "../../../shared/annotations";
import "../styles/index.css";
import { Layer, type LayerFleet } from "./Layer";
import type { Sink } from "./sink";

const PNG = Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10, 0]).buffer;

/** A sink in Bridge, which captures, holding notes and pictures in memory. */
function capturing(capture: Sink["capture"] = async () => PNG) {
  const notes = new Map<string, Annotation>();
  const shots = new Map<string, ArrayBuffer>();
  const sink: Sink = {
    via: "main",
    list: async () => [...notes.values()],
    save: async (note) => void notes.set(note.id, note),
    remove: async (id) => void notes.delete(id),
    root: async () => "/repo",
    capture,
    saveShot: async (id, png) => void shots.set(id, png),
    shot: async (id) => shots.get(id) ?? null,
  };
  return { sink, notes, shots };
}

const SESSION = "01HOSTEDAAAAAAAA";

/** A Fleet with one live Session, and a Doctor read with SQLite warning. */
function fleetWith(): LayerFleet {
  const state = {
    connection: { state: "connected", fleet: { protocolId: "23.62", pid: 4242, port: 7421, startedAt: "Mon Oct 12 09:00:00 2026" }, cursor: 0 },
    health: {
      state: "read",
      health: { probes: [{ module: "SQLite", outcome: "warn", detail: "3 rows will not load" }], not_probed: [], helm_action_authority: "read_only" },
    },
    sessions: { state: "read", sessions: [{ id: SESSION, title: "Fix the flaky store test", state: "live", harness: "agent", cwd: "/repo", origin: "bridge" }] },
  };
  return {
    startSession: vi.fn(async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }) as never),
    sendSessionMessage: vi.fn(async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }) as never),
    sendSessionNote: vi.fn(async () => ({ ok: false, outcome: { ok: false, why: "not_connected" } }) as never),
    state: vi.fn(async () => state as never),
    subscribe: () => () => undefined,
  } satisfies LayerFleet;
}

let mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  document.querySelectorAll("[data-noted]").forEach((one) => one.remove());
  sessionStorage.removeItem("armada.annotate.on");
});

async function annotating(sink: Sink, fleet?: LayerFleet): Promise<void> {
  sessionStorage.setItem("armada.annotate.on", "1");
  const host = document.createElement("div");
  host.setAttribute("data-armada-annotate", "");
  document.body.append(host);
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <Layer sink={sink} {...(fleet === undefined ? {} : { fleet })} />
    </StrictMode>,
  );
  mounted.push({ root, host });
  await expect.element(page.getByRole("status")).toBeVisible();
}

/** A page on the rail's Sessions row, with something stamped on it. */
function onScreen(name: string, stamp: Record<string, string> = {}): void {
  const rail = document.createElement("a");
  rail.setAttribute("data-noted", "");
  rail.setAttribute("aria-current", "page");
  rail.setAttribute("aria-label", name);
  document.body.append(rail);
  const view = document.createElement("div");
  view.setAttribute("data-noted", "");
  for (const [key, value] of Object.entries(stamp)) view.setAttribute(key, value);
  document.body.append(view);
}

const pressShoot = (): boolean => window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyS", key: "ß", metaKey: true, altKey: true, bubbles: true }));

async function say(text: string): Promise<void> {
  const card = page.getByRole("dialog", { name: "New note" });
  await expect.element(card).toBeVisible();
  await card.getByRole("textbox", { name: "Note" }).fill(text);
  await card.getByRole("button", { name: "Save note" }).click();
  await expect.element(card).not.toBeInTheDocument();
}

test("Screenshot window on the bar takes the whole window and opens a note on it", async () => {
  const capture = vi.fn(async () => PNG);
  const { sink, notes, shots } = capturing(capture);
  onScreen("Sessions");
  await annotating(sink);
  await page.getByRole("status").getByRole("button", { name: "Screenshot window" }).click();
  await expect.element(page.getByRole("dialog", { name: "New note" }).getByText("Whole window · Sessions")).toBeVisible();
  await say("Everything here is stale");

  const [saved] = [...notes.values()];
  expect(saved).toMatchObject({ scope: "window", text: "Everything here is stale", screen: "Sessions", selector: "body", screenshot: `${saved?.id}.png` });
  expect(capture).toHaveBeenCalledTimes(1);
  expect(capture).toHaveBeenCalledWith({ x: 0, y: 0, width: window.innerWidth, height: window.innerHeight });
  expect(shots.get(saved!.id)).toBe(PNG);
});

test("⌥⌘S does the same while annotating, taking the picture as it is pressed rather than at save", async () => {
  const capture = vi.fn(async () => PNG);
  const { sink, notes } = capturing(capture);
  await annotating(sink);
  pressShoot();
  await expect.element(page.getByRole("dialog", { name: "New note" })).toBeVisible();
  expect(capture).toHaveBeenCalledTimes(1);
  await say("The bar flickers");
  expect(capture).toHaveBeenCalledTimes(1);
  expect([...notes.values()][0]?.scope).toBe("window");
});

test("a whole-window note carries Doctor's read and the connection, and sends them", async () => {
  const { sink, notes } = capturing();
  const fleet = fleetWith();
  onScreen("Overview");
  await annotating(sink, fleet);
  await vi.waitFor(() => expect(fleet.state).toHaveBeenCalled());
  pressShoot();
  await say("This looks stuck");

  const saved = [...notes.values()][0]!;
  expect(saved.context).toMatchObject({ connection: "connected" });
  expect(saved.context?.session).toBeUndefined();
  expect(saved.context?.doctor.rows).toContainEqual({ module: "SQLite", result: "warn", detail: "3 rows will not load" });

  const request = requestOf(saved);
  expect(request).toContain("This looks stuck");
  expect(request).toContain("- Screen: Overview");
  expect(request).toContain("- Connection to Fleet: connected");
  expect(request).toContain("  - SQLite: warn — 3 rows will not load");
  expect(request).not.toContain("Selector");
});

test("a whole-window note left on a Session's page carries that Session", async () => {
  const { sink, notes } = capturing();
  const fleet = fleetWith();
  onScreen("Sessions", { "data-armada-open-session": SESSION });
  await annotating(sink, fleet);
  await vi.waitFor(() => expect(fleet.state).toHaveBeenCalled());
  pressShoot();
  await say("The thread stopped scrolling");

  const saved = [...notes.values()][0]!;
  expect(saved.context?.session).toEqual({ id: SESSION, title: "Fix the flaky store test", state: "live", harness: "agent", cwd: "/repo" });
  expect(requestOf(saved)).toContain(`- Job open: "Fix the flaky store test" (${SESSION}) — state live, harness agent, cwd /repo`);
});

test("a whole-window note draws no pin, and is counted on the screen it was left on", async () => {
  const { sink } = capturing();
  onScreen("Sessions");
  await annotating(sink);
  pressShoot();
  await say("Whole thing");
  await expect.element(page.getByRole("status").getByText("1 open, 0 done")).toBeVisible();
  expect(document.querySelector(".armada-annotate__pin")).toBeNull();
  await expect.element(page.getByRole("status").getByRole("button", { name: "Start job" })).toBeVisible();
});

test("⌥⌘S does nothing while the layer is off, and never throws away a note half written", async () => {
  const capture = vi.fn(async () => PNG);
  const { sink } = capturing(capture);
  await annotating(sink);
  pressShoot();
  const card = page.getByRole("dialog", { name: "New note" });
  await card.getByRole("textbox", { name: "Note" }).fill("Half written");
  pressShoot();
  await expect.element(card.getByRole("textbox", { name: "Note" })).toHaveValue("Half written");
  expect(capture).toHaveBeenCalledTimes(1);

  window.dispatchEvent(new KeyboardEvent("keydown", { code: "KeyA", metaKey: true, altKey: true, bubbles: true }));
  await expect.element(page.getByRole("status")).not.toBeInTheDocument();
  pressShoot();
  expect(capture).toHaveBeenCalledTimes(1);
});
