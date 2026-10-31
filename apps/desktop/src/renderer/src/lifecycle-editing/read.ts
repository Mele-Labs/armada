// The Workspace the Workflows page draws, read off the pieces Fleet serves. The editor's model is layered (a level changes only what it changes) and Fleet's pieces are whole
// (a level that has a piece runs that one), so each level's piece is compared with the one it would otherwise read and only the difference is a layer. Saving is the same
// comparison the other way (`write.ts`), so a piece nobody edited is never rewritten.
//
// **What the wire does not carry stays out of the model.** Where a lifecycle's states sit in columns beside the workflow's steps is laid out here (`placesOf`) and not kept;
// a rule that fires on a Job event, or that lands on no node, is not drawn and is left as it is.

import { DEFAULT, autonomyOfWire, existsAt } from "@armada/components";
import type { Held, Kept, LifecycleDef, LifecycleKind, Ref, Rule, RuleOverride, StateDef, Workspace, WorkflowPlace } from "@armada/components";
import type { LifecycleDefinition, LifecycleRule, RuleAct } from "@armada/protocol";
import { columnsOf } from "@armada/screens/src/session-lanes";

import { clashOf, effective, inherited, levelsOfRead, lifecycleLayers, ruleLayers } from "./levels";
import type { Layers, PiecesRead } from "./levels";
import { kindOf, laneOf, sayAct, sayCondition } from "./words";
import type { Named } from "./words";

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

function lifecycleOf(id: string, layers: Layers<LifecycleDefinition>, levels: readonly string[], read: PiecesRead, held: Held): LifecycleDef {
  const clash = clashOf(layers, read);
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
      const above = inherited(layers, level, read).found?.layer.piece.states.find((one) => one.id === stateId) ?? homeState;
      if (mine.label !== above.label) names.push({ scope: level, label: mine.label });
      const words = wordsOf(mine);
      const wordsAbove = wordsOf(above);
      if (words !== undefined && words.length > 0 && !same(words, wordsAbove)) reads.push({ source: source.said, values: words, scope: level });
    }
    // Fleet runs a tab's piece whole: a tab whose piece lacks the state does not have it, whatever a level above says.
    const absent = levels.filter((level) => {
      const found = effective(layers, level, read);
      return existsAt(at, level, held, clash) && found !== undefined && !found.layer.piece.states.some((one) => one.id === stateId);
    });
    return {
      id: stateId,
      label: homeState.label,
      ...(at === DEFAULT ? {} : { scope: at }),
      ...(names.length === 0 ? {} : { names }),
      reads,
      ...(absent.length === 0 ? {} : { absent }),
      ...(clash.length === 0 ? {} : { clash }),
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
  const widest = own[0]!;
  return { id, label: first.name, from: source.said, kind: source.kind, states, next, ...(widest === DEFAULT ? {} : { scope: widest }), kept: KEPT[layers.get(widest)!.kept.rank.source] ?? "repository" };
}

// ---- Rules ---------------------------------------------------------------------------------------------------------------

/** The node an act lands on, where it lands on one: the state a ticket is moved to, the state a pull request is brought to, the deploy a watch takes on. */
export function landing(rule: LifecycleRule, defs: ReadonlyMap<string, Named>): Ref | undefined {
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
export function drawnRule(rule: LifecycleRule, defs: ReadonlyMap<string, Named>): Pick<Rule, "label" | "from" | "at" | "to" | "ifs" | "does" | "autonomy" | "workflow"> | undefined {
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
    autonomy: autonomyOfWire(rule),
    ...(workflow?.kind === "workflow" ? { workflow: workflow.workflow } : {}),
  };
}

/** What a piece says apart from the scope it is kept at. */
const sans = (piece: LifecycleRule) => ({ ...piece, scope: undefined });

function ruleOf(id: string, layers: Layers<LifecycleRule>, defs: ReadonlyMap<string, Named>, levels: readonly string[], read: PiecesRead): Rule | undefined {
  const own = ownLevels(layers, levels);
  const at = own[0]!;
  const base = layers.get(at)!;
  const drawn = drawnRule(base.piece, defs);
  if (drawn === undefined) return undefined;
  // **A level that keeps a piece of its own has that piece whole**: Fleet runs it in place of the one above, and nothing of the one above reaches it.
  const overrides: RuleOverride[] = [];
  for (const level of own.slice(1)) {
    const piece = layers.get(level)!.piece;
    const mine = drawnRule(piece, defs);
    const above = inherited(layers, level, read).found?.layer.piece;
    if (mine === undefined || (above !== undefined && same(sans(piece), sans(above)))) continue;
    overrides.push({ scope: level, ...mine, workflow: mine.workflow, wire: piece });
  }
  const { workflow, ...fields } = drawn;
  const clash = clashOf(layers, read);
  return { id, ...fields, kept: KEPT[base.kept.rank.source] ?? "repository", scope: at, wire: base.piece, ...(overrides.length === 0 ? {} : { overrides }), ...(workflow === undefined ? {} : { workflow }), ...(clash.length === 0 ? {} : { clash }) };
}

/** The lifecycles a rule is worded by: each one's name, states and the states that end it. */
export const namedOf = (lifecycles: readonly LifecycleDef[]): Map<string, Named> =>
  new Map(lifecycles.map((one) => [one.id, { id: one.id, name: one.label, states: one.states.map((state) => ({ id: state.id, label: state.label })), ends: one.states.filter((state) => state.end === true).map((state) => state.id) }] as const));

/** A Workspace with every rule's fields drawn again from its piece, so a change to the piece is what the lanes show. A rule whose piece lands on nothing keeps the fields it had. */
export function withDrawn(workspace: Workspace): Workspace {
  const defs = namedOf(workspace.lifecycles);
  const from = <T extends Rule | RuleOverride>(one: T): T => {
    const drawn = one.wire === undefined ? undefined : drawnRule(one.wire, defs);
    return drawn === undefined ? one : { ...one, ...drawn, workflow: drawn.workflow };
  };
  return { ...workspace, rules: workspace.rules.map((rule): Rule => ({ ...from(rule), ...(rule.overrides === undefined ? {} : { overrides: rule.overrides.map(from) }) })) };
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

/** The Workspace the page draws, or none where Fleet served no lifecycle. */
export function workspaceOf(read: PiecesRead): Workspace | undefined {
  const levels = levelsOfRead(read);
  const one = read.workspaces[0];
  const primary = read.repositories[0];
  const named = {
    id: one?.id ?? "",
    name: one?.name ?? primary?.name ?? "",
    workspaces: read.workspaces.map((each) => ({ id: each.id, name: each.name })),
    repositories: read.repositories.map((each) => ({ id: each.id, name: each.name, in: read.workspaces.filter((there) => there.repositories.includes(each.id)).map((there) => there.id) })),
  };
  const lifecycleLayered = lifecycleLayers(read);
  // A piece scoped to a repository this read did not cover has no tab to be drawn on.
  const readable = <T,>(all: ReadonlyMap<string, Layers<T>>) => [...all].filter(([, layers]) => ownLevels(layers, levels).length > 0);
  const readLifecycles = readable(lifecycleLayered);
  if (readLifecycles.length === 0) return undefined;
  const lifecycles = readLifecycles.map(([id, layers]) => lifecycleOf(id, layers, levels, read, named));
  const defs = namedOf(lifecycles);
  const rules = readable(ruleLayers(read)).flatMap(([id, layers]) => ruleOf(id, layers, defs, levels, read) ?? []);
  return { ...named, lifecycles, workflows: {}, rules };
}
