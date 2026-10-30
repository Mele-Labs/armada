// The Workspace the Workflows page draws, read off the pieces Fleet serves. The editor's model is layered (a level changes only what it changes) and Fleet's pieces are whole
// (a level that has a piece runs that one), so each level's piece is compared with the one it would otherwise read and only the difference is a layer. Saving is the same
// comparison the other way (`write.ts`), so a piece nobody edited is never rewritten.
//
// **What the wire does not carry stays out of the model.** Where a lifecycle's states sit in columns beside the workflow's steps is laid out here (`placesOf`) and not kept;
// a rule that fires on a Job event, or that lands on no node, is not drawn and is left as it is.

import { DEFAULT } from "@armada/components";
import type { Kept, LifecycleDef, LifecycleKind, Ref, Rule, RuleOverride, StateDef, Workspace, WorkflowPlace } from "@armada/components";
import type { LifecycleDefinition, LifecycleRule, RuleAct } from "@armada/protocol";
import { columnsOf } from "@armada/screens/src/session-lanes";

import { effective, levelsOfRead, lifecycleLayers, parentOf, ruleLayers } from "./levels";
import type { Layers, PiecesRead } from "./levels";
import { kindOf, laneOf, sayAct, sayCondition } from "./words";

/** Where a lifecycle's states are read from, in the word the lane carries and the shape its nodes are drawn in. */
export const SOURCE: Readonly<Record<string, { kind: LifecycleKind; said: string }>> = {
  tracker: { kind: "tracker", said: "Tracker" },
  forge: { kind: "forge", said: "Forge" },
  watch: { kind: "watch", said: "Watch" },
  fleet: { kind: "job", said: "Fleet" },
};

const KEPT: Readonly<Record<string, Kept>> = { armada: "armada", kit: "yours", repository: "repository" };

const same = (a: unknown, b: unknown): boolean => JSON.stringify(a) === JSON.stringify(b);

/** The words a tracker reads a state as, absent for a state another source reads. */
export const wordsOf = (state: LifecycleDefinition["states"][number] | undefined): readonly string[] | undefined => (state?.read.source === "tracker" ? state.read.words : undefined);

// ---- Lifecycles ----------------------------------------------------------------------------------------------------------

/** The levels that have a piece of their own, widest first. */
const ownLevels = (layers: Layers<unknown>, levels: readonly string[]): string[] => levels.filter((level) => layers.has(level));

function lifecycleOf(id: string, layers: Layers<LifecycleDefinition>, levels: readonly string[]): LifecycleDef {
  const own = ownLevels(layers, levels);
  const pieceAt = (level: string) => layers.get(level)?.piece;
  const first = pieceAt(own[0]!)!;
  const source = SOURCE[first.states[0]?.read.source ?? "tracker"] ?? SOURCE["tracker"]!;
  const order: string[] = [];
  const home = new Map<string, string>();
  for (const level of own) {
    for (const state of pieceAt(level)!.states) {
      if (home.has(state.id)) continue;
      home.set(state.id, level);
      order.push(state.id);
    }
  }
  const states = order.map((stateId): StateDef => {
    const at = home.get(stateId)!;
    const homeState = pieceAt(at)!.states.find((one) => one.id === stateId)!;
    const narrower = own.slice(own.indexOf(at) + 1);
    const names: { scope: string; label: string }[] = [];
    const reads: { source: string; values: readonly string[]; scope: string }[] = [];
    const tracker = wordsOf(homeState);
    if (tracker !== undefined) reads.push({ source: source.said, values: tracker, scope: at });
    for (const level of narrower) {
      const mine = pieceAt(level)!.states.find((one) => one.id === stateId);
      if (mine === undefined) continue;
      const above = effective(layers, parentOf(level) ?? DEFAULT)?.layer.piece.states.find((one) => one.id === stateId) ?? homeState;
      if (mine.label !== above.label) names.push({ scope: level, label: mine.label });
      const words = wordsOf(mine);
      const wordsAbove = wordsOf(above);
      if (words !== undefined && words.length > 0 && !same(words, wordsAbove)) reads.push({ source: source.said, values: words, scope: level });
    }
    return {
      id: stateId,
      label: homeState.label,
      ...(at === DEFAULT ? {} : { scope: at }),
      ...(names.length === 0 ? {} : { names }),
      reads,
      ...(pieceAt(at)!.ends.includes(stateId) ? { end: true as const } : {}),
    };
  });
  const known = new Set(order);
  const next: (readonly [string, string])[] = [];
  for (const level of own) {
    for (const move of pieceAt(level)!.transitions) {
      if (known.has(move.from) && known.has(move.to) && !next.some(([from, to]) => from === move.from && to === move.to)) next.push([move.from, move.to]);
    }
  }
  return { id, label: first.name, from: source.said, kind: source.kind, states, next };
}

// ---- Rules ---------------------------------------------------------------------------------------------------------------

/** The strongest word among a rule's acts: any that asks makes it Asks, and all of them off make it Off. */
function autonomyOf(rule: LifecycleRule): Rule["autonomy"] {
  const words = rule.autonomy === undefined ? rule.does.map((one) => one.autonomy) : [rule.autonomy];
  if (words.includes("asks")) return "asks";
  return words.length > 0 && words.every((one) => one === "off") ? "off" : "acts";
}

/** The node an act lands on, where it lands on one: the state a ticket is moved to, the state a pull request is brought to, the deploy a watch takes on. */
function landing(rule: LifecycleRule, defs: ReadonlyMap<string, LifecycleDefinition>): Ref | undefined {
  for (const one of rule.does) {
    const act: RuleAct = one.what;
    if (act.act === "tracker_move") {
      const lane = laneOf(act.target.among, rule);
      if (lane !== undefined) return { lane, item: act.to };
    }
    if (act.act === "tracker_close") {
      const lane = laneOf(act.target.among, rule);
      const end = lane === undefined ? undefined : defs.get(lane)?.ends[0];
      if (lane !== undefined && end !== undefined) return { lane, item: end };
    }
    if (act.act === "forge_mark_ready" || act.act === "forge_merge") {
      const lane = laneOf(act.target.among, rule);
      if (lane !== undefined) return { lane, item: act.act === "forge_merge" ? "merged" : "ready" };
    }
    if (act.act === "start_watch" && act.deploy !== undefined && act.deploy !== "") {
      const first = defs.get("deploy")?.states[0]?.id;
      if (first !== undefined) return { lane: "deploy", item: first };
    }
  }
  const step = rule.conditions.find((one) => one.kind === "at_step");
  return step?.kind === "at_step" ? { lane: "workflow", item: step.step } : undefined;
}

/** What the lanes draw of one rule: its two ends and the words of it. Absent where a Job event sets it off or it lands on no node. */
export function drawnRule(rule: LifecycleRule, defs: ReadonlyMap<string, LifecycleDefinition>): Pick<Rule, "label" | "from" | "at" | "to" | "ifs" | "does" | "autonomy" | "workflow"> | undefined {
  const when = rule.when;
  if (when.kind === "job") return undefined;
  const to = landing(rule, defs);
  if (to === undefined) return undefined;
  const lifecycles = [...defs.values()];
  const workflow = rule.conditions.find((one) => one.kind === "workflow");
  return {
    label: rule.name,
    from: when.kind === "enters" ? { lane: when.lifecycle, item: when.state } : { lane: "workflow", item: when.step },
    at: when.kind === "enters" ? "enters" : when.kind === "step_ends" ? "ends" : "starts",
    to,
    ifs: rule.conditions.filter((one) => one.kind !== "workflow").map((one) => sayCondition(one, rule, lifecycles)),
    does: rule.does.map((one) => ({ act: kindOf(one.what), text: sayAct(one.what, rule, lifecycles) })),
    autonomy: autonomyOf(rule),
    ...(workflow?.kind === "workflow" ? { workflow: workflow.workflow } : {}),
  };
}

function ruleOf(id: string, layers: Layers<LifecycleRule>, defs: ReadonlyMap<string, LifecycleDefinition>, levels: readonly string[]): Rule | undefined {
  const own = ownLevels(layers, levels);
  const at = own[0]!;
  const base = layers.get(at)!;
  const drawn = drawnRule(base.piece, defs);
  if (drawn === undefined) return undefined;
  const overrides: RuleOverride[] = [];
  for (const level of own.slice(1)) {
    const mine = drawnRule(layers.get(level)!.piece, defs);
    const aboveLayer = effective(layers, parentOf(level) ?? DEFAULT)!.layer;
    const above = drawnRule(aboveLayer.piece, defs) ?? drawn;
    if (mine === undefined) continue;
    const fields: Partial<RuleOverride> = {};
    if (mine.label !== above.label) fields.label = mine.label;
    if (!same(mine.from, above.from) || mine.at !== above.at) {
      fields.from = mine.from;
      fields.at = mine.at;
    }
    if (!same(mine.ifs, above.ifs)) fields.ifs = mine.ifs;
    if (!same(mine.does, above.does)) fields.does = mine.does;
    if (mine.autonomy !== above.autonomy) fields.autonomy = mine.autonomy;
    if (Object.keys(fields).length > 0) overrides.push({ scope: level, ...fields });
  }
  const { workflow, ...fields } = drawn;
  return { id, ...fields, kept: KEPT[base.kept.rank.source] ?? "repository", scope: at, ...(overrides.length === 0 ? {} : { overrides }), ...(workflow === undefined ? {} : { workflow }) };
}

// ---- The Workspace -------------------------------------------------------------------------------------------------------

/** Where each state sits in a workflow's lanes: the wire has no columns, so a state sits after the states that lead to it, and every lifecycle starts at the left. */
export function placesOf(lifecycles: readonly LifecycleDef[], workflows: readonly string[]): Record<string, WorkflowPlace> {
  const states: Record<string, number> = {};
  for (const lifecycle of lifecycles) {
    const cols = columnsOf({
      states: lifecycle.states.map((one) => ({ id: one.id, label: one.label, read: { source: "tracker", words: [] } })),
      transitions: lifecycle.next.map(([from, to]) => ({ from, to })),
    });
    for (const state of lifecycle.states) states[`${lifecycle.id}/${state.id}`] = cols.get(state.id) ?? 0;
  }
  return Object.fromEntries(workflows.map((id) => [id, { steps: {}, states: { ...states } }]));
}

/** The workflows this Workspace has no place for yet, each placed as the lifecycles' columns lay it out. A place an edit has moved is kept. */
export function withPlaces(workspace: Workspace, workflows: readonly string[]): Workspace {
  const missing = workflows.filter((id) => workspace.workflows[id] === undefined);
  return missing.length === 0 ? workspace : { ...workspace, workflows: { ...workspace.workflows, ...placesOf(workspace.lifecycles, missing) } };
}

/** The definition each lifecycle runs by on the default's tab, for the labels and landings a rule is worded with. */
export const definitionsOf = (all: readonly (readonly [string, Layers<LifecycleDefinition>])[]): Map<string, LifecycleDefinition> =>
  new Map(all.map(([id, layers]) => [id, (effective(layers, DEFAULT) ?? { layer: [...layers.values()][0]! }).layer.piece] as const));

/** The Workspace the page draws, or none where Fleet served no lifecycle. */
export function workspaceOf(read: PiecesRead): Workspace | undefined {
  const levels = levelsOfRead(read);
  const lifecycleLayered = lifecycleLayers(read);
  // A piece scoped to a repository this read did not cover has no tab to be drawn on.
  const readable = <T,>(all: ReadonlyMap<string, Layers<T>>) => [...all].filter(([, layers]) => ownLevels(layers, levels).length > 0);
  const readLifecycles = readable(lifecycleLayered);
  if (readLifecycles.length === 0) return undefined;
  const lifecycles = readLifecycles.map(([id, layers]) => lifecycleOf(id, layers, levels));
  const defs = definitionsOf(readLifecycles);
  const rules = readable(ruleLayers(read)).flatMap(([id, layers]) => ruleOf(id, layers, defs, levels) ?? []);
  const primary = read.repositories[0];
  return {
    id: read.workspace?.id ?? "",
    name: read.workspace?.name ?? primary?.name ?? "",
    repositories: read.repositories.map((one) => ({ id: one.id, name: one.name })),
    lifecycles,
    workflows: {},
    rules,
  };
}
