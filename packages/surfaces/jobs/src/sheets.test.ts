// The detail sheets: which key opens one, and what the open one is reading.

import { describe, expect, it } from "vitest";

import { keyFor } from "@armada/components";

import type { Diff, JobDetail as JobWhole } from "@armada/protocol";

import { gaveBackTheWorktree, NO_SHEET, sheetMoved } from "./Sheets";

// The detail's keys, read off the registry rather than off this file.
//
// **The caption and the binding come from one place or they drift.** `Open the
// log` was captioned `Enter` for four rounds of feedback after `actions.toml`
// moved it to `L`: the shared configuration existed the whole time and the one
// call site drawing the caption did not read it. What is asserted here is that
// the reader agrees with the registry, so a move in `actions.toml` that this
// file does not follow fails rather than ships.
describe("the detail's key captions", () => {
  // The owner removed the log on 2 Oct 2026: nothing on a Job is a single log.
  it("carries no key for the log", () => {
    expect(() => keyFor("open_log")).toThrow(/actions\.toml/);
  });

  it("reads the diff's key from the registry", () => {
    expect(keyFor("open_diff")).toBe("f");
  });

  it("refuses an act the registry does not carry, rather than drawing nothing", () => {
    expect(() => keyFor("open_the_log")).toThrow(/actions\.toml/);
  });

  // The press map is the other half of this and is not asserted here: reading a
  // press goes through `holdsText`, which asks whether focus is in a text field
  // and needs a DOM to answer. It belongs in a browser test, and what stands in
  // for it meanwhile is the compiler — `DetailShape` requires the sheet
  // openers, so a screen that binds the key and passes no handler does not
  // build.
});

// Which silence the diff header names. #381.
//
// The Produced chapter lists what a job wrote from a record Fleet keeps, and
// the diff reads the worktree live — so a finished job whose worktree has been
// given back shows a file list above an empty patch, and neither surface says
// why. The header can say it, but only where Bridge actually knows: `work`
// absent is also a job that never got a worktree at all, and naming that one
// `given back` would be the same false certainty pointed the other way.
describe("whether a missing reading is a worktree that was given back", () => {
  const JOB = "01M130Y1380016YK5S0JXBXDQ5";

  /** A footprint with one file in it. Only its presence is read. */
  const KEPT = { files: [{ path: "packages/screens/src/Sheets.tsx", change: "modified" }] };

  function whole(footprint: unknown): JobWhole {
    return { footprint } as JobWhole;
  }

  it("says so where a footprint proves there was a worktree to lose", () => {
    expect(gaveBackTheWorktree({ state: "read", jobId: JOB }, JOB, whole(KEPT))).toBe(true);
  });

  it("stays neutral with no footprint, because that is where Bridge cannot tell", () => {
    expect(gaveBackTheWorktree({ state: "read", jobId: JOB }, JOB, whole(undefined))).toBe(false);
    expect(gaveBackTheWorktree({ state: "read", jobId: JOB }, JOB, null)).toBe(false);
  });

  it("says nothing about a reading that came back", () => {
    // Present with no files is a drone that changed nothing, which is a
    // reading and not a silence. The header counts it.
    const read = {
      state: "read",
      jobId: JOB,
      work: { files: [], plan_declared: false, measured_whole: true },
    } as unknown as Diff;
    expect(gaveBackTheWorktree(read, JOB, whole(KEPT))).toBe(false);
  });

  it("says nothing before a reading, or about another job's", () => {
    expect(gaveBackTheWorktree({ state: "none" }, JOB, whole(KEPT))).toBe(false);
    expect(gaveBackTheWorktree({ state: "reading", jobId: JOB }, JOB, whole(KEPT))).toBe(false);
    const other = { state: "read", jobId: "01M130Y1380016YK5S0JXBXDQ6" } as Diff;
    expect(gaveBackTheWorktree(other, JOB, whole(KEPT))).toBe(false);
  });
});

describe("what the sheet is reading, as one value", () => {
  it("closing clears everything", () => {
    const diff = sheetMoved(NO_SHEET, { move: "open", which: "diff" });
    expect(sheetMoved(diff, { move: "close" })).toBe(NO_SHEET);
  });

  // #1021 — a press names which Check, and that name has to survive the move.
  describe("the Check output sheet", () => {
    it("opens on the Check a press named", () => {
      expect(sheetMoved(NO_SHEET, { move: "open", which: "check", checkId: "check:test_suite" })).toEqual(
        { which: "check", checkId: "check:test_suite" },
      );
    });

    it("replaces one Check with another on a second press, rather than stacking", () => {
      const first = sheetMoved(NO_SHEET, { move: "open", which: "check", checkId: "check:build" });
      expect(sheetMoved(first, { move: "open", which: "check", checkId: "check:test" })).toEqual({
        which: "check",
        checkId: "check:test",
      });
    });

    it("is replaced by another sheet, and replaces one in turn", () => {
      const holds = sheetMoved(NO_SHEET, { move: "open", which: "holds" });
      const checked = sheetMoved(holds, { move: "open", which: "check", checkId: "check:test_suite" });
      expect(checked).toEqual({ which: "check", checkId: "check:test_suite" });
      expect(sheetMoved(checked, { move: "open", which: "diff" })).toEqual({ which: "diff" });
    });

    it("closes like every other sheet", () => {
      const checked = sheetMoved(NO_SHEET, { move: "open", which: "check", checkId: "check:test_suite" });
      expect(sheetMoved(checked, { move: "close" })).toBe(NO_SHEET);
    });
  });

  // `#1421`'s fields left the 380px rail for this layer. The task sheet carries
  // an id the way a Check's does, so it reads the plan as it stands rather
  // than a copy taken when it opened.
  describe("the task sheet", () => {
    it("carries which task it is reading", () => {
      expect(sheetMoved(NO_SHEET, { move: "open", which: "task", taskId: "T1" })).toEqual({
        which: "task",
        taskId: "T1",
      });
    });

    it("replaces one task with another on a second press, rather than stacking", () => {
      const first = sheetMoved(NO_SHEET, { move: "open", which: "task", taskId: "T1" });
      expect(sheetMoved(first, { move: "open", which: "task", taskId: "T4" })).toEqual({
        which: "task",
        taskId: "T4",
      });
    });

    it("is replaced by another sheet, and replaces one in turn", () => {
      const task = sheetMoved(NO_SHEET, { move: "open", which: "task", taskId: "T1" });
      expect(sheetMoved(task, { move: "open", which: "diff" })).toEqual({ which: "diff" });
      const diff = sheetMoved(NO_SHEET, { move: "open", which: "diff" });
      expect(sheetMoved(diff, { move: "open", which: "task", taskId: "T2" })).toEqual({
        which: "task",
        taskId: "T2",
      });
    });

    it("closes like every other sheet", () => {
      const task = sheetMoved(NO_SHEET, { move: "open", which: "task", taskId: "T1" });
      expect(sheetMoved(task, { move: "close" })).toBe(NO_SHEET);
    });
  });
});
