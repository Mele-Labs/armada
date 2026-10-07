// The branch a failed trigger with Self repair grows off a Job's workflow, a
// mock: a side lane at the step the trigger fired at, with the repair Drone at
// work, then the fix and the choice of where it goes, then what came of it. The
// Overview canvas, the Workflow tab's canvas and its stacked run draw the same
// branch from the same repair.

import { chooseRepair, RepairNode, RepairPrMark } from "@armada/components";
import type { Repair, WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";

import type { WorkflowRun } from "./workflow-canvas";

/** How far off the spine the branch stands, and how far under it the PR it became. */
const OFF = 56;
const PR_BELOW = 190;

const dummy = { kind: "step", name: "", activity: "not_started", said: "" } as const;

export const REPAIR_NODE = "repair:branch";

type Anchor = { id: string; x: number; y: number; width: number };

/** The branch as nodes and edges, hung off `anchor`. `rejoin` is the spine node a fix on this branch lands back in. */
export function repairNodes(
  repair: Repair,
  jobId: string,
  anchor: Anchor,
  rejoin?: string,
): { nodes: WorkflowCanvasNode[]; edges: WorkflowCanvasEdge[] } {
  const x = anchor.x + anchor.width + OFF;
  const nodes: WorkflowCanvasNode[] = [
    {
      id: REPAIR_NODE,
      position: { x, y: anchor.y },
      card: dummy,
      drawn: <RepairNode repair={repair} onChoose={(choice) => chooseRepair(jobId, choice)} />,
    },
  ];
  const edges: WorkflowCanvasEdge[] = [
    {
      id: `${anchor.id}>${REPAIR_NODE}`,
      source: anchor.id,
      target: REPAIR_NODE,
      kind: "leads",
      across: true,
      flowing: repair.phase === "working" || repair.phase === "rerunning",
    },
  ];
  if (repair.phase === "done" && repair.choice === "newpr") {
    nodes.push({ id: "repair:pr", position: { x, y: anchor.y + PR_BELOW }, card: dummy, drawn: <RepairPrMark /> });
    edges.push({ id: `${REPAIR_NODE}>repair:pr`, source: REPAIR_NODE, target: "repair:pr", kind: "leads" });
  }
  if (repair.phase === "done" && repair.choice === "branch" && rejoin !== undefined) {
    edges.push({ id: `${REPAIR_NODE}>${rejoin}`, source: REPAIR_NODE, target: rejoin, kind: "leads" });
  }
  return { nodes, edges };
}

/** A Workflow tab's run with the repair branch off the step it fired at, and the same branch under that step in the stacked run. */
export function withRepair(repair: Repair | undefined, jobId: string, stepNode: string, run: WorkflowRun): WorkflowRun {
  if (repair === undefined) return run;
  const at = run.nodes.find((node) => node.id === stepNode);
  if (at === undefined) return run;
  const branch = repairNodes(repair, jobId, { id: at.id, x: at.position.x, y: at.position.y, width: 260 });
  const rows = run.rows.map((row) =>
    row.id !== stepNode
      ? row
      : {
          ...row,
          trailing: (
            <>
              {row.trailing}
              <RepairNode repair={repair} onChoose={(choice) => chooseRepair(jobId, choice)} />
              {repair.phase === "done" && repair.choice === "newpr" ? <RepairPrMark /> : null}
            </>
          ),
        },
  );
  return { ...run, nodes: [...run.nodes, ...branch.nodes], edges: [...run.edges, ...branch.edges], rows };
}
