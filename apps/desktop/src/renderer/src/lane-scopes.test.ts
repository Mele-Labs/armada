// What a tab of the workflow editor adds is that level's own. A step or a state added on a repository's tab exists there and nowhere above, the tab says it differs, and the
// tab deletes it again. Added on the default, it belongs to the workflow. These are the model's and the edits', the canvas and the panel are the walks'.

import { changedAt, stepsOf } from "@armada/components/src/compositions/LaneCanvas/build";
import { withAddedStep, withReads, withRuleAt, withStateAfter, withStateLabelAt, withStepAddedAt, withStepAfter, withoutStep, ruleBetween, withNewRule } from "@armada/components/src/compositions/LaneCanvas/edit";
import { GEO, withToolbar } from "@armada/components/src/compositions/LaneCanvas/layout";
import { DEFAULT, existsAt, lifecyclesFor, stateDiffers, stateLabelAt, stepsAt, workspaceLevel } from "@armada/components/src/compositions/LaneCanvas/model";
import type { LifecycleDef, Workspace } from "@armada/components/src/compositions/LaneCanvas/model";
import { expect, test } from "vitest";

/** The level the Workspace `my-store` is kept at. */
const MY_STORE = workspaceLevel("my-store");

const DEFINED = [
  { id: "plan", label: "Plan" },
  { id: "build", label: "Build" },
  { id: "ship", label: "Ship" },
];
const ticket: LifecycleDef = {
  id: "ticket",
  label: "Ticket",
  from: "Jira",
  states: [
    { id: "todo", label: "To do", reads: [{ source: "Jira", values: ["To Do"], scope: DEFAULT }] },
    { id: "done", label: "Done", reads: [{ source: "Jira", values: ["Done"], scope: DEFAULT }], end: true },
  ],
  next: [["todo", "done"]],
};
const workspace: Workspace = {
  id: "my-store",
  name: "my-store",
  repositories: [
    { id: "store-api", name: "store-api" },
    { id: "store-web", name: "store-web" },
  ],
  lifecycles: [ticket],
  workflows: { feature: { steps: { plan: { col: 0 }, build: { col: 1 }, ship: { col: 2 } }, states: { "ticket/todo": 0, "ticket/done": 2 } } },
  rules: [],
};
const idsAt = (from: Workspace, scope: string) => stepsAt(DEFINED, from.workflows.feature, scope, from, (one) => ({ id: one.id, label: one.label })).map((one) => one.id);
const withApiStep = () => withStepAddedAt(workspace, "feature", stepsOf(workspace, "feature", DEFINED), "build", "lint", "Lint", "store-api");

test("a step added on a repository runs there and not on the tabs above or beside it", () => {
  const made = withApiStep();
  expect(idsAt(made, "store-api")).toEqual(["plan", "build", "lint", "ship"]);
  expect(idsAt(made, "store-web")).toEqual(["plan", "build", "ship"]);
  expect(idsAt(made, MY_STORE)).toEqual(["plan", "build", "ship"]);
  expect(idsAt(made, DEFAULT)).toEqual(["plan", "build", "ship"]);
});

test("a step added on the Workspace runs on its repositories too", () => {
  const made = withStepAddedAt(workspace, "feature", stepsOf(workspace, "feature", DEFINED), "build", "lint", "Lint", MY_STORE);
  expect(idsAt(made, "store-api")).toContain("lint");
  expect(idsAt(made, "store-web")).toContain("lint");
  expect(idsAt(made, DEFAULT)).not.toContain("lint");
  expect(existsAt(MY_STORE, "store-api", workspace)).toBe(true);
  expect(existsAt("store-api", MY_STORE, workspace)).toBe(false);
});

test("the tab that added a step says it differs, and no other tab does", () => {
  const made = withApiStep();
  expect(changedAt(workspace, "feature", DEFINED, "store-api")).toBe(false);
  expect(changedAt(made, "feature", DEFINED, "store-api")).toBe(true);
  expect(changedAt(made, "feature", DEFINED, "store-web")).toBe(false);
  expect(changedAt(made, "feature", DEFINED, MY_STORE)).toBe(false);
});

test("deleting a step the tab added takes its mark away, and the definition never held it", () => {
  const gone = withoutStep(withApiStep(), "feature", "lint");
  expect(changedAt(gone, "feature", DEFINED, "store-api")).toBe(false);
  expect(gone.workflows.feature!.added).toEqual([]);
  expect(Object.keys(gone.workflows.feature!.steps)).toEqual(["plan", "build", "ship"]);
});

test("a step added on a repository is named and worded on that tab alone", () => {
  const made = withAddedStep(withApiStep(), "feature", "lint", { label: "Lint it", words: "Run the linter." });
  expect(made.workflows.feature!.added).toEqual([{ id: "lint", label: "Lint it", after: "build", scope: "store-api", words: "Run the linter." }]);
  expect(DEFINED.map((one) => one.label)).toEqual(["Plan", "Build", "Ship"]);
});

test("a step added on the default moves the columns of one a repository added, which the default does not draw", () => {
  const api = withApiStep();
  const drawn = stepsOf(api, "feature", stepsAt(DEFINED, api.workflows.feature, DEFAULT, api, (one) => ({ id: one.id, label: one.label })));
  const made = withStepAfter(api, "feature", drawn, "build", "review");
  // The definition is the caller's: it writes the step into the workflow, so here it is added to the list that stands for it.
  const defined = [DEFINED[0]!, DEFINED[1]!, { id: "review", label: "Review" }, DEFINED[2]!];
  const at = (scope: string) => stepsOf(made, "feature", stepsAt(defined, made.workflows.feature, scope, made, (one) => ({ id: one.id, label: one.label })), scope).map((one) => `${one.id}:${one.col}`);
  expect(at(DEFAULT)).toEqual(["plan:0", "build:1", "review:2", "ship:4"]);
  expect(at("store-api")).toEqual(["plan:0", "build:1", "review:2", "lint:3", "ship:4"]);
});

test("a state added on a repository is in its lane there, and the lane reads straight through it elsewhere", () => {
  const { workspace: made, id } = withStateAfter(workspace, "feature", "ticket", "todo", "Blocked", "store-api");
  const states = (scope: string) => lifecyclesFor(made, "feature", scope)[0]!.states.map((one) => one.id);
  expect(states("store-api")).toEqual(["todo", id, "done"]);
  expect(states("store-web")).toEqual(["todo", "done"]);
  expect(states(DEFAULT)).toEqual(["todo", "done"]);
  expect(lifecyclesFor(made, "feature", "store-web")[0]!.next).toEqual([["todo", "done"]]);
  expect(stateDiffers(made.lifecycles[0]!.states.find((one) => one.id === id)!)).toEqual(["store-api"]);
});

test("a state a repository added is named and read as that repository's own, with no level above to differ from", () => {
  const { workspace: made, id } = withStateAfter(workspace, "feature", "ticket", "todo", "Blocked", "store-api");
  const named = withStateLabelAt(made, "ticket", id, "store-api", "On hold");
  const state = named.lifecycles[0]!.states.find((one) => one.id === id)!;
  expect(state.label).toBe("On hold");
  expect(state.names).toBeUndefined();
  expect(stateLabelAt(state, "store-api", made)).toBe("On hold");
  const cleared = withReads(named, "ticket", id, { source: "Jira", scope: "store-api", values: [] });
  expect(cleared.lifecycles[0]!.states.find((one) => one.id === id)!.reads).toEqual([{ source: "Jira", scope: "store-api", values: [] }]);
});

test("a rule drawn on a repository's tab is that repository's, and its fields are its own and not an override", () => {
  const rule = ruleBetween(workspace, "feature", { lane: "workflow", item: "build" }, { lane: "ticket", item: "done" }, { from: "Build", to: "Done", lane: "Ticket" }, "store-api");
  expect(rule.scope).toBe("store-api");
  const made = withRuleAt(withNewRule(workspace, rule), rule.id, "store-api", { label: "Close it" });
  const kept = made.rules.find((one) => one.id === rule.id)!;
  expect(kept.label).toBe("Close it");
  expect(kept.overrides ?? []).toEqual([]);
});

test("a toolbar's room is added over a node and widens a narrow one around its centre", () => {
  const node = { x: 400, y: 200, w: GEO.nodeW, h: GEO.stepH };
  const room = GEO.toolbarH + GEO.toolbarGap;
  expect(withToolbar(node, "above")).toEqual({ x: node.x - (GEO.toolbarW - node.w) / 2, y: node.y - room, w: GEO.toolbarW, h: node.h + room });
  expect(withToolbar(node, "below")).toEqual({ x: node.x - (GEO.toolbarW - node.w) / 2, y: node.y, w: GEO.toolbarW, h: node.h + room });
  const wide = { x: 0, y: 0, w: GEO.toolbarW + 40, h: 30 };
  expect(withToolbar(wide, "above").x).toBe(0);
  expect(withToolbar(wide, "above").w).toBe(wide.w);
});
