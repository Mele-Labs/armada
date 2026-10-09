import { describe, expect, it, vi } from "vitest";
import type { JobSummary, Proposed } from "@armada/protocol";

import { isAnnotation, requestOf, serializeAnnotation, type Annotation } from "../../../shared/annotations";
import { sendToFleet, sendToSession, titleOf, unsendable, type Proposer } from "./send";
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
    ...overrides,
  };
}

const JOB = { id: "01M3JOB00000000000000000000", handle: "42-add-an-illustration" } as JobSummary;

function fleet(answer: Proposed): Proposer & { proposeFromRequest: ReturnType<typeof vi.fn> } {
  return {
    stageAttachment: vi.fn(async (_bytes: ArrayBuffer, filename: string) => ({ path: `/tmp/staged/${filename}` })),
    proposeFromRequest: vi.fn(async () => answer),
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

describe("a sent note", () => {
  it("is still a note, and keeps where it went just after its status", () => {
    const sent = note({ sent: { jobId: JOB.id, handle: JOB.handle, at: AT.toISOString() } });
    expect(isAnnotation(sent)).toBe(true);
    const keys = Object.keys(JSON.parse(serializeAnnotation(sent)) as object);
    expect(keys.slice(0, 3)).toEqual(["id", "status", "sent"]);
  });

  it("is refused with a sent record missing its Job", () => {
    expect(isAnnotation({ ...note(), sent: { handle: "42-x", at: AT.toISOString() } })).toBe(false);
  });
});

describe("sending to Fleet", () => {
  it("proposes the request against the notes' repository, with the screenshot staged, and names the Job", async () => {
    const proposer = fleet({ ok: true, jobs: [JOB] });
    const answer = await sendToFleet(note(), note().box, sink(), proposer, AT);
    expect(answer).toEqual({ ok: true, sent: { jobId: JOB.id, handle: JOB.handle, at: AT.toISOString() } });
    expect(proposer.stageAttachment).toHaveBeenCalledWith(expect.any(ArrayBuffer), "annotation-20260917-135315-tsfq.png", "image/png");
    expect(proposer.proposeFromRequest).toHaveBeenCalledWith(
      requestOf(note()),
      [{ path: "/tmp/staged/annotation-20260917-135315-tsfq.png", filename: "annotation-20260917-135315-tsfq.png", mimeType: "image/png" }],
      "/Users/user/armada",
    );
  });

  it("sends without a screenshot where none can be taken", async () => {
    const proposer = fleet({ ok: true, jobs: [JOB] });
    await sendToFleet(note(), note().box, sink({ capture: async () => null }), proposer, AT);
    expect(proposer.stageAttachment).not.toHaveBeenCalled();
    expect(proposer.proposeFromRequest.mock.calls[0]![1]).toEqual([]);
  });

  it("takes no screenshot of a note whose element is not on this screen, and still sends it", async () => {
    const proposer = fleet({ ok: true, jobs: [JOB] });
    const capture = vi.fn(async () => new Uint8Array([137]).buffer);
    const answer = await sendToFleet(note(), null, sink({ capture }), proposer, AT);
    expect(capture).not.toHaveBeenCalled();
    expect(proposer.stageAttachment).not.toHaveBeenCalled();
    expect(proposer.proposeFromRequest.mock.calls[0]![1]).toEqual([]);
    expect(answer).toEqual({ ok: true, sent: { jobId: JOB.id, handle: JOB.handle, at: AT.toISOString() } });
  });

  it("says what Fleet said when it takes nothing, and sends nothing when there is no repository", async () => {
    const refused = await sendToFleet(note(), note().box, sink(), fleet({ ok: false, why: "refused", outcome: { ok: false, why: "not_connected" } }), AT);
    expect(refused).toEqual({ ok: false, saying: "Job not sent: Fleet is not connected. Nothing was sent." });

    const proposer = fleet({ ok: true, jobs: [JOB] });
    const rootless = await sendToFleet(note(), note().box, sink({ root: async () => null }), proposer, AT);
    expect(rootless.ok).toBe(false);
    expect(proposer.proposeFromRequest).not.toHaveBeenCalled();
  });

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
    expect(await sendToSession([item], null, sink(), sessions(refused), AT)).toEqual({ ok: false, saying: "Session not started: Fleet is not connected. Nothing was sent." });
    const fleet = { ...sessions(), sendSessionMessage: vi.fn(async () => refused as never) };
    expect(await sendToSession([item], { id: "01T", title: "t" }, sink(), fleet, AT)).toEqual({ ok: false, saying: 'Notes not sent to "t": Fleet is not connected. Nothing was sent.' });
  });

  it("gives a refusal with no coded copy its message, and a plain reason where there is none", async () => {
    const refusal = (message: string) => ({ ok: false, outcome: { ok: false, why: "refused", error: { code: "fleet.unknown_manifest", message } } });
    expect(await sendToSession([item], null, sink(), sessions(refusal("No such manifest")), AT)).toEqual({ ok: false, saying: "Session not started: No such manifest" });
    expect(await sendToSession([item], null, sink(), sessions(refusal("")), AT)).toEqual({ ok: false, saying: "Session not started: Fleet refused it" });
  });
});
