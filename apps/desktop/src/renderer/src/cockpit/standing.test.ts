import { describe, expect, test } from "vitest";

import { asksAnAgent } from "./standing";

const call = (kind: string, key: string, decisions?: unknown) => ({ kind, key, ...(decisions === undefined ? {} : { decisions }) }) as Parameters<typeof asksAnAgent>[0];

describe("the standing answers", () => {
  test.each([
    ["a Plan question", call("Plan question", "job:p", [{ id: "d", question: "?", options: [] }])],
    ["a Drone's question", call("Drone question", "job:d")],
    ["a Judge's question", call("Judge question", "job:j")],
    ["a Session's ask", call("Session", "session:s9")],
  ])("stand on %s, which an agent asked and could answer itself", (_, item) => {
    expect(asksAnAgent(item)).toBe(true);
  });

  test.each([
    ["a failing pull request", call("Pull request", "pull:1819")],
    ["main gone red", call("Main", "main:/repo")],
    ["a failed Check", call("Check failed", "job:c")],
    ["a stuck Drone", call("Drone stuck", "job:s")],
    ["a Job at its review", call("Job", "job:r")],
  ])("do not stand on %s, which Fleet raises about a state", (_, item) => {
    expect(asksAnAgent(item)).toBe(false);
  });
});
