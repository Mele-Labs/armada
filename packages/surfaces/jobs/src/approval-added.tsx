// Steps added to a Job at the gate, a mock: the same `+` and menu as the Workflow
// tab, in every gap between two steps of the Work lane, since nothing is done
// yet. The lane makes room by moving what is under each gap down, so an added
// step sits on the spine between its neighbours.

import { AddStep, insertedCard, insertStep } from "@armada/components";
import type { Inserted, WorkflowCanvasNode } from "@armada/components";

import type { ApprovalNode } from "./approval-canvas";
import { CARD, GATE } from "./approval-layout";
import type { Layout, Place } from "./approval-layout";
import type { AddedSteps } from "./added-steps";

const PLUS_HEIGHT = 36;
const ADDED_HEIGHT = CARD.height;
const SPACE = 8;

const dummy = { kind: "step", name: "", activity: "not_started", said: "" } as const;

const widthOf = (node: ApprovalNode) => (node.kind === "checks" ? GATE.width : CARD.width);

type Gap = { from: ApprovalNode; into: ApprovalNode; chain: Inserted[] };

/** The run's Work lane with its added steps, and what the lane had to give up to hold them. */
export function withAddedAtGate(
  added: AddedSteps,
  nodes: readonly ApprovalNode[],
  layout: Layout,
): { layout: Layout; extra: WorkflowCanvasNode[] } {
  const at = (node: ApprovalNode): Place => layout.places.get(node.id) ?? { x: 0, y: 0 };
  const spine = nodes
    .filter((node) => node.lane === "work" && node.from === undefined && node.chain === undefined && layout.places.has(node.id))
    .sort((a, b) => at(a).y - at(b).y);

  const chainAfter = (anchor: string): Inserted[] => {
    const next = added.inserted.find((one) => one.after === anchor);
    return next === undefined ? [] : [next, ...chainAfter(next.id)];
  };

  const gaps: Gap[] = [];
  spine.forEach((node, index) => {
    const into = spine[index + 1];
    if (node.stepId === undefined || into === undefined || into.stepId === node.stepId) return;
    gaps.push({ from: node, into, chain: chainAfter(node.stepId) });
  });
  const heightOfGap = (gap: Gap) => PLUS_HEIGHT + SPACE + gap.chain.length * (ADDED_HEIGHT + SPACE + PLUS_HEIGHT + SPACE);

  /** What the lane has moved something that stood at `y` down by: every gap above it. */
  const movedAt = (y: number) => gaps.filter((gap) => at(gap.into).y <= y).reduce((sum, gap) => sum + heightOfGap(gap), 0);
  const total = gaps.reduce((sum, gap) => sum + heightOfGap(gap), 0);

  const work = layout.frames.find((frame) => frame.id === "zone:work");
  const inLane = (frame: { x: number }) => work !== undefined && frame.x >= work.x && frame.x <= work.x + work.width;

  const places = new Map<string, Place>();
  for (const node of nodes) {
    const place = layout.places.get(node.id);
    if (place === undefined) continue;
    places.set(node.id, node.lane === "work" ? { x: place.x, y: place.y + movedAt(place.y) } : place);
  }
  const frames = layout.frames.map((frame) =>
    frame.kind === "zone"
      ? { ...frame, height: frame.height + total }
      : inLane(frame)
        ? { ...frame, y: frame.y + movedAt(frame.y) }
        : frame,
  );

  const extra: WorkflowCanvasNode[] = [];
  let edges = [...layout.edges];
  for (const gap of gaps) {
    const stepId = gap.from.stepId!;
    const centre = at(gap.from).x + widthOf(gap.from) / 2;
    const x = centre - CARD.width / 2;
    let y = at(gap.into).y + movedAt(at(gap.into).y - 1);
    let previous = gap.from.id;
    const link = (to: string) => edges.push({ id: `${previous}>${to}`, source: previous, target: to, kind: "leads" });
    const direct = edges.findIndex((edge) => edge.source === gap.from.id && edge.target === gap.into.id);
    if (direct !== -1) edges = edges.filter((_, i) => i !== direct);
    const plus = (key: string, anchor: string, name: string) => {
      const id = `add:${key}`;
      extra.push({
        id,
        position: { x, y },
        card: dummy,
        drawn: (
          <div className="armada-triggers__plus">
            <AddStep label={`Add a step after ${name}`} onPick={(kind) => added.onOpen(insertStep(added.jobId, anchor, kind))} />
          </div>
        ),
      });
      link(id);
      previous = id;
      y += PLUS_HEIGHT + SPACE;
    };
    plus(gap.from.id, stepId, nodes.find((node) => node.kind === "step" && node.stepId === stepId)?.name ?? gap.from.name);
    for (const one of gap.chain) {
      extra.push({
        id: one.id,
        position: { x, y },
        card: insertedCard(one, added.open === one.id, () => added.onOpen(one.id)),
      });
      link(one.id);
      previous = one.id;
      y += ADDED_HEIGHT + SPACE;
      plus(one.id, one.id, one.name === "" ? "Drone step" : one.name);
    }
    link(gap.into.id);
  }
  return { layout: { ...layout, places, frames, edges }, extra };
}
