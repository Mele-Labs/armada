// Where each node of the approval canvas sits: three lanes left to right —
// setup, the work, delivery — each its own spine top to bottom, a step's gate
// stages on the spine below it, and a fan under the node it falls from (the owner,
// 4 Oct 2026). Numbers, because React Flow places by number; each names the
// token it is read off.

import type { WorkflowCanvasEdge } from "@armada/components";

import { LANES } from "./approval-canvas";
import type { ApprovalNode, Lane } from "./approval-canvas";

/** A card, as `RunNode.css` draws it: full, and narrow (`--w-workflow-node`, `--w-workflow-task-node`). */
export const CARD = { height: 112, width: 260, narrow: 196 };

/** A gate stage, the lighter card between two steps (`RUN_NODE_GATE_HEIGHT`, `RUN_NODE_GATE_WIDTH`). */
const GATE = { height: 44, width: 216 };

/** Spine to spine (`--space-12` and `--space-2`), a step to its gate and gate stage to stage (`--space-4`), and the room a way back takes beside the spine (`--space-12`). */
const ROW_GAP = 56;
const CHAIN_GAP = 16;
const LOOP_ROOM = 64;

/** A task in a group's chain, as `RunNode.css` draws it (`RUN_NODE_TASK_HEIGHT`), and the room between two of a chain (`--space-6` and `--space-1`). */
const TASK = { height: 66, gap: 28 };

/** Across a fan, member to member (`--space-6`). */
const ACROSS = 24;

/** A lane's frame: its head, its padding (`--space-4` and `--space-1`), and the gutter between two lanes (`--space-12` and `--space-8`, and `--space-2`). */
const ZONE_HEAD = 40;
const ZONE_PAD = 20;
const LANE_GAP = 88;

/** The room under a lane's last card (`--space-8`), so an edge leaving it turns inside the lane. */
const ZONE_FOOT = 56;

/** The room above a lane's first card, so an edge crossing into it turns below the head. */
const LANE_TOP = ZONE_HEAD + 24;

/** A fan's frame: its head and padding, and the room an edge's own words take. */
const CLUSTER_HEAD = 28;
const CLUSTER_PAD = 12;
const LABELLED = 24;

export type Place = { x: number; y: number };

/** A frame drawn behind the nodes: a lane's Zone, or a fan's Cluster. */
export type Frame = { id: string; kind: "zone" | "cluster"; name?: string; title?: string } & Place & {
    width: number;
    height: number;
  };

export type Layout = {
  places: ReadonlyMap<string, Place>;
  frames: readonly Frame[];
  /** A node drawn as a frame, by id: a plan group as a Cluster. */
  sizes: ReadonlyMap<string, { width: number; height: number }>;
  /** Each edge as drawn: one crossing between lanes turns in the gutter between them. */
  edges: readonly WorkflowCanvasEdge[];
};

/** What a lane is called on its head. */
const LANE_NAME: Record<Lane, string> = { setup: "Setup", work: "Work", delivery: "Delivery" };

/** Whether a node is drawn narrow: a plan group in a fan, its tasks, the setup lane. */
export const narrowOf = (node: ApprovalNode): boolean =>
  node.kind === "group" || node.kind === "task" || node.kind === "more" || node.lane === "setup";

const widthOf = (node: ApprovalNode): number => (narrowOf(node) ? CARD.narrow : CARD.width);

/** A fan's members by depth: its rows, each laid across. */
function rowsOf(fan: readonly ApprovalNode[]): ApprovalNode[][] {
  const rows: ApprovalNode[][] = [];
  for (const member of fan) {
    const depth = member.band?.depth ?? 0;
    (rows[depth] ??= []).push(member);
  }
  return rows.filter((row) => row !== undefined);
}

const rowWidth = (row: readonly ApprovalNode[], nodes: readonly ApprovalNode[]): number =>
  row.reduce((sum, one) => sum + columnOf(one, nodes), 0) + ACROSS * Math.max(0, row.length - 1);

const wavesOfChain = (chain: readonly ApprovalNode[]): ApprovalNode[][] => {
  const waves: ApprovalNode[][] = [];
  for (const task of chain) (waves[task.chain!.wave] ??= []).push(task);
  return waves.filter((wave) => wave !== undefined);
};

const waveWidth = (wave: readonly ApprovalNode[]): number =>
  wave.reduce((sum, one) => sum + widthOf(one), 0) + ACROSS * Math.max(0, wave.length - 1);

/** How wide a group's column is: its card, or its widest wave of tasks. */
const columnOf = (group: ApprovalNode, nodes: readonly ApprovalNode[]): number =>
  Math.max(widthOf(group), ...wavesOfChain(nodes.filter((one) => one.chain?.group === group.id)).map(waveWidth)) +
  (group.kind === "group" ? CLUSTER_PAD * 2 : 0);

/**
 * **Where a group's tasks are put.** Each group
 * is a Cluster side by side with its neighbours: a head, then its waves as
 * rows inside. The group's own node is the frame, so an edge enters its top
 * and leaves its bottom. Returns where the row ends.
 */
function clustersUnder(
  fan: readonly ApprovalNode[],
  nodes: readonly ApprovalNode[],
  places: Map<string, Place>,
  sizes: Map<string, { width: number; height: number }>,
  spine: number,
  top: number,
): number {
  let x = spine - rowWidth(fan, nodes) / 2;
  let tallest = 0;
  for (const group of fan) {
    const width = columnOf(group, nodes);
    const waves = wavesOfChain(nodes.filter((one) => one.chain?.group === group.id));
    const body = waves.length === 0 ? 0 : waves.length * (TASK.height + TASK.gap) - TASK.gap + CLUSTER_PAD;
    const height = CLUSTER_HEAD + CLUSTER_PAD + body;
    places.set(group.id, { x, y: top });
    sizes.set(group.id, { width, height });
    for (const [n, wave] of waves.entries()) {
      let at = x + width / 2 - waveWidth(wave) / 2;
      for (const task of wave) {
        places.set(task.id, { x: at, y: top + CLUSTER_HEAD + CLUSTER_PAD + n * (TASK.height + TASK.gap) });
        at += widthOf(task) + ACROSS;
      }
    }
    tallest = Math.max(tallest, height);
    x += width + ACROSS;
  }
  // One height for the row, so every exit drops the same distance to the gate.
  for (const group of fan) sizes.set(group.id, { ...sizes.get(group.id)!, height: tallest });
  return top + tallest;
}

export function layoutOf(
  nodes: readonly ApprovalNode[],
  edges: readonly WorkflowCanvasEdge[],
  /** The workflow's name, which the Work lane's head carries. */
  workflowName?: string,
): Layout {
  const places = new Map<string, Place>();
  const frames: Frame[] = [];
  const sizes = new Map<string, { width: number; height: number }>();
  const labelled = new Set(edges.filter((edge) => edge.label !== undefined).map((edge) => edge.target));
  const laneOfId = new Map(nodes.map((node) => [node.id, node.lane]));
  /** Where each lane's left edge is, for the gutters. */
  const laneLeft = new Map<Lane, number>();
  const laneLoop = new Map<Lane, number>();
  let x = 0;
  let deepest = 0;
  const laneFrames: { lane: Lane; x: number; width: number }[] = [];

  for (const lane of LANES) {
    const inLane = nodes.filter((node) => node.lane === lane);
    if (inLane.length === 0) continue;
    const fans = new Map<string, ApprovalNode[]>();
    for (const node of inLane) {
      if (node.from !== undefined) fans.set(node.from, [...(fans.get(node.from) ?? []), node]);
    }
    // How far the lane reaches either side of its spine: a card, a way back beside it, the widest fan row.
    const hasLoop = edges.some((edge) => edge.kind === "returns" && laneOfId.get(edge.source) === lane);
    const widestRow = Math.max(0, ...[...fans.values()].flatMap((fan) => rowsOf(fan).map((row) => rowWidth(row, nodes))));
    const fanHalf = widestRow === 0 ? 0 : widestRow / 2 + CLUSTER_PAD;
    const half = (lane === "setup" ? CARD.narrow : CARD.width) / 2;
    const left = Math.max(half, fanHalf);
    const right = Math.max(half, fanHalf) + (hasLoop ? LOOP_ROOM : 0);
    const spine = x + ZONE_PAD + left;
    const spineCard = lane === "setup" ? CARD.narrow : CARD.width;
    laneLeft.set(lane, x);
    // Where a way back turns: past the widest thing on the spine, so it never runs through a card.
    laneLoop.set(lane, spine + Math.max(half, fanHalf) + LOOP_ROOM / 2);

    let y = LANE_TOP;
    for (const [at, node] of inLane.entries()) {
      if (node.from !== undefined || node.chain !== undefined) continue;
      if (labelled.has(node.id)) y += LABELLED;
      const gate = node.kind === "checks";
      places.set(node.id, { x: spine - (gate ? GATE.width : spineCard) / 2, y });
      y += gate ? GATE.height : CARD.height;
      const fan = fans.get(node.id);
      if (fan !== undefined) {
        const top = y + ROW_GAP;
        if (fan[0]?.kind === "group") {
          y = clustersUnder(fan, nodes, places, sizes, spine, top);
        } else {
        // The fan's Cluster: a head, then its rows centred on the spine.
        let rowY = top + CLUSTER_HEAD + CLUSTER_PAD;
        const rows = rowsOf(fan);
        const across = Math.max(...rows.map((row) => rowWidth(row, nodes)));
        for (const row of rows) {
          let at = spine - rowWidth(row, nodes) / 2;
          for (const member of row) {
            const column = columnOf(member, nodes);
            places.set(member.id, { x: at + (column - widthOf(member)) / 2, y: rowY });
            at += column + ACROSS;
          }
          rowY += CARD.height + ROW_GAP;
        }
        const bottom = rowY - ROW_GAP + CLUSTER_PAD;
        frames.push({
          id: `cluster:${node.id}`,
          kind: "cluster",
          name: fan[0]?.kind === "job" ? "Jobs" : "Groups",
          x: spine - across / 2 - CLUSTER_PAD,
          y: top,
          width: across + CLUSTER_PAD * 2,
          height: bottom - top,
        });
        y = bottom;
        }
      }
      // A gate's stages stand close under their step, and close to each other: one stage, not three.
      const next = inLane.slice(at + 1).find((one) => one.from === undefined);
      y += next?.kind === "checks" && next.stepId === node.stepId ? CHAIN_GAP : ROW_GAP;
    }
    // Room under the last card for an edge leaving the lane to turn inside it.
    deepest = Math.max(deepest, y - ROW_GAP + ZONE_FOOT);
    const width = ZONE_PAD * 2 + left + right;
    laneFrames.push({ lane, x, width });
    x += width + LANE_GAP;
  }

  // Every lane as tall as the deepest, so the three fill the canvas side by side.
  // Prepended in lane order, so the Zones draw first, behind every Cluster.
  frames.unshift(
    ...laneFrames.map(({ lane, x: at, width }): Frame => ({
      id: `zone:${lane}`,
      kind: "zone",
      name: lane === "work" && workflowName !== undefined ? workflowName : LANE_NAME[lane],
      x: at,
      y: 0,
      width,
      height: deepest,
    })),
  );

  const drawn = edges.map((edge) => {
    const from = laneOfId.get(edge.source);
    const into = laneOfId.get(edge.target);
    if (edge.kind === "returns" && from !== undefined) {
      const turns = laneLoop.get(from);
      return turns === undefined ? edge : { ...edge, via: turns };
    }
    if (from === undefined || into === undefined || from === into) return edge;
    // Across lanes: down out of the last, one bend in the gutter left of the lane it enters, into the first's side.
    const enters = laneLeft.get(into);
    return enters === undefined ? edge : { ...edge, via: enters - LANE_GAP / 2, intoSide: true };
  });
  return { places, frames, sizes, edges: drawn };
}
