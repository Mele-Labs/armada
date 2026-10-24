// What `session-attention.ts` tells and when: a new waiting item after the first reading, once per
// quiet window, in the tone its source sets. The effects are spies, so no Electron is in this.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { SessionRecord, WaitingItem } from "@armada/protocol";
import { QUIET_MS, SessionAttention } from "./session-attention";

const item = (id: string, source: WaitingItem["source"] = "agent", text = `text ${id}`, since = "2026-10-19T10:00:00Z"): WaitingItem => ({
  id,
  text,
  since,
  source,
});

function session(id: string, waiting: WaitingItem[] = [], extra: Partial<SessionRecord> = {}): SessionRecord {
  return {
    id,
    harness: "claude",
    origin: "hosted",
    cwd: "/x",
    state: "live",
    started_at: "2026-10-19T09:00:00Z",
    last_seen_at: "2026-10-19T09:00:00Z",
    usage: {},
    attachments: [],
    title: `Job ${id}`,
    ...(waiting.length > 0 ? { waiting_for: waiting } : {}),
    ...extra,
  } as SessionRecord;
}

function attention() {
  const show = vi.fn();
  const count = vi.fn();
  return { attention: new SessionAttention({ show, count }), show, count };
}

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe("SessionAttention", () => {
  it("tells nothing of what was already waiting at the first reading", () => {
    const { attention: a, show } = attention();
    a.read([session("a", [item("1")])], 1);
    vi.advanceTimersByTime(QUIET_MS * 2);
    expect(show).not.toHaveBeenCalled();
  });

  it("tells nothing before Fleet has answered once, and the next reading is still the first", () => {
    const { attention: a, show, count } = attention();
    a.read([], null);
    expect(count).not.toHaveBeenCalled();
    a.read([session("a", [item("1")])], 1);
    vi.advanceTimersByTime(QUIET_MS * 2);
    expect(show).not.toHaveBeenCalled();
  });

  it("tells a new item once the quiet window ends", () => {
    const { attention: a, show } = attention();
    a.read([session("a")], 1);
    a.read([session("a", [item("1", "agent", "Pick a slot")])], 2);
    vi.advanceTimersByTime(QUIET_MS - 1);
    expect(show).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(show).toHaveBeenCalledTimes(1);
    expect(show).toHaveBeenCalledWith({ sessionId: "a", title: "Job a", body: "Pick a slot", tone: "waiting" });
  });

  it("stays quiet for an item re-posted under the same id", () => {
    const { attention: a, show } = attention();
    a.read([session("a")], 1);
    a.read([session("a", [item("1")])], 2);
    vi.advanceTimersByTime(QUIET_MS);
    a.read([session("a", [{ ...item("1"), text: "worded again", since: "2026-10-19T11:00:00Z" }])], 3);
    vi.advanceTimersByTime(QUIET_MS * 2);
    expect(show).toHaveBeenCalledTimes(1);
  });

  it("tells two items inside the window as one banner that lands on the first", () => {
    const { attention: a, show } = attention();
    a.read([session("a"), session("b")], 1);
    a.read([session("a", [item("1", "agent", "First", "2026-10-19T10:00:00Z")]), session("b")], 2);
    vi.advanceTimersByTime(1500);
    a.read([session("a", [item("1")]), session("b", [item("2", "agent", "Second", "2026-10-19T10:00:01Z")])], 3);
    // The second entry does not push the window out.
    vi.advanceTimersByTime(500);
    expect(show).toHaveBeenCalledTimes(1);
    expect(show).toHaveBeenCalledWith({ sessionId: "a", title: "Job a", body: "First\nand 1 more", tone: "waiting" });
  });

  it.each(["ask_card", "permission", "window_ask", "plan", "slot_choice", "delivery"] as const)("sounds blocked for a %s item", (source) => {
    const { attention: a, show } = attention();
    a.read([session("a")], 1);
    a.read([session("a", [item("1", source)])], 2);
    vi.advanceTimersByTime(QUIET_MS);
    expect(show.mock.calls[0]![0].tone).toBe("blocked");
  });

  it.each(["agent", "walk", "first_message", "replied", "done", "judge"] as const)("sounds waiting for a %s item", (source) => {
    const { attention: a, show } = attention();
    a.read([session("a")], 1);
    a.read([session("a", [item("1", source)])], 2);
    vi.advanceTimersByTime(QUIET_MS);
    expect(show.mock.calls[0]![0].tone).toBe("waiting");
  });

  it("sounds blocked when one item of a batch is a call", () => {
    const { attention: a, show } = attention();
    a.read([session("a")], 1);
    a.read([session("a", [item("1", "walk"), item("2", "permission")])], 2);
    vi.advanceTimersByTime(QUIET_MS);
    expect(show.mock.calls[0]![0].tone).toBe("blocked");
  });

  it("tells the dock the count of Sessions waiting, on the first reading and then only when it changes", () => {
    const { attention: a, count } = attention();
    a.read([session("a", [item("1"), item("2")]), session("b")], 1);
    expect(count.mock.calls).toEqual([[1]]);
    a.read([session("a", [item("1"), item("2"), item("3")]), session("b")], 2);
    expect(count.mock.calls).toEqual([[1]]);
    a.read([session("a", [item("1")]), session("b", [item("4")])], 3);
    expect(count.mock.calls).toEqual([[1], [2]]);
    a.read([session("a"), session("b")], 4);
    expect(count.mock.calls).toEqual([[1], [2], [0]]);
  });

  it("does not count or tell for a Session that has ended", () => {
    const { attention: a, show, count } = attention();
    a.read([session("a", [item("1")], { state: "ended" })], 1);
    expect(count.mock.calls).toEqual([[0]]);
    a.read([session("a", [item("1")], { state: "ended" }), session("b")], 2);
    a.read([session("a", [item("1")], { state: "ended" }), session("b", [item("2")], { state: "ended" })], 3);
    vi.advanceTimersByTime(QUIET_MS);
    expect(show).not.toHaveBeenCalled();
    expect(count.mock.calls).toEqual([[0]]);
  });

  it("drops what is collected when closed", () => {
    const { attention: a, show } = attention();
    a.read([session("a")], 1);
    a.read([session("a", [item("1")])], 2);
    a.close();
    vi.advanceTimersByTime(QUIET_MS * 2);
    expect(show).not.toHaveBeenCalled();
  });

  it("counts a Job held on its start card as waiting, and tells one that arrives held as blocking", () => {
    const { attention: a, show, count } = attention();
    const held = (id: string): SessionRecord => session(id, [], { hosted: { start: { asks_delivery: true } } as SessionRecord["hosted"] });
    a.read([session("a")], 1);
    expect(count).toHaveBeenLastCalledWith(0);
    a.read([session("a"), held("b")], 2);
    expect(count).toHaveBeenLastCalledWith(1);
    vi.advanceTimersByTime(QUIET_MS);
    expect(show).toHaveBeenCalledWith({ sessionId: "b", title: "Job b", body: "Not started", tone: "blocked" });
    a.read([session("a"), session("b")], 3);
    expect(count).toHaveBeenLastCalledWith(0);
  });
});
