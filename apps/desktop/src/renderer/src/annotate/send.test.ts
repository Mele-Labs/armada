import { describe, expect, it, vi } from "vitest";

import type { SessionNote } from "@armada/protocol";

import type { RingedNotes } from "../../../shared/api/sessions";
import { isAnnotation, requestOf, serializeAnnotation, type Annotation } from "../../../shared/annotations";
import { annotationNote } from "../../../shared/pinned-note";
import { sendToSession, unsendable } from "./send";
import type { Sink } from "./sink";

function note(overrides: Partial<Annotation> = {}): Annotation {
  return {
    id: "20260917-135315-tsfq",
    status: "open",
    text: "Can we add an illustration here?",
    component: "BoardEmptyState",
    owners: ["BoardEmpty", "ActiveJobsList", "OverviewLists"],
    ownersFrom: "parent",
    selector: "#root div.armada-board-empty",
    element: { tag: "div", text: "No jobs. Propose one above.", label: null },
    screen: "Overview",
    layer: null,
    location: "/index.html",
    scenario: null,
    box: { x: 254, y: 184, width: 557, height: 200 },
    window: { width: 1280, height: 800 },
    createdAt: "2026-09-17T13:53:15.999Z",
    updatedAt: "2026-09-17T13:53:26.658Z",
    ...overrides,
  };
}

function sink(overrides: Partial<Sink> = {}): Sink {
  return {
    via: "main",
    list: async () => [],
    save: async () => undefined,
    remove: async () => undefined,
    root: async () => "/Users/user/armada",
    capture: async () => new Uint8Array([137, 80, 78, 71]).buffer,
    saveShot: async () => undefined,
    shot: async () => null,
    saveExample: async () => undefined,
    example: async () => null,
    removeExample: async () => undefined,
    ...overrides,
  };
}

const AT = new Date("2026-09-17T14:00:00.000Z");

describe("a note's request", () => {
  it("leads with the person's words, then says where in Bridge they point", () => {
    const text = requestOf(note());
    expect(text.split("\n")[0]).toBe("Can we add an illustration here?");
    expect(text).toContain("Component: BoardEmptyState inside BoardEmpty inside ActiveJobsList inside OverviewLists");
    expect(text).toContain('Element: <div> reading "No jobs. Propose one above."');
    expect(text).toContain("Screen: Overview");
    expect(text).toContain("Note file: .armada/annotations/20260917-135315-tsfq.json");
    expect(text).not.toContain("Mock scenario");
  });
});

describe("a note's source", () => {
  it("is written beside the component it names, and says where the code is in the request", () => {
    const located = note({ source: { file: "packages/screens/src/Board.tsx", line: 88 } });
    expect(Object.keys(JSON.parse(serializeAnnotation(located)) as object).slice(3, 6)).toEqual([
      "component",
      "source",
      "owners",
    ]);
    expect(requestOf(located)).toContain("Source: packages/screens/src/Board.tsx:88");
  });

  it("is absent from a note written before the build stamped anything, which still reads", () => {
    expect(isAnnotation(note())).toBe(true);
    expect(Object.keys(JSON.parse(serializeAnnotation(note())) as object)).not.toContain("source");
    expect(requestOf(note())).not.toContain("Source:");
    expect(isAnnotation({ ...note(), source: { file: "packages/screens/src/Board.tsx" } })).toBe(false);
  });
});

describe("sending", () => {
  it("is not offered where there is no Fleet behind the layer", () => {
    expect(unsendable(sink())).toBeNull();
    expect(unsendable(sink({ via: "dev server" }))).toMatch(/mock/);
  });
});

/** A PNG's signature and header, `width` by `height`: all `pngSize` reads. */
function pngOf(width: number, height: number): ArrayBuffer {
  const bytes = new Uint8Array(24);
  bytes.set([137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 0x49, 0x48, 0x44, 0x52]);
  const view = new DataView(bytes.buffer);
  view.setUint32(16, width);
  view.setUint32(20, height);
  return bytes.buffer;
}

describe("a note sent to a Session", () => {
  const ADDRESS = "http://localhost:5174/index.html";
  const refused = { ok: false, outcome: { ok: false, why: "not_connected" } };
  const sessions = (answer: unknown = { ok: true, value: { id: "01NEW", title: undefined } }) => ({
    startSession: vi.fn(async () => answer as never),
    sendSessionMessage: vi.fn(async () => ({ ok: true, value: {} }) as never),
    sendSessionNotes: vi.fn(async (_send: RingedNotes) => ({ ok: true, value: { session: {}, candidates: [] } }) as never),
  });
  /** Each `send_session_notes` call's body. */
  const turns = (fleet: ReturnType<typeof sessions>): RingedNotes[] => fleet.sendSessionNotes.mock.calls.map(([send]) => send);
  /** Every note sent, in the order sent. */
  const notesSent = (fleet: ReturnType<typeof sessions>): SessionNote[] => turns(fleet).flatMap((turn) => turn.notes.map((one) => one.note));
  const item = { note: note(), box: note().box };
  const WINDOW = { x: 0, y: 0, width: 1024, height: 768 };
  const PAGE = { address: ADDRESS, window: WINDOW };

  it("is kept with the Session's id and title, and still reads as a note", () => {
    const sent = note({ sent: { sessionId: "01S", title: "Release notes", at: AT.toISOString() } });
    expect(isAnnotation(sent)).toBe(true);
    expect(isAnnotation({ ...note(), sent: { title: "x", at: AT.toISOString() } })).toBe(false);
  });

  it("starts a Session with no title, for Fleet to name, then sends the note whole with its file's id and the window as its picture", async () => {
    const fleet = sessions();
    const answer = await sendToSession([item], null, sink(), fleet, PAGE, AT);
    // The notes' words ride as the request, so Fleet's proposer reads them at the start as it does a dispatch.
    expect(fleet.startSession).toHaveBeenCalledWith(undefined, "/Users/user/armada", "Can we add an illustration here?", undefined, undefined, true);
    expect(answer).toEqual({ ok: true, sent: { sessionId: "01NEW", title: "Can we add an illustration here?", at: AT.toISOString() } });
    expect(turns(fleet)).toEqual([{ session_id: "01NEW", notes: [{ note: annotationNote(note(), ADDRESS, "iVBORw==") }] }]);
    expect(notesSent(fleet)[0]).toMatchObject({ said: "Can we add an illustration here?", note_id: "20260917-135315-tsfq", crop: { media_type: "image/png", data: "iVBORw==" } });
    expect(fleet.sendSessionMessage).not.toHaveBeenCalled();
  });

  it("starts nothing where there is no repository to start it on", async () => {
    const fleet = sessions();
    expect((await sendToSession([item], null, sink({ root: async () => null }), fleet, PAGE, AT)).ok).toBe(false);
    expect(fleet.startSession).not.toHaveBeenCalled();
  });

  it("sends to the Session picked without starting one, and without a picture where none can be taken", async () => {
    const fleet = sessions();
    const answer = await sendToSession([item], { id: "01T", title: "Release notes" }, sink({ capture: async () => null }), fleet, PAGE, AT);
    expect(fleet.startSession).not.toHaveBeenCalled();
    expect(answer).toEqual({ ok: true, sent: { sessionId: "01T", title: "Release notes", at: AT.toISOString() } });
    expect(turns(fleet)[0]!.session_id).toBe("01T");
    expect(notesSent(fleet)[0]).not.toHaveProperty("crop");
    expect(turns(fleet)[0]!.notes[0]).not.toHaveProperty("ring");
  });

  it("sends the notes as one turn, in order, with the window of each one on this screen and none of one that is not", async () => {
    const fleet = sessions();
    const capture = vi.fn(async () => new Uint8Array([137, 80, 78, 71]).buffer);
    const away = note({ id: "20260917-140000-away", text: "Over on another screen" });
    const answer = await sendToSession([item, { note: away, box: null }], null, sink({ capture }), fleet, PAGE, AT);
    expect(capture).toHaveBeenCalledTimes(1);
    expect(capture).toHaveBeenCalledWith(WINDOW);
    expect(fleet.sendSessionNotes).toHaveBeenCalledTimes(1);
    expect(notesSent(fleet).map((one) => [one.note_id, one.said, one.crop?.data])).toEqual([
      ["20260917-135315-tsfq", "Can we add an illustration here?", "iVBORw=="],
      ["20260917-140000-away", "Over on another screen", undefined],
    ]);
    expect(answer.ok).toBe(true);
  });

  it("sends each note's kept picture, a note off this screen among them, and shoots the window only for an older note that is on it", async () => {
    const fleet = sessions();
    const kept = new Uint8Array([137, 80, 78, 71, 9]).buffer;
    const capture = vi.fn(async () => new Uint8Array([137, 80, 78, 71]).buffer);
    const shot = vi.fn(async (id: string) => (id === "20260917-140000-kept" ? kept : null));
    const away = note({ id: "20260917-140000-kept", text: "Kept from another screen", screenshot: "20260917-140000-kept.png" });
    const older = note({ id: "20260917-140100-gone", text: "Older, and away" });
    await sendToSession([{ note: away, box: null }, item, { note: older, box: null }], null, sink({ capture, shot }), fleet, PAGE, AT);
    expect(notesSent(fleet).map((one) => one.crop?.data)).toEqual(["iVBORwk=", "iVBORw==", undefined]);
    expect(capture).toHaveBeenCalledTimes(1);
    expect(capture).toHaveBeenCalledWith(WINDOW);
  });

  it("rings each note's box on its window, in the picture's own pixels: the kept ring, or the box scaled to a window shot now", async () => {
    const fleet = sessions();
    // Twice the window's width: a Retina window's picture.
    const capture = vi.fn(async () => pngOf(WINDOW.width * 2, WINDOW.height * 2));
    const ring = { x: 10, y: 20, width: 30, height: 40, scale: 2 };
    const kept = note({ id: "20260917-140000-kept", screenshot: "20260917-140000-kept.png", ring });
    await sendToSession([{ note: kept, box: null }, item], { id: "01T", title: "t" }, sink({ capture, shot: async () => pngOf(8, 8) }), fleet, PAGE, AT);
    expect(turns(fleet)[0]!.notes.map((one) => one.ring)).toEqual([ring, { x: 508, y: 368, width: 1114, height: 400, scale: 2 }]);
  });

  it("sends the notes on an element as one numbered turn and each whole-window note as its own message, where it stands", async () => {
    const fleet = sessions();
    const whole = note({ id: "20260917-140200-wide", scope: "window", text: "The whole window lags" });
    const second = note({ id: "20260917-140300-next", text: "And this" });
    const capture = vi.fn(async () => pngOf(WINDOW.width, WINDOW.height));
    await sendToSession([item, { note: whole, box: whole.box }, { note: second, box: second.box }], { id: "01T", title: "t" }, sink({ capture }), fleet, PAGE, AT);
    expect(fleet.sendSessionNotes).toHaveBeenCalledTimes(1);
    expect(notesSent(fleet).map((one) => one.note_id)).toEqual(["20260917-135315-tsfq", "20260917-140300-next"]);
    expect(turns(fleet)[0]!.notes.every((one) => one.ring !== undefined)).toBe(true);
    expect(fleet.sendSessionMessage).toHaveBeenCalledTimes(1);
    // The turn goes where its first note stands, before the window note behind it.
    expect(fleet.sendSessionNotes.mock.invocationCallOrder[0]!).toBeLessThan(fleet.sendSessionMessage.mock.invocationCallOrder[0]!);
  });

  it("sends a whole-window note as a message with the plain window, since what Bridge held has no place in a pinned note", async () => {
    const fleet = sessions();
    const whole = note({ id: "20260917-140200-wide", scope: "window", text: "The whole window lags" });
    await sendToSession([item, { note: whole, box: whole.box }], { id: "01T", title: "t" }, sink({ capture: async () => pngOf(WINDOW.width, WINDOW.height) }), fleet, PAGE, AT);
    expect(notesSent(fleet).map((one) => one.note_id)).toEqual(["20260917-135315-tsfq"]);
    const message = (fleet.sendSessionMessage.mock.calls[0] as unknown as [{ session_id: string; text: string; attachments: { data: string }[] }])[0];
    expect(message).toMatchObject({ session_id: "01T", text: requestOf(whole), attachments: [{ name: "annotation-20260917-140200-wide.png", media_type: "image/png" }] });
    expect(message).not.toHaveProperty("ring");
  });

  it("says what Fleet said when it refuses, and marks nothing", async () => {
    expect(await sendToSession([item], null, sink(), sessions(refused), PAGE, AT)).toEqual({ ok: false, saying: "Job not started: Fleet is not connected. Nothing was sent." });
    const fleet = { ...sessions(), sendSessionNotes: vi.fn(async () => refused as never) };
    expect(await sendToSession([item], { id: "01T", title: "t" }, sink(), fleet, PAGE, AT)).toEqual({ ok: false, saying: 'Notes not sent to "t": Fleet is not connected. Nothing was sent.' });
  });

  it("names the notes Fleet took before it refused, so only those are marked sent: a turn is taken whole or not at all", async () => {
    const whole = note({ id: "20260917-140200-wide", scope: "window", text: "The whole window lags" });
    const second = note({ id: "20260917-140300-next", text: "And this" });
    const notes = [{ note: whole, box: null }, item, { note: second, box: null }];
    const partly = (sentIds: string[]) => ({ ok: false, saying: 'Notes not sent to "t": Fleet is not connected. Nothing was sent.', sent: { sessionId: "01T", title: "t", at: AT.toISOString() }, sentIds });

    const turnRefused = { ...sessions(), sendSessionNotes: vi.fn(async () => refused as never) };
    expect(await sendToSession(notes, { id: "01T", title: "t" }, sink(), turnRefused, PAGE, AT)).toEqual(partly(["20260917-140200-wide"]));

    const messageRefused = { ...sessions(), sendSessionMessage: vi.fn(async () => refused as never) };
    const answer = await sendToSession([item, { note: second, box: null }, { note: whole, box: null }], { id: "01T", title: "t" }, sink(), messageRefused, PAGE, AT);
    expect(answer).toEqual(partly(["20260917-135315-tsfq", "20260917-140300-next"]));
  });

  it("gives a refusal with no coded copy its message, and a plain reason where there is none", async () => {
    const refusal = (message: string) => ({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.unknown_manifest", message } } });
    expect(await sendToSession([item], null, sink(), sessions(refusal("No such manifest")), PAGE, AT)).toEqual({ ok: false, saying: "Job not started: No such manifest" });
    expect(await sendToSession([item], null, sink(), sessions(refusal("")), PAGE, AT)).toEqual({ ok: false, saying: "Job not started: Fleet refused it" });
  });

  describe("with a Job open", () => {
    const open = { id: "01OPEN", facts: ["Job: Fix the flaky store test (01OPEN)", "State: working"] };
    const second = { note: note({ id: "20260917-140300-next" }), box: null };

    it("sends no message of the Job's own, and puts its facts on the first note alone", async () => {
      const fleet = sessions();
      await sendToSession([item, second], null, sink(), fleet, PAGE, AT, open);
      expect(fleet.sendSessionMessage).not.toHaveBeenCalled();
      expect(notesSent(fleet).map((one) => one.job)).toEqual([open.facts, undefined]);
    });

    it("puts them on a note for a live Session that is not the open Job", async () => {
      const fleet = sessions();
      await sendToSession([item], { id: "01T", title: "t" }, sink(), fleet, PAGE, AT, open);
      expect(fleet.sendSessionMessage).not.toHaveBeenCalled();
      expect(notesSent(fleet)[0]?.job).toEqual(open.facts);
    });

    it("puts them after the words of a whole-window first note, in the same message", async () => {
      const fleet = sessions();
      const whole = { note: note({ id: "20260917-140200-wide", scope: "window", text: "The whole window lags" }), box: null };
      await sendToSession([whole, item], { id: "01T", title: "t" }, sink(), fleet, PAGE, AT, open);
      const messages = (fleet.sendSessionMessage.mock.calls as unknown as [{ text: string }][]).map(([sent]) => sent.text);
      expect(messages).toHaveLength(1);
      expect(messages[0]?.startsWith("The whole window lags")).toBe(true);
      expect(messages[0]).toContain("\nBackground, not the task: the Job he had open when he pinned this\nJob: Fix the flaky store test (01OPEN)\nState: working");
      expect(notesSent(fleet).map((one) => one.job)).toEqual([undefined]);
    });

    it("sends none to the open Job itself, which holds them, nor where no Job is open", async () => {
      const toSelf = sessions();
      await sendToSession([item], { id: "01OPEN", title: "t" }, sink(), toSelf, PAGE, AT, open);
      expect(notesSent(toSelf)[0]).not.toHaveProperty("job");
      const none = sessions();
      await sendToSession([item], { id: "01T", title: "t" }, sink(), none, PAGE, AT, undefined);
      expect(notesSent(none)[0]).not.toHaveProperty("job");
      const empty = sessions();
      await sendToSession([item], { id: "01T", title: "t" }, sink(), empty, PAGE, AT, { id: "01OPEN", facts: [] });
      expect(notesSent(empty)[0]).not.toHaveProperty("job");
    });
  });
});
