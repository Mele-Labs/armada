import { describe, expect, test } from "vitest";

import { asksAnAgent } from "./standing";

const call = (kind: string, key: string, decisions?: unknown, source?: string) =>
  ({ kind, key, ...(decisions === undefined ? {} : { decisions }), ...(source === undefined ? {} : { waiting: { sessionId: "s", item: { source } } }) }) as unknown as Parameters<typeof asksAnAgent>[0];

describe("the standing answers", () => {
  test.each([
    ["a Plan question", call("Plan question", "job:p", [{ id: "d", question: "?", options: [] }])],
    ["a Drone's question", call("Drone question", "job:d")],
    ["a Judge's question", call("Judge question", "job:j")],
    ["a Session's permission", call("Session", "session:s9:perm:x", undefined, "permission")],
    ["a Session's own question", call("Session question", "session:s9:ask:x:0", undefined, "ask_card")],
    ["an agent's free question", call("Session", "session:s9:ask:y", undefined, "agent")],
  ])("stand on %s, which an agent asked and could answer itself", (_, item) => {
    expect(asksAnAgent(item)).toBe(true);
  });

  test.each([
    ["a failing pull request", call("Pull request", "pull:1819")],
    ["main gone red", call("Main", "main:/repo")],
    ["a failed Check", call("Check failed", "job:c")],
    ["a stuck Drone", call("Drone stuck", "job:s")],
    ["a Job at its review", call("Job", "job:r")],
    ["a page a Session asks him to look at", call("Session walk", "session:s9:walk:u", undefined, "walk")],
  ])("do not stand on %s, which Fleet raises about a state", (_, item) => {
    expect(asksAnAgent(item)).toBe(false);
  });
});
