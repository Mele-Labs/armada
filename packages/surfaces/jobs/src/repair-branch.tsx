// What stands off a Job's workflow: a leaf at the node each Trigger fired at, inside that node's lane,
// and for a failed Trigger with Self repair the branch with the repair Drone at work, then the fix and
// the choice of where it goes, then what came of it. **Drawn from `JobDetail.triggers`**, so a repair is the rows Fleet
// serves and `job.trigger_changed` moves, and the Overview canvas and the Workflow tab's canvas draw
// the same branch from the same rows.
//
// **Where it joins back.** A fix placed on `this_branch` is on the Job's own branch, so the lane
// returns into the node after the one it left. A fix that became a pull request ends in a mark of
// its own. A repair that found nothing ends red where it stands.

import type { ReactNode } from "react";
import { additionBranches, endsInPr, fixOf, HoldNode, holdsOf, repairPhase, repairsOf, RepairNode, RepairPrMark, TriggerLeaf } from "@armada/components";
import type { Held, HoldVerb, SideBranch } from "@armada/components";
import type { WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";
import type { AddedStep, HoldAct, JobTrigger, TriggerFixChoice, TriggerMoment } from "@armada/protocol";

import type { WorkflowRun } from "./workflow-canvas";

/** What names a fix: the Trigger, or the step added to the Job. */
export type FixOf = { trigger: string } | { addition: string };

/** Where a choice goes: the Job, which fix and the answer, which says whether Fleet took it. */
export type ChooseTriggerFixCall = (jobId: string, by: FixOf, choice: TriggerFixChoice) => Promise<{ ok: boolean }>;

/** Rerun or Skip on a hold: the Job, which act, and the Trigger or the added step it is on. The answer says whether Fleet took it. */
export type HoldActCall = (jobId: string, act: HoldVerb, by: HoldAct) => Promise<{ ok: boolean }>;

/** Where something fired: its moment and the step, which for `pr_opened` is the delivering one. */
type FiredAt = { when: TriggerMoment; step: string };

/** How far off the spine a leaf stands where nothing says, how far apart two stand in a stack, and the leaf's width (`--w-workflow-node`). */
const OFF = 56;
const APART = 12;
export const LEAF_WIDTH = 260;

/** What a leaf stands as tall as, for the room it asks of its lane: a row, a repair's head and what it holds. */
const ROW_HEIGHT = 44;
const REPAIR_HEAD = 64;
const FIX_ROW = 22;
const CHOICE = 52;

const dummy = { kind: "step", name: "", activity: "not_started", said: "" } as const;

/** One firing's branch, told apart from another Trigger's and from another firing of the same one. */
export const repairNodeId = (trigger: SideBranch): string => `repair:${trigger.when}|${trigger.step}|${trigger.name}|${trigger.addition ?? ""}|${trigger.started_at ?? ""}`;

export type Anchor = {
  id: string;
  x: number;
  y: number;
  width: number;
  /** Where the leaves' column starts, inside the lane. Absent stands them clear of the card. */
  leafX?: number;
};

/** What stands off a step: a Trigger's mark, a branch a repair or a Drone grew, or a hold with no line to sit on. */
export type Leaf = {
  id: string;
  /** The node it stands off. */
  anchor: string;
  drawn: ReactNode;
  height: number;
  flowing: boolean;
  asking: boolean;
  /** The repair it belongs to, for the way back into the spine and the pull request it became. */
  repair?: SideBranch;
  tail?: { id: string; drawn: ReactNode };
};

type Branches = { nodes: WorkflowCanvasNode[]; edges: WorkflowCanvasEdge[]; asking: string | null; onLine: Map<string, ReactNode> };

/** What holds the Job and how to act on it, with the spine a hold sits on. */
export type HoldsOn = { holds: readonly Held[]; spine: readonly WorkflowCanvasEdge[]; act: HoldActCall | undefined };

/** Opens a firing's line in the Job's log. */
export type OpenTriggerLog = (trigger: JobTrigger) => void;

const keyOf = (one: { when: TriggerMoment; step: string; name: string }): string => `${one.when}|${one.step}|${one.name}`;

/** How tall a repair's branch stands: its head, the files of the fix once it is found, and the choice while it waits. */
const repairHeight = (trigger: SideBranch): number => {
  const phase = repairPhase(trigger);
  const files = phase === "working" || phase === "running" ? 0 : (trigger.repair?.files ?? []).length;
  return REPAIR_HEAD + files * FIX_ROW + (phase === "asking" ? CHOICE : 0);
};

/**
 * **Everything that stands off a step, as leaves**: the holds with no line to sit on, the
 * branches a repair or a Drone grew, and a mark for each Trigger that fired and grew nothing.
 * `anchorId` says which node each fired at, and an absent one draws no leaf. The stack under a
 * step is asked of the lane before it is laid out, so this takes no positions.
 */
export function leavesOf(
  triggers: readonly SideBranch[],
  jobId: string,
  anchorId: (at: FiredAt) => string | undefined,
  choose: ChooseTriggerFixCall | undefined,
  openLog?: OpenTriggerLog,
  on?: HoldsOn,
): Leaf[] {
  const leaves: Leaf[] = [];
  for (const held of on?.holds ?? []) {
    const anchor = anchorId(held);
    if (anchor === undefined || (held.when !== "pr_opened" && holdEdge(on!.spine, anchor, held.when) !== undefined)) continue;
    leaves.push({ id: `hold:${held.key}`, anchor, height: ROW_HEIGHT, flowing: false, asking: false, drawn: <HoldNode held={held} {...holdAct(on!, jobId)} {...(openLog === undefined ? {} : { onOpenLog: openLog })} /> });
  }
  for (const trigger of repairsOf(triggers)) {
    const anchor = anchorId(trigger);
    if (anchor === undefined) continue;
    const id = repairNodeId(trigger);
    const phase = repairPhase(trigger);
    leaves.push({
      id,
      anchor,
      repair: trigger,
      height: repairHeight(trigger),
      flowing: phase === "working" || phase === "running" || phase === "rerunning",
      asking: phase === "asking",
      drawn: (
        <RepairNode
          trigger={trigger}
          {...(choose === undefined ? {} : { onChoose: (one, choice) => choose(jobId, fixOf(one), choice) })}
          {...(openLog === undefined ? {} : { onOpenLog: openLog })}
        />
      ),
      ...(endsInPr(trigger) ? { tail: { id: `${id}:pr`, drawn: <RepairPrMark trigger={trigger} /> } } : {}),
    });
  }
  // The latest firing of each Trigger that grew no branch and holds no one: a mark with its state.
  const latest = new Map<string, SideBranch>();
  // A step added to the Job is drawn here only while it waits on him to run it; its repair is the branch above.
  for (const one of triggers) {
    if (one.addition === undefined) latest.set(keyOf(one), one);
    else if (one.state === "awaiting_owner") latest.set(`addition|${one.addition}`, one);
  }
  const held = new Set((on?.holds ?? []).map((one) => one.key));
  for (const [key, one] of latest) {
    const anchor = anchorId(one);
    if (anchor === undefined || held.has(key) || repairPhase(one) !== "none") continue;
    leaves.push({ id: `leaf:${key}`, anchor, height: ROW_HEIGHT, flowing: false, asking: false, drawn: <TriggerLeaf trigger={one} {...(openLog === undefined ? {} : { onOpenLog: openLog })} {...(on === undefined ? {} : holdAct(on, jobId))} /> });
  }
  return leaves;
}

/** How tall the stack under each step is, so its lane can grow to hold it. */
export function leafRoom(leaves: readonly Leaf[]): Map<string, number> {
  const room = new Map<string, number>();
  for (const leaf of leaves) {
    const used = room.get(leaf.anchor);
    room.set(leaf.anchor, (used === undefined ? 0 : used + APART) + leaf.height + (leaf.tail === undefined ? 0 : APART + ROW_HEIGHT));
  }
  return room;
}

/**
 * The leaves as nodes and edges, stacked down the column beside the step each stands off.
 * `rejoin` is the node a fix placed on this branch lands back in.
 */
export function drawLeaves(
  leaves: readonly Leaf[],
  anchorOf: (id: string) => Anchor | undefined,
  rejoin: (anchor: string) => string | undefined,
): Omit<Branches, "onLine"> {
  const nodes: WorkflowCanvasNode[] = [];
  const edges: WorkflowCanvasEdge[] = [];
  let asking: string | null = null;
  const used = new Map<string, number>();
  for (const leaf of leaves) {
    const anchor = anchorOf(leaf.anchor);
    if (anchor === undefined) continue;
    const down = used.get(anchor.id);
    const x = anchor.leafX ?? anchor.x + anchor.width + OFF;
    const y = anchor.y + (down ?? 0);
    used.set(anchor.id, (down === undefined ? 0 : down) + leaf.height + APART + (leaf.tail === undefined ? 0 : ROW_HEIGHT + APART));
    if (leaf.asking && asking === null) asking = leaf.id;
    nodes.push({ id: leaf.id, position: { x, y }, card: dummy, drawn: leaf.drawn });
    edges.push({ id: `${anchor.id}>${leaf.id}`, source: anchor.id, target: leaf.id, kind: "leads", across: true, flowing: leaf.flowing });
    if (leaf.tail !== undefined) {
      nodes.push({ id: leaf.tail.id, position: { x, y: y + leaf.height + APART }, card: dummy, drawn: leaf.tail.drawn });
      edges.push({ id: `${leaf.id}>${leaf.tail.id}`, source: leaf.id, target: leaf.tail.id, kind: "leads" });
    }
    const back = rejoin(anchor.id);
    if (leaf.repair !== undefined && repairPhase(leaf.repair) === "done" && leaf.repair.repair?.choice === "this_branch" && back !== undefined) {
      edges.push({ id: `${leaf.id}>${back}`, source: leaf.id, target: back, kind: "leads" });
    }
  }
  return { nodes, edges, asking };
}

/** The holds drawn on the connectors they hold: by edge id, for `withHolds`. */
export function holdsOnLine(on: HoldsOn, jobId: string, anchorId: (at: FiredAt) => string | undefined, openLog?: OpenTriggerLog): Map<string, ReactNode> {
  const inline = new Map<string, Held[]>();
  for (const held of on.holds) {
    const anchor = anchorId(held);
    const spine = anchor === undefined || held.when === "pr_opened" ? undefined : holdEdge(on.spine, anchor, held.when);
    if (spine !== undefined) inline.set(spine.id, [...(inline.get(spine.id) ?? []), held]);
  }
  return new Map([...inline].map(([edge, holds]) => [edge, holdsAdd(holds, on, jobId, openLog)]));
}

/**
 * The branches for `triggers` as nodes and edges. `anchorOf` says where each fired, and an absent
 * one draws no leaf. `rejoin` is the node a fix placed on this branch lands back in.
 */
export function repairBranches(
  triggers: readonly SideBranch[],
  jobId: string,
  anchorOf: (at: FiredAt) => Anchor | undefined,
  rejoin: (anchor: string) => string | undefined,
  choose: ChooseTriggerFixCall | undefined,
  on?: HoldsOn,
  openLog?: OpenTriggerLog,
): Branches {
  const anchors = new Map<string, Anchor>();
  const idOf = (at: FiredAt): string | undefined => {
    const anchor = anchorOf(at);
    if (anchor !== undefined) anchors.set(anchor.id, anchor);
    return anchor?.id;
  };
  const leaves = leavesOf(triggers, jobId, idOf, choose, openLog, on);
  const drawn = drawLeaves(leaves, (id) => anchors.get(id), rejoin);
  // A hold on a spine edge is drawn on the line, so the line runs through it and ends in its one arrowhead.
  const onLine = on === undefined ? new Map<string, ReactNode>() : holdsOnLine(on, jobId, idOf, openLog);
  return { ...drawn, onLine };
}

/** `edges` with each hold drawn on the connector it holds. The edge stays one edge. */
export function withHolds(edges: readonly WorkflowCanvasEdge[], onLine: ReadonlyMap<string, ReactNode>): WorkflowCanvasEdge[] {
  // An edge may carry a `+` already: the hold draws first and the `+` stays on the same line after it.
  return edges.map((edge) =>
    onLine.has(edge.id)
      ? {
          ...edge,
          add:
            edge.add === undefined ? (
              onLine.get(edge.id)
            ) : (
              <>
                {onLine.get(edge.id)}
                {edge.add}
              </>
            ),
        }
      : edge,
  );
}

const holdAct = (on: HoldsOn, jobId: string): { onAct?: (act: HoldVerb, by: HoldAct) => Promise<{ ok: boolean }> } =>
  on.act === undefined ? {} : { onAct: (act, by) => on.act!(jobId, act, by) };

const holdsAdd = (holds: readonly Held[], on: HoldsOn, jobId: string, openLog?: OpenTriggerLog) => (
  <>
    {holds.map((held) => (
      <HoldNode key={held.key} held={held} {...holdAct(on, jobId)} {...(openLog === undefined ? {} : { onOpenLog: openLog })} />
    ))}
  </>
);

/** The spine edge a hold sits on: into the step before it starts, out of the step after it passes. */
function holdEdge(spine: readonly WorkflowCanvasEdge[], anchor: string, when: TriggerMoment): WorkflowCanvasEdge | undefined {
  return spine.find((edge) => edge.kind === "leads" && edge.across !== true && (when === "step_starts" ? edge.target === anchor : edge.source === anchor));
}

/** The node a spine leaves `anchor` for: the first forward edge out of it. */
export function nextAfter(edges: readonly WorkflowCanvasEdge[], anchor: string): string | undefined {
  return edges.find((edge) => edge.source === anchor && edge.kind === "leads" && edge.plain !== true)?.target;
}

/** The room under a step's stack that is left before the next node, so a leaf never touches it. */
const BELOW = 24;

/**
 * Moves what stands under a step with a tall stack of leaves down far enough to hold it. A step
 * and its leaves stay together; the nodes below it all move by the same distance.
 */
function roomFor(nodes: readonly WorkflowCanvasNode[], room: ReadonlyMap<string, number>): WorkflowCanvasNode[] {
  const shifts: { from: number; by: number }[] = [];
  const spine = nodes.filter((node) => node.backdrop !== true).sort((a, b) => a.position.y - b.position.y);
  for (const node of spine) {
    const stack = room.get(node.id);
    if (stack === undefined) continue;
    const next = spine.find((one) => one.position.y > node.position.y && Math.abs(one.position.x - node.position.x) < LEAF_WIDTH / 2);
    const free = next === undefined ? Infinity : next.position.y - node.position.y;
    if (stack + BELOW > free) shifts.push({ from: next!.position.y, by: stack + BELOW - free });
  }
  return nodes.map((node) => ({ ...node, position: { ...node.position, y: node.position.y + shifts.filter((one) => node.position.y >= one.from).reduce((sum, one) => sum + one.by, 0) } }));
}

/**
 * A Workflow tab's run with the leaves off the step each Trigger fired at, the holds on the lines
 * they hold. **The step is the Trigger's own**,
 * which for `pr_opened` is the one that delivers.
 */
export function withRepair(
  fired: readonly JobTrigger[] | undefined,
  jobId: string,
  nodeOf: (step: string) => string,
  run: WorkflowRun,
  choose: ChooseTriggerFixCall | undefined,
  additions: readonly AddedStep[] = [],
  hold?: HoldActCall,
  openLog?: OpenTriggerLog,
): { run: WorkflowRun; asking: string | null } {
  const holds = holdsOf(fired ?? [], additions);
  // A step added to the Job grows the same branch as a Trigger, from the same repair record.
  const triggers: SideBranch[] = [...(fired ?? []), ...additionBranches(additions)];
  const anchorId = (at: FiredAt): string | undefined => {
    const id = nodeOf(at.step);
    return run.nodes.some((one) => one.id === id) ? id : undefined;
  };
  const on = { holds, spine: run.edges, act: hold };
  const leaves = leavesOf(triggers, jobId, anchorId, choose, openLog, on);
  if (leaves.length === 0 && holds.length === 0) return { run, asking: null };
  const placed = roomFor(run.nodes, leafRoom(leaves));
  const branch = drawLeaves(
    leaves,
    (id) => {
      const node = placed.find((one) => one.id === id);
      return node === undefined ? undefined : { id, x: node.position.x, y: node.position.y, width: LEAF_WIDTH };
    },
    (anchor) => nextAfter(run.edges, anchor),
  );
  const onLine = holdsOnLine(on, jobId, anchorId, openLog);
  return {
    run: { ...run, nodes: [...placed, ...branch.nodes], edges: [...withHolds(run.edges, onLine), ...branch.edges] },
    asking: branch.asking,
  };
}
