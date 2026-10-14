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
