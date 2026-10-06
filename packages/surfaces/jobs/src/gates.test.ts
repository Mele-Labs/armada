// The one reading of `check_runs` and `judged`, tested as the answer it is.
//
// **Arithmetic, so it runs in node.** Which attempt's rows count, how a panel
// groups, whether one veto refuses a criterion and which output an `o` press
// opens are all functions of the wire — a hundred cases cost what one costs,
// and none of them needs a browser.

import { describe, expect, it } from "vitest";
import type { Criterion, Judged, StepDetail } from "@armada/protocol";

import { checksOf, outputOf, outputRunOf, panelsOf, placesOf } from "./gates";
import { noteFor, regionOf, rowsOf } from "./outputs";

function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "verify",
    label: "Verify",
    ordinal: 3,
    state: "stopped",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-09T09:30:00Z",
    updated_at: "2026-09-09T09:41:00Z",
    ...over,
  };
}

/** Two runs of the step, so "the current attempt" is a thing to get wrong. */
const TWICE: StepDetail["attempts"] = [
  { attempt: 1, outcome: "retrying", started_at: "2026-09-09T09:00:00Z" },
  { attempt: 2, outcome: "refused", started_at: "2026-09-09T09:20:00Z" },
];

const CRITERIA: Criterion[] = [
  { criterion_id: "c1", text: "The public API is byte-identical.", source: "brief" },
  { criterion_id: "c2", text: "Behaviour is unchanged.", source: "brief" },
];

/** One member's answer, with only what the case is about spelled out. */
function judged(over: Partial<Judged> & Pick<Judged, "criterion_id">): Judged {
  return { attempt: 1, verdict: "met", ...over };
}

describe("the Checks a step declares", () => {
  it("keeps a row for a Check the gate has not reached", () => {
    const reads = checksOf(
      step({ checks: [{ kind: "manifest_check", name: "build" }, { kind: "diff_nonempty" }] }),
    );
    expect(reads.map((read) => read.name)).toEqual(["build", "diff_nonempty"]);
    expect(reads.every((read) => read.run === undefined)).toBe(true);
  });

  it("joins a run to its declaration by name", () => {
    const reads = checksOf(
      step({
        checks: [{ kind: "manifest_check", name: "build" }],
        check_runs: [{ attempt: 1, name: "build", outcome: "failed", produced: "exit 101" }],
      }),
    );
    expect(reads[0]?.run?.produced).toBe("exit 101");
  });

  // `check_runs` has carried every attempt's rows since protocol 7.0, and both
  // surfaces draw the live gate. Read across all of them, a step retried once
  // reports the run before last.
  it("reads only the current attempt's runs", () => {
    const reads = checksOf(
      step({
        attempts: TWICE,
        checks: [{ kind: "manifest_check", name: "build" }],
        check_runs: [
          { attempt: 1, name: "build", outcome: "failed", produced: "exit 101" },
          { attempt: 2, name: "build", outcome: "passed" },
        ],
      }),
    );
    expect(reads[0]?.run?.outcome).toBe("passed");
  });
});

// #1102 — a waiting Check says how many places it takes, where it takes more
// than one.
describe("how many places a waiting Check takes", () => {
  it("reads the wire's places while the Check waits", () => {
    const reads = checksOf(
      step({
        checks: [{ kind: "manifest_check", name: "screens_test" }],
        checking: { attempt: 1, checks: [{ name: "screens_test", places: 3 }] },
      }),
    );
    expect(placesOf(reads[0]!)).toBe(3);
  });

  it("is undefined once the Check is no longer waiting", () => {
    const reads = checksOf(
      step({
        checks: [{ kind: "manifest_check", name: "screens_test" }],
        check_runs: [{ attempt: 1, name: "screens_test", outcome: "passed" }],
      }),
    );
    expect(placesOf(reads[0]!)).toBeUndefined();
  });
});

describe("which output a press opens", () => {
  it("takes the Check that did not pass, because that is why the step stopped", () => {
    expect(
      outputOf(
        step({
          check_runs: [
            { attempt: 1, name: "a", outcome: "passed", output_path: "checks/a.log" },
            { attempt: 1, name: "b", outcome: "failed", output_path: "checks/b.log" },
          ],
        }),
      ),
    ).toBe("checks/b.log");
  });

  it("takes the first output there is where nothing failed", () => {
    expect(
      outputOf(
        step({
          check_runs: [
            { attempt: 1, name: "a", outcome: "passed", output_path: "checks/a.log" },
            { attempt: 1, name: "b", outcome: "passed", output_path: "checks/b.log" },
          ],
        }),
      ),
    ).toBe("checks/a.log");
  });

  it("never offers a stale attempt's output", () => {
    expect(
      outputOf(
        step({
          attempts: TWICE,
          check_runs: [
            { attempt: 1, name: "build", outcome: "failed", output_path: "checks/1.log" },
            { attempt: 2, name: "build", outcome: "passed", output_path: "checks/2.log" },
          ],
        }),
      ),
    ).toBe("checks/2.log");
  });

  it("answers nothing where no Check kept a file, so the press is left alone", () => {
    expect(outputOf(step({ check_runs: [{ attempt: 1, name: "a", outcome: "passed" }] }))).toBe(
      undefined,
    );
  });

  // The header act, the `o` key and the chapter's own reader all come through
  // one call, so the row and the path cannot name two different Checks.
  it("names the row as well as the path, so a reader can say whose output it is", () => {
    const run = outputRunOf(
      step({
        check_runs: [
          { attempt: 1, name: "a", outcome: "passed", output_path: "checks/a.log" },
          { attempt: 1, name: "b", outcome: "failed", output_path: "checks/b.log" },
        ],
      }),
    );
    expect(run?.name).toBe("b");
    expect(run?.output_path).toBe("checks/b.log");
  });
});

describe("a Check's output, as a reading", () => {
  const window = {
    attempt: 1,
    name: "check:test_suite",
    path: ".armada/checks/01M130/verify.1.0.log",
    lines: ["--- stdout ---", "    left:  1970-01-01T00:00:00Z"],
    from_line: 1_939,
    total_lines: 2_180,
    bytes: 61_204,
    whole: false,
  };

  // A region is a window onto a file and a citation names the file, so a row
  // numbered from one would send a reader to the wrong line.
  it("numbers the rows as the file numbers them", () => {
    expect(rowsOf(window).map((row) => row.at)).toEqual([1_939, 1_940]);
  });

  it("keeps a line exactly as it was written", () => {
    const rows = rowsOf(window);
    expect(rows[1]).toMatchObject({ row: "line", text: "    left:  1970-01-01T00:00:00Z" });
  });

  it("says the reading is truncated, and whose output it is", () => {
    expect(regionOf(window).says).toBe("check:test_suite · lines 1,939–1,940 of 2,180");
    expect(regionOf(window).path).toBe(".armada/checks/01M130/verify.1.0.log");
  });

  it("says the whole of a short file is the whole of it", () => {
    const short = { ...window, from_line: 1, total_lines: 2, whole: true };
    expect(regionOf(short).says).toBe("check:test_suite · 2 lines");
  });

  // A fact or a failure, and nothing otherwise. Saying a Check printed nothing
  // when what is true is that nobody has asked would be a false fact.
  it("says nothing for a reading nobody asked for, and the fact for one that came back empty", () => {
    expect(noteFor(undefined)).toBeUndefined();
    expect(noteFor({ state: "fetching" })).toBeUndefined();
    expect(noteFor({ state: "got", output: { ...window, lines: [] } })).toContain("printed nothing");
    expect(noteFor({ state: "absent", note: "gone" })).toBe("gone");
  });
});

describe("a panel, read as one row per criterion", () => {
  it("groups a panel's members onto the criterion they answered", () => {
    const panels = panelsOf(
      step({
        judged: [
          judged({ criterion_id: "c1", member: 1 }),
          judged({ criterion_id: "c1", member: 2 }),
          judged({ criterion_id: "c1", member: 3 }),
        ],
      }),
      CRITERIA,
    );
    expect(panels.length).toBe(1);
    expect(panels[0]?.members.length).toBe(3);
  });

  // Unanimity, from `docs/concepts/judge.md`. A row that took the majority
  // would draw a criterion two judges met as met, and it is refused.
  it("refuses the criterion on one veto out of three", () => {
    const panels = panelsOf(
      step({
        judged: [
          judged({ criterion_id: "c1", member: 1 }),
          judged({ criterion_id: "c1", member: 2, verdict: "not_met" }),
          judged({ criterion_id: "c1", member: 3 }),
        ],
      }),
      CRITERIA,
    );
    expect(panels[0]?.verdict).toBe("not_met");
    expect(panels[0]?.refused.length).toBe(1);
  });

  it("puts the members in panel order, whatever order they arrived in", () => {
    const panels = panelsOf(
      step({
        judged: [
          judged({ criterion_id: "c1", member: 3 }),
          judged({ criterion_id: "c1", member: 1 }),
          judged({ criterion_id: "c1", member: 2 }),
        ],
      }),
      CRITERIA,
    );
    expect(panels[0]?.members.map((one) => one.member)).toEqual([1, 2, 3]);
  });

  // The ordinal is the frozen position a citation names — `02` — and never the
  // row's place on screen.
  it("carries each criterion's frozen position and its own words", () => {
    const panels = panelsOf(step({ judged: [judged({ criterion_id: "c2" })] }), CRITERIA);
    expect(panels[0]?.ordinal).toBe(2);
    expect(panels[0]?.criterion?.text).toBe("Behaviour is unchanged.");
  });

  it("leaves a criterion the Job does not carry without a position, rather than inventing one", () => {
    const panels = panelsOf(step({ judged: [judged({ criterion_id: "gone" })] }), CRITERIA);
    expect(panels[0]?.ordinal).toBe(undefined);
    expect(panels[0]?.criterion).toBe(undefined);
  });

  it("reads only the current attempt, so a rerun does not draw last run's verdicts", () => {
    const panels = panelsOf(
      step({
        attempts: TWICE,
        judged: [
          judged({ criterion_id: "c1", attempt: 1, verdict: "not_met" }),
          judged({ criterion_id: "c1", attempt: 2 }),
        ],
      }),
      CRITERIA,
    );
    expect(panels.length).toBe(1);
    expect(panels[0]?.verdict).toBe("met");
  });

  it("keeps the criteria in the order they were asked in, never sorted", () => {
    const panels = panelsOf(
      step({
        judged: [judged({ criterion_id: "c2" }), judged({ criterion_id: "c1" })],
      }),
      CRITERIA,
    );
    expect(panels.map((panel) => panel.criterionId)).toEqual(["c2", "c1"]);
  });

  // #1153: before the fallback, the criterion `step.judging` was out on drew
  // no row at all until `judged` had one for it.
  it("draws the criterion step.judging is out on, marked asking, before it is judged", () => {
    const panels = panelsOf(
      step({
        judged: [judged({ criterion_id: "c1" })],
        judging: {
          look: "criterion",
          criterion_id: "c2",
          model: "sonnet",
          call: 1,
          of: 2,
          since: "2026-09-09T09:40:00Z",
          budget_ms: 120000,
        },
      }),
      CRITERIA,
    );
    const asking = panels.find((panel) => panel.criterionId === "c2");
    expect(asking?.verdict).toBe("asking");
    expect(asking?.members).toEqual([]);
  });

  it("leaves a live JudgeQuestion's criterion in charge over a call in flight", () => {
    const panels = panelsOf(
      step({
        judged: [],
        judging: {
          look: "criterion",
          criterion_id: "c2",
          model: "sonnet",
          call: 1,
          of: 2,
          since: "2026-09-09T09:40:00Z",
          budget_ms: 120000,
        },
      }),
      CRITERIA,
      "c1",
    );
    expect(panels.map((panel) => panel.criterionId)).toEqual(["c1"]);
  });
});
