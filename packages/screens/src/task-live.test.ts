// What a task's own Drone is doing, for Plan's task sheet. `#1536`.
//
// **Every part is absent where the task has no Drone of its own**, which is
// every task on today's Fleet: a sentence about "its agent" there would be
// about the Job's one Drone.

import { describe, expect, test } from "vitest";

import type { TaskView } from "./draft/task";
import { executingConcurrent, executingSequential, groupFailed } from "./fixtures/build/arc";
import { doingOfTask, lastEditOf } from "./task-live";

type Moment = ReturnType<typeof executingSequential>;

const taskNamed = (moment: Moment, id: string): TaskView =>
  moment.draft!.groups!.flatMap((one) => one.tasks).find((one) => one.id === id)!;

const transcriptOf = (moment: Moment, id: string) =>
  moment.draft!.drones!.find((one) => one.task === id)?.transcript;

describe("what the sheet says a task is doing", () => {
  test("a working task shows its turns and no cost", () => {
    expect(doingOfTask(taskNamed(executingSequential(), "T5"))).toBe("14 turns");
  });

  test("a finished task shows its turns and what its own agent spent", () => {
    expect(doingOfTask(taskNamed(executingConcurrent(), "T5"))).toBe("27 turns · ~$1.90");
  });

  test("a working task with no turns read says nothing", () => {
    const task = { ...taskNamed(executingSequential(), "T5") };
    delete task.turns;
    expect(doingOfTask(task)).toBeUndefined();
  });

  // The sheet already draws why a failed task stopped; saying it twice is
  // the same sentence in two fields.
  test("a failed task says nothing here", () => {
    expect(doingOfTask(taskNamed(groupFailed(), "T6"))).toBeUndefined();
  });

  test("an open task has no agent to speak of", () => {
    expect(doingOfTask(taskNamed(executingSequential(), "T8"))).toBeUndefined();
  });

  // Today's wire: the task is worked by the step's Drone, and the Job's Drone
  // id is copied onto the working task. Nothing here is that task's own.
  test("a task worked by the step's Drone says nothing", () => {
    const task = { ...taskNamed(executingSequential(), "T5"), treatment: "step_drone" as const };
    expect(doingOfTask(task)).toBeUndefined();
  });
});

describe("its last edit", () => {
  test("is the last file its own Drone wrote", () => {
    const moment = executingSequential();
    expect(lastEditOf(transcriptOf(moment, "T5"))).toEqual({ path: taskNamed(moment, "T5").scope[0] });
  });

  test("is absent where no transcript is served", () => {
    expect(lastEditOf(undefined)).toBeUndefined();
  });

  test("is absent where the Drone has written nothing", () => {
    const read = transcriptOf(executingSequential(), "T5")!.filter((turn) => turn.saw.event !== "called");
    expect(lastEditOf(read)).toBeUndefined();
  });
});
