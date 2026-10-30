import { describe, expect, test } from "vitest";

import { asksAnAgent } from "./standing";

const call = (kind: string, key: string, decisions?: unknown, source?: string, act?: { kind: string; target: string }) =>
  ({ kind, key, ...(decisions === undefined ? {} : { decisions }), ...(source === undefined ? {} : { waiting: { sessionId: "s", item: { source, ...(act === undefined ? {} : { act }) } } }) }) as unknown as Parameters<typeof asksAnAgent>[0];

describe("the standing answers", () => {
  test.each([
    ["a Session's permission", call("Session", "session:s9:perm:x", undefined, "permission")],
    ["a Session's own question", call("Job question", "session:s9:ask:x:0", undefined, "ask_card")],
  ])("stand on %s, which an agent asked and could answer itself", (_, item) => {
    expect(asksAnAgent(item)).toBe(true);
  });

  test.each([
    ["a failing pull request", call("Pull request", "pull:1819")],
    ["main gone red", call("Main", "main:/repo")],
    ["a page a Session asks him to look at", call("Job walk", "session:s9:walk:u", undefined, "walk")],
    ["a Session's plain line, an errand for him", call("Session", "session:s9:ask:y", undefined, "agent")],
    ["a terminal's own permission prompt, answered at the terminal", call("Session", "session:t1:perm:x", undefined, "permission", { kind: "message", target: "t1" })],
  ])("do not stand on %s, which Fleet raises about a state", (_, item) => {
    expect(asksAnAgent(item)).toBe(false);
  });
});

/** A Session's waiting item as the call carries it: its source, and what it offers or acts on. */
const line = (source: string, extra: Record<string, unknown> = {}) =>
  ({ kind: "Session", key: "session:s9:x", waiting: { sessionId: "s9", item: { id: "x", text: "t", since: "", source, ...extra } } }) as unknown as Parameters<typeof asksAnAgent>[0];

describe("a Session's line follows its act", () => {
  test.each([
    ["a command to run", line("agent", { act: { kind: "run", target: "/land 2109" } })],
    ["a shell command to run", line("agent", { act: { kind: "run", target: "scripts/preview --restart --adopt" } })],
    ["a pull request to approve", line("agent", { act: { kind: "approve_pr", target: "2118" } })],
    ["a page to open", line("agent", { act: { kind: "walk", target: "https://git.example/page" } })],
    ["no act and no options", line("agent")],
    ["an empty list of options", line("agent", { options: [] })],
    ["a draft, which is his to OK and never the agent's", line("draft", { options: [{ label: "Approve" }] })],
    ["a sketch, which is his to approve as a draft is", line("sketch", { act: { kind: "message", target: "s9" }, options: [{ label: "Approve" }] })],
  ])("carries no standing answers on %s", (_, item) => {
    expect(asksAnAgent(item)).toBe(false);
  });

  test.each([
    ["a permission", line("permission", { options: [{ label: "Allow once" }] })],
    ["a permission with no options", line("permission")],
    ["an ask card", line("ask_card", { options: [{ label: "Fake" }] })],
    ["an ask card with no options", line("ask_card")],
    ["a question asked in a window", line("window_ask", { options: [{ label: "Yes" }] })],
    ["an agent's line with options", line("agent", { options: [{ label: "Fake" }, { label: "Frozen" }] })],
    ["an agent's line with options and an act", line("agent", { act: { kind: "run", target: "/land 1" }, options: [{ label: "Yes" }] })],
  ])("keeps them on %s", (_, item) => {
    expect(asksAnAgent(item)).toBe(true);
  });
});
