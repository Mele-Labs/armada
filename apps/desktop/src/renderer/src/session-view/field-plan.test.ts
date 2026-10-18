// The plan as the Field reads it: a piece for each task in the map card's marks, a piece for each group, and a frame for each plot a group's
// paths reach.

import { expect, test } from "vitest";
import type { WorkGroup, WorkPlan, WorkTask, WorkTaskState } from "@armada/screens/src/draft/sessions";

import { PIECE_SAID, groupsOf, pieceOfTask, pieceOfTasks, plotsOf } from "./field-plan";

const task = (id: string, state: WorkTaskState, seen?: boolean): WorkTask => ({ id, text: `Task ${id}`, state, ...(seen === undefined ? {} : { seen }) });
const group = (id: string, paths: string[], tasks: WorkTask[]): WorkGroup => ({ id, agent: "rust-engineer", title: `Group ${id}`, paths, tasks });
const plan = (groups: WorkGroup[]): WorkPlan => ({ scope: [], approved: true, groups });

test("a task is one of the four marks of the map card, or failed", () => {
  expect(pieceOfTask(task("a", "done", true))).toBe("solid");
  expect(pieceOfTask(task("a", "done", false))).toBe("hollow");
  expect(pieceOfTask(task("a", "done"))).toBe("hollow");
  expect(pieceOfTask(task("a", "working", true))).toBe("scaffold");
  expect(pieceOfTask(task("a", "handed_in"))).toBe("hollow");
  expect(pieceOfTask(task("a", "open"))).toBe("ghost");
  expect(pieceOfTask(task("a", "failed"))).toBe("failed");
});

test("each piece has the words its tooltip says", () => {
  expect(PIECE_SAID).toEqual({ solid: "Done, edits seen", hollow: "Done, no edits seen", scaffold: "Working", ghost: "Open", failed: "Failed" });
});

test("a set of tasks is whole when all are done, amber once any has begun, faint while none has, and failed if any failed", () => {
  expect(pieceOfTasks([task("a", "done", true), task("b", "done")])).toBe("solid");
  expect(pieceOfTasks([task("a", "done"), task("b", "done")])).toBe("hollow");
  expect(pieceOfTasks([task("a", "done", true), task("b", "handed_in")])).toBe("solid");
  expect(pieceOfTasks([task("a", "handed_in"), task("b", "handed_in")])).toBe("hollow");
  expect(pieceOfTasks([task("a", "done", true), task("b", "open")])).toBe("scaffold");
  expect(pieceOfTasks([task("a", "open"), task("b", "working")])).toBe("scaffold");
  expect(pieceOfTasks([task("a", "open"), task("b", "open")])).toBe("ghost");
  expect(pieceOfTasks([task("a", "done", true), task("b", "failed"), task("c", "working")])).toBe("failed");
});

test("the groups come in the plan's order with a segment for each task, and a group with no tasks is left out", () => {
  const groups = groupsOf(plan([group("g1", ["crates/store"], [task("a", "done", true), task("b", "working", true), task("c", "open")]), group("g2", ["crates/ipc"], []), group("g3", ["apps/desktop"], [task("d", "failed")])]));
  expect(groups.map((one) => one.id)).toEqual(["g1", "g3"]);
  expect(groups[0]!.tasks).toEqual([
    { id: "a", text: "Task a", piece: "solid" },
    { id: "b", text: "Task b", piece: "scaffold" },
    { id: "c", text: "Task c", piece: "ghost" },
  ]);
  expect(groups[0]).toMatchObject({ title: "Group g1", paths: ["crates/store"], piece: "scaffold" });
  expect(groups[1]).toMatchObject({ piece: "failed" });
});

test("the four marks and failed are each reachable from a plan", () => {
  const groups = groupsOf(
    plan([
      group("s", ["a"], [task("1", "done", true)]),
      group("h", ["b"], [task("2", "done", false)]),
      group("w", ["c"], [task("3", "working")]),
      group("o", ["d"], [task("4", "open")]),
      group("f", ["e"], [task("5", "failed")]),
    ]),
  );
  expect(groups.map((one) => one.piece)).toEqual(["solid", "hollow", "scaffold", "ghost", "failed"]);
});

test("a plot is framed by the groups whose paths are the plot or lie inside it, and says which group and how it stands", () => {
  const frames = plotsOf(
    plan([
      group("g1", ["crates/store"], [task("a", "done", true)]),
      group("g2", ["crates/ipc", "packages/protocol/src"], [task("b", "working", true)]),
      group("g3", ["apps/desktop"], [task("c", "open")]),
    ]),
    ["crates/store", "crates/ipc", "crates/fleet", "packages/protocol", "apps/desktop"],
  );
  expect([...frames.keys()]).toEqual(["crates/store", "crates/ipc", "packages/protocol", "apps/desktop"]);
  expect(frames.get("crates/store")).toEqual({ piece: "solid", said: "Group g1 · Done, edits seen" });
  expect(frames.get("crates/ipc")?.piece).toBe("scaffold");
  expect(frames.get("packages/protocol")).toEqual({ piece: "scaffold", said: "Group g2 · Working" });
  expect(frames.get("apps/desktop")?.piece).toBe("ghost");
  expect(frames.has("crates/fleet")).toBe(false);
});

test("a path that only begins with a plot's name does not reach it", () => {
  const frames = plotsOf(plan([group("g1", ["crates/store-extra"], [task("a", "working")])]), ["crates/store"]);
  expect(frames.size).toBe(0);
});

test("a plot two groups name stands as their tasks do together", () => {
  const frames = plotsOf(plan([group("g1", ["crates/fleet"], [task("a", "done", true)]), group("g2", ["crates/fleet"], [task("b", "open")])]), ["crates/fleet"]);
  expect(frames.get("crates/fleet")).toEqual({ piece: "scaffold", said: "Group g1, Group g2 · Working" });
});
