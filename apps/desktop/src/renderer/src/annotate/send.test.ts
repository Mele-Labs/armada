import { describe, expect, it, vi } from "vitest";

import type { SendSessionNote } from "@armada/protocol";

import { isAnnotation, requestOf, serializeAnnotation, type Annotation } from "../../../shared/annotations";
import { annotationNoteBody } from "../../../shared/pinned-note";
import { sendToSession, titleOf, unsendable } from "./send";
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

describe("a note sent to a Session", () => {
  const ADDRESS = "http://localhost:5174/index.html";
  const refused = { ok: false, outcome: { ok: false, why: "not_connected" } };
  const sessions = (answer: unknown = { ok: true, value: { id: "01NEW", title: undefined } }) => ({
    startSession: vi.fn(async () => answer as never),
    sendSessionMessage: vi.fn(async () => ({ ok: true, value: {} }) as never),
    sendSessionNote: vi.fn(async (_send: SendSessionNote) => ({ ok: true, value: { session: {}, candidates: [] } }) as never),
  });
  const notesSent = (fleet: ReturnType<typeof sessions>): SendSessionNote[] => fleet.sendSessionNote.mock.calls.map(([send]) => send);
  const item = { note: note(), box: note().box };

  it("is kept with the Session's id and title, and still reads as a note", () => {
    const sent = note({ sent: { sessionId: "01S", title: "Release notes", at: AT.toISOString() } });
    expect(isAnnotation(sent)).toBe(true);
    expect(isAnnotation({ ...note(), sent: { title: "x", at: AT.toISOString() } })).toBe(false);
  });

  it("starts a Session named for the first note, then sends the note whole with its file's id and the screenshot as the crop", async () => {
    const fleet = sessions();
    const answer = await sendToSession([item], null, sink(), fleet, ADDRESS, AT);
    expect(fleet.startSession).toHaveBeenCalledWith(titleOf([note()]), "/Users/user/armada");
    expect(answer).toEqual({ ok: true, sent: { sessionId: "01NEW", title: "Can we add an illustration here?", at: AT.toISOString() } });
    expect(notesSent(fleet)).toEqual([annotationNoteBody(note(), "01NEW", ADDRESS, "iVBORw==")]);
    expect(notesSent(fleet)[0]).toMatchObject({ said: "Can we add an illustration here?", note_id: "20260917-135315-tsfq", crop: { media_type: "image/png", data: "iVBORw==" } });
    expect(fleet.sendSessionMessage).not.toHaveBeenCalled();
  });

  it("starts nothing where there is no repository to start it on", async () => {
    const fleet = sessions();
    expect((await sendToSession([item], null, sink({ root: async () => null }), fleet, ADDRESS, AT)).ok).toBe(false);
    expect(fleet.startSession).not.toHaveBeenCalled();
  });

  it("sends to the Session picked without starting one, and without a crop where none can be taken", async () => {
    const fleet = sessions();
    const answer = await sendToSession([item], { id: "01T", title: "Release notes" }, sink({ capture: async () => null }), fleet, ADDRESS, AT);
    expect(fleet.startSession).not.toHaveBeenCalled();
    expect(answer).toEqual({ ok: true, sent: { sessionId: "01T", title: "Release notes", at: AT.toISOString() } });
    expect(notesSent(fleet)[0]!.session_id).toBe("01T");
    expect(notesSent(fleet)[0]).not.toHaveProperty("crop");
  });

  it("sends each note on its own, with a screenshot of each one on this screen and none of one that is not", async () => {
    const fleet = sessions();
    const capture = vi.fn(async () => new Uint8Array([137, 80, 78, 71]).buffer);
    const away = note({ id: "20260917-140000-away", text: "Over on another screen" });
    const answer = await sendToSession([item, { note: away, box: null }], null, sink({ capture }), fleet, ADDRESS, AT);
    expect(capture).toHaveBeenCalledTimes(1);
    expect(capture).toHaveBeenCalledWith(note().box);
    expect(notesSent(fleet).map((one) => [one.note_id, one.said, one.crop?.data])).toEqual([
      ["20260917-135315-tsfq", "Can we add an illustration here?", "iVBORw=="],
      ["20260917-140000-away", "Over on another screen", undefined],
    ]);
    expect(answer.ok).toBe(true);
  });

  it("crops each note to its kept picture, a note off this screen among them, and shoots only an older note that is on it", async () => {
    const fleet = sessions();
    const kept = new Uint8Array([137, 80, 78, 71, 9]).buffer;
    const capture = vi.fn(async () => new Uint8Array([137, 80, 78, 71]).buffer);
    const shot = vi.fn(async (id: string) => (id === "20260917-140000-kept" ? kept : null));
    const away = note({ id: "20260917-140000-kept", text: "Kept from another screen", screenshot: "20260917-140000-kept.png" });
    const older = note({ id: "20260917-140100-gone", text: "Older, and away" });
    await sendToSession([{ note: away, box: null }, item, { note: older, box: null }], null, sink({ capture, shot }), fleet, ADDRESS, AT);
    expect(notesSent(fleet).map((one) => one.crop?.data)).toEqual(["iVBORwk=", "iVBORw==", undefined]);
    expect(capture).toHaveBeenCalledTimes(1);
    expect(capture).toHaveBeenCalledWith(note().box);
  });

  it("sends a whole-window note as a message, since what Bridge held has no place in a pinned note", async () => {
    const fleet = sessions();
    const whole = note({ id: "20260917-140200-wide", scope: "window", text: "The whole window lags" });
    await sendToSession([item, { note: whole, box: whole.box }], { id: "01T", title: "t" }, sink(), fleet, ADDRESS, AT);
    expect(notesSent(fleet).map((one) => one.note_id)).toEqual(["20260917-135315-tsfq"]);
    const message = (fleet.sendSessionMessage.mock.calls[0] as unknown as [{ session_id: string; text: string; attachments: unknown[] }])[0];
    expect(message).toEqual({ session_id: "01T", text: requestOf(whole), attachments: [{ name: "annotation-20260917-140200-wide.png", media_type: "image/png", data: "iVBORw==" }] });
  });

  it("says what Fleet said when it refuses, and marks nothing", async () => {
    expect(await sendToSession([item], null, sink(), sessions(refused), ADDRESS, AT)).toEqual({ ok: false, saying: "Job not started: Fleet is not connected. Nothing was sent." });
    const fleet = { ...sessions(), sendSessionNote: vi.fn(async () => refused as never) };
    expect(await sendToSession([item], { id: "01T", title: "t" }, sink(), fleet, ADDRESS, AT)).toEqual({ ok: false, saying: 'Notes not sent to "t": Fleet is not connected. Nothing was sent.' });
  });

  it("names the notes Fleet took before it refused one, so only those are marked sent", async () => {
    const fleet = sessions();
    fleet.sendSessionNote.mockResolvedValueOnce({ ok: true, value: { session: {}, candidates: [] } } as never).mockResolvedValueOnce(refused as never);
    const second = note({ id: "20260917-140300-next", text: "And this" });
    const third = note({ id: "20260917-140400-last", text: "And that" });
    const answer = await sendToSession([item, { note: second, box: null }, { note: third, box: null }], { id: "01T", title: "t" }, sink(), fleet, ADDRESS, AT);
    expect(answer).toEqual({
      ok: false,
      saying: 'Notes not sent to "t": Fleet is not connected. Nothing was sent.',
      sent: { sessionId: "01T", title: "t", at: AT.toISOString() },
      sentIds: ["20260917-135315-tsfq"],
    });
    expect(fleet.sendSessionNote).toHaveBeenCalledTimes(2);
  });

  it("gives a refusal with no coded copy its message, and a plain reason where there is none", async () => {
    const refusal = (message: string) => ({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.unknown_manifest", message } } });
    expect(await sendToSession([item], null, sink(), sessions(refusal("No such manifest")), ADDRESS, AT)).toEqual({ ok: false, saying: "Job not started: No such manifest" });
    expect(await sendToSession([item], null, sink(), sessions(refusal("")), ADDRESS, AT)).toEqual({ ok: false, saying: "Job not started: Fleet refused it" });
  });
});
