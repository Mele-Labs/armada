// The branch a failed Trigger with Self repair grows off a Job's workflow: a side lane at the node
// the Trigger fired at, with the repair Drone at work, then the fix and the choice of where it
// goes, then what came of it. **Drawn from `JobDetail.triggers`**, so a repair is the rows Fleet
// serves and `job.trigger_changed` moves, and the Overview canvas, the Workflow tab's canvas and its
// stacked run draw the same branch from the same rows.
//
// **Where it joins back.** A fix placed on `this_branch` is on the Job's own branch, so the lane
// returns into the node after the one it left. A fix that became a pull request ends in a mark of
// its own. A repair that found nothing ends red where it stands.

import { endsInPr, repairPhase, repairsOf, RepairNode, RepairPrMark } from "@armada/components";
import type { WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";
import type { JobTrigger, TriggerFixChoice } from "@armada/protocol";

import type { WorkflowRun } from "./workflow-canvas";

/** Where a Trigger's choice goes: the Job, the Trigger's name and the answer, which says whether Fleet took it. */
export type ChooseTriggerFixCall = (jobId: string, trigger: string, choice: TriggerFixChoice) => Promise<{ ok: boolean }>;

/** How far off the spine the branch stands, how far a second one stands under the first, and the PR it became under its branch. */
const OFF = 56;
const APART = 160;
const PR_BELOW = 190;

const dummy = { kind: "step", name: "", activity: "not_started", said: "" } as const;

/** One firing's branch, told apart from another Trigger's and from another firing of the same one. */
export const repairNodeId = (trigger: JobTrigger): string => `repair:${trigger.when}|${trigger.step}|${trigger.name}|${trigger.started_at ?? ""}`;

export type Anchor = { id: string; x: number; y: number; width: number };

type Branches = { nodes: WorkflowCanvasNode[]; edges: WorkflowCanvasEdge[]; asking: string | null };

/**
 * The branches for `triggers` as nodes and edges. `anchorOf` says where each fired, and an absent
 * one draws no branch. `rejoin` is the node a fix placed on this branch lands back in.
 */
export function repairBranches(
  triggers: readonly JobTrigger[],
  jobId: string,
  anchorOf: (trigger: JobTrigger) => Anchor | undefined,
  rejoin: (anchor: string) => string | undefined,
  choose: ChooseTriggerFixCall | undefined,
): Branches {
  const nodes: WorkflowCanvasNode[] = [];
  const edges: WorkflowCanvasEdge[] = [];
  let asking: string | null = null;
  const beside = new Map<string, number>();
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
  return { nodes, edges, asking };
}

/** The node a spine leaves `anchor` for: the first forward edge out of it. */
export function nextAfter(edges: readonly WorkflowCanvasEdge[], anchor: string): string | undefined {
  return edges.find((edge) => edge.source === anchor && edge.kind === "leads")?.target;
}

/**
 * A Workflow tab's run with the repair branches off the step each Trigger fired at, and the same
 * under that step in the stacked run. **The step is the Trigger's own**, which for `pr_opened` is the
 * one that delivers.
 */
export function withRepair(
  triggers: readonly JobTrigger[] | undefined,
  jobId: string,
  nodeOf: (step: string) => string,
  run: WorkflowRun,
  choose: ChooseTriggerFixCall | undefined,
): { run: WorkflowRun; asking: string | null } {
  if (triggers === undefined || repairsOf(triggers).length === 0) return { run, asking: null };
  const anchorOf = (trigger: JobTrigger): Anchor | undefined => {
    const at = run.nodes.find((node) => node.id === nodeOf(trigger.step));
    return at === undefined ? undefined : { id: at.id, x: at.position.x, y: at.position.y, width: 260 };
  };
  const branch = repairBranches(triggers, jobId, anchorOf, (anchor) => nextAfter(run.edges, anchor), choose);
  const rows = run.rows.map((row) => {
    const mine = repairsOf(triggers).filter((one) => nodeOf(one.step) === row.id);
    if (mine.length === 0) return row;
    return {
      ...row,
      trailing: (
        <>
          {row.trailing}
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
    run: { ...run, nodes: [...run.nodes, ...branch.nodes], edges: [...run.edges, ...branch.edges], rows },
    asking: branch.asking,
  };
}
