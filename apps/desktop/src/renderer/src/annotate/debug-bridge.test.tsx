// Debug Bridge, from the palette or ⇧⌘D, with the annotation layer off: one press shoots the
// window, keeps it as a whole-window note with Bridge's state, starts a Session, leaves all of it
// as a draft in that Session's message box, and opens it. Nothing is sent: the first turn is the
// owner's Enter.

import { StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { forgetAllKept, readKept } from "@armada/components";
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { Annotation } from "../../../shared/annotations";
import { askToDebugBridge } from "../debug-bridge";
import "../styles/index.css";
import { Layer, type LayerFleet } from "./Layer";
import type { Sink } from "./sink";

const PNG = Uint8Array.from([137, 80, 78, 71, 13, 10, 26, 10, 0]).buffer;
const JOB = "01M22TYSAE0023MADDP5ZQEYGW";

function capturing() {
  const notes = new Map<string, Annotation>();
  const shots = new Map<string, ArrayBuffer>();
  const capture = vi.fn(async () => PNG);
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
  return { sink, notes, capture };
}

function fleetWith(refuse = false) {
  const sent: { session_id: string; text: string; attachments?: { name: string }[] }[] = [];
  const fleet = {
    startSession: vi.fn(async (title?: string) =>
      (refuse ? { ok: false, outcome: { ok: false, why: "not_connected" } } : { ok: true, value: { id: "01DEBUGSESSION00", title, state: "live" } }) as never,
    ),
    sendSessionMessage: vi.fn(async (send: (typeof sent)[number]) => {
      sent.push(send);
      return { ok: true, value: { id: send.session_id } } as never;
    }),
    sendSessionNote: vi.fn(async () => ({ ok: true, value: { session: {}, candidates: [] } }) as never),
    state: vi.fn(async () => ({
      connection: { state: "connected", fleet: { protocolId: "23.62", pid: 4242, port: 7421, startedAt: "Mon Oct 12 09:00:00 2026" }, cursor: 0 },
      health: { state: "read", health: { probes: [], not_probed: [], helm_action_authority: "read_only" } },
      jobs: [{ id: JOB, handle: "12-the-drone-count-is-wrong", title: "The Drone count is wrong", status: "running", workflow_id: "default" }],
      sessions: { state: "read", sessions: [] },
    }) as never),
    subscribe: () => () => undefined,
  } satisfies LayerFleet;
  return { fleet, sent };
}

let mounted: { root: Root; host: HTMLElement }[] = [];

afterEach(() => {
  for (const one of mounted) {
    one.root.unmount();
    one.host.remove();
  }
  mounted = [];
  document.querySelectorAll("[data-noted]").forEach((one) => one.remove());
  forgetAllKept();
  sessionStorage.removeItem("armada.annotate.on");
});

/** The layer mounted and off, as Bridge under `pnpm dev` runs it until ⌥⌘A. */
async function layerOff(sink: Sink, fleet: LayerFleet, openSession = vi.fn(), tell = vi.fn()) {
  const host = document.createElement("div");
  host.setAttribute("data-armada-annotate", "");
  document.body.append(host);
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <Layer sink={sink} fleet={fleet} openSession={openSession} tell={tell} />
    </StrictMode>,
  );
  mounted.push({ root, host });
  await vi.waitFor(() => expect(fleet.state).toHaveBeenCalled());
  return { openSession, tell };
}

function onOverview(): void {
  const rail = document.createElement("a");
  rail.setAttribute("data-noted", "");
  rail.setAttribute("aria-current", "page");
  rail.setAttribute("aria-label", "Overview");
  document.body.append(rail);
}

test("the palette's Debug Bridge opens a Session holding the picture and the state as a draft, and sends nothing", async () => {
  const { sink, notes, capture } = capturing();
  const { fleet, sent } = fleetWith();
  onOverview();
  const { openSession } = await layerOff(sink, fleet);
  askToDebugBridge();

  await vi.waitFor(() => expect(openSession).toHaveBeenCalledWith("01DEBUGSESSION00"));
  expect(capture).toHaveBeenCalledWith({ x: 0, y: 0, width: window.innerWidth, height: window.innerHeight });
  expect(fleet.startSession).toHaveBeenCalledWith("Debug Bridge on Overview", "/repo");
  // No message: Fleet starts no turn until the owner sends the draft.
  expect(sent).toHaveLength(0);
  expect(fleet.sendSessionMessage).not.toHaveBeenCalled();

  const draft = readKept<{ text: string; tags: unknown[]; files: { name: string; file?: File }[] }>("session:01DEBUGSESSION00");
  expect(draft?.text).toContain("Work out what is going on in Bridge");
  expect(draft?.text).toContain("- Doctor: ");
  // A blank line under the state, where his context goes.
  expect(draft?.text.endsWith("\n\n")).toBe(true);
  expect(draft?.files.map((one) => one.name)).toEqual([expect.stringMatching(/^annotation-.*\.png$/)]);
  expect(draft?.files[0]?.file?.type).toBe("image/png");

  const [note] = [...notes.values()];
  expect(note).toMatchObject({ scope: "window", screenshot: `${note?.id}.png` });
  // Nothing was sent, so the note says so.
  expect(note?.sent).toBeUndefined();
  // Nothing of the layer was drawn: it stayed off.
  expect(page.getByRole("status").query()).toBeNull();
});

test("⇧⌘D does the same from anywhere, and a second press while the first is out starts no second Session", async () => {
  const { sink } = capturing();
  const { fleet } = fleetWith();
  const { openSession } = await layerOff(sink, fleet);
  const press = () => window.dispatchEvent(new KeyboardEvent("keydown", { key: "d", code: "KeyD", metaKey: true, shiftKey: true, bubbles: true }));
  press();
  press();
  await vi.waitFor(() => expect(openSession).toHaveBeenCalledTimes(1));
  expect(fleet.startSession).toHaveBeenCalledTimes(1);
  expect(fleet.sendSessionMessage).not.toHaveBeenCalled();
});

test("a Session Fleet refuses is told, and the note is kept unsent for /annotations", async () => {
  const { sink, notes } = capturing();
  const { fleet } = fleetWith(true);
  const { openSession, tell } = await layerOff(sink, fleet);
  askToDebugBridge();
  await vi.waitFor(() => expect(tell).toHaveBeenCalledWith(expect.stringMatching(/^Job not started: /)));
  expect(openSession).not.toHaveBeenCalled();
  expect(readKept("session:01DEBUGSESSION00")).toBeUndefined();
  const [note] = [...notes.values()];
  expect(note?.scope).toBe("window");
  expect(note?.sent).toBeUndefined();
});
