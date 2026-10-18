// The message a Session's Report starts a new Session with: the owner's words first and untouched, every fact Bridge holds as a
// labelled line, nothing for a fact it does not hold, and the last few of each list.

import type { TimelineEvent } from "@armada/protocol";
import type { Session, SessionRow } from "@armada/screens/src/draft/sessions";
import { expect, test } from "vitest";

import { reportBrief, reportFacts, reportLists, transcriptOf, type ReportFrom } from "./report-brief";

const ID = "6f2b9c1e-3d47-4a8b-9e10-5c7d2a4b8f31";
const message = (n: number, from: "you" | "agent"): SessionRow => ({ id: `m${n}`, at: `13:0${n}:00`, kind: "message", from: { kind: from }, text: `said ${n}` });
const tool = (n: number): SessionRow => ({ id: `t${n}`, at: `14:0${n}:00`, kind: "tool", text: `Bash step ${n}` });

const bare: Session = { id: ID, title: "Pin the store clock", manifestId: "m", turn: { state: "idle" }, rows: [], attachments: [] } as unknown as Session;

const full: Session = {
  ...bare,
  cwd: "/Users/user/repo",
  model: "opus",
  effort: "high",
  context: { tokens: 142_000, window: 200_000 },
  rows: [...Array.from({ length: 8 }, (_, n) => message(n, n % 2 === 0 ? "you" : "agent")), ...Array.from({ length: 7 }, (_, n) => tool(n))],
  attachments: [
    { kind: "branch", name: "fix/pin-store-clock", slot: 4 },
    { kind: "pull_request", number: 1861, title: "Pin the store clock", branch: "fix/pin-store-clock", address: "https://git.example/pull/1861", checks: { state: "failed", failing: "desktop_test" }, state: "open", auto: false },
  ],
  waitingFor: [{ id: "q", text: "Revert the push to main, or leave it?", since: "2026-10-15T19:25:00.000Z", source: "agent" }],
} as unknown as Session;

const event = (id: number, subject: string, says: string): TimelineEvent =>
  ({ id, at: `2026-10-15T19:0${id}:00.000Z`, kind: "session.turn", tone: "neutral", says, subject: { kind: "session", id: subject } }) as unknown as TimelineEvent;

const from = (change: Partial<ReportFrom> = {}): ReportFrom => ({ at: new Date("2026-10-15T19:30:00.000Z"), screen: "Cockpit, flight controls", root: "/Users/user/repo", events: [], ...change });

test("the transcript is under the working directory with every non-alphanumeric as a dash", () => {
  expect(transcriptOf("/Users/a.b/my repo", "s1")).toBe("~/.claude/projects/-Users-a-b-my-repo/s1.jsonl");
});

test("the owner's words come first, untouched, and the brief ends with what to do", () => {
  const brief = reportBrief("  It pushed to main.\nI asked for a pull request.  ", full, from());
  expect(brief.startsWith("It pushed to main.\nI asked for a pull request.\n")).toBe(true);
  expect(brief.trimEnd().endsWith("If it is not small, file an issue.")).toBe(true);
});

test("every fact Bridge holds is a labelled line", () => {
  const lines = reportFacts(full, from({ repository: "repo" }));
  expect(lines).toEqual(
    expect.arrayContaining([
      `Job: Pin the store clock (${ID})`,
      `Claude Code session id: ${ID}`,
      "Repository: repo, /Users/user/repo",
      `Transcript: ~/.claude/projects/-Users-user-repo/${ID}.jsonl`,
      "Branch: fix/pin-store-clock, slot 4, /Users/user/repo/.armada/slots/slot-4",
      "State: idle",
      "Model: opus, effort high",
      "Context: 142k of 200k tokens",
      "Pull request: #1861 Pin the store clock, open, checks failed (desktop_test), branch fix/pin-store-clock, https://git.example/pull/1861",
      "Waiting on the owner: Revert the push to main, or leave it? (agent, since 2026-10-15T19:25:00.000Z)",
    ]),
  );
});

test("a fact Bridge does not hold is no line", () => {
  const labels = reportFacts(bare, from()).map((one) => one.split(":")[0]);
  for (const missing of ["Started in", "Transcript", "Branch", "Slot", "Model", "Context", "Pull request", "Waiting on the owner", "Last Job act Fleet refused"]) expect(labels).not.toContain(missing);
  expect(reportLists(bare, from())).toEqual([]);
});

test("an ended Job reads as ended", () => {
  expect(reportFacts({ ...bare, dead: "ended" } as Session, from())).toContain("State: ended");
});

test("the lists keep the last messages, tool calls and this Session's own timeline events, oldest first", () => {
  const events = [event(1, ID, "one"), event(2, "other", "not this"), ...[3, 4, 5, 6, 7, 8].map((n) => event(n, ID, `e${n}`))];
  const lists = reportLists(full, from({ events }));
  const after = (head: string) => {
    const rest = lists.slice(lists.indexOf(head) + 1);
    const end = rest.findIndex((one) => !one.startsWith("- "));
    return end === -1 ? rest : rest.slice(0, end);
  };
  expect(after("Last messages, oldest first:")).toEqual([2, 3, 4, 5, 6, 7].map((n) => `- ${n % 2 === 0 ? "Owner" : "Agent"} at 13:0${n}:00: said ${n}`));
  expect(after("Last tool calls, oldest first:")).toEqual([2, 3, 4, 5, 6].map((n) => `- 14:0${n}:00 Bash step ${n}`));
  expect(after("Last timeline events, oldest first:").map((one) => one.split(" session.turn: ")[1])).toEqual(["e4", "e5", "e6", "e7", "e8"]);
});
