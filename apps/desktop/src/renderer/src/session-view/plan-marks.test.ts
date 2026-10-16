// What he marks on the plan reads over it and goes to the agent as lines, and nothing marked changes the plan itself.

import { describe, expect, it } from "vitest";
import type { PlanMark, WorkPlan } from "@armada/screens/src/draft/sessions";
import { wireMarksOf } from "@armada/screens/src/session-work";

import { addGroup, addTask, linesOf, move, remove, rename, viewOf } from "./plan-marks";

const PLAN: WorkPlan = {
  version: 1,
  scope: ["crates/fleet"],
  approved: false,
  groups: [
    { id: "g1", agent: "rust-engineer", title: "Retry the lease", paths: ["crates/fleet"], tasks: [{ id: "t1", text: "Back off", state: "open" }, { id: "t2", text: "Cap it", state: "open" }] },
    { id: "g2", agent: "protocol-engineer", title: "Wire", paths: ["crates/ipc"], tasks: [] },
  ],
};

describe("marks over the plan", () => {
  it("an added task is a task of its group and not a group of its own", () => {
    const marks = addTask([], "g1", "Log each attempt");
    const view = viewOf(PLAN, marks);
    expect(view.map((one) => one.id)).toEqual(["g1", "g2"]);
    expect(view[0]?.tasks.map((one) => one.text)).toEqual(["Back off", "Cap it", "Log each attempt"]);
    expect(view[0]?.tasks[2]).toMatchObject({ added: true, state: "open" });
  });

  it("an added group is a group with no tasks yet", () => {
    const view = viewOf(PLAN, addGroup([], "Docs"));
    expect(view.map((one) => one.title)).toEqual(["Retry the lease", "Wire", "Docs"]);
    expect(view[2]).toMatchObject({ added: true, tasks: [] });
  });

  it("a rename shows what the text was, and putting it back leaves no mark", () => {
    const renamed = rename(PLAN, [], { group: "g1", task: "t1" }, "Back off with a ceiling");
    expect(viewOf(PLAN, renamed)[0]?.tasks[0]).toMatchObject({ text: "Back off with a ceiling", was: "Back off" });
    expect(rename(PLAN, renamed, { group: "g1", task: "t1" }, "Back off")).toEqual([]);
  });

  it("removing something he added takes it out, and removing the plan's own strikes it", () => {
    const added = addTask([], "g1", "Log it");
    const id = (added[0] as PlanMark & { kind: "add" }).id;
    expect(remove(added, { group: "g1", task: id })).toEqual([]);
    expect(viewOf(PLAN, remove([], { group: "g1", task: "t2" }))[0]?.tasks[1]).toMatchObject({ removed: true });
  });

  it("a task moved to another group is drawn there, and moved back is not moved", () => {
    const moved = move(PLAN, [], "t1", "g2");
    expect(viewOf(PLAN, moved)[1]?.tasks[0]).toMatchObject({ id: "t1", movedFrom: "Retry the lease" });
    expect(move(PLAN, moved, "t1", "g1")).toEqual([]);
  });

  it("an earlier plan's changes mark the group and task the agent added or renamed", () => {
    const again: WorkPlan = { ...PLAN, version: 2, changes: { added: ["t2"], renamed: [{ id: "t1", from: "Wait" }], removed: [{ group: "g1", text: "Sleep" }] } };
    const group = viewOf(again, [])[0]!;
    expect(group.tasks[0]?.since).toEqual({ kind: "renamed", from: "Wait" });
    expect(group.tasks[1]?.since).toEqual({ kind: "added" });
    expect(group.gone).toEqual(["Sleep"]);
  });
});

describe("marks sent to the agent", () => {
  it("are a line each, against the group or task as the plan named it", () => {
    const marks: PlanMark[] = [...rename(PLAN, [], { group: "g1", task: "t1" }, "Back off with a ceiling"), ...remove([], { group: "g1", task: "t2" })];
    expect(linesOf(PLAN, marks)).toEqual([
      { verb: "Renamed", where: "Retry the lease › Back off", from: "Back off", text: "Back off with a ceiling" },
      { verb: "Removed", where: "Retry the lease › Cap it" },
    ]);
  });

  it("go out as the marks Fleet takes: an added task names its group and an added group stands alone", () => {
    const marks = addGroup(addTask([], "g1", "Log it"), "Docs");
    const wire = wireMarksOf(marks);
    expect(wire[0]).toMatchObject({ kind: "add_task", group: "g1", text: "Log it" });
    expect(wire[1]).toMatchObject({ kind: "add_group", text: "Docs" });
    expect(Object.keys(wire[1]!)).not.toContain("group");
  });
});
