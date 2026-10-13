// The Commander's thread as the Job's page draws it, from what its socket carries. Over the wire's
// own shapes (`packages/protocol/src/commander.ts`): a row says what the session did and nothing
// more, so an act states the figure its call carried, and the card puts the Commander at `waiting`.

import { describe, expect, it } from "vitest";

import type { CommanderCard, CommanderThread, CommanderThreadItem, Saw } from "@armada/protocol";

import { commanderThreadOf, commanderViewOf } from "./commander-rows";

const JOB = "01M3WJ6FGZ003DCX123T7W6YP0";
const TS = "2026-10-13T10:00:00.000Z";

const row = (id: string, saw: Saw, by: "drone" | "armada" | "fleet" = "drone"): CommanderThreadItem => ({ kind: "row", id, turn: { ts: TS, seq: Number(id), by, saw } });
const called = (id: string, tool: string, detail = JOB): CommanderThreadItem => row(id, { event: "called", tool, call: `call-${id}`, detail, truncated: false });

describe("the thread's rows", () => {
  it("draws what woke it, and what the owner said, as the owner's own words", () => {
    const rows = commanderThreadOf(
      [
        { kind: "woke", id: "0", ts: TS, why: "Drone on T7 ended with nothing handed in" },
        { kind: "asked", id: "1", ts: TS, text: "why did it stop?" },
      ],
      JOB,
    );
    expect(rows).toMatchObject([
      { id: "0", kind: "woke", why: "Drone on T7 ended with nothing handed in" },
      { id: "1", kind: "message", from: "you", text: "why did it stop?" },
    ]);
  });

  it("draws what the Commander said, and not the brief Fleet sent that the harness replayed", () => {
    const rows = commanderThreadOf([row("0", { event: "said", text: "T7 hit the cap." }), row("1", { event: "said", text: "Instruction from Fleet" }, "armada")], JOB);
    expect(rows).toMatchObject([{ kind: "message", from: "agent", text: "T7 hit the cap." }]);
    expect(rows).toHaveLength(1);
  });

  it("draws an act with the figure the call carried, and no old value", () => {
    const rows = commanderThreadOf(
      [
        called("0", "mcp__armada__raise_turn_cap", "600"),
        called("1", "mcp__armada__raise_cost_cap", "$12"),
        called("2", "mcp__armada__restart_task", "T7"),
        called("3", "mcp__armada__edit_task", "T7"),
        called("4", "mcp__armada__add_task", "Draw the ask on the card"),
        called("5", "mcp__armada__drop_task", "T4"),
        called("6", "mcp__armada__move_plan", "T4 to G3"),
        called("7", "mcp__armada__send_repair_drone", "merge_conflict"),
      ],
      JOB,
    );
    expect(rows.map((one) => (one.kind === "act" ? [one.said, one.text] : one.kind))).toEqual([
      ["Raised cap", "Turn cap 600"],
      ["Raised cap", "Cost cap $12"],
      ["Restarted task", "T7 restarted"],
      ["Edited task", "T7 edited"],
      ["Added task", "Added Draw the ask on the card"],
      ["Dropped task", "T4 dropped"],
      ["Moved in plan", "Moved T4 to G3"],
      ["Sent repair Drone", "Repair: merge_conflict"],
    ]);
  });

  it("draws an act by name alone where the call carried no figure, or only the Job's own id", () => {
    const rows = commanderThreadOf([called("0", "mcp__armada__raise_turn_cap"), called("1", "mcp__armada__raise_cost_cap"), called("2", "mcp__armada__restart_task", "")], JOB);
    expect(rows.map((one) => (one.kind === "act" ? one.text : one.kind))).toEqual(["Turn cap", "Cost cap", "Task restarted"]);
  });

  it("draws any other call as a quiet tool row, without the server's name or the Job's own id", () => {
    const rows = commanderThreadOf([called("0", "mcp__armada__get_drone"), called("1", "mcp__armada__get_job_log", "build.log")], JOB);
    expect(rows).toMatchObject([
      { kind: "tool", text: "get_drone" },
      { kind: "tool", text: "get_job_log build.log" },
    ]);
  });

  it("draws nothing for what a turn leaves behind: its end, and a refused call", () => {
    const rows = commanderThreadOf(
      [row("0", { event: "ended", turns: 3, cost_micros: 12_000, refusals: 1 }), row("1", { event: "refused", tool: "Edit", call: "c1", because: "no write tool" })],
      JOB,
    );
    expect(rows).toEqual([]);
  });

  it("says why no reply came, in Fleet's own sentence", () => {
    const rows = commanderThreadOf([{ kind: "unanswered", id: "0", ts: TS, why: "the agent did not answer in 20 minutes" }], JOB);
    expect(rows).toMatchObject([{ kind: "message", from: "agent", text: "the agent did not answer in 20 minutes" }]);
  });
});

describe("the panel's state", () => {
  const open = (over: Partial<Extract<CommanderThread, { state: "open" }>> = {}): CommanderThread => ({
    state: "open",
    jobId: JOB,
    replying: false,
    skipped: 0,
    missed: 0,
    items: [],
    ...over,
  });
  const CARD: CommanderCard = { card_id: "01CARD", job_id: JOB, by: "commander", raised_at: TS, text: "t", reach: "r", question: "q", options: [{ id: "1", label: "a" }, { id: "2", label: "b" }] };

  it("is idle between turns and working while one is out", () => {
    expect(commanderViewOf(open(), JOB, null)?.state).toBe("idle");
    expect(commanderViewOf(open({ replying: true }), JOB, null)?.state).toBe("working");
  });

  it("waits on the owner while a card is out, whatever else it is doing", () => {
    expect(commanderViewOf(open({ replying: true }), JOB, { commander_card: CARD })?.state).toBe("waiting");
  });

  it("is drawn empty while the socket opens, and not at all for no thread or another Job's", () => {
    expect(commanderViewOf({ state: "opening", jobId: JOB }, JOB, null)).toEqual({ state: "idle", rows: [] });
    expect(commanderViewOf({ state: "none" }, JOB, null)).toBeUndefined();
    expect(commanderViewOf(undefined, JOB, null)).toBeUndefined();
    expect(commanderViewOf(open(), "another", null)).toBeUndefined();
  });

  it("keeps what had arrived when the socket failed", () => {
    const view = commanderViewOf({ state: "failed", jobId: JOB, replying: false, skipped: 0, missed: 0, detail: "the connection closed", items: [{ kind: "woke", id: "0", ts: TS, why: "w" }] }, JOB, null);
    expect(view?.rows).toHaveLength(1);
  });
});
