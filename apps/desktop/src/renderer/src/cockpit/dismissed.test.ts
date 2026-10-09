import { afterEach, beforeEach, describe, expect, test, vi } from "vitest";

import { callsFromWaiting } from "./waiting";
import { dismissCall, dismissedCalls, dismissItem, dismissWaiting, forgetCleared, forgetDismissals, identityOf } from "./dismissed";

type Held = Parameters<typeof callsFromWaiting>[0][number];

const session = (id: string, ids: string[]): Held =>
  ({
    id,
    title: id,
    turn: { state: "idle" },
    rows: [],
    attachments: [],
    waitingFor: ids.map((item) => ({ id: item, text: item, since: "2026-10-09T09:00:00.000Z", source: "agent" })),
  }) as unknown as Held;

/** A window with a storage that remembers, and `armada` as given. */
function windowWith(armada: object = {}) {
  const store = new Map<string, string>();
  vi.stubGlobal("window", {
    armada,
    localStorage: { getItem: (key: string) => store.get(key) ?? null, setItem: (key: string, value: string) => void store.set(key, value), removeItem: (key: string) => void store.delete(key) },
  });
  return store;
}

beforeEach(() => windowWith());
afterEach(() => {
  forgetDismissals();
  vi.unstubAllGlobals();
});

const refusal = (code: string, message: string) => ({ ok: false as const, outcome: { ok: false as const, why: "refused" as const, error: { code, message, run_id: "", fields: {}, chain: [] as string[] } } });

describe("dismissing what a Session waits on", () => {
  test("until Fleet has the route it is remembered by session and item, and the call is gone", async () => {
    const sessions = [session("s1", ["ask:q1", "walk:u"])];
    expect(await dismissWaiting("s1", "ask:q1")).toEqual({ kind: "dismissed" });
    expect([...dismissedCalls()]).toEqual(["s1:ask:q1"]);
    expect(callsFromWaiting(sessions, dismissedCalls()).map((one) => one.item.id)).toEqual(["walk:u"]);
  });

  test("it is kept across a restart", async () => {
    const store = windowWith();
    await dismissWaiting("s1", "ask:q1");
    expect(JSON.parse(store.get("armada.bridge.dismissed-calls") ?? "[]")).toEqual(["s1:ask:q1"]);
  });

  test("with the route it goes to Fleet and nothing is kept here", async () => {
    const route = vi.fn(async () => ({ ok: true as const, value: {} }));
    windowWith({ dismissWaiting: route });
    expect(await dismissWaiting("s1", "ask:q1")).toEqual({ kind: "dismissed" });
    expect(route.mock.calls).toEqual([[{ session_id: "s1", item_id: "ask:q1" }]]);
    expect(dismissedCalls().size).toBe(0);
  });

  test("an item nothing holds is already gone, and any other refusal is told in Fleet's words", async () => {
    windowWith({ dismissWaiting: async () => refusal("fleet.session_waiting_unheld", "nothing is waiting under that item.") });
    expect(await dismissWaiting("s1", "ask:q1")).toEqual({ kind: "dismissed" });
    windowWith({ dismissWaiting: async () => refusal("fleet.session_closed", "session s1 was closed") });
    expect(await dismissWaiting("s1", "ask:q1")).toEqual({ kind: "refused", said: "session s1 was closed" });
  });
});

describe("dismissing a call Fleet raised about a state", () => {
  const failing = { key: "pull:1823", fact: "checks failed" };

  test("is remembered by the state, so another failure of the same pull request is a different call", () => {
    dismissCall(failing);
    expect(dismissedCalls().has(identityOf(failing))).toBe(true);
    expect(dismissedCalls().has(identityOf({ key: "pull:1823", fact: "unmergeable" }))).toBe(false);
    expect(dismissedCalls().has(identityOf({ key: "pull:1824", fact: "checks failed" }))).toBe(false);
  });

  test("is let go once the state no longer raises it, so the next failure shows", () => {
    dismissCall(failing);
    forgetCleared(new Set([identityOf(failing)]), true);
    expect(dismissedCalls().size).toBe(1);
    forgetCleared(new Set(), false);
    expect(dismissedCalls().size).toBe(1);
    forgetCleared(new Set(), true);
    expect(dismissedCalls().size).toBe(0);
  });

  test("a Session's dismissal is not let go with them", async () => {
    await dismissWaiting("s1", "ask:q1");
    forgetCleared(new Set(), true);
    expect(dismissedCalls().size).toBe(1);
  });

  test("dismissItem keeps a state's call here, and tells nobody", async () => {
    const tell = vi.fn();
    await dismissItem(failing, tell);
    expect(dismissedCalls().has(identityOf(failing))).toBe(true);
    expect(tell).not.toHaveBeenCalled();
  });
});
