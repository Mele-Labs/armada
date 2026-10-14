import { describe, expect, it } from "vitest";

import type { Session } from "@armada/screens/src/draft/sessions";

import { QUIET_AFTER_MS, lineOf, nowOf, quietOf, todosOf } from "./session-activity";

const NOW = Date.parse("2026-10-14T14:00:00Z");
const ago = (ms: number) => new Date(NOW - ms).toISOString();

const session = (change: Partial<Session> = {}): Session =>
  ({ id: "a", attachments: [], rows: [], turn: { state: "working" }, ...change }) as Session;

describe("a running turn that has made no tool call for a while", () => {
  it("is marked from ten minutes, and not before", () => {
    const under = session({ activity: { quietSince: ago(QUIET_AFTER_MS - 1) } });
    const over = session({ activity: { quietSince: ago(QUIET_AFTER_MS) } });
    expect(quietOf(under, NOW)).toBeUndefined();
    expect(quietOf(over, NOW)).toMatch(/^No tool call since \d{1,2}:\d{2} [AP]M$/);
  });
});

describe("the todos", () => {
  it("come in the order the band draws them: doing, then to do, then done", () => {
    const todos = [
      { text: "a", state: "done" as const },
      { text: "b", state: "todo" as const },
      { text: "c", state: "doing" as const },
      { text: "d", state: "done" as const },
    ];
    expect(todosOf(session({ activity: { todos } })).map((one) => one.text)).toEqual(["c", "b", "a", "d"]);
  });

  it("headline a tile with the one being done, else what Fleet detected", () => {
    const activity = { now: "Editing gate.rs", todos: [{ text: "Run it on every Job", state: "doing" as const }] };
    expect(lineOf(session({ activity }))).toBe("Run it on every Job");
    expect(lineOf(session({ activity: { now: "Editing gate.rs" } }))).toBe("Editing gate.rs");
  });
});

describe("a Session whose turn is not running", () => {
  const activity = { now: "Editing gate.rs", quietSince: ago(QUIET_AFTER_MS * 4), todos: [{ text: "Run it", state: "doing" as const }] };

  it.each([
    ["idle", session({ activity, turn: { state: "idle" } })],
    ["ended", session({ activity, dead: "ended" })],
  ])("draws nothing when it is %s", (_, one) => {
    expect(quietOf(one, NOW)).toBeUndefined();
    expect(todosOf(one)).toEqual([]);
    expect(nowOf(one)).toBeUndefined();
    expect(lineOf(one)).toBeUndefined();
  });

  it("draws nothing for one with no activity", () => {
    expect(lineOf(session())).toBeUndefined();
    expect(todosOf(session())).toEqual([]);
  });
});
