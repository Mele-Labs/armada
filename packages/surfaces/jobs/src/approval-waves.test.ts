import { describe, expect, it } from "vitest";

import { CHAIN_SHOWN, chainOf, wavesOf } from "./approval-canvas";
import type { TaskView } from "./draft/task";

const task = (id: string, concurrent_with: string[] = []): TaskView =>
  ({ id, title: id, scope: [], state: "open", touched_after_done: false, group: "G1", concurrent_with }) as unknown as TaskView;

const ids = (waves: TaskView[][]) => waves.map((wave) => wave.map((one) => one.id));

describe("wavesOf", () => {
  it("chains tasks with no declarations", () => {
    expect(ids(wavesOf([task("T1"), task("T2"), task("T3")]))).toEqual([["T1"], ["T2"], ["T3"]]);
  });

  it("puts mutually concurrent tasks in one wave", () => {
    expect(ids(wavesOf([task("T1"), task("T2", ["T3"]), task("T3", ["T2"])]))).toEqual([["T1"], ["T2", "T3"]]);
  });

  it("starts the next wave for a task concurrent with one member but not another", () => {
    const tasks = [task("T1", ["T2"]), task("T2", ["T1", "T3"]), task("T3", ["T2"])];
    expect(ids(wavesOf(tasks))).toEqual([["T1", "T2"], ["T3"]]);
  });
});

describe("chainOf", () => {
  it("draws four cards, then one for the rest, by wave", () => {
    const tasks = Array.from({ length: 6 }, (_, at) => task(`T${at + 1}`));
    const drawn = chainOf(tasks, "Group 1");
    expect(drawn).toHaveLength(CHAIN_SHOWN + 1);
    expect(drawn.at(-1)).toMatchObject({ more: 2, wave: CHAIN_SHOWN });
  });

  it("counts cards, not rows, so a wave of four fills the cap", () => {
    const all = ["T1", "T2", "T3", "T4"];
    const tasks = [...all.map((id) => task(id, all.filter((one) => one !== id))), task("T5")];
    const drawn = chainOf(tasks, "Group 1");
    expect(drawn.map((one) => one.wave)).toEqual([0, 0, 0, 0, 1]);
    expect(drawn.at(-1)?.more).toBe(1);
  });
});
