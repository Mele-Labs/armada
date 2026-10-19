// A Job's held notes on the IPC seam: what the panel and the bar reach, and only on the Job main reads off the view that
// called. `docs/practices/capture-window.md`, *Held notes*.

import { beforeEach, expect, it, vi } from "vitest";

vi.mock("electron", () => ({}));

import type { Outcome, SendSessionNote, SendSessionNotes } from "@armada/protocol";
import { CHANNELS } from "../shared/bridge";
import { HeldNotes } from "./capture/held-notes";
import { handleHeldNotes, sendHeld, tellHeld } from "./held-notes-channels";

const note = (sessionId: string, said: string): SendSessionNote => ({
  session_id: sessionId,
  said,
  address: "http://localhost:5173/",
  place: { selector: "main > button", element: { tag: "button", text: "Reload", label: "Reload the table" }, location: "/" },
  state: {},
  crop: { media_type: "image/png", data: "iVBORw0KGgo=" },
});

const PANEL = { id: 2, isDestroyed: () => false, send: vi.fn() };
const BAR = { id: 1 };
/** The panel shows Job `a`; the bar's window holds its notes on `a` too. */
const captureWindows = {
  panelFrom: (contents: { id: number }) => (contents.id === PANEL.id ? { kind: "session", id: "a" } : null),
  panels: () => [PANEL],
  from: (contents: { id: number }) => (contents.id === BAR.id ? { notesOn: "a" } : null),
  heldChanged: vi.fn(),
};

let store: HeldNotes;
let sent: SendSessionNotes[];
let answer: Outcome;
const handlers = new Map<string, (event: { sender: unknown }, ...args: unknown[]) => unknown>();
const call = (channel: string, sender: unknown, ...args: unknown[]) => handlers.get(channel)!({ sender }, ...args);

beforeEach(() => {
  let minted = 0;
  store = new HeldNotes(null, (sessionId) => tellHeld(store, captureWindows as never, sessionId), () => `n${(minted += 1)}`);
  sent = [];
  answer = { ok: true };
  handlers.clear();
  PANEL.send.mockClear();
  captureWindows.heldChanged.mockClear();
  handleHeldNotes({
    ipc: { handle: (channel: string, run: never) => void handlers.set(channel, run) } as never,
    store,
    captureWindows: captureWindows as never,
    send: async (body) => (sent.push(body), answer),
  });
});

it("lists the panel's Job's notes, its words, what it is on and its crop, and nothing of another Job", () => {
  store.hold(note("a", "Too tight"));
  store.hold(note("b", "Elsewhere"));
  expect(call(CHANNELS.walkPanelHeld, PANEL)).toEqual([
    { id: "n1", at: expect.any(String), said: "Too tight", on: "button “Reload the table”", crop: { media_type: "image/png", data: "iVBORw0KGgo=" } },
  ]);
  expect(call(CHANNELS.walkPanelHeld, BAR)).toEqual([]);
});

it("pushes the list to the panel showing that Job on every change, and tells the bar again", () => {
  store.hold(note("a", "Too tight"));
  store.hold(note("b", "Elsewhere"));
  expect(PANEL.send.mock.calls).toEqual([[CHANNELS.walkPanelHeldChanged, [expect.objectContaining({ said: "Too tight" })]]]);
  expect(captureWindows.heldChanged.mock.calls).toEqual([["a"], ["b"]]);
});

it("edits and removes a note by its id on the panel's Job alone, and refuses an edit to nothing", async () => {
  store.hold(note("a", "Too tight"));
  store.hold(note("b", "Elsewhere"));
  expect(call(CHANNELS.walkPanelEditNote, PANEL, "n1", "Too tight on the left")).toEqual({ ok: true });
  expect(call(CHANNELS.walkPanelEditNote, PANEL, "n1", "  ")).toEqual({ ok: false, why: "empty_brief" });
  // `n2` is Job b's, which this panel does not show.
  expect(call(CHANNELS.walkPanelRemoveNote, PANEL, "n2")).toEqual({ ok: false, why: "not_understood" });
  expect(call(CHANNELS.walkPanelEditNote, BAR, "n1", "From the bar")).toEqual({ ok: false, why: "not_understood" });
  expect(store.of("a").map((one) => one.note.said)).toEqual(["Too tight on the left"]);
  expect(call(CHANNELS.walkPanelRemoveNote, PANEL, "n1")).toEqual({ ok: true });
  expect(store.of("a")).toEqual([]);
  expect(store.of("b")).toHaveLength(1);
});

it("sends every note of the Job as one turn, in the order taken, and drops them once Fleet took them", async () => {
  store.hold(note("a", "Too tight"));
  store.hold(note("a", "Wrong colour"));
  store.hold(note("b", "Elsewhere"));
  expect(await call(CHANNELS.walkPanelSendNotes, PANEL)).toEqual({ ok: true });
  expect(sent).toHaveLength(1);
  expect(sent[0]!.session_id).toBe("a");
  expect(sent[0]!.notes.map((one) => one.said)).toEqual(["Too tight", "Wrong colour"]);
  expect(sent[0]!.notes[0]).not.toHaveProperty("session_id");
  expect(sent[0]!.notes[0]!.crop).toEqual({ media_type: "image/png", data: "iVBORw0KGgo=" });
  expect(store.of("a")).toEqual([]);
  expect(store.of("b")).toHaveLength(1);
  expect(PANEL.send.mock.calls.at(-1)).toEqual([CHANNELS.walkPanelHeldChanged, []]);
});

it("keeps every note held when Fleet refuses the turn, and says why", async () => {
  store.hold(note("a", "Too tight"));
  answer = { ok: false, why: "not_connected" };
  expect(await call(CHANNELS.walkPanelSendNotes, PANEL)).toEqual({ ok: false, why: "not_connected" });
  expect(store.of("a").map((one) => one.note.said)).toEqual(["Too tight"]);
});

it("sends from the bar the notes of the Job its window's notes are held on, before Approve", async () => {
  store.hold(note("a", "Too tight"));
  expect(await call(CHANNELS.captureWindowSendNotes, BAR)).toEqual({ ok: true });
  expect(sent.map((one) => one.session_id)).toEqual(["a"]);
  expect(await call(CHANNELS.captureWindowSendNotes, PANEL)).toEqual({ ok: false, why: "not_understood" });
});

it("sends nothing for a Job that holds none, since Fleet refuses an empty turn", async () => {
  expect(await sendHeld(store, "a", async () => ({ ok: false, why: "not_connected" }))).toEqual({ ok: true });
});

it("keeps a note held while the send was out, and drops only those sent", async () => {
  store.hold(note("a", "Too tight"));
  let release: (outcome: Outcome) => void = () => {};
  const out = sendHeld(store, "a", () => new Promise((done) => (release = done)));
  store.hold(note("a", "Taken while sending"));
  release({ ok: true });
  await out;
  expect(store.of("a").map((one) => one.note.said)).toEqual(["Taken while sending"]);
});
