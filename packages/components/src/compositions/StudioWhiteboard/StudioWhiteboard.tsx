import {
  BaseEdge,
  EdgeLabelRenderer,
  Handle,
  MarkerType,
  applyNodeChanges,
  getBezierPath,
  useReactFlow,
  useStore,
  type Edge,
  type EdgeProps,
  type Node,
  type NodeChange,
  type NodeProps,
} from "@xyflow/react";
import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";

import { StudioFrame, studioFrameLabel, type StudioFrameKind } from "../StudioFrame/StudioFrame";
import { frameSizes, landing, onTheBoard, parentsFirst, pressedIn, type Landing } from "./frames";

import { GRAPH_CANVAS_SIDES, GraphCanvas, clearOf, facingSides } from "../GraphCanvas/GraphCanvas";
import {
  STUDIO_NODE_KIND,
  StudioNode,
  StudioNodeDraft,
  studioNodeLabel,
  type StudioNodeKind,
  type StudioNodeOf,
} from "../StudioNode/StudioNode";

/**
 * Studio whiteboard — a Studio's nodes and edges, placed freely: drag, pan,
 * zoom, fit and select.
 *
 * **`GraphCanvas` is the graph half**, shared with a Job's workflow canvas
 * (`#1539`). What is left here is free placement and what a Studio draws: every
 * node is a `StudioNode` and every edge is drawn below.
 *
 * **No relation carries colour; one standing does.** Produced is a bare line;
 * Same as, Blocks and Answers carry their label and nothing more, so a hue
 * never says which kind an edge is. A relation nobody has answered is dashed,
 * with a dot on it in `awaiting_review`'s amber that opens a small card on
 * hover or focus: who proposed it, the relation, and Accept and Reject under
 * it. **The answer is given where the relation lands** — the owner, 29 Sep
 * 2026: a queue in the corner was disconnected from the edge it was about. Nothing here draws or deletes a
 * relation on its own; accepting one is the caller's `onDecide`.
 */

export type StudioWhiteboardNode = {
  id: string;
  /**
   * The frame it sits in — a Zone, or a Note's Cluster — by its id. Absent is
   * the board. A Zone and a Cluster draw as frames round what names them here,
   * sized to it. `#1620`.
   */
  within?: string;
  /** Where the node sits: from its frame's corner, or the board's origin where it is in none. */
  position: { x: number; y: number };
  node: StudioNodeOf & { title: string; facts?: readonly string[] };
};

/**
 * A node still being written, drawn where it will land — the owner's notes of
 * 1 Oct 2026. **Bridge's and nobody else's** until the caller sends it: its
 * moves go to `onMoved` rather than `onNodeMoved`, it is never reported as
 * selected, and the field is the caller's.
 */
export type StudioWhiteboardDraft = {
  /** A fresh id per draft, so a second draft is a fresh field rather than the first one re-kinded. */
  id: string;
  kind: StudioNodeKind;
  position: { x: number; y: number };
  /** The field. Focused once the board has drawn the card. */
  field: ReactNode;
  pending?: boolean;
  /**
   * Dragged somewhere else, so what is sent lands where it was dropped:
   * `position` on the board, where the draft is drawn, and `landed` the Zone
   * it was dropped in and its spot there, as a node's own drop is read.
   */
  onMoved?: (position: { x: number; y: number }, landed: StudioWhiteboardLanding) => void;
};

/**
 * Where something put down lands: the Zone it is in, `null` for the board, and
 * its spot from that frame's corner — or on the board, where it is in none.
 */
export type StudioWhiteboardLanding = Landing;

export type StudioEdgeRelation = "same_as" | "blocks" | "answers";

export type StudioWhiteboardEdge = {
  id: string;
  source: string;
  target: string;
} & (
  | { kind: "produced" }
  /**
   * Proposed by Helm or a scout, and not yet accepted by a person. `proposer`
   * is who drew it, already said (`Helm proposes`); absent is the bare fact.
   */
  | { kind: StudioEdgeRelation; proposed: boolean; proposer?: string }
);

export type StudioWhiteboardProps = {
  nodes: readonly StudioWhiteboardNode[];
  edges: readonly StudioWhiteboardEdge[];
  /**
   * A node was put down somewhere new, by pointer or by arrow key: the frame it
   * landed in, `null` for the board, and its spot from that frame's corner.
   * **Dropped on a Zone it goes in, and dragged off one it comes out**; a Note
   * in a Cluster stays in it. Dragging a frame carries what it holds.
   */
  onNodeMoved?: (id: string, position: { x: number; y: number }, within: string | null) => void;
  onSelectionChange?: (ids: readonly string[]) => void;
  /**
   * One node to arrive selected — the board opened *at* something rather than
   * at nothing. `#1362`: a Job's detail opens the Studio that dispatched it,
   * and the person lands on the part of the graph it came from.
   *
   * **Applied once, never held.** `onSelectionChange` stays the only report of
   * what is selected; this is the other direction, and a prop that re-asserted
   * itself would take the board back off the person clicking on it.
   */
  pick?: string | null;
  /**
   * Nothing moves. A Studio reopens read-only (`docs/concepts/studio.md`), so
   * no node drags, by pointer or by arrow key, and `onNodeMoved` is never
   * called. Pan, zoom, fit and selection still work: reading is looking around.
   */
  readOnly?: boolean;
  /**
   * A proposed relation answered, from the Accept or Reject on its own label.
   * Absent draws neither. While `readOnly`, the label says how to answer
   * instead, and this is never called.
   */
  onDecide?: (edgeId: string, accepted: boolean) => void;
  /** The relation whose answer is out to Fleet. Its Accept spins, and every answer waits. */
  deciding?: string | null;
  /** The node a press on the canvas put down, with its field in it. Absent is none. */
  draft?: StudioWhiteboardDraft | null;
  /**
   * A press where a kind armed on the rail goes: on empty board, or inside a
   * Zone. `at` is the point on the board, and `landed` the Zone the press puts
   * it in, as a drop would — the owner, 2 Oct 2026. `GraphCanvas`'s rule: a
   * drag is no press.
   */
  onPanePress?: (at: { x: number; y: number }, landed: StudioWhiteboardLanding) => void;
  /**
   * The kind armed, and the board draws a crosshair. **The kind and not a
   * flag**, because which frame a press lands in is the kind's: a Zone pressed
   * inside a Zone lands on the board.
   */
  placing?: StudioNodeKind | null;
  /**
   * ⌘V while the board has focus and no field does — the owner's note of
   * 1 Oct 2026. `at` is under the pointer where it is over the board, and the
   * middle of the view where it is not. Answers whether it took the paste;
   * one it did not is left to the browser. Absent, a paste does nothing here.
   */
  onPaste?: (clipboard: DataTransfer, at: { x: number; y: number }) => boolean;
  /**
   * What sits over the board's top-right corner — a Studio's Run control.
   * **Never the field behind a rail press**: that is the draft, put down where
   * the person pressed (the owner, 1 Oct 2026). Nor the relations
   * waiting on a person — a panel doing two jobs with neither of them named is
   * what the owner read on 28 Sep 2026 as a card showing when nothing was
   * selected. What waits on a person is answered on its own edge.
   */
  children?: ReactNode;
  /**
   * The bar down the board's leading edge — what a person puts on a Studio.
   * `GraphCanvasRail` is what goes here, and the sketch pad mounts the same
   * one: `docs/contracts/iconography.md`, *The canvas rail*.
   */
  rail?: ReactNode;
  /**
   * The acts on what is picked, hovering over it — `GraphCanvasNodeBar`.
   *
   * **Drawn inside the board rather than beside it**, which is where React
   * Flow's own store resolves and so the only place the bar can find the nodes
   * it hangs over.
   */
  nodeBar?: ReactNode;
};

/** What a proposed relation is read aloud as, after the relation itself. */
const EDGE_PROPOSED = "proposed, waiting on you";

/** Who proposed it, where the record does not say. */
const PROPOSED = "Proposed";

/** What a proposed relation's footer says while the Studio is read-only. */
const CONTINUE_TO_ANSWER = "Continue to accept or reject.";

/** The relation's label, as `studio.md`, Edges, names it. Produced has none. */
export const STUDIO_EDGE_LABEL: Readonly<Record<StudioEdgeRelation, string>> = {
  same_as: "same as",
  blocks: "blocks",
  answers: "answers",
};

/**
 * `within` is the frame the caller last said the node is in. **Kept beside the
 * card rather than read off `parentId`**, which is the board's own and moves
 * the moment a node is dropped somewhere new: a frame the caller names that
 * differs from the one it named before is the caller moving it.
 */
type BoardNodeData = StudioWhiteboardNode["node"] & { within?: string };
type DraftNodeData = { kind: StudioNodeKind; field: ReactNode; pending: boolean };
type FrameNodeData = { kind: StudioFrameKind; title: string; within?: string };
type BoardNode = Node<BoardNodeData, "studio"> | Node<DraftNodeData, "draft"> | Node<FrameNodeData, "frame">;
/** A proposed relation's card: who drew it, the sentence its buttons are named by, and the answer. */
type Proposal = {
  proposer: string;
  said: string;
  readOnly: boolean;
  deciding: string | null;
  onDecide?: (edgeId: string, accepted: boolean) => void;
};
type BoardEdgeData = { label: string | null; proposed: boolean; proposal: Proposal | null };
type BoardEdge = Edge<BoardEdgeData, "studio">;

/**
 * The card, with an anchor for each end of an edge on every side. Which side
 * an edge takes is chosen from where the two nodes sit — `facingSides`.
 * Nothing connects by hand, so none is drawn.
 */
function BoardNodeView({ data, selected }: NodeProps<Node<BoardNodeData, "studio">>) {
  const { within: _, ...card } = data;
  return (
    <>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <StudioNode {...card} selected={selected} />
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`s-${side}`} id={`s-${side}`} type="source" position={side} isConnectable={false} />
      ))}
    </>
  );
}

/** A Zone or a Cluster: a region the whiteboard sizes round what it holds. */
function FrameNodeView({ data, selected }: NodeProps<Node<FrameNodeData, "frame">>) {
  return (
    <>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <StudioFrame kind={data.kind} title={data.title} selected={selected} />
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`s-${side}`} id={`s-${side}`} type="source" position={side} isConnectable={false} />
      ))}
    </>
  );
}

/**
 * A draft's card. **Focused once React Flow has measured it**, not on mount:
 * a node is drawn `visibility: hidden` until its size is known, and focus on a
 * hidden field does nothing — so `autoFocus` alone left the caret nowhere.
 *
 * **And brought into view when it is not.** A press near the board's edge, or
 * a Link's offer arriving under its address, can put the field half past it,
 * and a caret below the fold is not typing straight in. The focus takes no scroll of its own:
 * the browser would scroll React Flow's clipped pane, which nothing pans back.
 */
function DraftNodeView({
  data,
  width = 0,
  height = 0,
  positionAbsoluteX,
  positionAbsoluteY,
}: NodeProps<Node<DraftNodeData, "draft">>) {
  const at = useRef<HTMLDivElement>(null);
  const flow = useReactFlow();
  const shown = width > 0;
  useEffect(() => {
    const card = at.current;
    if (!shown || card === null) return;
    card.querySelector<HTMLElement>("textarea, input")?.focus({ preventScroll: true });
  }, [shown]);
  useEffect(() => {
    const card = at.current;
    if (!shown || card === null) return;
    const board = card.closest(".react-flow")?.getBoundingClientRect();
    const box = card.getBoundingClientRect();
    if (board === undefined) return;
    const inView =
      box.left >= board.left && box.right <= board.right && box.top >= board.top && box.bottom <= board.bottom;
    if (inView) return;
    void flow.setCenter(positionAbsoluteX + width / 2, positionAbsoluteY + height / 2, { zoom: flow.getZoom() });
    // When it is drawn and when it grows — a Link's offer arrives with the
    // address — and never otherwise: a person who pans away is not pulled back.
  }, [shown, height]);
  return (
    <div ref={at}>
      <StudioNodeDraft kind={data.kind} pending={data.pending}>
        {data.field}
      </StudioNodeDraft>
    </div>
  );
}

/**
 * A proposed relation's label: a dot on its line, and the card it opens — who
 * proposed it, the relation, and the answer. **Its buttons are named by the
 * whole sentence**, `Accept: Note A same as Note B`, since a board can hold
 * several proposals and a bare Accept names none of them. `nodrag nopan` are
 * React Flow's own: a press here is a press, not the start of a pan.
 *
 * **A dot until it is asked for** — the owner, 2 Oct 2026: *"The label stops
 * covering the cards it runs between."* The card sat at the line's middle and
 * hid the text of the Note under it. Moving it along the line was the other
 * offer, and it fails on the board he drew it on: a line out of one column of
 * a Zone crosses the next one's cards, and the gaps between columns are
 * narrower than the card. **It opens on hover and on focus**, so the keyboard
 * reaches Accept by tabbing onto the dot, and stays open while the pointer or
 * the focus is anywhere inside it.
 */
function ProposalCard({
  id,
  label,
  proposal,
  at,
}: {
  id: string;
  label: string;
  proposal: Proposal;
  at: CSSProperties;
}) {
  const { proposer, said, readOnly, deciding, onDecide } = proposal;
  const named = `${proposer}: ${said}`;
  return (
    <span className="armada-studio-edge__proposal-at nodrag nopan" style={at}>
      <Button variant="ghost" size="sm" iconOnly aria-label={named}>
        <span className="armada-studio-edge__mark" aria-hidden />
      </Button>
    <Card className="armada-studio-edge__proposal" role="group" aria-label={named}>
      <span className="armada-studio-edge__proposer">{proposer}</span>
      <span className="armada-studio-edge__relation">{label}</span>
      {readOnly ? (
        <span className="armada-studio-edge__continue">{CONTINUE_TO_ANSWER}</span>
      ) : onDecide === undefined ? null : (
        <span className="armada-studio-edge__acts">
          <Button
            size="sm"
            pending={deciding === id}
            disabled={deciding !== null}
            aria-label={`Accept: ${said}`}
            onClick={() => onDecide(id, true)}
          >
            Accept
          </Button>
          <Button
            variant="ghost"
            size="sm"
            disabled={deciding !== null}
            aria-label={`Reject: ${said}`}
            onClick={() => onDecide(id, false)}
          >
            Reject
          </Button>
        </span>
      )}
    </Card>
    </span>
  );
}

function BoardEdgeView(props: EdgeProps<BoardEdge>) {
  const [path, labelX, labelY] = getBezierPath(props);
  const label = props.data?.label ?? null;
  const proposal = props.data?.proposal ?? null;
  const at = { transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` };
  return (
    <>
      <BaseEdge
        id={props.id}
        path={path}
        markerEnd={props.markerEnd}
        className={props.data?.proposed ? "armada-studio-edge--proposed" : undefined}
      />
      {label === null ? null : (
        <EdgeLabelRenderer>
          {/* **A dashed line is not a sentence.** It was the whole of what said
              a relation had been proposed, and the owner read the label beside
              it as a fact the Studio had decided — *where did same as come
              from*. So a proposed one names who drew it, and is answered here
              rather than in a queue in the corner. */}
          {proposal === null ? (
            <span className="armada-studio-edge__label" style={at}>
              {label}
            </span>
          ) : (
            <ProposalCard id={props.id} label={label} proposal={proposal} at={at} />
          )}
        </EdgeLabelRenderer>
      )}
    </>
  );
}

/**
 * Held down, a second node joins the selection rather than replacing it — which is how a person
 * picks the Notes they are accepting as one Cluster (`#1291`).
 *
 * **Named rather than left to React Flow's default**, which reads the platform off the user agent
 * and so differs between the app and a test of it. **A constant rather than a literal in the
 * element**: React Flow writes this into its own store on every render it sees a new value, and a
 * fresh array each time is an update loop it never leaves.
 */
const JOINS_THE_SELECTION = ["Meta", "Control"];

const NODE_TYPES = { studio: BoardNodeView, draft: DraftNodeView, frame: FrameNodeView };
const EDGE_TYPES = { studio: BoardEdgeView };

function toBoardNode({ id, position, node, within }: StudioWhiteboardNode): BoardNode {
  const held = within === undefined ? {} : { parentId: within };
  if (node.kind === "zone" || node.kind === "cluster") {
    // A Zone holds no words; a Cluster's title is its head.
    const title = node.kind === "cluster" ? node.title : "";
    return {
      id,
      position,
      ...held,
      type: "frame",
      data: { kind: node.kind, title, ...(within === undefined ? {} : { within }) },
      ariaLabel: studioFrameLabel({ kind: node.kind, title }),
    };
  }
  const data = within === undefined ? node : { ...node, within };
  return { id, position, ...held, type: "studio", data, ariaLabel: studioNodeLabel(node) };
}

/** The frame the caller last named for a node, off its data. */
function namedWithin(node: BoardNode): string | undefined {
  return node.type === "draft" ? undefined : node.data.within;
}

/**
 * Each frame sized round what it holds, and every node after the frame it
 * sits in. **Never kept**: worked out from where things sit, every render.
 */
function framed(nodes: BoardNode[]): BoardNode[] {
  const sizes = frameSizes(nodes.map(asFraming));
  return parentsFirst(nodes).map((node) => {
    const size = sizes.get(node.id);
    return size === undefined ? node : { ...node, width: size.width, height: size.height };
  });
}

function asFraming(node: BoardNode) {
  return {
    id: node.id,
    kind: node.data.kind,
    position: node.position,
    ...(node.parentId === undefined ? {} : { parentId: node.parentId }),
    ...(node.measured === undefined ? {} : { measured: node.measured }),
  };
}

/** What a draft is read aloud as: the kind, and that it is not on the Studio yet. */
function toDraftNode({ id, kind, position, field, pending = false }: StudioWhiteboardDraft): BoardNode {
  return {
    id,
    position,
    type: "draft",
    data: { kind, field, pending },
    ariaLabel: `New ${STUDIO_NODE_KIND[kind]}`,
    // Named as every node is, though it takes no focus stop of its own: the field inside is the stop.
    ariaRole: "group",
    // Dragged by its head and its edge; the field inside is `nodrag`, so a
    // press there is a press on the text.
    draggable: true,
    selectable: false,
    focusable: false,
    // **Over what a frame holds.** React Flow draws a frame's cards above any
    // node outside it, so a draft put down in a Zone sat under the Notes there
    // with its field unreachable. A picked node is lifted to 1000
    // (`SELECTED_NODE_Z` in `@xyflow/system`), and the card being written is
    // the one a person is looking at.
    zIndex: 1000,
  };
}

/**
 * What the caller gave, over what React Flow keeps per node — where it was put,
 * its measured size, whether it is selected. A node's kind, title and state
 * always come from the caller, so a Finding that freezes redraws in place.
 *
 * **Where it sits is the board's, until the caller moves it.** A frame the
 * caller names that differs from the one it named last time is the caller
 * putting the node somewhere — grouping Notes into a Cluster, or a frame
 * deleted out from under it — and the node takes the caller's frame and spot.
 */
function merged(
  given: readonly StudioWhiteboardNode[],
  draft: StudioWhiteboardDraft | null,
  kept: readonly BoardNode[],
): BoardNode[] {
  const byId = new Map(kept.map((node) => [node.id, node]));
  const fresh = [...given.map(toBoardNode), ...(draft === null ? [] : [toDraftNode(draft)])];
  return framed(
    fresh.map((node) => {
      const held = byId.get(node.id);
      if (held === undefined) return node;
      const { parentId: _, ...placed } = held;
      const moved = namedWithin(held) !== namedWithin(node);
      const at = moved
        ? { position: node.position, ...(node.parentId === undefined ? {} : { parentId: node.parentId }) }
        : { position: held.position, ...(held.parentId === undefined ? {} : { parentId: held.parentId }) };
      return { ...placed, ...at, data: node.data, ariaLabel: node.ariaLabel } as BoardNode;
    }),
  );
}

/** The edge as a sentence: `Note A same as Note B`. */
function edgeSaid(edge: StudioWhiteboardEdge, titleOf: (id: string) => string): string {
  const said = edge.kind === "produced" ? "produced" : STUDIO_EDGE_LABEL[edge.kind];
  return `${titleOf(edge.source)} ${said} ${titleOf(edge.target)}`;
}

function edgeLabel(edge: StudioWhiteboardEdge, titleOf: (id: string) => string): string {
  const proposed = edge.kind !== "produced" && edge.proposed ? `, ${EDGE_PROPOSED}` : "";
  return `${edgeSaid(edge, titleOf)}${proposed}`;
}

/**
 * Where a node a person adds now should land: the middle of what they are
 * looking at, in the board's own coordinates. `#1364`.
 *
 * **Only callable from inside the board**, since what it reads is the viewport
 * React Flow is holding — which is why the aside is a component of the
 * caller's rather than markup passed down. A node placed at the origin on a
 * board panned somewhere else is a node a person has to go and find.
 */
export function useStudioPlacement(): () => { x: number; y: number } {
  const flow = useReactFlow();
  return useCallback(() => {
    const board = document.querySelector(".armada-studio-whiteboard");
    const at = board?.getBoundingClientRect();
    if (board === null || at === undefined) return { x: 0, y: 0 };
    const middle = flow.screenToFlowPosition({ x: at.x + at.width / 2, y: at.y + at.height / 2 });
    // A node is placed by its top-left corner, so half a card back puts the
    // card in the middle rather than starting there. The width is read off the
    // token the card is drawn at rather than restated here.
    const wide = Number.parseFloat(getComputedStyle(board).getPropertyValue("--w-studio-node"));
    const half = Number.isFinite(wide) ? wide / 2 : 0;
    // And off whatever is already at that point: two added in a row would
    // otherwise land on one spot with the second hiding the first.
    return clearOf(
      flow.getNodes().map((node) => node.position),
      { x: middle.x - half, y: middle.y },
      Number.isFinite(wide) ? wide : 0,
    );
  }, [flow]);
}

/** A paste aimed at somewhere a person types, which is that field's and not the board's. */
function typing(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || target.closest("input, textarea") !== null);
}

/**
 * The board's ⌘V. **Inside the board**, for `useStudioPlacement`'s reason: where
 * a paste lands is read off the viewport React Flow holds.
 *
 * **The board is a focus target, and so takes focus from a press on it.** React
 * Flow's pane is none, so a press on empty board left focus on the body and ⌘V
 * never reached here. `-1` keeps it out of the Tab order.
 */
function BoardPaste({ onPaste }: { onPaste: NonNullable<StudioWhiteboardProps["onPaste"]> }) {
  const board = useStore((state) => state.domNode);
  const flow = useReactFlow();
  const middle = useStudioPlacement();
  const latest = useRef(onPaste);
  latest.current = onPaste;
  useEffect(() => {
    if (board === null) return;
    let pointer: { x: number; y: number } | null = null;
    if (!board.hasAttribute("tabindex")) board.tabIndex = -1;
    const moved = (event: PointerEvent) => void (pointer = { x: event.clientX, y: event.clientY });
    const left = () => void (pointer = null);
    const pasted = (event: ClipboardEvent) => {
      if (event.clipboardData === null || typing(event.target)) return;
      const at = pointer === null ? middle() : flow.screenToFlowPosition(pointer);
      if (latest.current(event.clipboardData, { x: Math.round(at.x), y: Math.round(at.y) })) event.preventDefault();
    };
    board.addEventListener("pointermove", moved);
    board.addEventListener("pointerleave", left);
    board.addEventListener("paste", pasted);
    return () => {
      board.removeEventListener("pointermove", moved);
      board.removeEventListener("pointerleave", left);
      board.removeEventListener("paste", pasted);
    };
  }, [board, flow, middle]);
  return null;
}

function Board({
  nodes: given,
  edges: givenEdges,
  onNodeMoved,
  onSelectionChange,
  pick = null,
  readOnly = false,
  onDecide,
  deciding = null,
  draft = null,
  onPanePress,
  placing = null,
  onPaste,
  children,
  rail,
  nodeBar,
}: StudioWhiteboardProps) {
  // Placement is the whiteboard's to hold between moves; the caller hears each
  // one through `onNodeMoved` and keeps it wherever positions are kept.
  // **`pick` is read at mount and never again**, which is why it is folded into
  // the initial state rather than applied by an effect: a later write here
  // lands in the middle of React Flow's own change for a click, and the first
  // node of a multiple pick came out as the whole of it. The board is keyed by
  // its Studio, so opening another one is a fresh mount and a fresh pick.
  const [kept, setKept] = useState<BoardNode[]>(() =>
    given.map((entry) => ({ ...toBoardNode(entry), selected: pick !== null && entry.id === pick })),
  );
  // **A frame is not picked by the press that places in it**: with a kind
  // armed, that press is the kind's, and a Zone picked under the new card
  // would hang its bar over it.
  const nodes = useMemo(() => {
    const drawn = merged(given, draft, kept);
    return placing === null ? drawn : drawn.map((node) => (node.type === "frame" ? { ...node, selectable: false } : node));
  }, [given, draft, kept, placing]);
  const showing = useRef(nodes);
  showing.current = nodes;

  const onNodesChange = useCallback(
    (changes: NodeChange<BoardNode>[]) => {
      // `dragging: false` is a node put down: a drag ending, or an arrow key.
      // **Where it lands is worked out against the board as it was drawn**, so
      // a frame is measured without the node leaving it.
      const landed = new Map<string, { position: { x: number; y: number }; within: string | null }>();
      for (const change of changes) {
        if (readOnly) break;
        if (change.type === "position" && change.dragging === false && change.position) {
          if (change.id === draft?.id) {
            draft.onMoved?.(change.position, landing(showing.current.map(asFraming), change.id, change.position));
            continue;
          }
          const where = landing(showing.current.map(asFraming), change.id, change.position);
          landed.set(change.id, where);
          onNodeMoved?.(change.id, where.position, where.within);
        }
      }
      setKept((current) =>
        applyNodeChanges(changes, merged(given, draft, current)).map((node) => {
          const where = landed.get(node.id);
          if (where === undefined) return node;
          const { parentId: _, ...placed } = node;
          return { ...placed, position: where.position, ...(where.within === null ? {} : { parentId: where.within }) };
        }),
      );
    },
    [given, draft, onNodeMoved, readOnly],
  );

  const edges = useMemo<BoardEdge[]>(() => {
    const titles = new Map(given.map(({ id, node }) => [id, `${STUDIO_NODE_KIND[node.kind]} ${node.title}`.trim()]));
    const titleOf = (id: string) => titles.get(id) ?? id;
    // Which sides an edge leaves and lands on is read off where both ends sit
    // on the board, not inside whatever frame each is in.
    const framing = new Map(nodes.map((node) => [node.id, asFraming(node)]));
    const placed = new Map(
      nodes.map((node) => [
        node.id,
        {
          position: onTheBoard(node.id, framing),
          measured: { width: node.width ?? node.measured?.width, height: node.height ?? node.measured?.height },
        },
      ]),
    );
    return givenEdges.map((edge) => ({
      id: edge.id,
      source: edge.source,
      target: edge.target,
      ...facingSides(placed.get(edge.source), placed.get(edge.target)),
      type: "studio",
      ariaLabel: edgeLabel(edge, titleOf),
      markerEnd: { type: MarkerType.ArrowClosed },
      data: {
        label: edge.kind === "produced" ? null : STUDIO_EDGE_LABEL[edge.kind],
        proposed: edge.kind !== "produced" && edge.proposed,
        proposal:
          edge.kind === "produced" || !edge.proposed
            ? null
            : { proposer: edge.proposer ?? PROPOSED, said: edgeSaid(edge, titleOf), readOnly, deciding, onDecide },
      },
    }));
  }, [given, givenEdges, nodes, readOnly, deciding, onDecide]);

  // Where a press puts the armed kind: the Zone under it, or the board.
  const pressed = (at: { x: number; y: number }) =>
    onPanePress?.(at, placing === null ? { within: null, position: at } : pressedIn(nodes.map(asFraming), placing, at));
  const onNodePress = (nodeId: string, at: { x: number; y: number }) => {
    // A press on a card picks it, as it always has. Only a frame's own ground places.
    if (nodes.find((node) => node.id === nodeId)?.type === "frame") pressed(at);
  };

  return (
    <GraphCanvas<BoardNode, BoardEdge>
      surface="armada-studio-whiteboard"
      label="Studio whiteboard"
      nodes={nodes}
      edges={edges}
      nodeTypes={NODE_TYPES}
      edgeTypes={EDGE_TYPES}
      onNodesChange={onNodesChange}
      onSelectionChange={onSelectionChange}
      nodesDraggable={!readOnly}
      multiSelectionKeyCode={JOINS_THE_SELECTION}
      rail={rail}
      {...(onPanePress === undefined ? {} : { onPanePress: pressed, onNodePress })}
      placing={placing !== null}
      aside={children}
    >
      {nodeBar}
      {onPaste === undefined || readOnly ? null : <BoardPaste onPaste={onPaste} />}
    </GraphCanvas>
  );
}

export function StudioWhiteboard(props: StudioWhiteboardProps) {
  return <Board {...props} />;
}
