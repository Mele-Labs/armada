// A plan recorded again after work was done reaches the thread's card with each task's real state and what changed on it; a first plan,
// or a later one that says nothing changed, reaches it with none.

import { describe, expect, it } from "vitest";
import type { WorkPlan } from "@armada/screens/src/draft/sessions";

import { planRevision } from "./plan-revision";

const PLAN: WorkPlan = {
  version: 5,
  scope: ["crates/fleet"],
  approved: false,
  groups: [
    { id: "g1", agent: "rust-engineer", title: "Warm a bay", paths: ["crates/fleet"], tasks: [{ id: "t1", text: "Find the bay", state: "done" }, { id: "t2", text: "Warm it", state: "done" }] },
    {
      id: "g2",
      agent: "bridge-engineer",
      title: "Show it",
      paths: ["apps/desktop"],
      tasks: [
        { id: "t3", text: "Draw it", state: "done" },
        { id: "t4", text: "Keep the tile still", state: "working" },
        { id: "t5", text: "Say how long it took", state: "open" },
        { id: "t6", text: "Show it in the pane", state: "open" },
      ],
    },
  ],
  changes: { added: ["t6"], renamed: [{ id: "t5", from: "Show how long it took" }], removed: [{ group: "g2", text: "Add a button" }] },
};

describe("a plan as its revision", () => {
  it("carries every task's own state, not open for all", () => {
    const groups = planRevision(PLAN).revision?.groups ?? [];
    expect(groups.map((group) => group.tasks.map((task) => task.state))).toEqual([["done", "done"], ["done", "working", "open", "open"]]);
  });

  it("marks the task added and the task renamed, with what it was, and names what was taken out", () => {
    const [first, second] = planRevision(PLAN).revision?.groups ?? [];
    expect(first?.tasks.every((task) => task.since === undefined)).toBe(true);
    expect(first?.gone).toEqual([]);
    expect(second?.tasks.map((task) => task.since)).toEqual([undefined, undefined, { kind: "renamed", from: "Show how long it took" }, { kind: "added" }]);
    expect(second?.gone).toEqual(["Add a button"]);
  });

  it("keeps the plan's order, titles and subagents", () => {
    const groups = planRevision(PLAN).revision?.groups ?? [];
    expect(groups.map((group) => [group.id, group.title, group.agent])).toEqual([["g1", "Warm a bay", "rust-engineer"], ["g2", "Show it", "bridge-engineer"]]);
  });

  it("is nothing for a first plan", () => {
    expect(planRevision({ ...PLAN, version: 1 })).toEqual({});
    const { version: _version, ...unversioned } = PLAN;
    expect(planRevision(unversioned)).toEqual({});
  });

  it("is nothing for a later plan that records no change", () => {
    expect(planRevision({ ...PLAN, changes: { added: [], renamed: [], removed: [] } })).toEqual({});
    const { changes: _changes, ...unchanged } = PLAN;
    expect(planRevision(unchanged)).toEqual({});
  });
});
