// Steps added to a Job on the approval canvas, a mock: the same `+` and menu as
// the Workflow tab, on the connector in every gap of the Work lane and of the
// Delivery lane, and ahead of the first step. Before the Job starts every gap
// takes one; once it runs, the step it is on and those after it do. A lane makes
// room by moving what is under each gap down, and only the lane that took a step
// grows, so an added step sits on the spine between its neighbours.

import { AddStep, insertedCard, insertStep } from "@armada/components";
import type { Inserted, WorkflowCanvasEdge, WorkflowCanvasNode } from "@armada/components";

import type { ApprovalNode, Lane } from "./approval-canvas";
import { CARD, GATE } from "./approval-layout";
import type { Layout, Place } from "./approval-layout";
import { ADDED_HEIGHT, AT, BEFORE, PLUS_HEIGHT, chainAfter, plusNode } from "./added-steps";
import type { AddedSteps, reach } from "./added-steps";

const LANES_THAT_TAKE_ONE: readonly Lane[] = ["work", "delivery"];

const heightOf = (node: ApprovalNode) => (node.kind === "checks" ? GATE.height : CARD.height);
const widthOf = (node: ApprovalNode) => (node.kind === "checks" ? GATE.width : CARD.width);

type Gap = {
  lane: Lane;
  from?: ApprovalNode;
  into?: ApprovalNode;
  /** What an added step here hangs from. */
  key: string;
  /** What the `+` says it is after, or before. */
  name: string;
  chain: Inserted[];
};

const nameOfAdded = (one: Inserted) => (one.name === "" ? "Drone step" : one.name);

/** The approval canvas's layout with its added steps, and what each lane had to give up to hold them. */
export function withAddedAtGate(
  added: AddedSteps,
  nodes: readonly ApprovalNode[],
  layout: Layout,
  reaches: ReturnType<typeof reach>,
): { layout: Layout; extra: WorkflowCanvasNode[]; edges: WorkflowCanvasEdge[] } {
  const { may, unstarted, deliverer } = reaches;
  const at = (node: ApprovalNode): Place => layout.places.get(node.id) ?? { x: 0, y: 0 };
  const stepNamed = (stepId: string) => nodes.find((node) => node.kind === "step" && node.stepId === stepId)?.name;

  const gaps: Gap[] = [];
  for (const lane of LANES_THAT_TAKE_ONE) {
    const spine = nodes
      .filter((node) => node.lane === lane && node.from === undefined && node.chain === undefined && layout.places.has(node.id))
      .sort((a, b) => at(a).y - at(b).y);
    const delivery = lane === "delivery";
    /** What a gap after this node hangs from, where the node may take one. */
    const keyAfter = (node: ApprovalNode): string | undefined =>
      delivery
        ? deliverer !== undefined && may.has(deliverer)
          ? AT + node.id
          : undefined
        : node.stepId !== undefined && may.has(node.stepId)
          ? node.stepId
          : undefined;
    const first = spine.find((node) => node.stepId !== undefined);
    if (!delivery && unstarted && first !== undefined && first === spine[0]) {
      gaps.push({ lane, into: first, key: BEFORE + first.stepId!, name: first.name, chain: chainAfter(added, BEFORE + first.stepId!) });
    }
    spine.forEach((node, index) => {
      const into = spine[index + 1];
      const key = keyAfter(node);
      if (key === undefined) return;
      if (!delivery && into !== undefined && into.stepId === node.stepId) return;
      gaps.push({ lane, from: node, ...(into === undefined ? {} : { into }), key, name: delivery ? node.name : (stepNamed(node.stepId!) ?? node.name), chain: chainAfter(added, key) });
    });
  }

  /** What a gap takes: the `+` itself, and each added step with the room for the connector after it. */
  const heightOfGap = (gap: Gap) => PLUS_HEIGHT + gap.chain.length * (ADDED_HEIGHT + PLUS_HEIGHT) + (gap.into === undefined ? PLUS_HEIGHT : 0);
  /** What a lane has moved something that stood at `y` down by: every gap above it. */
  const movedAt = (lane: Lane, y: number) =>
    gaps.filter((gap) => gap.lane === lane && gap.into !== undefined && at(gap.into).y <= y).reduce((sum, gap) => sum + heightOfGap(gap), 0);
  const grewBy = (lane: Lane) => gaps.filter((gap) => gap.lane === lane).reduce((sum, gap) => sum + heightOfGap(gap), 0);

  const places = new Map<string, Place>();
  for (const node of nodes) {
    const place = layout.places.get(node.id);
    if (place === undefined) continue;
    places.set(node.id, LANES_THAT_TAKE_ONE.includes(node.lane) ? { x: place.x, y: place.y + movedAt(node.lane, place.y) } : place);
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
  const button = (key: string, name: string, where: "after" | "before" = "after") => (
    <AddStep label={`Add a step ${where} ${name}`} onPick={(kind) => added.onOpen(insertStep(added.jobId, key, kind))} />
  );
  for (const gap of gaps) {
    const anchor = gap.from ?? gap.into!;
    const x = at(anchor).x + widthOf(anchor) / 2 - CARD.width / 2;
    let y =
      gap.into === undefined
        ? (places.get(gap.from!.id)?.y ?? 0) + heightOf(gap.from!)
        : at(gap.into).y + movedAt(gap.lane, at(gap.into).y - 1);
    let previous = gap.from?.id;
    let from = { key: gap.from === undefined ? undefined : gap.key, name: gap.name };
    if (gap.from === undefined) {
      const id = `add:before:${gap.lane}`;
      extra.push(plusNode(id, x, y, button(gap.key, gap.name, "before")));
      previous = id;
      y += PLUS_HEIGHT;
    } else {
      y += PLUS_HEIGHT;
    }
    const direct = edges.findIndex((edge) => edge.source === gap.from?.id && edge.target === gap.into?.id);
    if (direct !== -1) edges = edges.filter((_, i) => i !== direct);
    const link = (to: string, plain = false) =>
      edges.push({
        id: `${previous}>${to}`,
        source: previous!,
        target: to,
        kind: "leads",
        ...(from.key === undefined ? {} : { add: button(from.key, from.name) }),
        ...(plain ? { plain: true } : {}),
      });
    for (const one of gap.chain) {
      extra.push({ id: one.id, position: { x, y }, card: insertedCard(one, added.open === one.id, () => added.onOpen(one.id)) });
      link(one.id);
      previous = one.id;
      from = { key: one.id, name: nameOfAdded(one) };
      y += ADDED_HEIGHT + PLUS_HEIGHT;
    }
    if (gap.into !== undefined) {
      link(gap.into.id);
    } else {
      // The end of the lane: a stub that ends at a `+`, with no arrowhead going into it.
      const stub = `add:end:${gap.lane}`;
      extra.push(plusNode(stub, x, y - PLUS_HEIGHT / 2, button(from.key!, from.name)));
      from = { key: undefined, name: from.name };
      link(stub, true);
    }
  }
  return { layout: { ...layout, places, frames, edges }, extra, edges };
}
