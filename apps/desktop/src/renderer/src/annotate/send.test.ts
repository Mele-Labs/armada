import { describe, expect, it, vi } from "vitest";

import { isAnnotation, requestOf, serializeAnnotation, type Annotation } from "../../../shared/annotations";
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
  const sessions = (answer: unknown = { ok: true, value: { id: "01NEW", title: undefined } }) => ({
    startSession: vi.fn(async () => answer as never),
    sendSessionMessage: vi.fn(async () => ({ ok: true, value: {} }) as never),
  });
  const item = { note: note(), box: note().box };

  it("is kept with the Session's id and title, and still reads as a note", () => {
    const sent = note({ sent: { sessionId: "01S", title: "Release notes", at: AT.toISOString() } });
    expect(isAnnotation(sent)).toBe(true);
    expect(isAnnotation({ ...note(), sent: { title: "x", at: AT.toISOString() } })).toBe(false);
  });

  it("starts a Session named for the first note, then sends the request with the screenshot as an upload", async () => {
    const fleet = sessions();
    const answer = await sendToSession([item], null, sink(), fleet, AT);
    expect(fleet.startSession).toHaveBeenCalledWith(titleOf([note()]), "/Users/user/armada");
    expect(answer).toEqual({ ok: true, sent: { sessionId: "01NEW", title: "Can we add an illustration here?", at: AT.toISOString() } });
    const message = (fleet.sendSessionMessage.mock.calls[0] as unknown as [{ session_id: string; text: string; attachments: unknown[] }])[0];
    expect(message.session_id).toBe("01NEW");
    expect(message.text).toBe(requestOf(note()));
    expect(message.attachments).toEqual([{ name: "annotation-20260917-135315-tsfq.png", media_type: "image/png", data: "iVBORw==" }]);
  });

  it("starts nothing where there is no repository to start it on", async () => {
    const fleet = sessions();
    expect((await sendToSession([item], null, sink({ root: async () => null }), fleet, AT)).ok).toBe(false);
    expect(fleet.startSession).not.toHaveBeenCalled();
  });

  it("sends to the Session picked without starting one, and without a screenshot where none can be taken", async () => {
    const fleet = sessions();
    const answer = await sendToSession([item], { id: "01T", title: "Release notes" }, sink({ capture: async () => null }), fleet, AT);
    expect(fleet.startSession).not.toHaveBeenCalled();
    expect(answer).toEqual({ ok: true, sent: { sessionId: "01T", title: "Release notes", at: AT.toISOString() } });
    expect((fleet.sendSessionMessage.mock.calls[0] as unknown as [object])[0]).not.toHaveProperty("attachments");
  });

  it("takes a screenshot of each note that is on this screen and none of one that is not, and sends them all", async () => {
    const fleet = sessions();
    const capture = vi.fn(async () => new Uint8Array([137, 80, 78, 71]).buffer);
    const away = note({ id: "20260917-140000-away", text: "Over on another screen" });
    const answer = await sendToSession([item, { note: away, box: null }], null, sink({ capture }), fleet, AT);
    expect(capture).toHaveBeenCalledTimes(1);
    expect(capture).toHaveBeenCalledWith(note().box);
    const message = (fleet.sendSessionMessage.mock.calls[0] as unknown as [{ text: string; attachments: { name: string }[] }])[0];
    expect(message.attachments.map((one) => one.name)).toEqual(["annotation-20260917-135315-tsfq.png"]);
    expect(message.text).toContain("Over on another screen");
    expect(answer.ok).toBe(true);
  });

  it("attaches each note's kept picture, a note off this screen among them, and shoots only an older note that is on it", async () => {
    const fleet = sessions();
    const kept = new Uint8Array([137, 80, 78, 71, 9]).buffer;
    const capture = vi.fn(async () => new Uint8Array([137, 80, 78, 71]).buffer);
    const shot = vi.fn(async (id: string) => (id === "20260917-140000-kept" ? kept : null));
    const away = note({ id: "20260917-140000-kept", text: "Kept from another screen", screenshot: "20260917-140000-kept.png" });
    const older = note({ id: "20260917-140100-gone", text: "Older, and away" });
    await sendToSession([{ note: away, box: null }, item, { note: older, box: null }], null, sink({ capture, shot }), fleet, AT);
    const message = (fleet.sendSessionMessage.mock.calls[0] as unknown as [{ attachments: { name: string; data: string }[] }])[0];
    expect(message.attachments).toEqual([
      { name: "annotation-20260917-140000-kept.png", media_type: "image/png", data: "iVBORwk=" },
      { name: "annotation-20260917-135315-tsfq.png", media_type: "image/png", data: "iVBORw==" },
    ]);
    expect(capture).toHaveBeenCalledTimes(1);
    expect(capture).toHaveBeenCalledWith(note().box);
  });

  it("sends a note with no screenshot at all where none is on this screen", async () => {
    const fleet = sessions();
    const capture = vi.fn(async () => new Uint8Array([137]).buffer);
    const answer = await sendToSession([{ note: note(), box: null }], null, sink({ capture }), fleet, AT);
    expect(capture).not.toHaveBeenCalled();
    expect((fleet.sendSessionMessage.mock.calls[0] as unknown as [object])[0]).not.toHaveProperty("attachments");
    expect(answer.ok).toBe(true);
  });

  it("says what Fleet said when it refuses, and marks nothing", async () => {
    const refused = { ok: false, outcome: { ok: false, why: "not_connected" } };
    expect(await sendToSession([item], null, sink(), sessions(refused), AT)).toEqual({ ok: false, saying: "Job not started: Fleet is not connected. Nothing was sent." });
    const fleet = { ...sessions(), sendSessionMessage: vi.fn(async () => refused as never) };
    expect(await sendToSession([item], { id: "01T", title: "t" }, sink(), fleet, AT)).toEqual({ ok: false, saying: 'Notes not sent to "t": Fleet is not connected. Nothing was sent.' });
  });

  it("gives a refusal with no coded copy its message, and a plain reason where there is none", async () => {
    const refusal = (message: string) => ({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.unknown_manifest", message } } });
    expect(await sendToSession([item], null, sink(), sessions(refusal("No such manifest")), AT)).toEqual({ ok: false, saying: "Job not started: No such manifest" });
    expect(await sendToSession([item], null, sink(), sessions(refusal("")), AT)).toEqual({ ok: false, saying: "Job not started: Fleet refused it" });
  });
});
