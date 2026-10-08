// Steps added to a Job on the approval canvas: the same `+` and menu as the Workflow tab, on the
// connector in every gap of the Work lane and of the Delivery lane, and ahead of the first Work step.
// A lane makes room by moving what is under each gap down, and only the lane that took a step
// grows, so an added step sits on the spine between its neighbours.
//
// **The Delivery lane's gaps are the pull request's.** Before it opens is the delivering step's
// `step_starts`, after it opens is `pr_opened`; no pull request, no second gap.

import type { WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";
import { addedCard, addedName } from "@armada/components";
import type { AddedStep, JobDetail as JobWhole } from "@armada/protocol";

import type { ApprovalNode, Lane } from "./approval-canvas";
import { CARD, GATE } from "./approval-layout";
import type { Layout, Place } from "./approval-layout";
import { ahead, chainAt } from "./added-reach";
import type { Gap } from "./added-reach";
import { ADDED_HEIGHT, addedNodeId, PLUS_HEIGHT, plusFor, plusNode } from "./added-steps";
import type { AddedSteps } from "./added-steps";

const LANES_THAT_TAKE_ONE: readonly Lane[] = ["work", "delivery"];

const heightOf = (node: ApprovalNode) => (node.kind === "checks" ? GATE.height : CARD.height);
const widthOf = (node: ApprovalNode) => (node.kind === "checks" ? GATE.width : CARD.width);

type Gate = {
  lane: Lane;
  from?: ApprovalNode;
  into?: ApprovalNode;
  /** What an added step here is placed at. */
  gap: Gap;
  /** What the `+` says it is after, or before. */
  name: string;
  chain: AddedStep[];
  /** Whether a `+` may be drawn: not where the moment has come. */
  offered: boolean;
};

/** The approval canvas's layout with its added steps, and what each lane had to give up to hold them. */
export function withAddedAtGate(
  added: AddedSteps,
  whole: JobWhole,
  nodes: readonly ApprovalNode[],
  layout: Layout,
  /** The step the picked workflow delivers on, which the gate may not share with the Job's own. */
  deliverer: string | undefined,
): { layout: Layout; extra: WorkflowCanvasNode[]; edges: WorkflowCanvasEdge[] } {
  const at = (node: ApprovalNode): Place => layout.places.get(node.id) ?? { x: 0, y: 0 };
  const stepNamed = (stepId: string) => nodes.find((node) => node.kind === "step" && node.stepId === stepId)?.name;

  const gates: Gate[] = [];
  const put = (gate: Omit<Gate, "chain" | "offered">) => {
    if (gates.some((one) => one.gap.when === gate.gap.when && one.gap.step === gate.gap.step)) return;
    const chain = chainAt(added.rows, gate.gap);
    const offered = added.offers && ahead(whole, gate.gap);
    if (offered || chain.length > 0) gates.push({ ...gate, chain, offered });
  };
  for (const lane of LANES_THAT_TAKE_ONE) {
    const spine = nodes
      .filter((node) => node.lane === lane && node.from === undefined && node.chain === undefined && layout.places.has(node.id))
      .sort((a, b) => at(a).y - at(b).y);
    spine.forEach((node, index) => {
      const into = spine[index + 1];
      const ahead0 = index === 0 ? undefined : spine[index - 1];
      // Ahead of the first Work step: nothing leads into it.
      if (lane === "work" && ahead0 === undefined && node.kind === "step" && node.stepId !== undefined) {
        put({ lane, into: node, gap: { when: "step_starts", step: node.stepId }, name: node.name });
      }
      if (lane === "delivery" && deliverer !== undefined) {
        // Before the pull request opens, and after.
        if (node.id === "done") put({ lane, from: node, ...(into === undefined ? {} : { into }), gap: { when: "step_starts", step: deliverer }, name: node.name });
        if (node.kind === "pr") put({ lane, from: node, ...(into === undefined ? {} : { into }), gap: { when: "pr_opened", step: deliverer }, name: node.name });
      }
      // After a step, once its gate has passed: the step passing.
      if (node.stepId !== undefined && into?.stepId !== node.stepId) {
        put({ lane, from: node, ...(into === undefined ? {} : { into }), gap: { when: "step_passes", step: node.stepId }, name: stepNamed(node.stepId) ?? node.name });
      }
    });
  }

  /** What a gap takes: the `+` itself, and each added step with the room for the connector after it. */
  const heightOfGate = (gate: Gate) => PLUS_HEIGHT + gate.chain.length * (ADDED_HEIGHT + PLUS_HEIGHT) + (gate.into === undefined ? PLUS_HEIGHT : 0);
  /** What a lane has moved something that stood at `y` down by: every gap above it. */
  const movedAt = (lane: Lane, y: number) =>
    gates.filter((gate) => gate.lane === lane && gate.into !== undefined && at(gate.into).y <= y).reduce((sum, gate) => sum + heightOfGate(gate), 0);
  const grewBy = (lane: Lane) => gates.filter((gate) => gate.lane === lane).reduce((sum, gate) => sum + heightOfGate(gate), 0);

  const places = new Map<string, Place>();
  for (const node of nodes) {
    const place = layout.places.get(node.id);
    if (place === undefined) continue;
    places.set(node.id, LANES_THAT_TAKE_ONE.includes(node.lane) ? { x: place.x, y: place.y + movedAt(node.lane, place.y) } : place);
  }
  const leaves = new Map<string, Place>();
  for (const [id, place] of layout.leaves) {
    const lane = nodes.find((node) => node.id === id)?.lane;
    leaves.set(id, lane !== undefined && LANES_THAT_TAKE_ONE.includes(lane) ? { x: place.x, y: place.y + movedAt(lane, place.y) } : place);
  }
  const zones = layout.frames.filter((frame) => frame.kind === "zone");
  const laneOfFrame = (frame: { x: number }): Lane | undefined => {
    const zone = zones.find((one) => frame.x >= one.x && frame.x <= one.x + one.width);
    return LANES_THAT_TAKE_ONE.find((lane) => zone?.id === `zone:${lane}`);
  };
  const frames = layout.frames.map((frame) => {
    if (frame.kind === "zone") {
      const lane = LANES_THAT_TAKE_ONE.find((one) => frame.id === `zone:${one}`);
      return lane === undefined ? frame : { ...frame, height: frame.height + grewBy(lane) };
    }
    const lane = laneOfFrame(frame);
    return lane === undefined ? frame : { ...frame, y: frame.y + movedAt(lane, frame.y) };
  });

  const extra: WorkflowCanvasNode[] = [];
  let edges = [...layout.edges];
  for (const gate of gates) {
    const anchor = gate.from ?? gate.into!;
    const x = at(anchor).x + widthOf(anchor) / 2 - CARD.width / 2;
    let y =
      gate.into === undefined
        ? (places.get(gate.from!.id)?.y ?? 0) + heightOf(gate.from!)
        : at(gate.into).y + movedAt(gate.lane, at(gate.into).y - 1);
    let previous = gate.from?.id;
    // The `+` sits after the last step added here, or on the connector leaving what the gap is after.
    const stop = gate.chain.length - 1;
    const button = (item: number) => {
      if (!gate.offered || item !== stop) return undefined;
      return item === -1 ? plusFor(added, gate.gap, gate.name, gate.from === undefined ? "before" : "after") : plusFor(added, gate.gap, addedName(gate.chain[item]!.runs));
    };
    let item = -1;
    if (gate.from === undefined) {
      if (gate.offered && stop === -1) {
        const id = `add:before:${gate.lane}`;
        extra.push(plusNode(id, x, y, button(-1)));
        previous = id;
        item = -2;
      }
      y += PLUS_HEIGHT;
    } else {
      y += PLUS_HEIGHT;
    }
    const direct = edges.findIndex((edge) => edge.source === gate.from?.id && edge.target === gate.into?.id);
    if (direct !== -1) edges = edges.filter((_, i) => i !== direct);
    const link = (to: string, plain = false) => {
      if (previous === undefined) return;
      const add = item === -2 ? undefined : button(item);
      edges.push({ id: `${previous}>${to}`, source: previous, target: to, kind: "leads", ...(add === undefined ? {} : { add }), ...(plain ? { plain: true } : {}) });
    };
    gate.chain.forEach((one, index) => {
      const id = addedNodeId(one.id);
      extra.push({ id, position: { x, y }, card: addedCard(one, added.open === one.id, () => added.onOpen(one.id)) });
      link(id);
      previous = id;
      item = index;
      y += ADDED_HEIGHT + PLUS_HEIGHT;
    });
    if (gate.into !== undefined) {
      link(gate.into.id);
    } else if (gate.offered && item === stop) {
      // The end of the lane: a stub that ends at a `+`, with no arrowhead going into it.
      const stub = `add:end:${gate.lane}:${gate.gap.step}`;
      extra.push(plusNode(stub, x, y - PLUS_HEIGHT / 2, button(item)));
      link(stub, true);
    }
  }
  return { layout: { ...layout, places, frames, edges, leaves }, extra, edges };
}
