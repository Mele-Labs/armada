import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { forgetViewed, markViewed } from "./cockpit/viewed";
import { stateOf } from "./session-state";

type Held = Parameters<typeof stateOf>[0];

const TURN_ENDED = "2026-10-09T10:00:00.000Z";
const at = Date.parse(TURN_ENDED);

const session = (extra: Record<string, unknown>): Held =>
  ({ id: "s1", title: "Pin the clock", turn: { state: "idle" }, lastTurnAt: TURN_ENDED, rows: [{ id: "m", at: "10:00:00", kind: "message", from: { kind: "agent" }, text: "Pinned." }], attachments: [], standing: "replied", ...extra }) as unknown as Held;

beforeEach(() => {
  const store = new Map<string, string>();
  vi.stubGlobal("window", {
    localStorage: { getItem: (key: string) => store.get(key) ?? null, setItem: (key: string, value: string) => void store.set(key, value), removeItem: (key: string) => void store.delete(key) },
  });
  forgetViewed();
});
afterEach(() => vi.unstubAllGlobals());

describe("a reply is unread until he has been in the Session since it ended", () => {
  test("never opened, it reads as an unread reply", () => {
    expect(stateOf(session({}))).toEqual({ state: "replied", said: "Unread reply" });
  });

  test("opened before the turn ended, it still does", () => {
    markViewed("s1", at - 1);
    expect(stateOf(session({})).state).toBe("replied");
  });

  test("opened after it, the Session stands as idle", () => {
    markViewed("s1", at + 1);
    expect(stateOf(session({}))).toEqual({ state: "idle", said: "Idle" });
  });

  test("a done is not read away by opening the Session", () => {
    markViewed("s1", at + 1);
    expect(stateOf(session({ standing: "done" })).state).toBe("done");
  });

  test("a reply that came with an item the agent stated is waiting on him, not a reply", () => {
    const stated = { id: "a", text: "Pick one", since: TURN_ENDED, source: "agent" };
    expect(stateOf(session({ waitingFor: [stated] })).state).toBe("waiting");
  });
});

describe("working is what Fleet derived, not only a turn of its own", () => {
  test("a Session whose own turn ended, waiting on its subagents, is working", () => {
    expect(stateOf(session({ standing: "working" }))).toEqual({ state: "working", said: "Working" });
  });

  test("a terminal Session, which reports no turn of its own, is working while Fleet says so", () => {
    expect(stateOf(session({ turn: { state: "idle" }, standing: "working", lastTurnAt: undefined })).state).toBe("working");
  });
});

describe("the delivery question holds the Session on him", () => {
  test("a Session at work with the question on it is waiting on him, whatever its turn says", () => {
    const options = [{ label: "Auto land" }, { label: "Open PR only" }, { label: "Keep local" }];
    const question = { id: "delivery", text: "How should this work be delivered?", since: TURN_ENDED, source: "delivery", options };
    expect(stateOf(session({ turn: { state: "working" }, standing: "working", waitingFor: [question] }))).toEqual({ state: "waiting", said: "Waiting on you" });
  });
});

describe("a full pool holds the Session on him", () => {
  test("a Session at work with the pool's call on it is waiting on him, whatever its turn says", () => {
    const choice = { id: "slot-choice", text: "Every slot is in use", since: TURN_ENDED, source: "slot_choice", options: [{ label: "Wait for a slot" }, { label: "Add a slot" }] };
    expect(stateOf(session({ turn: { state: "working" }, standing: "working", waitingFor: [choice] }))).toEqual({ state: "waiting", said: "Waiting on you" });
  });
});

describe("a Job holding a first message nobody has sent is a draft", () => {
  const first = { id: "unsent:s1", text: "First message", since: TURN_ENDED, source: "first_message", options: [] };

  test("its unsent message makes it Draft, ahead of what it waits on him for", () => {
    expect(stateOf(session({ unsent: "Read the loader.", standing: "idle", lastTurnAt: undefined, rows: [], waitingFor: [first] }))).toEqual({ state: "draft", said: "Draft" });
  });

  test("while its request is read, it is still Draft", () => {
    expect(stateOf(session({ unsent: "Read the loader.", work: { proposal: { reading: true } } })).state).toBe("draft");
  });

  test("once sent it is not", () => {
    expect(stateOf(session({ turn: { state: "working" }, standing: "working" })).state).toBe("working");
  });
});
