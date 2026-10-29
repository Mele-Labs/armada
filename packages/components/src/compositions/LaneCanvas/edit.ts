// Editing the lanes by hand: what a press on a node's toolbar, a field in its panel or a connector dragged between two nodes comes to. Each is a pure change to the
// Workspace, so the editor can draw what it makes and the host holds it. **A mock's edits**: no Fleet serves a Workspace yet.

import { nodeId } from "./model";
import type { Autonomy, LaneStep, Ref, Rule, StateDef, StateReads, Workspace, WorkflowPlace } from "./model";

const slug = (label: string): string => label.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "") || "new";

/** `base`, or `base_2`, `base_3`, until it is one no `taken` holds. */
export function uniqueId(label: string, taken: readonly string[]): string {
  const base = slug(label);
  let id = base;
  for (let n = 2; taken.includes(id); n += 1) id = `${base}_${n}`;
  return id;
}

const replaceWorkflow = (workspace: Workspace, workflow: string, place: WorkflowPlace): Workspace => ({ ...workspace, workflows: { ...workspace.workflows, [workflow]: place } });

/** Every column from `col` on moves one to the right, so a node can be put at `col`. Steps and states share the columns, so both move. */
function shiftedFrom(place: WorkflowPlace, col: number): WorkflowPlace {
  const move = (at: number) => (at >= col ? at + 1 : at);
  return {
    steps: Object.fromEntries(Object.entries(place.steps).map(([id, one]) => [id, { ...one, col: move(one.col) }])),
    states: Object.fromEntries(Object.entries(place.states).map(([id, at]) => [id, move(at)])),
  };
}

// ---- Rules ---------------------------------------------------------------------------------------------------------------

export type RulePatch = Partial<Pick<Rule, "label" | "from" | "at" | "to" | "ifs" | "does" | "autonomy" | "overrides">>;

export const withRule = (workspace: Workspace, id: string, patch: RulePatch): Workspace => ({
  ...workspace,
  rules: workspace.rules.map((one) => (one.id === id ? { ...one, ...patch } : one)),
});

export const withoutRule = (workspace: Workspace, id: string): Workspace => ({ ...workspace, rules: workspace.rules.filter((one) => one.id !== id) });

export const withNewRule = (workspace: Workspace, rule: Rule): Workspace => (workspace.rules.some((one) => one.id === rule.id) ? workspace : { ...workspace, rules: [...workspace.rules, rule] });

/** One repository's autonomy for a rule, where it differs from the Workspace's; the same as the Workspace's takes the override away. */
export function withOverride(workspace: Workspace, id: string, scope: string, autonomy: Autonomy): Workspace {
  const rule = workspace.rules.find((one) => one.id === id);
  if (rule === undefined) return workspace;
  const rest = (rule.overrides ?? []).filter((one) => one.scope !== scope);
  return withRule(workspace, id, { overrides: autonomy === rule.autonomy ? rest : [...rest, { scope, autonomy }] });
}

/** The words a rule is drawn with for an edge between two nodes: its source, its target, and what moving there says. */
export function ruleBetween(
  workspace: Workspace,
  workflow: string,
  from: Ref,
  to: Ref,
  names: { from: string; to: string; lane: string },
): Rule {
  const toStep = to.lane === "workflow";
  return {
    id: uniqueId(`${names.from}_to_${names.to}`, workspace.rules.map((one) => one.id)),
    label: toStep ? `Hand to ${names.to}` : `Move to ${names.to}`,
    from,
    ...(from.lane === "workflow" ? { at: "starts" as const } : {}),
    to,
    ifs: [],
    does: [toStep ? { act: "agent" as const, text: `Hand over to ${names.to}` } : { act: "tracker" as const, text: `Move the ${names.lane.toLowerCase()} to ${names.to}` }],
    autonomy: "asks",
    kept: "yours",
    scope: "workspace",
    workflow,
  };
}

// ---- States --------------------------------------------------------------------------------------------------------------

const mapState = (workspace: Workspace, lane: string, state: string, change: (one: StateDef) => StateDef): Workspace => ({
  ...workspace,
  lifecycles: workspace.lifecycles.map((one) => (one.id === lane ? { ...one, states: one.states.map((each) => (each.id === state ? change(each) : each)) } : one)),
});

export const withStateLabel = (workspace: Workspace, lane: string, state: string, label: string): Workspace => mapState(workspace, lane, state, (one) => ({ ...one, label }));

/** The values that count as being in a state, for one source and one scope: the Workspace's, or a repository's. Empty values take a repository's own away. */
export function withReads(workspace: Workspace, lane: string, state: string, read: { source: string; scope: string; values: readonly string[] }): Workspace {
  return mapState(workspace, lane, state, (one) => {
    const others: readonly StateReads[] = one.reads.filter((each) => !(each.source === read.source && each.scope === read.scope));
    return { ...one, reads: read.values.length === 0 ? others : [...others, { source: read.source, scope: read.scope, values: read.values }] };
  });
}

/** A new state after `after` in a lifecycle, placed one column on from it, with the later columns moved over. */
export function withStateAfter(workspace: Workspace, workflow: string, lane: string, after: string, label: string): { workspace: Workspace; id: string } {
  const lifecycle = workspace.lifecycles.find((one) => one.id === lane);
  const place = workspace.workflows[workflow];
  if (lifecycle === undefined || place === undefined) return { workspace, id: after };
  const id = uniqueId(label, lifecycle.states.map((one) => one.id));
  const at = lifecycle.states.findIndex((one) => one.id === after);
  const source = lifecycle.states.find((one) => one.id === after)?.reads[0]?.source ?? lifecycle.from;
  const state: StateDef = { id, label, reads: [{ source, values: [label], scope: "workspace" }] };
  const states = [...lifecycle.states.slice(0, at + 1), state, ...lifecycle.states.slice(at + 1)];
  const leaves = lifecycle.next.filter(([from]) => from === after);
  const next = [...lifecycle.next.filter(([from]) => from !== after), [after, id] as const, ...leaves.map(([, to]) => [id, to] as const)];
  const col = (place.states[nodeId(lane, after)] ?? 0) + 1;
  const moved = shiftedFrom(place, col);
  return {
    workspace: replaceWorkflow({ ...workspace, lifecycles: workspace.lifecycles.map((one) => (one.id === lane ? { ...one, states, next } : one)) }, workflow, { ...moved, states: { ...moved.states, [nodeId(lane, id)]: col } }),
    id,
  };
}

/** A state taken off its lifecycle: the ways in and out are joined, and the rules that named it go with it. */
export function withoutState(workspace: Workspace, lane: string, state: string): Workspace {
  const gone = nodeId(lane, state);
  return {
    ...workspace,
    lifecycles: workspace.lifecycles.map((one) => {
      if (one.id !== lane) return one;
      const into = one.next.filter(([, to]) => to === state).map(([from]) => from);
      const out = one.next.filter(([from]) => from === state).map(([, to]) => to);
      const kept = one.next.filter(([from, to]) => from !== state && to !== state);
      return { ...one, states: one.states.filter((each) => each.id !== state), next: [...kept, ...into.flatMap((from) => out.map((to) => [from, to] as const))] };
    }),
    workflows: Object.fromEntries(Object.entries(workspace.workflows).map(([id, place]) => [id, { ...place, states: Object.fromEntries(Object.entries(place.states).filter(([key]) => key !== gone)) }])),
    rules: workspace.rules.filter((rule) => nodeId(rule.from.lane, rule.from.item) !== gone && nodeId(rule.to.lane, rule.to.item) !== gone),
  };
}

// ---- Steps ---------------------------------------------------------------------------------------------------------------

/** A step placed one column on from `after`, with every step's column kept as it is drawn so the later ones move over. */
export function withStepAfter(workspace: Workspace, workflow: string, current: readonly LaneStep[], after: string, id: string): Workspace {
  const here = current.find((one) => one.id === after);
  const was = workspace.workflows[workflow] ?? { steps: {}, states: {} };
  const kept: WorkflowPlace = { ...was, steps: Object.fromEntries(current.map((one) => [one.id, { col: one.col, ...(one.endsOn === undefined ? {} : { endsOn: one.endsOn }) }])) };
  const col = (here?.col ?? current.length - 1) + 1;
  const moved = shiftedFrom(kept, col);
  return replaceWorkflow(workspace, workflow, { ...moved, steps: { ...moved.steps, [id]: { col } } });
}

/** A step taken off the workflow, and the rules that named it. */
export function withoutStep(workspace: Workspace, workflow: string, id: string): Workspace {
  const gone = nodeId("workflow", id);
  const place = workspace.workflows[workflow];
  return {
    ...(place === undefined ? workspace : replaceWorkflow(workspace, workflow, { ...place, steps: Object.fromEntries(Object.entries(place.steps).filter(([key]) => key !== id)) })),
    rules: workspace.rules.filter((rule) => !((rule.workflow === undefined || rule.workflow === workflow) && (nodeId(rule.from.lane, rule.from.item) === gone || nodeId(rule.to.lane, rule.to.item) === gone))),
  };
}

/** The steps run in the order given: the columns they held are dealt out again in that order, so what sits beside them on the other lanes stays where it is. */
export function withStepsOrder(workspace: Workspace, workflow: string, current: readonly LaneStep[], order: readonly string[]): Workspace {
  const cols = current.map((one) => one.col).sort((a, b) => a - b);
  const was = new Map(current.map((one) => [one.id, one] as const));
  const place = workspace.workflows[workflow] ?? { steps: {}, states: {} };
  const steps: Record<string, Omit<LaneStep, "id" | "label">> = {};
  order.forEach((id, at) => {
    const one = was.get(id);
    steps[id] = { col: cols[at] ?? at, ...(one?.endsOn === undefined ? {} : { endsOn: one.endsOn }) };
  });
  return replaceWorkflow(workspace, workflow, { ...place, steps });
}
