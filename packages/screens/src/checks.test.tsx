// A step whose gate is running its Checks, as Job detail draws it.
//
// **`#628`, from the screen's side.** A person watching a Job after its Drone
// submitted saw every Check say nothing had run it and the strip say the Drone
// was working, for as long as the gate took. What is pinned here is what the
// rows and the strip say while `StepDetail.checking` is there: a running Check
// reads as running with how long it has run, one waiting for a slot reads as
// waiting, a finished one reads as its result, and the Drone is no longer the
// one working. `checkRow` builds each row.

import { describe, expect, it } from "vitest";

import type { CheckRun as CheckRunRow } from "@armada/components";
import type { CheckRun, StepDetail } from "@armada/protocol";

import { checkRow, checkSheetOf, saidOf } from "./checks";
import { checksOf } from "./gates";

const STARTED = "2026-09-11T09:00:00Z";

/** A minute and four seconds after the running Check started. */
const NOW = Date.parse(STARTED) + 64_000;

function gating(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "implement",
    label: "Write tests",
    ordinal: 1,
    state: "running",
    checks: [
      { kind: "manifest_check", name: "build", run: "cargo build --workspace --locked" },
      { kind: "manifest_check", name: "test", run: "cargo nextest run --workspace" },
      { kind: "manifest_check", name: "format", run: "cargo fmt --check" },
    ],
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: STARTED,
    updated_at: STARTED,
    checking: {
      attempt: 1,
      checks: [
        {
          name: "build",
          started_at: STARTED,
          took_ms: 9_000,
          ran: { attempt: 1, name: "build", outcome: "passed" },
          output_path: ".armada/checks/01M1/implement.1.live.0.log",
        },
        {
          name: "test",
          started_at: STARTED,
          output_path: ".armada/checks/01M1/implement.1.live.1.log",
        },
        { name: "format" },
      ],
    },
    ...over,
  };
}

function rowsOf(step: StepDetail): CheckRunRow[] {
  return checksOf(step).map((read) => checkRow(read, NOW));
}

describe("a step whose gate is running its Checks", () => {
  it("draws a running Check as running, with how long it has run", () => {
    const test = rowsOf(gating()).find((row) => row.id === "test");
    expect(test?.named).toBe("running");
    expect(test?.result).toBe("running");
    expect(test?.says).toBe("Running for 1m 04s.");
    expect(test?.output).toBe("implement.1.live.1.log");
  });

  it("draws a Check waiting for a slot as waiting, and a finished one as its result", () => {
    const rows = rowsOf(gating());
    const format = rows.find((row) => row.id === "format");
    expect(format?.named).toBe("queued");
    expect(format?.result).toBe("waiting");
    expect(rows.find((row) => row.id === "build")?.named).toBe("passed");
  });
});

// #1063 — Checks waiting for room other work holds on the machine read as
// queued behind it, so a slow gate does not read as a stuck one.
describe("a gate waiting for room on the machine", () => {
  const queued = gating({
    checking: {
      attempt: 1,
      checks: [
        { name: "build", waiting_behind: 3 },
        { name: "test", waiting_behind: 3 },
        { name: "format", waiting_behind: 3 },
      ],
    },
  });

  it("says each Check waits behind other work, and how much", () => {
    const build = rowsOf(queued).find((row) => row.id === "build");
    expect(build?.says).toBe("Waiting for room behind 3 other Checks on this machine.");
    expect(build?.result).toBe("waiting");
    expect(build?.named).toBe("queued");
  });

  it("names one other Check in the singular", () => {
    const one = gating({ checking: { attempt: 1, checks: [{ name: "format", waiting_behind: 1 }] } });
    expect(rowsOf(one).find((row) => row.id === "format")?.says).toBe(
      "Waiting for room behind 1 other Check on this machine.",
    );
  });

  it("says a Check waiting only on its own run is waiting to start", () => {
    expect(rowsOf(gating()).find((row) => row.id === "format")?.says).toBe("Waiting to start.");
  });
});

// #1102 — a heavier Check says how many places it takes, only where that is
// more than one.
describe("a Check that takes more than one place", () => {
  it("names the places it needs, waiting behind other work", () => {
    const heavy = gating({
      checking: {
        attempt: 1,
        checks: [{ name: "build", waiting_behind: 2, places: 3 }],
      },
    });
    expect(rowsOf(heavy).find((row) => row.id === "build")?.says).toBe(
      "Waiting for room behind 2 other Checks on this machine. It takes 3 places.",
    );
  });

  it("names the places it needs, waiting on nothing but its own run", () => {
    const heavy = gating({
      checking: { attempt: 1, checks: [{ name: "build", places: 3 }] },
    });
    expect(rowsOf(heavy).find((row) => row.id === "build")?.says).toBe(
      "Waiting to start. It takes 3 places.",
    );
  });

  it("says nothing extra for a Check that takes one place", () => {
    const one = gating({
      checking: { attempt: 1, checks: [{ name: "build", waiting_behind: 2, places: 1 }] },
    });
    expect(rowsOf(one).find((row) => row.id === "build")?.says).toBe(
      "Waiting for room behind 2 other Checks on this machine.",
    );
  });
});

// The sheet's own question — `Sheets.tsx`'s `CheckSheet` calls this on every
// render, so a Check that finishes while its sheet is open moves from live to
// kept without the sheet closing.
describe("what a Check's output sheet should read", () => {
  it("reads live while the gate is still writing it", () => {
    expect(checkSheetOf(gating(), "test")).toEqual({
      kind: "live",
      kept: "implement.1.live.1.log",
    });
  });

  it("reads kept once the gate has ruled and stopped writing", () => {
    const step = gating({
      checking: undefined,
      check_runs: [
        { attempt: 1, name: "test", outcome: "passed", output_path: ".armada/checks/01M1/implement.1.test.log" },
      ],
    });
    expect(checkSheetOf(step, "test")).toEqual({ kind: "kept", kept: "implement.1.test.log" });
  });

  it("reads nothing for a Check that has never run", () => {
    expect(checkSheetOf(gating(), "format")).toBeUndefined();
  });

  it("reads nothing for a Check nobody declared", () => {
    expect(checkSheetOf(gating(), "nonexistent")).toBeUndefined();
  });
});

describe("a Check the gate reused from the Drone's own dry run", () => {
  const passed: CheckRun = { attempt: 1, name: "build", outcome: "passed" };
  const reused: CheckRun = { ...passed, reused_from_dry_run: "2026-09-13T09:00:00Z" };

  it("says it was reused, and a Check the gate ran itself does not", () => {
    expect(saidOf(reused)).toBe("Passed — reused from the drone's run");
    expect(saidOf(passed)).toBe("Passed");
  });
});
