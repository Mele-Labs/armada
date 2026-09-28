// The Record's words: which filter a row answers to, and what hue it takes.

import { describe, expect, it } from "vitest";

import { filtersOf, toneOf, underFilter, unfiledSays } from "./record";
import type { LedgerRow } from "./draft/ledger";

function row(over: Partial<LedgerRow> = {}): LedgerRow {
  return {
    at: "2026-09-22T10:00:00Z",
    coord: null,
    actor: "fleet",
    kind: "checked",
    what: "typecheck",
    outcome: "passed",
    cursor: 1,
    ...over,
  };
}

describe("what a row came to, as a hue", () => {
  /**
   * **The defect, pinned.** `outside the plan` was in the failing arm until 28
   * September 2026, so a file nobody declared filled its row with
   * `--status-completed-failed`. `grounds.ts` called the same fact `quiet` and
   * `packages/protocol/src/events.ts` calls `outside_plan` *a mark, not a
   * judgement* — and the owner read the red and asked whether it hurt the Job.
   */
  it("gives a file nobody declared no hue at all, because drift fails nothing", () => {
    const drift = row({
      kind: "task_files",
      outcome: "1 file this Job never said it would change: crates/ipc/operations.toml",
    });

    expect(toneOf(drift)).toBeUndefined();
  });

  it("gives a file the step never named no hue either, on the footprint's own row", () => {
    const drift = row({
      kind: "file_written",
      outcome: "modified, which the step never said it would change",
    });

    expect(toneOf(drift)).toBeUndefined();
  });

  it("still draws a failed Check and a refused criterion red, which do fail a step", () => {
    expect(toneOf(row({ outcome: "failed — 1 of 1384 failed" }))).toBe("failed");
    expect(toneOf(row({ kind: "judged", outcome: "not met" }))).toBe("failed");
  });
});

describe("the Job filter", () => {
  const ROWS = [
    row({ kind: "created", what: "this Job was created", outcome: "", cursor: 1 }),
    row({ kind: "status_completed_success", what: "the Job ended", outcome: "", cursor: 2 }),
    row({ kind: "checked", cursor: 3 }),
    row({ kind: "unheard_of", cursor: 4 }),
  ];

  it("is in the strip, counting the Job's own rows and nothing else", () => {
    const job = filtersOf(ROWS).find((one) => one.id === "job");

    expect(job).toEqual({ id: "job", label: "Job", count: 2 });
  });

  it("narrows the table to the Job's own machine moving", () => {
    expect(underFilter(ROWS, "job").map((one) => one.kind)).toEqual([
      "created",
      "status_completed_success",
    ]);
  });

  it("leaves a kind nothing has heard of under All alone, and says so", () => {
    expect(unfiledSays(ROWS)).toBe("One more row is under All alone: a kind no filter names.");
  });
});
