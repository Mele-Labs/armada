// The folds main runs a Job's two sockets through, over a full backfill.
//
// Here rather than beside `folding.ts` because `@armada/protocol` has no runner
// of its own, and main is the fold's heaviest caller.

import { describe, expect, it } from "vitest";

import type { JournalMessage, Noted, Turn, TurnMessage } from "@armada/protocol";
import { NO_NOTES, NO_TURNS, noteArrived, notesArrived, turnArrived, turnsArrived } from "@armada/protocol";

/** Fleet's bounded backfill, at its ceiling. */
const BACKFILL = 2048;

const OPENED: TurnMessage & { message: "opened" } = {
  message: "opened",
  protocol_version: { major: 4, minor: 12 },
  job_id: "01M1HQZAKN001AJ5MT3PT09KKY",
  live: true,
  skipped: 3,
};

function rowAt(at: number): TurnMessage {
  return {
    message: "row",
    ts: `2026-09-02T19:10:04.${String(at % 1000).padStart(3, "0")}Z`,
    step: "implement",
    ...(at % 2 === 0 ? { by: "fleet" as const } : {}),
    ...(at % 3 === 0 ? { drone_id: "01DRONEAAAAAAAAAAAAAAAAAAA" } : {}),
    event: "said",
    text: `row ${at}`,
  };
}

const ROWS = Array.from({ length: BACKFILL }, (_, at) => rowAt(at));

/** What every row folds to, written out rather than folded, so the fold is not checked against itself. */
const EXPECTED_ROWS: Turn[] = ROWS.map((_, at) => ({
  ts: `2026-09-02T19:10:04.${String(at % 1000).padStart(3, "0")}Z`,
  seq: at,
  step: "implement",
  by: at % 2 === 0 ? "fleet" : "drone",
  ...(at % 3 === 0 ? { drone_id: "01DRONEAAAAAAAAAAAAAAAAAAA" } : {}),
  saw: { event: "said", text: `row ${at}` },
}));

/** One message at a time, the way main folded before it batched. */
function oneByOne(messages: readonly TurnMessage[]) {
  let turns = NO_TURNS;
  let seq = 0;
  for (const message of messages) {
    const next = turnArrived(turns, message, seq);
    if (message.message === "row") seq += 1;
    turns = next.turns;
    if (next.ended !== undefined) return { turns, ended: next.ended };
  }
  return { turns, ended: undefined };
}

describe("a transcript backfill, folded", () => {
  const stream: TurnMessage[] = [OPENED, ...ROWS, { message: "missed", dropped: 4 }];
  const expected = { live: true, skipped: 3, missed: 4, rows: EXPECTED_ROWS };

  it("comes to the same turns one message at a time and in one batch", () => {
    expect(oneByOne(stream).turns).toEqual(expected);
    const batched = turnsArrived(NO_TURNS, stream, 0);
    expect(batched.turns).toEqual(expected);
    expect(batched.seq).toBe(BACKFILL);
    expect(batched.ended).toBeUndefined();
  });

  it("comes to the same turns however the batches fall", () => {
    let turns = NO_TURNS;
    let seq = 0;
    for (let at = 0; at < stream.length; at += 7) {
      ({ turns, seq } = turnsArrived(turns, stream.slice(at, at + 7), seq));
    }
    expect(turns).toEqual(expected);
  });

  it("leaves turns it already published as they were", () => {
    const first = turnsArrived(NO_TURNS, [OPENED, ...ROWS.slice(0, 1000)], 0);
    const rows = first.turns.rows;
    const later = turnsArrived(first.turns, ROWS.slice(1000), first.seq);

    expect(first.turns.rows).toBe(rows);
    expect(rows).toHaveLength(1000);
    expect(later.turns.rows).toHaveLength(BACKFILL);
    expect(later.turns.rows.slice(0, 1000)).toEqual(rows);
  });

  it("stops at `closed`, keeping the rows before it and nothing after", () => {
    const ended = turnsArrived(NO_TURNS, [OPENED, rowAt(0), { message: "closed", because: "drone_ended" }, rowAt(1)], 0);
    expect(ended.ended).toBe("drone_ended");
    expect(ended.turns).toEqual({ live: false, skipped: 3, missed: 0, rows: [EXPECTED_ROWS[0]] });
    expect(oneByOne([OPENED, rowAt(0), { message: "closed", because: "drone_ended" }])).toEqual({
      turns: ended.turns,
      ended: "drone_ended",
    });
  });

  it("starts over on a second `opened`", () => {
    const again = turnsArrived(NO_TURNS, [OPENED, ...ROWS.slice(0, 5), { ...OPENED, live: false, skipped: 0 }, rowAt(5)], 0);
    expect(again.turns).toEqual({ live: false, skipped: 0, missed: 0, rows: [EXPECTED_ROWS[5]] });
  });
});

describe("a log backfill, folded", () => {
  const OPENED_LOG: JournalMessage = {
    message: "opened",
    protocol_version: { major: 4, minor: 12 },
    job_id: "01M1HQZAKN001AJ5MT3PT09KKY",
    skipped: 2,
  };
  const NOTES: JournalMessage[] = Array.from({ length: BACKFILL }, (_, at) => ({
    message: "note" as const,
    at: "2026-09-02T19:10:04.000Z",
    by: "fleet" as const,
    level: "info" as const,
    msg: `note ${at}`,
  }));

  it("comes to the same log one message at a time and in batches", () => {
    let log = NO_NOTES;
    let seq = 0;
    for (const message of [OPENED_LOG, ...NOTES]) {
      log = noteArrived(log, message, seq).log;
      if (message.message === "note") seq += 1;
    }

    const batched = notesArrived(NO_NOTES, [OPENED_LOG, ...NOTES], 0);
    expect(batched.log).toEqual(log);
    expect(batched.seq).toBe(BACKFILL);
    expect(batched.log.notes.map((note: Noted) => note.seq)).toEqual(NOTES.map((_, at) => at));
  });

  it("leaves a log it already published as it was", () => {
    const first = notesArrived(NO_NOTES, [OPENED_LOG, ...NOTES.slice(0, 1000)], 0);
    const notes = first.log.notes;
    notesArrived(first.log, NOTES.slice(1000), first.seq);
    expect(first.log.notes).toBe(notes);
    expect(notes).toHaveLength(1000);
  });
});
