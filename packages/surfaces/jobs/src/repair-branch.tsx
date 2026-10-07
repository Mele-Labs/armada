// The branch a failed Trigger with Self repair grows off a Job's workflow: a side lane at the node
// the Trigger fired at, with the repair Drone at work, then the fix and the choice of where it
// goes, then what came of it. **Drawn from `JobDetail.triggers`**, so a repair is the rows Fleet
// serves and `job.trigger_changed` moves, and the Overview canvas, the Workflow tab's canvas and its
// stacked run draw the same branch from the same rows.
//
// **Where it joins back.** A fix placed on `this_branch` is on the Job's own branch, so the lane
// returns into the node after the one it left. A fix that became a pull request ends in a mark of
// its own. A repair that found nothing ends red where it stands.

import type { ReactNode } from "react";
import { endsInPr, HoldNode, holdsOf, repairPhase, repairsOf, RepairNode, RepairPrMark } from "@armada/components";
import type { Held, HoldVerb } from "@armada/components";
import type { WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";
import type { AddedStep, HoldAct, JobTrigger, TriggerFixChoice, TriggerMoment } from "@armada/protocol";

import type { WorkflowRun } from "./workflow-canvas";

/** Where a Trigger's choice goes: the Job, the Trigger's name and the answer, which says whether Fleet took it. */
export type ChooseTriggerFixCall = (jobId: string, trigger: string, choice: TriggerFixChoice) => Promise<{ ok: boolean }>;

/** Rerun or Skip on a hold: the Job, which act, and the Trigger or the added step it is on. The answer says whether Fleet took it. */
export type HoldActCall = (jobId: string, act: HoldVerb, by: HoldAct) => Promise<{ ok: boolean }>;

/** Where something fired: its moment and the step, which for `pr_opened` is the delivering one. */
type FiredAt = { when: TriggerMoment; step: string };

/** How far off the spine the branch stands, how far a second one stands under the first, and the PR it became under its branch. */
const OFF = 56;
const APART = 160;
const PR_BELOW = 190;

const dummy = { kind: "step", name: "", activity: "not_started", said: "" } as const;

/** One firing's branch, told apart from another Trigger's and from another firing of the same one. */
export const repairNodeId = (trigger: JobTrigger): string => `repair:${trigger.when}|${trigger.step}|${trigger.name}|${trigger.started_at ?? ""}`;

export type Anchor = { id: string; x: number; y: number; width: number };

type Branches = { nodes: WorkflowCanvasNode[]; edges: WorkflowCanvasEdge[]; asking: string | null; onLine: Map<string, ReactNode> };

/** What holds the Job and how to act on it, with the spine a hold sits on. */
export type HoldsOn = { holds: readonly Held[]; spine: readonly WorkflowCanvasEdge[]; act: HoldActCall | undefined };

/**
 * The branches for `triggers` as nodes and edges. `anchorOf` says where each fired, and an absent
 * one draws no branch. `rejoin` is the node a fix placed on this branch lands back in.
 */
export function repairBranches(
  triggers: readonly JobTrigger[],
  jobId: string,
  anchorOf: (at: FiredAt) => Anchor | undefined,
  rejoin: (anchor: string) => string | undefined,
  choose: ChooseTriggerFixCall | undefined,
  on?: HoldsOn,
): Branches {
  const nodes: WorkflowCanvasNode[] = [];
  const edges: WorkflowCanvasEdge[] = [];
  let asking: string | null = null;
  const beside = new Map<string, number>();
  const inline = new Map<string, Held[]>();
  for (const held of on?.holds ?? []) {
    const anchor = anchorOf(held);
    if (anchor === undefined) continue;
    const spine = held.when === "pr_opened" ? undefined : holdEdge(on!.spine, anchor.id, held.when);
    if (spine !== undefined) {
      inline.set(spine.id, [...(inline.get(spine.id) ?? []), held]);
      continue;
    }
    const at = beside.get(anchor.id) ?? 0;
    beside.set(anchor.id, at + 1);
    const id = `hold:${held.key}`;
    nodes.push({ id, position: { x: anchor.x + anchor.width + OFF, y: anchor.y + at * APART }, card: dummy, drawn: <HoldNode held={held} {...holdAct(on!, jobId)} /> });
    edges.push({ id: `${anchor.id}>${id}`, source: anchor.id, target: id, kind: "leads", across: true });
  }
  for (const trigger of repairsOf(triggers)) {
    const anchor = anchorOf(trigger);
    if (anchor === undefined) continue;
    const id = repairNodeId(trigger);
    const phase = repairPhase(trigger);
    const at = beside.get(anchor.id) ?? 0;
    beside.set(anchor.id, at + 1);
    const x = anchor.x + anchor.width + OFF;
    const y = anchor.y + at * APART;
    if (phase === "asking" && asking === null) asking = id;
    nodes.push({
      id,
      position: { x, y },
      card: dummy,
      drawn: (
        <RepairNode
          trigger={trigger}
          {...(choose === undefined ? {} : { onChoose: (one, choice) => choose(jobId, one.name, choice) })}
        />
      ),
    });
    edges.push({
      id: `${anchor.id}>${id}`,
      source: anchor.id,
      target: id,
      kind: "leads",
      across: true,
      flowing: phase === "working" || phase === "rerunning",
    });
    if (endsInPr(trigger)) {
      const pr = `${id}:pr`;
      nodes.push({ id: pr, position: { x, y: y + PR_BELOW }, card: dummy, drawn: <RepairPrMark trigger={trigger} /> });
      edges.push({ id: `${id}>${pr}`, source: id, target: pr, kind: "leads" });
    }
    const back = rejoin(anchor.id);
    if (phase === "done" && trigger.repair?.choice === "this_branch" && back !== undefined) {
      edges.push({ id: `${id}>${back}`, source: id, target: back, kind: "leads" });
    }
  }
  // A hold on a spine edge is drawn on the line, so the line runs through it and ends in its one arrowhead.
  const onLine = new Map([...inline].map(([edge, holds]) => [edge, holdsAdd(holds, on!, jobId)]));
  return { nodes, edges, asking, onLine };
}

/** `edges` with each hold drawn on the connector it holds. The edge stays one edge. */
export function withHolds(edges: readonly WorkflowCanvasEdge[], onLine: ReadonlyMap<string, ReactNode>): WorkflowCanvasEdge[] {
  return edges.map((edge) => (onLine.has(edge.id) ? { ...edge, add: onLine.get(edge.id) } : edge));
}

const holdAct = (on: HoldsOn, jobId: string): { onAct?: (act: HoldVerb, by: HoldAct) => Promise<{ ok: boolean }> } =>
  on.act === undefined ? {} : { onAct: (act, by) => on.act!(jobId, act, by) };

const holdsAdd = (holds: readonly Held[], on: HoldsOn, jobId: string) => (
  <>
    {holds.map((held) => (
      <HoldNode key={held.key} held={held} {...holdAct(on, jobId)} />
    ))}
  </>
);

/** The spine edge a hold sits on: into the step before it starts, out of the step after it passes. */
function holdEdge(spine: readonly WorkflowCanvasEdge[], anchor: string, when: TriggerMoment): WorkflowCanvasEdge | undefined {
  return spine.find((edge) => edge.kind === "leads" && edge.across !== true && (when === "step_starts" ? edge.target === anchor : edge.source === anchor));
}

/** The node a spine leaves `anchor` for: the first forward edge out of it. */
export function nextAfter(edges: readonly WorkflowCanvasEdge[], anchor: string): string | undefined {
  return edges.find((edge) => edge.source === anchor && edge.kind === "leads")?.target;
}

/**
 * A Workflow tab's run with the repair branches off the step each Trigger fired at, the holds on
 * the lines they hold, and the same under that step in the stacked run. **The step is the
 * Trigger's own**, which for `pr_opened` is the one that delivers.
 */
export function withRepair(
  triggers: readonly JobTrigger[] | undefined,
  jobId: string,
  nodeOf: (step: string) => string,
  run: WorkflowRun,
  choose: ChooseTriggerFixCall | undefined,
  additions: readonly AddedStep[] = [],
  hold?: HoldActCall,
): { run: WorkflowRun; asking: string | null } {
  const holds = holdsOf(triggers ?? [], additions);
  if (triggers === undefined || (repairsOf(triggers).length === 0 && holds.length === 0)) return { run, asking: null };
  const anchorOf = (at: FiredAt): Anchor | undefined => {
    const node = run.nodes.find((one) => one.id === nodeOf(at.step));
    return node === undefined ? undefined : { id: node.id, x: node.position.x, y: node.position.y, width: 260 };
  };
  const branch = repairBranches(triggers, jobId, anchorOf, (anchor) => nextAfter(run.edges, anchor), choose, { holds, spine: run.edges, act: hold });
  const rows = run.rows.map((row) => {
    const mine = repairsOf(triggers).filter((one) => nodeOf(one.step) === row.id);
    const mineHeld = holds.filter((one) => nodeOf(one.step) === row.id);
    if (mine.length === 0 && mineHeld.length === 0) return row;
    return {
      ...row,
      trailing: (
        <>
          {row.trailing}
          {mineHeld.map((one) => (
            <HoldNode key={one.key} held={one} {...(hold === undefined ? {} : { onAct: (act: HoldVerb, by: HoldAct) => hold(jobId, act, by) })} />
          ))}
          {mine.map((one) => (
            <div key={repairNodeId(one)}>
              <RepairNode trigger={one} {...(choose === undefined ? {} : { onChoose: (t: JobTrigger, choice: TriggerFixChoice) => choose(jobId, t.name, choice) })} />
              {endsInPr(one) ? <RepairPrMark trigger={one} /> : null}
            </div>
          ))}
        </>
      ),
    };
  });
  return {
    run: { ...run, nodes: [...run.nodes, ...branch.nodes], edges: [...withHolds(run.edges, branch.onLine), ...branch.edges], rows },
    asking: branch.asking,
  };
}
