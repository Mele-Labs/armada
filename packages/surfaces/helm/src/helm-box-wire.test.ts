// What Helm's message box puts on the wire and takes off it, read as the TypeScript mirror types it: the
// exact JSON each shape comes as with optional fields absent rather than null, and the socket's messages
// folded. **Held here and not in `packages/protocol`**, which has no test runner (`thought-wire.test.ts`).

import { describe, expect, it } from "vitest";

import { helmArrived, NO_HELM_ITEMS } from "@armada/protocol";
import type { AskHelm, HelmBoxFacts, HelmMessage, IssueRead, SessionTag, TuneHelm } from "@armada/protocol";
import { helmRowsOf } from "./helm-thread";

/** What Fleet's JSON decodes to, typed as the mirror says. */
const decoded = <T>(json: string): T => JSON.parse(json) as T;

const ISSUE: SessionTag = { kind: "issue", id: "2214", title: "#2214 Sessions list drops a quiet Session", url: "https://forge.invalid/issues/2214" };
const PULL: SessionTag = { kind: "pull_request", id: "2219", title: "#2219 Pin the Bridge store clock", url: "https://forge.invalid/pull/2219" };
const FILE = { id: "file-1", name: "Screenshot.png", media_type: "image/png" };

const OPENED = { message: "opened", protocol_id: "0000000000000000", manifest_id: "01M1", replying: false, skipped: 0 } as const;

describe("ask_helm", () => {
  it("asks with the words alone when nothing is carried, the fields left out and never null", () => {
    const body: AskHelm = { text: "why did 12 stop", context: { screen: "board" } };
    expect(JSON.stringify(body)).toBe('{"text":"why did 12 stop","context":{"screen":"board"}}');
  });

  it("carries uploads and mentions beside the words, and the words may be blank", () => {
    const body: AskHelm = {
      text: "",
      attachments: [{ name: "Screenshot.png", media_type: "image/png", data: "iVBOR" }],
      mentions: [ISSUE],
    };
    expect(decoded<AskHelm>(JSON.stringify(body))).toEqual(body);
    expect(JSON.stringify(body)).toContain('"kind":"issue"');
    expect(JSON.stringify(body)).toContain('"url":"https://forge.invalid/issues/2214"');
  });
});

describe("tune_helm and the box's facts", () => {
  it("is Auto where a field is absent", () => {
    const body: TuneHelm = { effort: "high" };
    expect(JSON.stringify(body)).toBe('{"effort":"high"}');
  });

  it("answers the facts, whose model, effort and commands are each optional", () => {
    const facts = decoded<HelmBoxFacts>('{"model":"sonnet","commands":["review","init"]}');
    expect(facts).toEqual({ model: "sonnet", commands: ["review", "init"] });
    expect("effort" in facts).toBe(false);
  });
});

describe("get_issue", () => {
  it("answers one issue with where it stands", () => {
    const read = decoded<IssueRead>('{"number":61,"title":"Doctor reports a stale pid as running","url":"https://forge.invalid/issues/61","state":"closed"}');
    expect(read).toEqual({ number: 61, title: "Doctor reports a stale pid as running", url: "https://forge.invalid/issues/61", state: "closed" });
  });
});

describe("the thread's fold", () => {
  it("opens with the facts the socket was given", () => {
    const next = helmArrived(NO_HELM_ITEMS, { ...OPENED, facts: { model: "opus", commands: ["review"] } }, 0);
    expect(next.added).toBe(false);
    expect(next.held.facts).toEqual({ model: "opus", commands: ["review"] });
    expect(next.held.items).toEqual([]);
  });

  it("opens again onto a conversation whose facts replace the last one's, never merge with them", () => {
    const first = helmArrived(NO_HELM_ITEMS, { ...OPENED, facts: { model: "opus", effort: "high" } }, 0);
    const second = helmArrived(first.held, { ...OPENED, facts: {} }, 0);
    expect(second.held.facts).toEqual({});
  });

  it("keeps the files and tags an ask carried on its row", () => {
    const message: HelmMessage = { message: "asked", ts: "2026-10-12T10:00:00.000Z", text: "do these explain it?", files: [FILE], tags: [ISSUE, PULL] };
    const next = helmArrived(NO_HELM_ITEMS, message, 3);
    expect(next.added).toBe(true);
    expect(next.held.items).toEqual([
      { kind: "asked", id: "3", ts: "2026-10-12T10:00:00.000Z", text: "do these explain it?", files: [FILE], tags: [ISSUE, PULL] },
    ]);
  });

  it("leaves the files and tags off an ask that carried none", () => {
    const next = helmArrived(NO_HELM_ITEMS, { message: "asked", ts: "2026-10-12T10:00:00.000Z", text: "why" }, 0);
    const [item] = next.held.items;
    expect(item && "files" in item).toBe(false);
    expect(item && "tags" in item).toBe(false);
  });

  it("replaces the held facts with a facts message, and adds no item and counts none", () => {
    const opened = helmArrived(NO_HELM_ITEMS, { ...OPENED, facts: { model: "opus", effort: "high", commands: ["review"] } }, 0).held;
    const asked = helmArrived(opened, { message: "asked", ts: "2026-10-12T10:00:00.000Z", text: "why" }, 0).held;
    const next = helmArrived(asked, { message: "facts", effort: "low", commands: ["review", "init"] }, 1);
    expect(next.added).toBe(false);
    expect(next.held.items).toEqual(asked.items);
    expect(next.held.replying).toBe(asked.replying);
    // A model Fleet no longer names is Auto again: the message is the whole of the facts.
    expect(next.held.facts).toEqual({ effort: "low", commands: ["review", "init"] });
    expect("message" in next.held.facts).toBe(false);
  });

  it("keeps the facts when the thread it was showing is closed, since the socket reopens onto the new one", () => {
    const opened = helmArrived(NO_HELM_ITEMS, { ...OPENED, facts: { model: "opus" } }, 0).held;
    const next = helmArrived(opened, { message: "closed", because: "started_fresh" }, 0);
    expect(next.ended).toBe("started_fresh");
    expect(next.held.facts).toEqual({ model: "opus" });
  });
});

describe("words sent on to other work", () => {
  const TARGET = { kind: "session", id: "01S", title: "Sessions list views", manifest_id: "01M1", slot: 3 } as const;
  const SENT: HelmMessage = { message: "sent", ts: "2026-10-12T10:00:00.000Z", text: "the words", target: TARGET, files: [FILE], tags: [ISSUE] };

  it("keeps the files and tags the thought went with on its item", () => {
    const next = helmArrived(NO_HELM_ITEMS, SENT, 2);
    expect(next.held.items).toEqual([
      { kind: "sent", id: "2", ts: "2026-10-12T10:00:00.000Z", text: "the words", target: TARGET, files: [FILE], tags: [ISSUE] },
    ]);
  });

  it("keeps them after a reload, which folds the backfill again from an opened message", () => {
    const first = helmArrived(NO_HELM_ITEMS, SENT, 0).held;
    const reopened = helmArrived(first, { ...OPENED, facts: {} }, 0).held;
    expect(reopened.items).toEqual([]);
    const again = helmArrived(reopened, SENT, 0).held;
    expect(again.items).toEqual(first.items);
  });

  it("leaves the fields off a thought that went with nothing", () => {
    const [item] = helmArrived(NO_HELM_ITEMS, { message: "sent", ts: "2026-10-12T10:00:00.000Z", text: "x", target: TARGET }, 0).held.items;
    expect(item && "files" in item).toBe(false);
    expect(item && "tags" in item).toBe(false);
  });

  it("is folded to a row of the person's own with its files, tags and target", () => {
    const [item] = helmArrived(NO_HELM_ITEMS, SENT, 0).held.items;
    const [row] = helmRowsOf(item === undefined ? [] : [item]);
    expect(row).toMatchObject({ actor: "you", message: "the words", sent: TARGET, files: [FILE], tags: [ISSUE] });
  });
});

describe("the row of what a person sent", () => {
  it("is folded with its files and tags as Fleet kept them", () => {
    const [row] = helmRowsOf([{ kind: "asked", id: "0", ts: "2026-10-12T10:00:00.000Z", text: "see this", files: [FILE], tags: [ISSUE] }]);
    expect(row).toMatchObject({ actor: "you", message: "see this", files: [FILE], tags: [ISSUE] });
  });

  it("carries none where the ask carried none", () => {
    const [row] = helmRowsOf([{ kind: "asked", id: "0", ts: "2026-10-12T10:00:00.000Z", text: "see this" }]);
    expect(row && "files" in row).toBe(false);
    expect(row && "tags" in row).toBe(false);
  });
});
