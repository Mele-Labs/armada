// The card a canvas hangs beside a node whose Drone is held on a command: the
// same box the panels draw, joined to the node by an edge, off to its right.
//
// **Placed in free space and never moving another node.** It starts one gap
// past its node and steps right of anything it would cover, so the nodes
// around it stay where the plan or the run put them.

import { CanvasAsk, type WorkflowCanvasEdge, type WorkflowCanvasNode } from "@armada/components";

import type { HeldCommand } from "./drone-held";

/** Canvas widths of the cards, off `--w-workflow-node`, `-group-node` and `-task-node`. */
const WIDTH = { step: 260, group: 228, task: 196 } as const;
/** `--w-dock`, the width `CanvasAsk` takes. */
const ASK_WIDTH = 380;
/** What the box stands as tall as with its three answers, and what a node is as tall as. */
const ASK_HEIGHT = 480;
const NODE_HEIGHT = 120;
/** The gap an edge turns in, as `TASK_ACROSS` leaves between a group and its tasks. */
const GAP = 88;

/**
 * What the card says it is. **The one seam for the Drone's name**: once the
 * wire says which Drone is held, `held` carries it and the title names it here.
 */
export function askTitleOf(_held: HeldCommand): string {
  return "A Drone wants to run a command";
}

export const askNodeId = (hostId: string): string => `ask:${hostId}`;

/** The ask card for `host` and the edge joining them; `undefined` where `host` is not among `nodes`. */
export function askBeside(
  hostId: string,
  nodes: readonly WorkflowCanvasNode[],
  held: HeldCommand,
): { node: WorkflowCanvasNode; edge: WorkflowCanvasEdge } | undefined {
  const host = nodes.find((one) => one.id === hostId);
  if (host === undefined) return undefined;
  const reach = (one: WorkflowCanvasNode): number => WIDTH[one.card.kind];
  const others = nodes.filter((one) => one.id !== hostId && one.backdrop !== true);
  const y = host.position.y;
  let x = host.position.x + reach(host) + GAP;
  // Right of whatever it would cover, until nothing is.
  for (let again = 0; again < others.length; again += 1) {
    const covered = others.find(
      (one) =>
        one.position.x < x + ASK_WIDTH &&
        one.position.x + reach(one) > x &&
        one.position.y < y + ASK_HEIGHT &&
        one.position.y + NODE_HEIGHT > y,
    );
    if (covered === undefined) break;
    x = covered.position.x + reach(covered) + GAP;
  }
  const id = askNodeId(hostId);
  return {
    node: {
      id,
      position: { x, y },
      card: { kind: "step", name: "Command to allow", activity: "awaiting_human", said: "Needs you" },
      drawn: <CanvasAsk title={askTitleOf(held)}>{held.node}</CanvasAsk>,
    },
    edge: { id: `${hostId}>${id}`, source: hostId, target: id, kind: "asks", across: true },
  };
}

/** `nodes` and `edges` with the ask hung on `hostId`, or as they were where nothing is held there. */
export function withAsk<N extends WorkflowCanvasNode, E extends WorkflowCanvasEdge>(
  nodes: readonly N[],
  edges: readonly E[],
  hostId: string | undefined,
  held: HeldCommand | undefined,
): { nodes: (N | WorkflowCanvasNode)[]; edges: (E | WorkflowCanvasEdge)[] } {
  const ask = hostId === undefined || held === undefined ? undefined : askBeside(hostId, nodes, held);
  return ask === undefined
    ? { nodes: [...nodes], edges: [...edges] }
    : { nodes: [...nodes, ask.node], edges: [...edges, ask.edge] };
}
