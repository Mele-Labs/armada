// What a reply is on the deck: a reply that asks him something is a `replied` item and a call under Needs you; a reply that only
// tells has no item, raises no call, and is calm with a quiet mark.

import type { Session } from "@armada/screens/src/draft/sessions";
import { expect, test } from "vitest";

import { callsFromWaiting } from "../waiting";
import { groupOf } from "./order";
import { shipOf } from "./ship";

const SINCE = "2026-10-18T09:00:00.000Z";

const reply = (id: string, extra: Partial<Session> = {}): Session => ({
  id,
  title: `Session ${id}`,
  turn: { state: "idle" },
  lastTurnAt: SINCE,
  standing: "replied",
  rows: [{ id: `${id}-a`, at: "09:00:00", kind: "message", from: { kind: "agent" }, text: "The retry loop is capped." }],
  attachments: [],
  ...extra,
});

const asks = reply("asks", { waitingFor: [{ id: `replied:${SINCE}`, text: "Should the cap be five or ten?", since: SINCE, source: "replied" }] });
const tells = reply("tells");

test("a reply that asks raises a call and sits under Needs you, and a reply that only tells raises none", () => {
  expect(callsFromWaiting([asks, tells]).map((one) => [one.session.id, one.item.text])).toEqual([["asks", "Should the cap be five or ten?"]]);

  const ship = shipOf({ sessions: [asks, tells], lines: [], slots: [], repositories: [], calls: [], now: Date.parse(SINCE) + 60_000 });
  const of = (id: string) => ship.subjects.find((one) => one.id === id)!;
  expect(of("asks").needs).toBe(true);
  expect(of("asks").call).toBeDefined();
  expect(groupOf(of("asks"))).toBe("needs");
  expect(of("tells").needs).toBe(false);
  expect(of("tells").call).toBeUndefined();
  expect(of("tells").reply).toBe(true);
  expect(groupOf(of("tells"))).not.toBe("needs");
  expect(ship.calls.map((one) => one.key)).toEqual([expect.stringContaining("session:asks")]);
});

test("a handed-off Job is a past subject behind the new one, which names it as earlier and carries a ring or nothing", () => {
  const old = reply("old", { dead: "ended", replacedBy: "new", attachments: [] });
  const made = reply("new", { replaces: "old", handoff: "starting" });
  const ship = shipOf({ sessions: [old, made], lines: [], slots: [], repositories: [], calls: [], now: Date.parse(SINCE) + 60_000 });
  const of = (id: string) => ship.subjects.find((one) => one.id === id)!;
  expect(of("old")).toMatchObject({ past: "ended", behind: true });
  expect(of("new")).toMatchObject({ earlier: "old", handoff: "starting" });
  expect(of("new").behind).toBeUndefined();
});

test("a Job whose own turn ended while its subagent runs is working, not done", () => {
  const running = { kind: "subagent", id: "sub-1", task: "Build the Fleet side of Kit's install", state: "running" } as const;
  const held = reply("held", { standing: "working", attachments: [running] });
  const ship = shipOf({ sessions: [held], lines: [], slots: [], repositories: [], calls: [], now: Date.parse(SINCE) + 60_000 });
  const subject = ship.subjects.find((one) => one.id === "held")!;
  expect(subject.phase).toBe("working");
  expect(subject.stoppable).toBe(false);
});

test("a Job held on its start card is a draft: its row is quiet in the draft hue, and its call stays after he has seen it", () => {
  const held = reply("held", { standing: undefined, lastTurnAt: undefined, start: { asksDelivery: true } });
  const ship = shipOf({ sessions: [held], lines: [], slots: [], repositories: [], calls: [], now: Date.parse(SINCE) + 60_000 });
  const subject = ship.subjects.find((one) => one.id === "held")!;
  expect(subject.mark.bucket).toBe("draft");
  expect(subject.needs).toBe(false);
  expect(subject.call).toBeDefined();
  expect(ship.asking.has(subject.key)).toBe(true);

  const begun = reply("begun", { standing: undefined, lastTurnAt: undefined });
  const after = shipOf({ sessions: [begun], lines: [], slots: [], repositories: [], calls: [], now: Date.parse(SINCE) + 60_000 });
  expect(after.asking.has("session:begun")).toBe(false);
});

test("a Job at work with a plan put to him is under Needs you, and its call stays after he has seen it", () => {
  const plan = { id: "plan:1", text: "Plan to approve", since: SINCE, source: "plan" } as const;
  const at = reply("at", { turn: { state: "working" }, standing: "working", waitingFor: [plan] });
  const ship = shipOf({ sessions: [at], lines: [], slots: [], repositories: [], calls: [], now: Date.parse(SINCE) + 60_000 });
  const subject = ship.subjects.find((one) => one.id === "at")!;
  expect(subject.needs).toBe(true);
  expect(groupOf(subject)).toBe("needs");
  expect(subject.call).toBeDefined();
  expect(ship.asking.has(subject.key)).toBe(true);
});

test("a Job holding a question nobody answered, with no item behind it, is under Needs you and its row pulses", () => {
  const night = { question: "Which night?", header: "Night", multi_select: false, options: [{ label: "Thursday", description: "" }, { label: "Friday", description: "" }] };
  const lapsed = reply("lapsed", { lapsed: { id: "l3", questions: [night] } });
  const ship = shipOf({ sessions: [lapsed], lines: [], slots: [], repositories: [], calls: [], now: Date.parse(SINCE) + 60_000 });
  const subject = ship.subjects.find((one) => one.id === "lapsed")!;
  expect(subject.mark.said).toBe("Waiting on you");
  expect(subject.needs).toBe(true);
  expect(groupOf(subject)).toBe("needs");
  expect(ship.asking.has(subject.key)).toBe(true);
});

test("a Mod Job's subject carries the mark the list groups it by, and another's does not", () => {
  const ship = shipOf({ sessions: [reply("mod", { mod: true }), reply("plain")], lines: [], slots: [], repositories: [], calls: [], now: Date.parse(SINCE) + 60_000 });
  const of = (id: string) => ship.subjects.find((one) => one.id === id)!;
  expect(of("mod").mod).toBe(true);
  expect(of("plain").mod).toBeUndefined();
});

test("each Job's mark wears one of five buckets: stuck and red Checks with the agent idle need him, a turn works, a held limit waits, the rest is idle", () => {
  const red = { kind: "pull_request" as const, number: 7, title: "Trim the cache", branch: "trim", address: "https://git.example/pull/7", checks: { state: "failed" as const, failing: "unit" }, state: "open" as const, auto: false };
  const sessions = [
    reply("stuck", { turn: { state: "working" }, standing: "stuck" }),
    reply("red", { standing: "idle", attachments: [red] }),
    reply("working", { turn: { state: "working" }, standing: "working" }),
    reply("limited", { standing: "idle", limitedUntil: "2026-10-18T14:00:00.000Z" }),
    reply("idle", { standing: "idle" }),
  ];
  const ship = shipOf({ sessions, lines: [], slots: [], repositories: [], calls: [], now: Date.parse(SINCE) + 60_000 });
  const of = (id: string) => ship.subjects.find((one) => one.id === id)!;
  expect(sessions.map((one) => [one.id, of(one.id).mark.bucket, of(one.id).mark.tone])).toEqual([
    ["stuck", "needs", "ask"],
    ["red", "needs", "ask"],
    ["working", "working", "calm"],
    ["limited", "waiting", "calm"],
    ["idle", "idle", "calm"],
  ]);
  expect(groupOf(of("stuck"))).toBe("needs");
  expect(groupOf(of("red"))).toBe("needs");
});
