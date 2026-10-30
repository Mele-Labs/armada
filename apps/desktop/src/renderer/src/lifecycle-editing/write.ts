// What a change to the Workspace comes to on Fleet's side: the pieces to save, to delete, and what could not be kept. The editor changes the layered model and hands back
// the whole; this compares it with the one it was given, level by level, and **writes a level only where its own words changed**, so a piece nobody edited is never
// rewritten. A level that had no piece and is changed gets one (Fleet runs the narrower piece whole), and a level changed back to what the one above says loses its piece.
//
// **Every save is to the person's own Kit.** An edit to a piece the repository committed, or to one of the four Armada carries, is a copy in Kit that hides it; the repository's
// folder changes only by Commit to repository, or by a removal that goes through a Job of its own.

import { DEFAULT, existsAt, readsAt, ruleAt, stateLabelAt } from "@armada/components";
import type { LifecycleDef, Rule, Workspace } from "@armada/components";
import type { LifecycleDefinition, LifecyclePiece, LifecyclePieceKind, LifecycleRule, PieceScope, RuleAct, RuleTarget, RuleWhen, StateReadFrom } from "@armada/protocol";

import { effective, levelsOfRead, lifecycleLayers, parentOf, ruleLayers, scopeOf } from "./levels";
import type { Layer, Layers, PiecesRead } from "./levels";

/** A piece by what identifies it: its kind, id and scope together. Only these two kinds are drawn on the canvas. */
export type PieceId = { kind: Extract<LifecyclePieceKind, "lifecycle" | "rule">; id: string; scope: PieceScope };

export type Op =
  /** Write the piece to Kit's folder. `overwrite` where Kit already holds this kind, id and scope. */
  | { op: "save"; piece: LifecyclePiece; overwrite: boolean }
  /** Delete the Kit file that declares this piece. */
  | { op: "delete"; id: PieceId }
  /** Take the piece out of the repository whose folder holds `path`, by a Job of its own. */
  | { op: "remove"; id: PieceId; path: string }
  /** The change stands in the window, and Fleet keeps no part of it. */
  | { op: "unsaved"; said: string }
  /** The change has no piece to be kept as, so it is not made. */
  | { op: "refuse"; said: string };

const same = (a: unknown, b: unknown): boolean => JSON.stringify(a) === JSON.stringify(b);

// ---- Lifecycles ----------------------------------------------------------------------------------------------------------

/** What one level would run of a lifecycle: its states as that level names and reads them, the moves between them, and what ends it. */
type View = { name: string; states: { id: string; label: string; words: readonly string[] | undefined; end: boolean }[]; transitions: (readonly [string, string])[] };

function viewOf(def: LifecycleDef, level: string): View {
  const here = def.states.filter((one) => existsAt(one.scope, level));
  const ids = new Set(here.map((one) => one.id));
  // A state that does not exist at this level is stepped over: whatever led into it leads to whatever it led to.
  let next = def.next;
  for (const gone of def.states.filter((one) => !ids.has(one.id))) {
    const into = next.filter(([, to]) => to === gone.id).map(([from]) => from);
    const out = next.filter(([from]) => from === gone.id).map(([, to]) => to);
    next = [...next.filter(([from, to]) => from !== gone.id && to !== gone.id), ...into.flatMap((from) => out.map((to) => [from, to] as const))];
  }
  const seen = new Set<string>();
  const transitions = next.filter(([from, to]) => {
    const key = `${from}>${to}`;
    if (!ids.has(from) || !ids.has(to) || seen.has(key)) return false;
    seen.add(key);
    return true;
  });
  return {
    name: def.label,
    states: here.map((one) => ({ id: one.id, label: stateLabelAt(one, level), words: def.kind === "tracker" ? (readsAt(one, def.from, level) ?? []) : undefined, end: one.end === true })),
    transitions,
  };
}

/** What a level adds to the one above it: the states it homes, the names it gives, and the words it reads them as. */
const layerOf = (def: LifecycleDef, level: string) => ({
  states: def.states.filter((one) => (one.scope ?? DEFAULT) === level).map((one) => [one.id, one.label, one.end === true]),
  names: def.states.flatMap((one) => (one.names ?? []).filter((each) => each.scope === level).map((each) => [one.id, each.label])),
  reads: def.states.flatMap((one) => one.reads.filter((each) => each.scope === level).map((each) => [one.id, each.source, each.values])),
});

function pieceOf(def: LifecycleDef, level: string, base: LifecycleDefinition, workspace: string | undefined): LifecycleDefinition | string {
  const view = viewOf(def, level);
  const fresh = view.states.find((one) => !base.states.some((each) => each.id === one.id));
  if (fresh !== undefined && def.kind !== "tracker") return `${def.label} states are read from ${def.from.toLowerCase()}, so none can be added`;
  return {
    id: def.id,
    name: view.name,
    scope: scopeOf(level, workspace),
    states: view.states.map((one) => {
      const old = base.states.find((each) => each.id === one.id);
      const read: StateReadFrom = old !== undefined && old.read.source !== "tracker" ? old.read : { source: "tracker", words: [...(one.words ?? [])] };
      return { id: one.id, label: one.label, read };
    }),
    transitions: view.transitions.map(([from, to]) => ({ from, to })),
    ends: view.states.filter((one) => one.end).map((one) => one.id),
  };
}

/** The place a layer was kept decides how a piece in it is taken away: Kit's file is deleted, a repository's goes by a Job, and Armada's four are not files. */
function takenAway(id: PieceId, layer: Layer<unknown>): Op[] {
  const { source } = layer.kept.rank;
  if (source === "kit") return [{ op: "delete", id }];
  return source === "repository" && layer.kept.path !== undefined ? [{ op: "remove", id, path: layer.kept.path }] : [];
}

function lifecycleOps(read: PiecesRead, before: Workspace, after: Workspace): Op[] {
  const layers = lifecycleLayers(read);
  const levels = levelsOfRead(read);
  const workspace = read.workspace?.id;
  const ops: Op[] = [];
  for (const next of after.lifecycles) {
    const was = before.lifecycles.find((one) => one.id === next.id);
    const own: Layers<LifecycleDefinition> | undefined = layers.get(next.id);
    if (was === undefined || own === undefined || same(was, next)) continue;
    const edited = levels.filter((level) => !same(layerOf(was, level), layerOf(next, level)) || (own.has(level) && !same(viewOf(was, level), viewOf(next, level))));
    for (const level of edited) {
      const id: PieceId = { kind: "lifecycle", id: next.id, scope: scopeOf(level, workspace) };
      const above = parentOf(level);
      const held = own.get(level);
      // A level changed back to what the one above says has no business holding a piece.
      if (above !== undefined && same(viewOf(next, level), viewOf(next, above))) {
        if (held !== undefined) ops.push(...takenAway(id, held));
        continue;
      }
      const base = held?.piece ?? effective(own, level)?.layer.piece ?? [...own.values()][0]!.piece;
      const piece = pieceOf(next, level, base, workspace);
      if (typeof piece === "string") return [{ op: "refuse", said: piece }];
      ops.push({ op: "save", piece: { kind: "lifecycle", lifecycle: piece }, overwrite: held?.kept.rank.source === "kit" });
    }
  }
  return ops;
}

// ---- Rules ---------------------------------------------------------------------------------------------------------------

/** The moment a rule is for, as Fleet words it. Absent for leaving a state, which no rule runs on. */
const whenOf = (from: Rule["from"], at: Rule["at"]): RuleWhen | undefined => {
  if (from.lane !== "workflow") return at === "leaves" ? undefined : { kind: "enters", lifecycle: from.lane, state: from.item };
  return at === "ends" ? { kind: "step_ends", step: from.item } : { kind: "step_starts", step: from.item };
};

const LEAVES = "Fleet runs no rule on leaving a state";

/** The rule a connector between two nodes comes to, or why Fleet has no rule for it. */
function ruleBetween(rule: Rule, after: Workspace, workspace: string | undefined): LifecycleRule | string {
  const lane = after.lifecycles.find((one) => one.id === rule.to.lane);
  if (lane === undefined) return "A connector to a step makes no rule Fleet can run";
  const when = whenOf(rule.from, rule.at);
  if (when === undefined) return LEAVES;
  // The instance the moment is about when it is of the lifecycle the rule lands on; else the ones of it the Job holds.
  const target: RuleTarget = { among: when.kind === "enters" && when.lifecycle === lane.id ? { kind: "entered" } : { kind: "held", lifecycle: lane.id } };
  let what: RuleAct;
  let kind: string;
  if (lane.kind === "tracker") [what, kind] = [{ act: "tracker_move", to: rule.to.item, target }, "tracker_write"];
  else if (lane.kind === "forge" && rule.to.item === "merged") [what, kind] = [{ act: "forge_merge", target }, "forge_write"];
  else if (lane.kind === "forge" && rule.to.item === "ready") [what, kind] = [{ act: "forge_mark_ready", target }, "forge_write"];
  else return `Fleet has no act that brings ${lane.label.toLowerCase()} to ${lane.states.find((one) => one.id === rule.to.item)?.label ?? rule.to.item}`;
  return {
    id: rule.id,
    name: rule.label,
    scope: scopeOf(rule.scope, workspace),
    when,
    conditions: rule.workflow === undefined ? [] : [{ kind: "workflow", workflow: rule.workflow }],
    does: [{ what, kind, autonomy: rule.autonomy }],
    autonomy: rule.autonomy,
    on_failure: { retries: 2, then: "ask_owner" },
  };
}

/** The fields of a rule the editor changes and a piece holds. */
const fieldsOf = (rule: Rule) => ({ label: rule.label, from: rule.from, at: rule.at, autonomy: rule.autonomy });

/** What the rows say, which Fleet's structure is not drawn back from. */
const rowsOf = (rule: Rule) => ({ ifs: rule.ifs, does: rule.does.map((one) => one.text) });

type Fields = ReturnType<typeof fieldsOf>;

/** A piece with the fields the editor changed, or why they cannot be kept. */
function patched(base: LifecycleRule, was: Fields, now: Fields, level: string, workspace: string | undefined): LifecycleRule | string {
  const next: LifecycleRule = { ...base, scope: scopeOf(level, workspace) };
  if (was.label !== now.label) next.name = now.label;
  if (!same(was.from, now.from) || was.at !== now.at) {
    const when = whenOf(now.from, now.at);
    if (when === undefined) return LEAVES;
    next.when = when;
  }
  if (was.autonomy !== now.autonomy) {
    next.autonomy = now.autonomy;
    next.does = base.does.map((one) => ({ ...one, autonomy: now.autonomy }));
  }
  return next;
}

function ruleOps(read: PiecesRead, before: Workspace, after: Workspace): Op[] {
  const layers = ruleLayers(read);
  const workspace = read.workspace?.id;
  const ops: Op[] = [];
  /** The levels a rule has words at: where it applies, and each that changes it. */
  const levelsOf = (rule: Rule): string[] => [...new Set([rule.scope, ...(rule.overrides ?? []).map((one) => one.scope)])];
  let rows = false;
  for (const rule of after.rules) {
    const was = before.rules.find((one) => one.id === rule.id);
    if (was === undefined) {
      const made = ruleBetween(rule, after, workspace);
      if (typeof made === "string") return [{ op: "refuse", said: made }];
      ops.push({ op: "save", piece: { kind: "rule", rule: made }, overwrite: false });
      continue;
    }
    const own = layers.get(rule.id);
    if (own === undefined || same(was, rule)) continue;
    for (const level of new Set([...levelsOf(was), ...levelsOf(rule)])) {
      const id: PieceId = { kind: "rule", id: rule.id, scope: scopeOf(level, workspace) };
      const held = own.get(level);
      // An override the level no longer makes is its piece taken away.
      if (level !== rule.scope && (was.overrides ?? []).some((one) => one.scope === level) && !(rule.overrides ?? []).some((one) => one.scope === level)) {
        if (held !== undefined) ops.push(...takenAway(id, held));
        continue;
      }
      if (!same(rowsOf(ruleAt(was, level)), rowsOf(ruleAt(rule, level)))) rows = true;
      const then = fieldsOf(ruleAt(was, level));
      const now = fieldsOf(ruleAt(rule, level));
      if (same(then, now)) continue;
      const base = held?.piece ?? effective(own, level)?.layer.piece;
      if (base === undefined) continue;
      const piece = patched(base, then, now, level, workspace);
      if (typeof piece === "string") return [{ op: "refuse", said: piece }];
      ops.push({ op: "save", piece: { kind: "rule", rule: piece }, overwrite: held?.kept.rank.source === "kit" });
    }
  }
  for (const gone of before.rules.filter((one) => !after.rules.some((each) => each.id === one.id))) {
    const own = layers.get(gone.id);
    if (own === undefined) continue;
    for (const level of levelsOf(gone)) {
      const held = own.get(level);
      if (held !== undefined) ops.push(...takenAway({ kind: "rule", id: gone.id, scope: scopeOf(level, workspace) }, held));
    }
  }
  return rows ? [...ops, { op: "unsaved", said: "If and Do rows are not saved" }] : ops;
}

// ---- What is not kept ----------------------------------------------------------------------------------------------------

/** What a step waits on and passes over, and the steps a level below the default changes or adds: no piece holds them, and the workflow's own steps are saved by the page. */
const stepsKept = (workspace: Workspace) =>
  Object.entries(workspace.workflows).map(([id, place]) => [id, place.layers, place.added, Object.entries(place.steps).map(([step, one]) => [step, one.ends, one.endsOn, one.passedOver])]);

/** The ops a change to the Workspace comes to. */
export function opsFrom(read: PiecesRead, before: Workspace, after: Workspace): Op[] {
  const ops = [...lifecycleOps(read, before, after), ...ruleOps(read, before, after)];
  const stop = ops.find((one) => one.op === "refuse");
  if (stop !== undefined) return [stop];
  const said = "What a step ends on, and steps changed below the default, are not saved";
  const steps = same(stepsKept(before), stepsKept(after)) || ops.some((one) => one.op === "unsaved" && one.said === said) ? [] : ([{ op: "unsaved", said }] as const);
  return [...ops, ...steps];
}

/** The piece of the person's own that a tab runs for `id`, which Commit to repository copies. Absent where the piece it runs is a repository's or Armada's. */
export function kitPieceAt(read: PiecesRead, kind: PieceId["kind"], id: string, level: string): PieceId | undefined {
  const layers = (kind === "rule" ? ruleLayers(read) : lifecycleLayers(read)).get(id) as Layers<{ scope: PieceScope }> | undefined;
  const found = layers === undefined ? undefined : effective(layers, level);
  return found?.layer.kept.rank.source === "kit" ? { kind, id, scope: found.layer.piece.scope } : undefined;
}
