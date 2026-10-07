// A group's Checks across a skipped Check and a second run of the gate, read off
// what Fleet served for the owner's Job 13 (7 Oct 2026). Fleet recorded `skipped`
// for the Checks no changed file reaches; Bridge drew them failed while the gate
// ran and passed after, and drew the first run's red beside the second run's rows.

import { describe, expect, test } from "vitest";
import type { CheckRun, CheckUnderway, DeclaredCheck } from "@armada/protocol";

import { taskGroupsOf } from "./draft/group";
import { sampleDetail, samplePlan, sampleStep, sampleTask } from "@armada/screens/src/draft/sample";
import { boundaryOf } from "./plan-board";

const NAMES = ["build", "typecheck", "desktop_test", "xtask_test"];
const SKIPPED = "build";
const RUNNING = "desktop_test";
const WHY = "no changed file is under paths no workspace owns";
const declared = (name: string): DeclaredCheck => ({ kind: "manifest_check", name, run: name });

const ran = (name: string, over: Partial<CheckRun> = {}): CheckRun => ({ attempt: 2, name, outcome: "passed", ...over });
const kept = (name: string, run: number, over: Partial<CheckRun> = {}): CheckRun =>
  ran(name, { attempt: run + 1, group: "G1", group_attempt: run, ...over });

/** Run one of the group: `build` skipped, `desktop_test` red, the rest green. */
const FIRST = NAMES.map((name) =>
  name === SKIPPED
    ? kept(name, 1, { outcome: "skipped", produced: WHY })
    : kept(name, 1, name === RUNNING ? { outcome: "failed", expected: "exits 0", produced: "it exited 1" } : {}),
);

const answered = (name: string, over: Partial<CheckRun> = {}): CheckUnderway => ({
  name,
  started_at: "2026-10-07T05:00:00Z",
  ran: ran(name, over),
});

/** The group of two done tasks, in the given state, over a step holding these. */
function read(state: string, check_runs: CheckRun[], checking?: CheckUnderway[]) {
  const step = sampleStep({
    checks: NAMES.map(declared),
    check_runs,
    ...(checking === undefined ? {} : { checking: { attempt: 3, checks: checking } }),
  });
  const detail = sampleDetail({
    steps: [step],
    work_plan: samplePlan(
      [sampleTask({ id: "T1", group: "G1", state: "done" }), sampleTask({ id: "T2", group: "G1", state: "done" })],
      { groups: [{ id: "G1", tasks: ["T1", "T2"], state }] },
    ),
  });
  const [group] = taskGroupsOf(detail);
  return boundaryOf(group!, [], detail, detail.steps[0]);
}

const readsOf = (boundary: ReturnType<typeof read>) => Object.fromEntries(boundary.checks.map((one) => [one.name, one.reads]));

describe("a skipped Check", () => {
  test("reads skipped while the first run's gate is live, never failed", () => {
    const live = [answered(SKIPPED, { outcome: "skipped", produced: WHY }), answered("typecheck"), { name: RUNNING, started_at: "2026-10-07T05:00:00Z" }, { name: "xtask_test" }];
    const boundary = read("running", [], live);
    expect(readsOf(boundary)).toEqual({ build: "skipped", typecheck: "passed", desktop_test: "running", xtask_test: "not run" });
    expect(boundary.verdictNamed).toBeUndefined();
  });

  test("says why on hover, and opens no log", () => {
    const build = read("passed", FIRST).checks.find((one) => one.name === SKIPPED)!;
    expect(build.why).toBe(WHY);
    expect(build.onOpen).toBeUndefined();
  });

  test("reads skipped once its run is kept, not passed", () => {
    expect(readsOf(read("passed", FIRST)).build).toBe("skipped");
  });

  test("a boundary whose every Check skipped says nothing of passing", () => {
    const all = NAMES.map((name) => kept(name, 1, { outcome: "skipped", produced: WHY }));
    expect(read("passed", all).verdictSays).toBeUndefined();
  });
});

describe("a group run again", () => {
  const second = [answered(SKIPPED, { attempt: 3, outcome: "skipped", produced: WHY }), answered("typecheck", { attempt: 3 }), { name: RUNNING, started_at: "2026-10-07T05:01:00Z" }, { name: "xtask_test" }];

  test("while its gate is live, every row is the live run's: the running Check reads running", () => {
    const boundary = read("retrying", FIRST, second);
    expect(readsOf(boundary)).toEqual({ build: "skipped", typecheck: "passed", desktop_test: "running", xtask_test: "not run" });
  });

  test("while its gate is live, the collapsed line is the live run's too", () => {
    const boundary = read("retrying", FIRST, second);
    expect(boundary.verdictSays).toBe("running now");
    expect(boundary.verdictNamed).toBeUndefined();
    expect(boundary.retrySays).toBeUndefined();
  });

  test("with no gate live, the latest finished run is read whole", () => {
    const boundary = read("retrying", FIRST);
    expect(readsOf(boundary)).toEqual({ build: "skipped", typecheck: "passed", desktop_test: "failed", xtask_test: "passed" });
    expect(boundary.verdictSays).toBe("desktop_test failed");
  });

  test("never reads two runs at once: the later one decides every row", () => {
    const later = NAMES.map((name) => kept(name, 2, name === SKIPPED ? { outcome: "skipped", produced: WHY } : {}));
    const boundary = read("passed", [...FIRST, ...later]);
    expect(readsOf(boundary)).toEqual({ build: "skipped", typecheck: "passed", desktop_test: "passed", xtask_test: "passed" });
    // `build` skipped in the later run too, so it did not all pass.
    expect(boundary.verdictSays).toBe("none failed");
  });

  test("says all passed where nothing was skipped", () => {
    const green = ["typecheck", RUNNING].map((name) => kept(name, 1));
    expect(read("passed", green).verdictSays).toBe("all passed");
  });
});
