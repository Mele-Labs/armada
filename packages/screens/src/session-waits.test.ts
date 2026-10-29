// A Session that waits on another, between the wire and the screens: the wait a record holds, the condition a start sends, the
// condition a reading names, and the rows both threads carry.

import { describe, expect, it } from "vitest";

import type { SessionPlan, SessionRecord, SessionRow, SessionWait } from "@armada/protocol";
import { conditionOfWhen, waitOfWire, whenOfCondition } from "./session-waits";
import type { PlanPart } from "./session-waits";
import { rowsOfThread, sessionOfRecord } from "./sessions-wire";
import type { Beside } from "./sessions-wire";

const AT = "2026-10-16T13:48:02Z";

const record = (id: string, change: Partial<SessionRecord> = {}): SessionRecord => ({
  id,
  harness: "a_harness",
  origin: "bridge",
  manifest_id: "armada",
  cwd: "/repo",
  state: "live",
  started_at: AT,
  last_seen_at: AT,
  usage: {},
  attachments: [],
  hosted: { turn: { state: "idle" }, mode: "auto", running: false },
  ...change,
});

const beside: Beside = { picture: () => undefined, sketches: [], pending: [] };

const wait = (change: Partial<SessionWait> = {}): SessionWait => ({ on: { id: "a", title: "Retry loop" }, condition: { kind: "ends" }, base: "main", brief: "Write the docs", by: "you", ...change });

const PARTS: readonly PlanPart[] = [
  { id: "g1", title: "Schema", kind: "group" },
  { id: "t1", title: "Add the column", kind: "task" },
];

describe("the wait a record holds", () => {
  it("reads every condition as the screens draw it, a group or a task as a plan part titled as Fleet read it", () => {
    expect(waitOfWire(wait())).toEqual({ on: { id: "a", title: "Retry loop" }, when: "ends", base: "main", needs: "", brief: "Write the docs", by: "you" });
    expect(waitOfWire(wait({ condition: { kind: "group_done", group: "g1" }, part_title: "Schema", needs: "the schema" }))).toMatchObject({ when: "plan", part: { id: "g1", title: "Schema" }, needs: "the schema" });
    expect(waitOfWire(wait({ condition: { kind: "task_done", task: "t1" } }))).toMatchObject({ when: "plan", part: { id: "t1", title: "t1" } });
    expect(waitOfWire(wait({ condition: { kind: "pr_pushed" }, base: "branch", pull: { number: 12, branch: "retry/loop" }, by: "agent" }))).toMatchObject({ when: "pushed", base: "branch", pull: { number: 12, branch: "retry/loop" }, by: "agent" });
    expect(waitOfWire(wait({ condition: { kind: "pr_lands" } }))).toMatchObject({ when: "lands" });
  });

  it("puts it on a Session that waits and on no other, and not on one that ended", () => {
    expect(sessionOfRecord(record("w", { waits: wait() }), undefined, beside).waits).toMatchObject({ when: "ends", brief: "Write the docs" });
    expect(sessionOfRecord(record("w"), undefined, beside)).not.toHaveProperty("waits");
    expect(sessionOfRecord(record("w", { waits: wait(), state: "ended" }), undefined, beside)).not.toHaveProperty("waits");
  });
});

describe("the condition a start sends", () => {
  it("is a group or a task by what the plan of the Session waited on says it is", () => {
    expect(conditionOfWhen("plan", { id: "g1" }, PARTS)).toEqual({ kind: "group_done", group: "g1" });
    expect(conditionOfWhen("plan", { id: "t1" }, PARTS)).toEqual({ kind: "task_done", task: "t1" });
    expect(conditionOfWhen("pushed", undefined, PARTS)).toEqual({ kind: "pr_pushed" });
    expect(conditionOfWhen("lands", undefined, PARTS)).toEqual({ kind: "pr_lands" });
    expect(conditionOfWhen("ends", undefined, PARTS)).toEqual({ kind: "ends" });
  });

  it("is ends where the plan no longer holds the part, which a wrong id would have been refused for", () => {
    expect(conditionOfWhen("plan", { id: "gone" }, PARTS)).toEqual({ kind: "ends" });
    expect(conditionOfWhen("plan", undefined, PARTS)).toEqual({ kind: "ends" });
  });
});

describe("the condition a reading names", () => {
  it("is drawn with the plan part titled, and as ends where the part is not in the plan", () => {
    expect(whenOfCondition({ kind: "group_done", group: "g1" }, PARTS)).toEqual({ when: "plan", part: { id: "g1", title: "Schema" } });
    expect(whenOfCondition({ kind: "task_done", task: "t1" }, PARTS)).toEqual({ when: "plan", part: { id: "t1", title: "Add the column" } });
    expect(whenOfCondition({ kind: "group_done", group: "gone" }, PARTS)).toEqual({ when: "ends" });
    expect(whenOfCondition({ kind: "pr_pushed" }, PARTS)).toEqual({ when: "pushed" });
    expect(whenOfCondition({ kind: "pr_lands" }, PARTS)).toEqual({ when: "lands" });
    expect(whenOfCondition({ kind: "ends" }, PARTS)).toEqual({ when: "ends" });
  });
});

describe("the rows a wait writes", () => {
  const rows = (side?: "waiting"): SessionRow[] => [
    { kind: "waiter", id: "w1", at: AT, session: { id: "b", title: "Docs" }, event: "changed", ...(side === undefined ? {} : { side }), from: "It ends", to: "Its pull request lands", by: "agent" },
  ];

  it("draws a row in the thread of the Session waited on, naming the one that waits", () => {
    expect(rowsOfThread("a", rows(), () => undefined)).toEqual([expect.objectContaining({ kind: "waiter", session: { id: "b", title: "Docs" }, event: "changed", from: "It ends", to: "Its pull request lands", by: "agent" })]);
    expect(rowsOfThread("a", rows(), () => undefined)[0]).not.toHaveProperty("waiting");
  });

  it("marks the row in the thread of the waiting Session, which names the one it waits on", () => {
    expect(rowsOfThread("b", rows("waiting"), () => undefined)[0]).toMatchObject({ kind: "waiter", waiting: true });
  });
});

describe("a plan on the Session waited on", () => {
  it("is where the parts a condition names come from", () => {
    const plan: SessionPlan = {
      version: 1,
      summary: "",
      recorded_at: AT,
      approved: true,
      scope: [],
      groups: [{ id: "g1", agent: "rust-engineer", title: "Schema", paths: [], lands_in: { label: "PR 1" }, tasks: [{ id: "t1", text: "Add the column", state: "done" }] }],
    };
    const drawn = sessionOfRecord(record("a", { plan }), undefined, beside);
    expect(drawn.work?.plan?.groups[0]?.id).toBe("g1");
  });
});
