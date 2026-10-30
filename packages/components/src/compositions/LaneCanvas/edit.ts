// Editing the lanes by hand: what a press on a node's toolbar, a field in its panel or a connector dragged between two nodes comes to. Each is a pure change to the
// Workspace, so the editor can draw what it makes and the host holds it. **A mock's edits**: no Fleet serves a Workspace yet.

import { DEFAULT, levelsOf, nodeId, ruleAt, stateLabelAt } from "./model";
import type { AddedStep, LaneStep, Ref, Rule, RuleOverride, StateDef, StateReads, StepLayer, Waits, Workspace, WorkflowPlace } from "./model";
import { momentOf } from "./waits";

const slug = (label: string): string => label.toLowerCase().replace(/[^a-z0-9]+/g, "_").replace(/^_|_$/g, "") || "new";

/** `base`, or `base_2`, `base_3`, until it is one no `taken` holds. */
export function uniqueId(label: string, taken: readonly string[]): string {
  const base = slug(label);
  let id = base;
  for (let n = 2; taken.includes(id); n += 1) id = `${base}_${n}`;
  return id;
}

const replaceWorkflow = (workspace: Workspace, workflow: string, place: WorkflowPlace): Workspace => ({ ...workspace, workflows: { ...workspace.workflows, [workflow]: place } });

/** A step as the place holds it: everything but its id and its name. */
const placed = ({ id: _id, label: _label, ...rest }: LaneStep): Omit<LaneStep, "id" | "label"> => rest;

/** Every column from `col` on moves one to the right, so a node can be put at `col`. Steps and states share the columns, so both move. */
function shiftedFrom(place: WorkflowPlace, col: number): WorkflowPlace {
  const move = (at: number) => (at >= col ? at + 1 : at);
  return {
    ...place,
    steps: Object.fromEntries(Object.entries(place.steps).map(([id, one]) => [id, { ...one, col: move(one.col) }])),
    states: Object.fromEntries(Object.entries(place.states).map(([id, at]) => [id, move(at)])),
  };
}

// ---- Rules ---------------------------------------------------------------------------------------------------------------

export type RulePatch = Partial<Pick<Rule, "label" | "from" | "at" | "to" | "ifs" | "does" | "autonomy" | "overrides">>;

/** The fields of a rule that can differ at a level below the default. */
export type RuleFields = Partial<Pick<Rule, "label" | "from" | "at" | "ifs" | "does" | "autonomy">>;

export const withRule = (workspace: Workspace, id: string, patch: RulePatch): Workspace => ({
  ...workspace,
  rules: workspace.rules.map((one) => (one.id === id ? { ...one, ...patch } : one)),
});

export const withoutRule = (workspace: Workspace, id: string): Workspace => ({ ...workspace, rules: workspace.rules.filter((one) => one.id !== id) });

export const withNewRule = (workspace: Workspace, rule: Rule): Workspace => (workspace.rules.some((one) => one.id === rule.id) ? workspace : { ...workspace, rules: [...workspace.rules, rule] });

const same = (a: unknown, b: unknown): boolean => JSON.stringify(a) === JSON.stringify(b);

/**
 * A rule's fields set at a level. At the default, or at the level that added the rule, they are the rule's own. At the Workspace or a repository each is an override of the level above, and one that says what that level
 * already does is no override, so it is dropped. A moment left unnamed is the first its node has.
 */
export function withRuleAt(workspace: Workspace, id: string, scope: string, patch: RuleFields): Workspace {
  const rule = workspace.rules.find((one) => one.id === id);
  if (rule === undefined) return workspace;
  // A rule added at a level is that level's own: there is no level above to differ from.
  if (scope === DEFAULT || rule.scope === scope) return withRule(workspace, id, patch);
  const above = ruleAt(rule, levelsOf(scope)[1]!);
  const { scope: _level, ...own } = rule.overrides?.find((one) => one.scope === scope) ?? { scope };
  const next: Record<string, unknown> = { ...own };
  for (const [key, value] of Object.entries(patch)) {
    if (key === "at" ? value === momentOf(above) : same(value, (above as Record<string, unknown>)[key])) delete next[key];
    else next[key] = value;
  }
  const rest = (rule.overrides ?? []).filter((one) => one.scope !== scope);
  return withRule(workspace, id, { overrides: Object.keys(next).length === 0 ? rest : [...rest, { scope, ...next } as RuleOverride] });
}

/** Everything a level changes of a rule taken away: the rule reads as the level above says. */
export function withoutRuleOverride(workspace: Workspace, id: string, scope: string): Workspace {
  const rule = workspace.rules.find((one) => one.id === id);
  return rule === undefined ? workspace : withRule(workspace, id, { overrides: (rule.overrides ?? []).filter((one) => one.scope !== scope) });
}

/** The words a rule is drawn with for an edge between two nodes: its source, its target, and what moving there says. */
export function ruleBetween(
  workspace: Workspace,
  workflow: string,
  from: Ref,
  to: Ref,
  names: { from: string; to: string; lane: string },
  scope: string = DEFAULT,
): Rule {
  const toStep = to.lane === "workflow";
  return {
    id: uniqueId(`${names.from}_to_${names.to}`, workspace.rules.map((one) => one.id)),
    label: toStep ? `Hand to ${names.to}` : `Move to ${names.to}`,
    from,
    ...(from.lane === "workflow" ? { at: "starts" as const } : {}),
    to,
    ifs: [],
    does: [toStep ? { act: "agent" as const, text: `Hand over to ${names.to}` } : { act: "tracker" as const, text: `Move the ${/^[A-Z0-9]+$/.test(names.lane) ? names.lane : names.lane.toLowerCase()} to ${names.to}` }],
    autonomy: "asks",
    kept: "yours",
    scope,
    workflow,
  };
}

// ---- States --------------------------------------------------------------------------------------------------------------

const mapState = (workspace: Workspace, lane: string, state: string, change: (one: StateDef) => StateDef): Workspace => ({
  ...workspace,
  lifecycles: workspace.lifecycles.map((one) => (one.id === lane ? { ...one, states: one.states.map((each) => (each.id === state ? change(each) : each)) } : one)),
});

export const withStateLabel = (workspace: Workspace, lane: string, state: string, label: string): Workspace => mapState(workspace, lane, state, (one) => ({ ...one, label }));

/** A state's name set at a level. At the default, or at the level that added it, it is the state's own; below, a name that is the level above's is no override. */
export function withStateLabelAt(workspace: Workspace, lane: string, state: string, scope: string, label: string): Workspace {
  return mapState(workspace, lane, state, (one) => {
    // A state added at a level is that level's own.
    if (scope === DEFAULT || one.scope === scope) return { ...one, label };
    const others = (one.names ?? []).filter((each) => each.scope !== scope);
    const names = label === stateLabelAt({ ...one, names: others }, levelsOf(scope)[1]!) ? others : [...others, { scope, label }];
    return { ...one, names };
  });
}

/** The values that count as being in a state, for one source and one level: the default's, the Workspace's, or a repository's. Empty values take a level below the default's own away. */
export function withReads(workspace: Workspace, lane: string, state: string, read: { source: string; scope: string; values: readonly string[] }): Workspace {
  return mapState(workspace, lane, state, (one) => {
    const others: readonly StateReads[] = one.reads.filter((each) => !(each.source === read.source && each.scope === read.scope));
    const own = read.scope === DEFAULT || read.scope === one.scope;
    return { ...one, reads: read.values.length === 0 && !own ? others : [...others, { source: read.source, scope: read.scope, values: read.values }] };
  });
}

/** Everything a level changes of a state taken away: its name and its words read as the level above says. */
export function withoutStateOverride(workspace: Workspace, lane: string, state: string, scope: string): Workspace {
  return mapState(workspace, lane, state, (one) => ({ ...one, names: (one.names ?? []).filter((each) => each.scope !== scope), reads: one.reads.filter((each) => each.scope !== scope) }));
}

/** A new state after `after` in a lifecycle, placed one column on from it, with the later columns moved over. */
export function withStateAfter(workspace: Workspace, workflow: string, lane: string, after: string, label: string, scope: string = DEFAULT): { workspace: Workspace; id: string } {
  const lifecycle = workspace.lifecycles.find((one) => one.id === lane);
  const place = workspace.workflows[workflow];
  if (lifecycle === undefined || place === undefined) return { workspace, id: after };
  const id = uniqueId(label, lifecycle.states.map((one) => one.id));
  const at = lifecycle.states.findIndex((one) => one.id === after);
  const source = lifecycle.states.find((one) => one.id === after)?.reads[0]?.source ?? lifecycle.from;
  const state: StateDef = { id, label, ...(scope === DEFAULT ? {} : { scope }), reads: [{ source, values: [label], scope }] };
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

/**
 * A step placed one column on from `after`, with every step's column kept as it is drawn so the later ones move over. The places of steps that are not drawn here, ones added at another
 * level, are kept and move with the rest.
 */
export function withStepAfter(workspace: Workspace, workflow: string, current: readonly LaneStep[], after: string, id: string): Workspace {
  const here = current.find((one) => one.id === after);
  const was = workspace.workflows[workflow] ?? { steps: {}, states: {} };
  const kept: WorkflowPlace = { ...was, steps: { ...was.steps, ...Object.fromEntries(current.map((one) => [one.id, placed(one)])) } };
  const col = (here?.col ?? current.length - 1) + 1;
  const moved = shiftedFrom(kept, col);
  return replaceWorkflow(workspace, workflow, { ...moved, steps: { ...moved.steps, [id]: { col } } });
}

/**
 * A step added at a level below the default: it is placed like any other, and runs there and below. The definition does not hold it, so it is not written to the workflow, and a tab that
 * is not that level or under it does not draw it.
 */
export function withStepAddedAt(workspace: Workspace, workflow: string, current: readonly LaneStep[], after: string, id: string, label: string, scope: string): Workspace {
  const made = withStepAfter(workspace, workflow, current, after, id);
  const place = made.workflows[workflow]!;
  return replaceWorkflow(made, workflow, { ...place, added: [...(place.added ?? []), { id, label, after, scope }] });
}

/** The name or the words of a step that was added at a level, set at that level. */
export function withAddedStep(workspace: Workspace, workflow: string, id: string, patch: Partial<Pick<AddedStep, "label" | "words">>): Workspace {
  const place = workspace.workflows[workflow];
  return place === undefined ? workspace : replaceWorkflow(workspace, workflow, { ...place, added: (place.added ?? []).map((one) => (one.id === id ? { ...one, ...patch } : one)) });
}

/** A step taken off the workflow, or a step added at a level, and the rules that named it. */
export function withoutStep(workspace: Workspace, workflow: string, id: string): Workspace {
  const gone = nodeId("workflow", id);
  const place = workspace.workflows[workflow];
  return {
    ...(place === undefined
      ? workspace
      : replaceWorkflow(workspace, workflow, {
          ...place,
          steps: Object.fromEntries(Object.entries(place.steps).filter(([key]) => key !== id)),
          ...(place.layers === undefined ? {} : { layers: place.layers.filter((one) => one.step !== id) }),
          ...(place.added === undefined ? {} : { added: place.added.filter((one) => one.id !== id) }),
        })),
    rules: workspace.rules.filter((rule) => !((rule.workflow === undefined || rule.workflow === workflow) && (nodeId(rule.from.lane, rule.from.item) === gone || nodeId(rule.to.lane, rule.to.item) === gone))),
  };
}

/** The steps run in the order given: the columns they held are dealt out again in that order, so what sits beside them on the other lanes stays where it is. */
export function withStepsOrder(workspace: Workspace, workflow: string, current: readonly LaneStep[], order: readonly string[]): Workspace {
  const cols = current.map((one) => one.col).sort((a, b) => a - b);
  const was = new Map(current.map((one) => [one.id, one] as const));
  const place = workspace.workflows[workflow] ?? { steps: {}, states: {} };
  const steps: Record<string, Omit<LaneStep, "id" | "label">> = { ...place.steps };
  order.forEach((id, at) => {
    const one = was.get(id);
    steps[id] = { ...(one === undefined ? {} : placed(one)), col: cols[at] ?? at };
  });
  return replaceWorkflow(workspace, workflow, { ...place, steps });
}

/**
 * What ends a step, or what it is passed over for: the thing and its state, as the person chose them. A choice that is a lifecycle's state also makes the step wait on it (`endsOn`), and
 * any other takes that away. Every step's place is kept as it is drawn, since a step given none sits where its order puts it.
 */
export function withStepWhen(
  workspace: Workspace,
  workflow: string,
  current: readonly LaneStep[],
  id: string,
  which: "ends" | "passedOver",
  value: Waits,
  endsOn: { lane: string; state: string } | undefined,
): Workspace {
  const place = workspace.workflows[workflow] ?? { steps: {}, states: {} };
  const steps: Record<string, Omit<LaneStep, "id" | "label">> = { ...place.steps, ...Object.fromEntries(current.map((one) => [one.id, placed(one)])) };
  const was = steps[id] ?? { col: current.length };
  const { endsOn: _gone, ...rest } = was;
  steps[id] = { ...(which === "ends" ? rest : was), [which]: value, ...(which === "ends" && endsOn !== undefined ? { endsOn } : {}) };
  return replaceWorkflow(workspace, workflow, { ...place, steps });
}

/**
 * A step's fields set at a level below the default: its name, its words, what ends it and what passes it over. Each is an override of the level above, and one that says what that level
 * already does is no override. `above` is what the level above says. `endsOn` goes with `ends`.
 */
export type StepFields = Partial<Pick<StepLayer, "label" | "words" | "ends" | "endsOn" | "passedOver">>;

export function withStepLayer(workspace: Workspace, workflow: string, step: string, scope: string, patch: StepFields, above: StepFields): Workspace {
  const place = workspace.workflows[workflow] ?? { steps: {}, states: {} };
  const layers = place.layers ?? [];
  const { step: _step, scope: _level, ...own } = layers.find((one) => one.step === step && one.scope === scope) ?? { step, scope };
  const next: Record<string, unknown> = { ...own };
  for (const [key, value] of Object.entries(patch)) {
    if (key === "endsOn") continue;
    if (same(value, (above as Record<string, unknown>)[key])) {
      delete next[key];
      if (key === "ends") delete next.endsOn;
    } else {
      next[key] = value;
      if (key === "ends") {
        if (patch.endsOn === undefined) delete next.endsOn;
        else next.endsOn = patch.endsOn;
      }
    }
  }
  const rest = layers.filter((one) => !(one.step === step && one.scope === scope));
  return replaceWorkflow(workspace, workflow, { ...place, layers: Object.keys(next).length === 0 ? rest : [...rest, { step, scope, ...next } as StepLayer] });
}

/** Everything a level changes of a step taken away: it reads as the level above says. */
export function withoutStepLayer(workspace: Workspace, workflow: string, step: string, scope: string): Workspace {
  const place = workspace.workflows[workflow];
  return place === undefined ? workspace : replaceWorkflow(workspace, workflow, { ...place, layers: (place.layers ?? []).filter((one) => !(one.step === step && one.scope === scope)) });
}
