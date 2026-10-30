// What a change to the Workspace comes to on Fleet's side: the pieces to save and to delete. The editor changes the layered model and hands back the whole; this compares it with the
// one it was given, level by level, and **writes a level only where its own words changed**, so a piece nobody edited is never rewritten, and an edit on one tab writes that tab's piece
// alone. A level that had no piece and is changed gets one (Fleet runs the narrower piece whole), and a level changed back to what the one above says loses its piece.
//
// **Every save is to the person's own Kit.** An edit to a piece the repository committed, or to one of the four Armada carries, is a copy in Kit that hides it; the repository's
// folder changes only by Commit to repository, or by a removal that goes through a Job of its own. **Nothing here says why a change was not kept**: the editor offers only what Fleet can
// run (`RuleForm`, `canConnect`), and Fleet's own refusal words come back from the save.

import { DEFAULT, readsAt, stateExistsAt, stateLabelAt } from "@armada/components";
import type { LifecycleDef, Rule, Workspace } from "@armada/components";
import type { LifecycleDefinition, LifecyclePiece, LifecyclePieceKind, LifecycleRule, PieceScope, RuleAct, RuleTarget, StateReadFrom } from "@armada/protocol";

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
  | { op: "remove"; id: PieceId; path: string };

const same = (a: unknown, b: unknown): boolean => JSON.stringify(a) === JSON.stringify(b);

// ---- Lifecycles ----------------------------------------------------------------------------------------------------------

/** What one level would run of a lifecycle: its states as that level names and reads them, the moves between them, and what ends it. */
type View = { name: string; states: { id: string; label: string; words: readonly string[] | undefined; end: boolean }[]; transitions: (readonly [string, string])[] };

/** The moves of a lifecycle with the states in `gone` stepped over: whatever led into one leads to whatever it led to. */
function steppedOver(next: LifecycleDef["next"], gone: readonly string[]): (readonly [string, string])[] {
  let out: (readonly [string, string])[] = [...next];
  for (const one of gone) {
    const into = out.filter(([, to]) => to === one).map(([from]) => from);
    const leaves = out.filter(([from]) => from === one).map(([, to]) => to);
    out = [...out.filter(([from, to]) => from !== one && to !== one), ...into.flatMap((from) => leaves.map((to) => [from, to] as const))];
  }
  return out;
}

function viewOf(def: LifecycleDef, level: string): View {
  const here = def.states.filter((one) => stateExistsAt(one, level));
  const ids = new Set(here.map((one) => one.id));
  const seen = new Set<string>();
  const transitions = steppedOver(
    def.next,
    def.states.filter((one) => !ids.has(one.id)).map((one) => one.id),
  ).filter(([from, to]) => {
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

/**
 * What a level adds to the one above it: the states it homes, the names it gives, the words it reads them as, and the lifecycle's own name where it is the level that added the lifecycle. The names and
 * words are of `only`, the states both the lifecycle as it was and as it is have, so a state taken away elsewhere is not a change to this level.
 */
const layerOf = (def: LifecycleDef, level: string, only: ReadonlySet<string>) => ({
  name: (def.scope ?? DEFAULT) === level ? def.label : undefined,
  states: def.states.filter((one) => (one.scope ?? DEFAULT) === level).map((one) => [one.id, one.label, one.end === true]),
  names: def.states.filter((one) => only.has(one.id)).flatMap((one) => (one.names ?? []).filter((each) => each.scope === level).map((each) => [one.id, each.label])),
  reads: def.states.filter((one) => only.has(one.id)).flatMap((one) => one.reads.filter((each) => each.scope === level).map((each) => [one.id, each.source, each.values])),
});

/**
 * The piece a level runs once it is changed: the one it ran (`base`) with the states, names, words and moves the change made. **The moves are the piece's own plus the ones the change added, less the ones it
 * took away**, so a move the piece lacked is not made by an edit elsewhere.
 */
function pieceOf(def: LifecycleDef, was: LifecycleDef | undefined, level: string, base: LifecycleDefinition | undefined, workspace: string | undefined): LifecycleDefinition | undefined {
  const view = viewOf(def, level);
  const ids = new Set(view.states.map((one) => one.id));
  const fresh = view.states.find((one) => base !== undefined && !base.states.some((each) => each.id === one.id));
  // A state of a lifecycle another source reads has no words to be read from: none is added.
  if (fresh !== undefined && def.kind !== "tracker") return undefined;
  const has = (list: readonly (readonly [string, string])[], move: readonly [string, string]) => list.some(([from, to]) => from === move[0] && to === move[1]);
  const gone = was === undefined ? [] : was.next.filter((move) => !has(def.next, move));
  const added = was === undefined ? [] : def.next.filter((move) => !has(was.next, move));
  const transitions =
    base === undefined || was === undefined
      ? view.transitions.map(([from, to]) => ({ from, to }))
      : [
          ...base.transitions.filter((move) => ids.has(move.from) && ids.has(move.to) && !has(gone, [move.from, move.to])),
          ...added.filter(([from, to]) => ids.has(from) && ids.has(to) && !base.transitions.some((move) => move.from === from && move.to === to)).map(([from, to]) => ({ from, to })),
        ];
  return {
    id: def.id,
    name: view.name,
    scope: scopeOf(level, workspace),
    states: view.states.map((one) => {
      const old = base?.states.find((each) => each.id === one.id);
      const read: StateReadFrom = old !== undefined && old.read.source !== "tracker" ? old.read : { source: "tracker", words: [...(one.words ?? [])] };
      return { id: one.id, label: one.label, read };
    }),
    transitions,
    ends: base === undefined ? view.states.filter((one) => one.end).map((one) => one.id) : base.ends.filter((one) => ids.has(one)),
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
    if (was === undefined) {
      // A lifecycle added on a tab is that tab's own piece.
      const piece = pieceOf(next, undefined, next.scope ?? DEFAULT, undefined, workspace);
      if (piece !== undefined) ops.push({ op: "save", piece: { kind: "lifecycle", lifecycle: piece }, overwrite: false });
      continue;
    }
    const own: Layers<LifecycleDefinition> | undefined = layers.get(next.id);
    if (own === undefined || same(was, next)) continue;
    const common = new Set(next.states.filter((one) => was.states.some((each) => each.id === one.id)).map((one) => one.id));
    // **A level is written where its own words changed**, and no other: the default's edit leaves a repository's own piece as it is.
    for (const level of levels.filter((one) => !same(layerOf(was, one, common), layerOf(next, one, common)))) {
      const id: PieceId = { kind: "lifecycle", id: next.id, scope: scopeOf(level, workspace) };
      const above = parentOf(level);
      const held = own.get(level);
      // A level changed back to what the one above says has no business holding a piece.
      if (above !== undefined && same(viewOf(next, level), viewOf(next, above))) {
        if (held !== undefined) ops.push(...takenAway(id, held));
        continue;
      }
      const base = held?.piece ?? effective(own, level)?.layer.piece ?? [...own.values()][0]!.piece;
      const piece = pieceOf(next, was, level, base, workspace);
      if (piece !== undefined) ops.push({ op: "save", piece: { kind: "lifecycle", lifecycle: piece }, overwrite: held?.kept.rank.source === "kit" });
    }
  }
  for (const gone of before.lifecycles.filter((one) => !after.lifecycles.some((each) => each.id === one.id))) {
    for (const [level, held] of layers.get(gone.id) ?? []) ops.push(...takenAway({ kind: "lifecycle", id: gone.id, scope: scopeOf(level, workspace) }, held));
  }
  return ops;
}

// ---- Rules ---------------------------------------------------------------------------------------------------------------

/**
 * The piece a connector between two nodes comes to, or none where Fleet has no act for it. A rule fires on a state entered or a step starting, and lands on a state a ticket is moved to or a pull request is
 * made ready or merged in. The instance the moment is about is the one the rule lands on when it is of that lifecycle; else the ones of it the Job holds.
 */
export function connectorWire(rule: Rule, after: Workspace, workspace: string | undefined): LifecycleRule | undefined {
  const lane = after.lifecycles.find((one) => one.id === rule.to.lane);
  if (lane === undefined || rule.at === "leaves") return undefined;
  const when: LifecycleRule["when"] =
    rule.from.lane === "workflow" ? (rule.at === "ends" ? { kind: "step_ends", step: rule.from.item } : { kind: "step_starts", step: rule.from.item }) : { kind: "enters", lifecycle: rule.from.lane, state: rule.from.item };
  const target: RuleTarget = { among: when.kind === "enters" && when.lifecycle === lane.id ? { kind: "entered" } : { kind: "held", lifecycle: lane.id } };
  let what: RuleAct;
  let kind: string;
  if (lane.kind === "tracker" || lane.kind === undefined) [what, kind] = [{ act: "tracker_move", to: rule.to.item, target }, "tracker_write"];
  else if (lane.kind === "forge" && rule.to.item === "merged") [what, kind] = [{ act: "forge_merge", target }, "forge_write"];
  else if (lane.kind === "forge" && rule.to.item === "ready") [what, kind] = [{ act: "forge_mark_ready", target }, "forge_write"];
  else return undefined;
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

/** The Workspace with the piece of each rule a connector drew, which is the piece the rule is kept as. A connector that makes no rule Fleet can run draws none. */
export function withWires(before: Workspace, after: Workspace, workspace: string | undefined): Workspace {
  return {
    ...after,
    rules: after.rules.flatMap((rule): Rule[] => {
      if (rule.wire !== undefined || before.rules.some((one) => one.id === rule.id)) return [rule];
      const wire = connectorWire(rule, after, workspace);
      return wire === undefined ? [] : [{ ...rule, wire }];
    }),
  };
}

/** A piece's words, apart from the scope it is kept at. */
const sans = (piece: LifecycleRule) => ({ ...piece, scope: undefined });

function ruleOps(read: PiecesRead, before: Workspace, after: Workspace): Op[] {
  const layers = ruleLayers(read);
  const workspace = read.workspace?.id;
  const ops: Op[] = [];
  /** The levels a rule has a piece at: where it applies, and each that keeps its own. */
  const levelsOf = (rule: Rule): string[] => [...new Set([rule.scope, ...(rule.overrides ?? []).map((one) => one.scope)])];
  /** The piece a rule has at exactly one level. */
  const wireAt = (rule: Rule, level: string): LifecycleRule | undefined => (level === rule.scope ? rule.wire : rule.overrides?.find((one) => one.scope === level)?.wire);
  for (const rule of after.rules) {
    const was = before.rules.find((one) => one.id === rule.id);
    if (was === undefined) {
      // A rule drawn by a connector has the piece the connector comes to.
      const made = rule.wire ?? connectorWire(rule, after, workspace);
      if (made !== undefined) ops.push({ op: "save", piece: { kind: "rule", rule: { ...made, scope: scopeOf(rule.scope, workspace) } }, overwrite: false });
      continue;
    }
    const own = layers.get(rule.id);
    if (own === undefined || same(was, rule)) continue;
    // **Each level keeps its own piece whole**: one is written where its piece changed, and a level that keeps none is written by nobody.
    for (const level of new Set([...levelsOf(was), ...levelsOf(rule)])) {
      const id: PieceId = { kind: "rule", id: rule.id, scope: scopeOf(level, workspace) };
      const held = own.get(level);
      const then = wireAt(was, level);
      const now = wireAt(rule, level);
      if (now === undefined) {
        // An override the level no longer makes is its piece taken away.
        if (then !== undefined && held !== undefined) ops.push(...takenAway(id, held));
        continue;
      }
      if (then !== undefined && same(sans(then), sans(now))) continue;
      ops.push({ op: "save", piece: { kind: "rule", rule: { ...now, scope: id.scope } }, overwrite: held?.kept.rank.source === "kit" });
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
  return ops;
}

/** The ops a change to the Workspace comes to. */
export const opsFrom = (read: PiecesRead, before: Workspace, after: Workspace): Op[] => [...lifecycleOps(read, before, after), ...ruleOps(read, before, after)];

/** The piece of the person's own that a tab runs for `id`, which Commit to repository copies. Absent where the piece it runs is a repository's or Armada's. */
export function kitPieceAt(read: PiecesRead, kind: PieceId["kind"], id: string, level: string): PieceId | undefined {
  const layers = (kind === "rule" ? ruleLayers(read) : lifecycleLayers(read)).get(id) as Layers<{ scope: PieceScope }> | undefined;
  const found = layers === undefined ? undefined : effective(layers, level);
  return found?.layer.kept.rank.source === "kit" ? { kind, id, scope: found.layer.piece.scope } : undefined;
}
