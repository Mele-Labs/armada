// What the Plan destination says, tested where a hundred cases cost what one
// costs. The board's own rendering is claimed through `App`, in
// `apps/desktop/src/renderer/src/mock/arc.test.tsx`.

import { describe, expect, test } from "vitest";

import { arcGroups, arcCases } from "./fixtures/build/arc-plan";
import { doneTouched, groupFailed, plannedMoment, planRevisionRefused } from "./fixtures/build/arc";
import type { CaseView } from "./draft/cases";
import {
  besideSaid,
  boundaryClause,
  caseReads,
  casesOf,
  costSaid,
  droppedSaid,
  groupsOf,
  retrySaid,
  revisionsOf,
  runBySaid,
  spentSaid,
  taskSheetOf,
  tasksOf,
  overlapsOf,
  testsClause,
  turnsSaid,
  touchedByOf,
} from "./tab-plan-read";
import { groupCardOf, planBoardOf } from "./plan-board";

const GROUPS = arcGroups();
const CASES = arcCases();

/** The Job as one arc moment froze it, or `null` where it holds none. */
function wholeOf(moment: ReturnType<typeof plannedMoment>) {
  const watched = moment.fixtures[0]?.watched;
  return watched?.state === "read" ? watched.detail : null;
}

describe("the boundary's own clause", () => {
  test("a group that has not run says what will run at it", () => {
    expect(boundaryClause("pending")).toBe("will run at this boundary");
  });

  test("a group being checked says so in the present", () => {
    expect(boundaryClause("checking")).toBe("running at this boundary");
  });

  test("a group that has run says what ran", () => {
    expect(boundaryClause("passed")).toBe("ran at this boundary");
    expect(boundaryClause("retrying")).toBe("ran at this boundary");
  });

  // The strip draws the count, so the clause never carries one. A clause
  // that said "7 checks" beside the strip's own 7 is the number twice.
  test("the clause carries no count", () => {
    expect(boundaryClause("pending")).not.toMatch(/\d/);
    expect(testsClause("pending", 2)).not.toMatch(/\d/);
  });

  test("a boundary with no test says nothing about tests", () => {
    expect(testsClause("pending", 0)).toBeUndefined();
  });

  test("tests agree with their own count", () => {
    expect(testsClause("pending", 1)).toBe("runs at this boundary");
    expect(testsClause("pending", 2)).toBe("run at this boundary");
    expect(testsClause("passed", 2)).toBe("ran at this boundary");
  });
});

describe("a case reads what it is, never green", () => {
  const owed = CASES.find((one) => one.id === "c-api")!;
  const noSpec = CASES.find((one) => one.id === "c-board")!;

  test("a case with no spec is not covered", () => {
    expect(caseReads(noSpec)).toBe("not covered");
  });

  test("a case with a spec is owed", () => {
    expect(caseReads(owed)).toBe("owed");
  });

  test("a dropped case says what dropped it", () => {
    const revised: CaseView = {
      ...owed,
      state: "dropped",
      dropped_by: { dropped: "scope_revision", at: "2026-09-22T09:34:00Z" },
    };
    expect(caseReads(revised)).toBe("dropped");
    expect(droppedSaid(revised)).toBe("dropped by a scope revision");
  });

  test("a case that fell out of a retry names where", () => {
    const retried: CaseView = {
      ...owed,
      state: "dropped",
      dropped_by: {
        dropped: "retry",
        coord: { step: "implement", step_attempt: 2, group: "g3", task: "T5" },
      },
    };
    expect(droppedSaid(retried)).toBe("dropped on a retry of T5");
  });

  test("a case still owed had nothing drop it", () => {
    expect(droppedSaid(owed)).toBeUndefined();
  });
});

describe("what a task has spent", () => {
  test("a task nothing has run has spent nothing to read", () => {
    const task = tasksOf(GROUPS).find((one) => one.id === "T1")!;
    expect(spentSaid(task)).toBeUndefined();
  });

  test("a working task shows turns and no cost", () => {
    const moment = groupsOf(wholeOf(doneTouched()), doneTouched().draft);
    const working = tasksOf(moment).find((one) => one.id === "T7")!;
    expect(spentSaid(working)).toBe("6 turns");
  });

  test("a finished task shows what its own agent cost", () => {
    const moment = groupsOf(wholeOf(doneTouched()), doneTouched().draft);
    const done = tasksOf(moment).find((one) => one.id === "T1")!;
    expect(spentSaid(done)).toBe("34 turns · ~$2.40");
    // The row splits the pair into two columns so they line up down a card.
    expect(turnsSaid(done)).toBe("34 turns");
    expect(costSaid(done)).toBe("~$2.40");
  });
});

describe("what runs beside what", () => {
  test("a task of a concurrent group names its neighbour", () => {
    const t5 = tasksOf(GROUPS).find((one) => one.id === "T5")!;
    expect(besideSaid(t5)).toBe("beside T6");
  });

  test("a task that runs alone says nothing", () => {
    const t1 = tasksOf(GROUPS).find((one) => one.id === "T1")!;
    expect(besideSaid(t1)).toBeUndefined();
  });

  test("an unmarked task gets its own agent", () => {
    const t1 = tasksOf(GROUPS).find((one) => one.id === "T1")!;
    expect(runBySaid(t1)).toBe("its own agent");
  });
});

describe("a done task a later one edited", () => {
  test("the flag names the task that reached into it", () => {
    const groups = groupsOf(wholeOf(doneTouched()), doneTouched().draft);
    expect(touchedByOf(groups).get("T6")).toBe("T7");
  });

  test("nothing is flagged where no task was touched", () => {
    expect(touchedByOf(GROUPS).size).toBe(0);
  });

  test("the inspector carries the flag into the brief", () => {
    const groups = groupsOf(wholeOf(doneTouched()), doneTouched().draft);
    const sheet = taskSheetOf("T6", groups, CASES, touchedByOf(groups))!;
    expect(sheet.note).toContain("T7 edited a file this task had already finished.");
  });
});

describe("an order that contradicts the scopes", () => {
  const overlaps = overlapsOf(GROUPS);

  test("the warning is on the group that has the clash, both ways", () => {
    const fourth = overlaps.get(GROUPS[3]!.id) ?? [];
    expect(fourth.map((one) => one.says)).toEqual([
      "Group 2 writes these files too",
      "Group 3 writes these files too",
    ]);
    expect(fourth[1]?.paths).toEqual(["packages/screens/src/running-rows.tsx"]);
    const third = overlaps.get(GROUPS[2]!.id) ?? [];
    expect(third.map((one) => one.says)).toEqual(["Group 4 writes these files too"]);
  });

  // The whole of the owner's note of 28 Sep: a clash said once, above the
  // plan, made a reader go and find the two groups it named.
  test("a group with no clash carries no warning at all", () => {
    expect(overlaps.has(GROUPS[0]!.id)).toBe(false);
  });

  test("a file one group claims twice is not a clash", () => {
    expect(overlapsOf([GROUPS[0]!]).size).toBe(0);
  });
});

describe("how many times a group has run", () => {
  test("a first run says nothing", () => {
    expect(retrySaid(0)).toBeUndefined();
  });

  test("one retry is the second run", () => {
    expect(retrySaid(1)).toBe("second run");
    expect(retrySaid(2)).toBe("third run");
    expect(retrySaid(4)).toBe("run 5");
  });
});

describe("one group's card", () => {
  const touched = touchedByOf(GROUPS);

  test("the group writing Rust runs four checks and the ones writing Bridge run seven", () => {
    const rust = groupCardOf(GROUPS[0]!, CASES, touched, null);
    const bridge = groupCardOf(GROUPS[1]!, CASES, touched, null);
    expect(rust.boundary.checks).toHaveLength(4);
    expect(bridge.boundary.checks).toHaveLength(7);
    expect(rust.boundary.clause).toBe("will run at this boundary");
  });

  test("a case covering a file two groups touch is drawn at the last of them", () => {
    const second = groupCardOf(GROUPS[1]!, CASES, touched, null);
    const last = groupCardOf(GROUPS[3]!, CASES, touched, null);
    expect(second.boundary.tests?.map((one) => one.id)).not.toContain("c-overview");
    expect(last.boundary.tests?.map((one) => one.id)).toContain("c-overview");
  });

  // The chip carried the Job's Drone cap behind a middle dot until 28 Sep,
  // and the owner cut it: two facts in one sentence, the second the Job's
  // rather than the group's, and Overview draws it frozen at the gate.
  test("a concurrent group says its tasks run at once, and nothing about the Drone cap", () => {
    const chip = groupCardOf(GROUPS[2]!, CASES, touched, null).shapeSays;
    expect(chip).toBe("2 tasks, at the same time");
    expect(chip).not.toContain("Drones at once");
  });

  test("only the group carrying the failure names a check that failed", () => {
    const moment = groupFailed();
    const whole = wholeOf(moment);
    const groups = groupsOf(whole, moment.draft);
    const passed = groupCardOf(groups[1]!, CASES, touched, whole);
    const broke = groupCardOf(groups[2]!, CASES, touched, whole);
    expect(passed.boundary.checks.some((one) => one.reads === "failed")).toBe(false);
    expect(
      broke.boundary.checks.filter((one) => one.reads === "failed").map((one) => one.name),
    ).toEqual(["screens_test"]);
    expect(broke.boundary.retrySays).toBe("second run");
  });
});

describe("the whole board", () => {
  test("the plan draws four groups and eight tasks, and no task twice", () => {
    const moment = plannedMoment();
    const board = planBoardOf(wholeOf(moment), moment.draft, () => undefined)!;
    expect(board.groups.length).toBe(4);
    const ids = board.groups.flatMap((group) => group.tasks.map((task) => task.id));
    expect(ids.length).toBe(8);
    expect(new Set(ids).size).toBe(8);
  });

  test("each task names the tier the planner gave it and the model it resolved to", () => {
    const moment = plannedMoment();
    const board = planBoardOf(wholeOf(moment), moment.draft, () => undefined)!;
    const first = board.groups[0]!.tasks[0]!;
    expect(first.tier).toBe("difficult");
    expect(first.model).toBe("opus");
  });

  test("the approach is the plan's own line", () => {
    const moment = plannedMoment();
    const board = planBoardOf(wholeOf(moment), moment.draft, () => undefined)!;
    expect(board.approach).toContain("One read of everything running");
  });

  test("a Job with no plan and no draft draws no board at all", () => {
    expect(planBoardOf(null, undefined, () => undefined)).toBeUndefined();
  });

  test("today's wire derives one task per group where no draft is carried", () => {
    const moment = plannedMoment();
    const whole = wholeOf(moment);
    const derived = groupsOf(whole, undefined);
    expect(derived.length).toBe(8);
    expect(derived.every((group) => group.tasks.length === 1)).toBe(true);
  });

  test("the cases derive from the specs the Job's Drones named where none is carried", () => {
    expect(casesOf(wholeOf(plannedMoment()), undefined)).toEqual([]);
  });
});

describe("one task's inspector", () => {
  const touched = touchedByOf(GROUPS);

  test("it names what the task runs beside and what covers it", () => {
    const sheet = taskSheetOf("T5", GROUPS, CASES, touched)!;
    expect(sheet.beside).toEqual(["T6"]);
    expect(sheet.tier).toBe("difficult");
    expect(sheet.model).toBe("opus");
    expect(sheet.runBy).toBe("its own agent");
    expect(sheet.tests?.map((one) => one.id)).toEqual(["c-panel"]);
  });

  test("a case with no spec reads not covered in the inspector too", () => {
    const sheet = taskSheetOf("T4", GROUPS, CASES, touched)!;
    expect(sheet.tests?.map((one) => one.reads)).toEqual(["not covered"]);
  });

  test("a task the plan does not hold opens nothing", () => {
    expect(taskSheetOf("T99", GROUPS, CASES, touched)).toBeUndefined();
  });
});

describe("a plan that may still be argued with", () => {
  const whole = wholeOf(planRevisionRefused());

  test("a plan at its gate puts the same three controls on every card, and draws the one mark that says what they are", () => {
    const board = planBoardOf(whole, planRevisionRefused().draft, () => {}, undefined, true)!;
    expect(board.askable).toBe(true);
    expect(board.groups.map((group) => group.asks?.map((ask) => ask.id))).toEqual([
      ["move_up", "move_down", "remove"],
      ["move_up", "move_down", "remove"],
      ["move_up", "move_down", "remove"],
      ["move_up", "move_down", "remove"],
    ]);
  });

  test("a plan past its gate offers nothing and draws no mark about asking", () => {
    const board = planBoardOf(whole, planRevisionRefused().draft, () => {})!;
    expect(board.askable).toBeUndefined();
    expect(board.groups.every((group) => group.asks === undefined)).toBe(true);
  });

  test("the refusal is read off the step that recorded the plan, and names the task it reached", () => {
    const step = whole?.steps.find((one) => one.step_id === "plan");
    const [one] = revisionsOf(whole, planRevisionRefused().draft, step);
    expect(one?.answer).toBe("refused");
    expect(one?.task).toBe("T5");
    expect(one?.refusal?.produced).toContain("no other task claims it");
  });
});
