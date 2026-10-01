// What the plan board says about a group that has run, over the four moments
// inside the step. `#1536`. Arithmetic and copy are proved here so the browser
// tests are left with what only a rendering can show.
//
// **One board.** These claims were made against a second one under the Workflow
// canvas until the owner took the groups off it (28 Sep 2026); they are made
// against the card on the Plan tab's List view now, which is the same reading.

import { describe, expect, test } from "vitest";

import {
  doneTouched,
  executingConcurrent,
  executingSequential,
  groupFailed,
} from "./fixtures/build/arc";
import type { ArcMoment } from "./fixtures/build/arc-base";
import { boundaryOf, planBoardOf, shapeSaid, verdictSaid } from "./plan-board";
import { runBySaid } from "./tab-plan-read";
import { tasksField } from "./step";

/** The Job whole behind a moment, and the step its groups hang under. */
function reading(moment: ArcMoment) {
  const fixture = moment.fixtures[0]!;
  const held = fixture.watched.state === "read" ? fixture.watched.detail : null;
  const groups = [...moment.draft.groups!];
  const step = held?.steps.find((one) => one.step_id === "implement");
  return {
    whole: held,
    groups,
    step,
    cases: [...(moment.draft.cases ?? [])],
  };
}

/** The board a moment draws, past its gate, with no task open. */
function board(moment: ArcMoment) {
  const read = reading(moment);
  return planBoardOf(read.whole, moment.draft, () => undefined, undefined, false, read.step);
}

const groupAt = (moment: ArcMoment, ordinal: number) =>
  board(moment)!.groups.find((one) => one.ordinal === ordinal)!;

const taskIn = (moment: ArcMoment, ordinal: number, id: string) =>
  groupAt(moment, ordinal).tasks.find((one) => one.id === id)!;

describe("one group at a time", () => {
  test("every group is drawn in the order it runs", () => {
    expect(board(executingSequential())!.groups.map((one) => one.ordinal)).toEqual([1, 2, 3, 4]);
  });

  // The commit is at the boundary and nowhere else on the card: it is what the
  // group's end produced, and a chip beside the state would say it twice.
  test("groups one and two read passed with the commit each left", () => {
    expect(groupAt(executingSequential(), 1).says).toBe("passed");
    expect(groupAt(executingSequential(), 1).boundary.commit).toBe("4c1b9d2");
    expect(groupAt(executingSequential(), 2).boundary.commit).toBe("7a2f0c5");
  });

  test("group four has not started, and its Checks read as not run", () => {
    const four = groupAt(executingSequential(), 4);
    expect(four.says).toBe("not started");
    expect(four.boundary.checks).toHaveLength(7);
    expect(new Set(four.boundary.checks.map((one) => one.reads))).toEqual(new Set(["not run"]));
    expect(four.boundary.verdictSays).toBeUndefined();
  });
});

describe("a task carries only its own agent", () => {
  test("a working task shows turns and no cost, because its agent has not stopped", () => {
    const five = taskIn(executingSequential(), 3, "T5");
    expect(five.turnsSays).toBe("14 turns");
    expect(five.costSays).toBeUndefined();
  });

  test("a finished task shows what it cost, in a group that has already passed", () => {
    const one = taskIn(executingSequential(), 1, "T1");
    expect(one.turnsSays).toBe("34 turns");
    expect(one.costSays).toBe("~$2.40");
  });

  // Three fields, not one line: the board joins them, and a caller that joined
  // them first would be composing prose. `PlanBoard`'s own rule.
  test("the tier, the model it resolved to and how it is run are the row's three fields", () => {
    const one = taskIn(executingSequential(), 1, "T1");
    expect([one.tier, one.model, one.runBy]).toEqual(["difficult", "opus", "its own agent"]);
    const three = taskIn(executingSequential(), 2, "T3");
    expect([three.tier, three.model, three.runBy]).toEqual(["easy", "haiku", "its own agent"]);
  });

  test("a cost is on a task the moment its agent stopped, before its group is checked", () => {
    const five = taskIn(executingConcurrent(), 3, "T5");
    const six = taskIn(executingConcurrent(), 3, "T6");
    expect([five.turnsSays, five.costSays]).toEqual(["27 turns", "~$1.90"]);
    expect([six.turnsSays, six.costSays]).toEqual(["15 turns", "~$0.72"]);
    expect(groupAt(executingConcurrent(), 3).boundary.verdictSays).toBeUndefined();
  });
});

describe("fan out, then join", () => {
  test("a concurrent group says its tasks run at the same time, and names what each is beside", () => {
    expect(groupAt(executingConcurrent(), 3).shapeSays).toContain("2 tasks, at the same time");
    expect(taskIn(executingConcurrent(), 3, "T5").besideSays).toBe("beside T6");
    expect(taskIn(executingConcurrent(), 3, "T6").besideSays).toBe("beside T5");
  });

  test("a group whose tasks run in order says so, and a group of one says it runs alone", () => {
    const first = reading(executingConcurrent()).groups[0]!;
    expect(groupAt(executingConcurrent(), 1).shapeSays).toBe("2 tasks, one after another");
    expect(shapeSaid({ ...first, tasks: [first.tasks[0]!] })).toBe("1 task, on its own");
  });

  test("the group joining its work says so, which is not the same as checking", () => {
    expect(groupAt(executingConcurrent(), 3).says).toBe("joining its work");
  });
});

describe("a boundary that failed", () => {
  test("the Check that failed is named, the six that passed are drawn beside it", () => {
    const three = groupAt(groupFailed(), 3).boundary;
    expect(three.verdictSays).toBe("screens_test failed");
    expect(three.verdictNamed).toBe("failed");
    expect(three.checks.filter((one) => one.reads === "failed").map((one) => one.name)).toEqual([
      "screens_test",
    ]);
    expect(three.checks.filter((one) => one.reads === "passed")).toHaveLength(6);
  });

  // The boundary said which group it holds back until 28 Sep. The owner cut
  // it: the ordering rule is true of a step that never ran, guide 4 carries
  // it, and group 4's own row says it is waiting.
  test("it says this is its second attempt, and nothing about what the next group may do", () => {
    const three = groupAt(groupFailed(), 3).boundary;
    expect(three.retrySays).toBe("attempt 2");
    expect(JSON.stringify(three)).not.toContain("No task of group");
  });

  // Run together with no labels, the two read as one claim that contradicted
  // itself (the owner, 29 Sep 2026). Each is its own field on the Check's row.
  test("the Check that failed carries what it was held to and what it got, apart", () => {
    const screens = groupAt(groupFailed(), 3).boundary.checks.find((one) => one.name === "screens_test")!;
    expect(screens.expected).toBe("Every test in the screens package passes");
    expect(screens.result).toBe("1 of 1384 failed: the Drones row opened the Board");
  });

  test("pressing a Check that ran opens its run, by name and step attempt", () => {
    const read = reading(groupFailed());
    const opened: [string, number][] = [];
    const onOpenCheck = (name: string, at: number) => void opened.push([name, at]);
    const drawn = planBoardOf(read.whole, groupFailed().draft, () => undefined, undefined, false, read.step, onOpenCheck)!;
    drawn.groups[2]!.boundary.checks.find((one) => one.name === "screens_test")!.onOpen!();
    expect(opened).toEqual([["screens_test", 1]]);
  });

  // The step's runs are every group's, so a group nothing reached would
  // otherwise open another group's run, and a passed one another group's red.
  test("a Check with no run of its own to open is not pressable", () => {
    const read = reading(groupFailed());
    const onOpenCheck = () => undefined;
    const drawn = planBoardOf(read.whole, groupFailed().draft, () => undefined, undefined, false, read.step, onOpenCheck)!;
    expect(drawn.groups[3]!.boundary.checks.every((one) => one.onOpen === undefined)).toBe(true);
    const first = drawn.groups[0]!.boundary.checks;
    expect(first.filter((one) => one.onOpen !== undefined).map((one) => one.name)).toEqual(["test"]);
  });

  // A step's `check_runs` is one list for every group in it, so a passed group
  // taking a later group's red is the defect this guards.
  test("a group that passed takes no red from another group's failed run", () => {
    const one = groupAt(groupFailed(), 1).boundary;
    expect(one.verdictSays).toBe("all passed");
    expect(one.checks.some((check) => check.reads === "failed")).toBe(false);
    expect(one.checks.some((check) => check.result !== undefined)).toBe(false);
  });

  test("a task whose own agent stopped without finishing carries its reason", () => {
    expect(taskIn(groupFailed(), 3, "T6").failedReason).toBe(
      "The row's press opened the Board rather than the Job",
    );
  });
});

describe("a done task a later task edited", () => {
  test("it stays done and is flagged with the task that did it", () => {
    const six = taskIn(doneTouched(), 3, "T6");
    expect(six.mark).toBe("done");
    expect(six.touchedSays).toBe("touched later · T7");
  });

  test("the task that did it is still working, with turns and no cost", () => {
    const seven = taskIn(doneTouched(), 4, "T7");
    expect(seven.turnsSays).toBe("6 turns");
    expect(seven.costSays).toBeUndefined();
  });
});

describe("what the board refuses to draw", () => {
  test("a Job with no group draws no board at all, rather than an empty one", () => {
    expect(planBoardOf(null, undefined, () => undefined)).toBeUndefined();
  });

  // The step is only where a boundary's runs are read from, so a plan whose
  // step cannot be found still draws: the groups are the plan's, and the
  // boundary simply carries no run. The second board needed the step for its
  // own heading, and there is no second board.
  test("a plan whose step cannot be found still draws its groups, and carries no run", () => {
    const read = reading(groupFailed());
    const drawn = planBoardOf(read.whole, groupFailed().draft, () => undefined)!;
    expect(drawn.groups.map((one) => one.ordinal)).toEqual([1, 2, 3, 4]);
    expect(drawn.groups[2]!.boundary.checks.some((one) => one.result !== undefined)).toBe(false);
    expect(drawn.groups[2]!.boundary.verdictSays).toBe("screens_test failed");
  });

  test("a boundary with no Check says so rather than drawing an empty bar", () => {
    const read = reading(executingSequential());
    const bare = { ...read.groups[3]!, checks_selected: [] };
    const drawn = boundaryOf(bare, [], read.whole, read.step);
    expect(drawn.checks).toEqual([]);
    expect(drawn.checksAbsent).toBe("No Check runs at this group's end.");
  });

  // The band said Fleet serves no case yet, which is a fact about the build.
  // The owner cut it on 28 Sep, and the boundary now carries no case and no
  // sentence about one.
  test("a boundary with no case says nothing about cases at all", () => {
    const three = groupAt(executingSequential(), 3).boundary;
    expect(three.tests).toBeUndefined();
    expect(JSON.stringify(three)).not.toContain("Fleet");
  });

  test("a boundary nothing has reached says nothing about a verdict", () => {
    const read = reading(executingSequential());
    expect(verdictSaid(read.groups[3]!, [])).toBeUndefined();
  });

  test("how a task is run is read off its treatment and never invented", () => {
    const task = reading(executingSequential()).groups[0]!.tasks[0]!;
    expect(runBySaid({ ...task, treatment: "step_drone" })).toBe("the step's Drone");
    expect(runBySaid({ ...task, treatment: "job" })).toBe("a Job of its own");
  });
});

describe("the open step's own line on Overview", () => {
  test("it counts tasks and never Drones", () => {
    const groups = reading(executingSequential()).groups;
    expect(tasksField(groups)).toEqual({ label: "Tasks", value: "4 of 8 done · 1 working", mono: true });
  });

  test("a step whose groups are all through drops the working half rather than printing a zero", () => {
    const groups = reading(executingConcurrent()).groups;
    expect(tasksField(groups)?.value).toBe("6 of 8 done");
  });

  test("a step with no group draws no line at all", () => {
    expect(tasksField([])).toBeUndefined();
  });
});

// The cap rode on a concurrent group's shape line until 28 Sep, behind a
// middle dot: `2 tasks, at the same time · this Job runs 2 Drones at once`.
// The owner cut it. Two facts in one sentence, and the second is the Job's
// rather than the group's, which Overview already draws frozen at the gate.
describe("a group's shape says the shape and nothing else", () => {
  test("a concurrent group says how many tasks and that they run together", () => {
    expect(groupAt(executingConcurrent(), 3).shapeSays).toBe("2 tasks, at the same time");
  });

  test("a group whose tasks run in order says so", () => {
    expect(groupAt(executingConcurrent(), 1).shapeSays).toBe("2 tasks, one after another");
  });

  test("no group's line mentions the Job's Drone cap, whatever the gate settled", () => {
    const drawn = board(executingConcurrent())!;
    expect(drawn.groups.map((one) => one.shapeSays).join(" ")).not.toContain("Drones at once");
  });
});
