// What the Land board reads off a Job that finished — the arithmetic, which is
// where the old boards contradicted themselves.

import { GitMerge } from "lucide-react";
import { describe, expect, it } from "vitest";

import { landed } from "./fixtures/build/arc-landed";
import type { JobFixture } from "./fixtures/fixture";
import { landBoardDraws, landedOf, type LandedInput, type LandedRead } from "./landed";

const MOMENT = landed();
const FIXTURE: JobFixture = MOMENT.fixtures[0]!;

/** The moment as the screen is handed it: the Job's reads, and its draft. */
function inputOf(over: Partial<LandedInput> = {}): LandedInput {
  return {
    job: FIXTURE.job,
    whole: FIXTURE.watched.state === "read" ? FIXTURE.watched.detail : null,
    draft: MOMENT.draft,
    manifest: FIXTURE.manifests[0],
    holding: FIXTURE.resources.state === "read" ? FIXTURE.resources.resources : null,
    ...over,
  };
}

function read(over: Partial<LandedInput> = {}): LandedRead {
  const board = landedOf(inputOf(over));
  expect(board, "arc/landed draws a board").toBeDefined();
  return board!;
}

/** One figure of what it cost, by its label. */
function figure(board: LandedRead, label: string): string {
  return board.cost.figures.find((one) => one.label === label)?.value ?? "";
}

describe("a Job that has not finished draws no board", () => {
  it("draws nothing while the Job is running", () => {
    expect(landedOf(inputOf({ job: { ...FIXTURE.job, status: "running" } }))).toBeUndefined();
  });

  it("draws nothing before the Job's own read has arrived", () => {
    expect(landedOf(inputOf({ whole: null }))).toBeUndefined();
  });
});

describe("the headline says what the Job was held to, and counts none of it", () => {
  it("says everything was met where a step met every criterion", () => {
    expect(read().says).toBe("Everything this Job was held to was met.");
  });

  /**
   * **Two facts, two sentences.** A criterion nothing wrote a verdict for is
   * not a case with no spec, and a Check-verified criterion lands here because
   * nothing on the wire links a Check to what it answers.
   */
  it("says no verdict was recorded where nothing answered a criterion, and never green", () => {
    const criteria = [
      ...MOMENT.draft.criteria!,
      { criterion_id: "a9", text: "Nobody judged this", verified_by: "judge" as const, origin: { origin: "prompt" as const } },
    ];
    const board = read({ draft: { ...MOMENT.draft, criteria } });
    expect(board.criteria[2]?.verdict).toBe("no verdict recorded");
    expect(board.criteria[2]?.verdict).not.toBe("not covered");
    expect(board.criteria[2]?.status).not.toBe("completed-success");
  });

  // `No verdict was recorded for 1 of them` summed up what each row already
  // said, and the owner asked what it meant (1 Oct 2026). The row says it.
  it("says nothing over a criterion nobody answered", () => {
    const criteria = [
      ...MOMENT.draft.criteria!,
      { criterion_id: "a9", text: "Nobody judged this", verified_by: "check" as const, origin: { origin: "prompt" as const } },
    ];
    expect(read({ draft: { ...MOMENT.draft, criteria } }).says).toBeUndefined();
  });
});

/**
 * The Job with its own totals taken off, which is what these three claims are
 * about. **The arc Job carries `spend` since 29 Sep 2026** — it was added so
 * Overview's figures strip had something to draw — and before that the absence
 * was incidental, which is why they read the derived figures without asking
 * for it. The claim never changed; what it rests on is now written down.
 */
function derived(): LandedRead {
  const whole = FIXTURE.watched.state === "read" ? FIXTURE.watched.detail : null;
  const { spend: _counted, ...rest } = whole!;
  return read({ whole: rest });
}

describe("every figure is derived with its retries", () => {
  // Group three retried once, so its two tasks ran twice: eight tasks, ten
  // agents. A count of eight would contradict the retry on the row below it.
  it("counts an agent per task, and again for the group that was retried", () => {
    expect(figure(derived(), "Drones")).toBe("10");
  });

  // Four Checks at group one's boundary, seven at each of the others, and
  // group three's seven twice.
  it("counts a boundary's Checks once per run of that boundary", () => {
    expect(figure(derived(), "Checks")).toBe("32");
  });

  it("adds the spend up from each task's own agent, since the Job carries no total", () => {
    expect(figure(derived(), "Spend")).toBe("$7.53");
    expect(figure(derived(), "Turns")).toBe("135");
  });

  it("names the Job's own totals where Fleet counted them", () => {
    const whole = FIXTURE.watched.state === "read" ? FIXTURE.watched.detail : null;
    const board = read({
      whole: {
        ...whole!,
        spend: {
          cost_micros: 9_000_000,
          cost_cap_micros: 20_000_000,
          turns: 140,
          turn_cap: 400,
          ran_ms: 1,
          drones: 10,
        },
      },
    });
    expect(figure(board, "Spend")).toBe("$9.00 of $20.00");
    expect(figure(board, "Turns")).toBe("140 of 400");
  });

  it("times the Job from its own two instants", () => {
    expect(figure(read(), "Run time")).toBe("2h 02m");
  });
});

describe("the groups", () => {
  it("draws one row per group, with the commit each left", () => {
    const groups = read().groups;
    expect(groups.map((one) => one.name)).toEqual([
      "Group one",
      "Group two",
      "Group three",
      "Group four",
    ]);
    expect(groups[3]?.commit).toBe("e0d47a1");
  });

  it("says the retried group ran its Checks twice", () => {
    expect(read().groups[2]?.checks).toBe("7 Checks, twice");
  });

  it("leaves the time taken absent, because nothing in the record times a group", () => {
    expect(read().groups.every((one) => one.took === undefined)).toBe(true);
  });

  /**
   * The Record is the only source a group's span has — a task carries no
   * instants either — so a group whose rows the Record holds is timed by the
   * first and last of them, and one it holds nothing for stays untimed.
   */
  it("times a group by the first and last thing the Record says happened in it", () => {
    const record = [
      { at: "2026-09-22T09:22:00Z", coord: { step: "implement", step_attempt: 1, group: "g1" }, actor: "drone" as const, kind: "drone_started", what: "T1's agent started", outcome: "", cursor: 1 },
      { at: "2026-09-22T09:56:00Z", coord: { step: "implement", step_attempt: 1, group: "g1" }, actor: "check" as const, kind: "checked", what: "four Checks", outcome: "all four passed", cursor: 2 },
    ];
    const groups = read({ draft: { ...MOMENT.draft, record } }).groups;
    expect(groups[0]?.took).toBe("34m 00s");
    expect(groups[1]?.took).toBeUndefined();
  });

  it("counts the files the plan claims, de-duplicated", () => {
    expect(read().groupsSummary).toBe("4 groups · 8 tasks · 9 files");
  });
});

describe("the run", () => {
  it("draws every step with what it came to", () => {
    const steps = read().steps.steps;
    expect(steps.map((one) => one.label)).toEqual([
      "Plan the change",
      "Implement",
      "Write tests",
      "Review the change",
    ]);
    expect(steps.every((one) => one.verdict === "passed")).toBe(true);
  });
});

describe("the test set", () => {
  it("draws the handoff run of every case and the run a person made, in one table saying who ran each", () => {
    expect(read().runs).toHaveLength(1);
    const set = read().runs[0]!;
    expect(set.runs).toHaveLength(5);
    expect(set.runs.filter((one) => one.who === "Fleet")).toHaveLength(4);
    expect(set.runs.at(-1)?.who).toBe("you");
  });

  it("reads a case with no spec as not covered, and says why it did not run", () => {
    const row = read().runs[0]!.runs.find((one) => one.spec.endsWith("Board.test.tsx"));
    expect(row?.outcome).toBe("not covered");
    expect(row?.meta).toBe("no spec covers Board.tsx");
  });

  // Two headings over two sentences saying nothing ran read as something
  // still to run (owner, 1 Oct 2026), so a Job nothing was run against has
  // no table at all.
  it("draws no table where nothing ran", () => {
    expect(read({ draft: { ...MOMENT.draft, runs: [] } }).runs).toEqual([]);
  });
});

describe("what it produced and what it left", () => {
  it("names the pull request and the branch it merged into", () => {
    const delivered = read().sections[0]!;
    expect(delivered.parts[0]?.value).toBe("https://git.example/armada/pull/1604");
    expect(delivered.parts[0]?.badge).toEqual({ status: "completed-success", icon: GitMerge, label: "Merged" });
    expect(delivered.parts[0]?.meta).toBe("into main");
  });

  it("names the branch, the worktree still on disk and the record", () => {
    const left = read().sections[1]!;
    expect(left.parts.map((one) => one.name)).toEqual(["Branch", "Worktree", "Record"]);
    expect(left.parts[1]?.meta).toContain("on disk");
    expect(left.parts[2]?.value).toContain(".armada/logs/");
  });

  // Job 2 gave its checkout back, and the row said nobody had read what it
  // held, on a read answering `held: none` (owner, 1 Oct 2026).
  it("draws no worktree row where no worktree is held", () => {
    expect(read({ holding: null }).sections[1]!.parts.map((one) => one.name)).toEqual(["Branch", "Record"]);
  });

  it("draws no record row where no Manifest was read, and no section where nothing is left", () => {
    const board = read({
      holding: null,
      manifest: undefined,
      job: { ...FIXTURE.job, branch: undefined },
      whole: { ...inputOf().whole!, branch: undefined },
    });
    expect(board.sections.map((one) => one.name)).not.toContain("Left behind");
  });

  it("names the record where Fleet says it is, rather than by the Job's id", () => {
    const holding = {
      ...inputOf().holding!,
      logs: [{ kind: "job" as const, path: ".armada/logs/3-show-what-s-running.jsonl" }],
    };
    expect(read({ holding }).sections[1]!.parts.at(-1)?.value).toMatch(/\/\.armada\/logs\/3-show-what-s-running\.jsonl$/);
  });
});

describe("what Where things are stops drawing", () => {
  /**
   * The board carries the branch and the worktree off the same derivation, so
   * *Where things are* drops both — but only where the board is actually
   * drawn. A finished Job whose read has not arrived keeps them, because
   * nothing else on that screen would say them.
   */
  it("names the Job the board draws, which is the one that drops the two rows", () => {
    expect(landBoardDraws(FIXTURE.job)).toBe(true);
    expect(landBoardDraws({ ...FIXTURE.job, status: "running" })).toBe(false);
    expect(landBoardDraws({ ...FIXTURE.job, status: "completed_failed" })).toBe(false);
  });
});
