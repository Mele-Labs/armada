import {
  BaseEdge,
  EdgeLabelRenderer,
  Handle,
  MarkerType,
  applyNodeChanges,
  getBezierPath,
  useReactFlow,
  type Edge,
  type EdgeProps,
  type Node,
  type NodeChange,
  type NodeProps,
} from "@xyflow/react";
import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";

import { Button } from "../../primitives/Button/Button";
import { Card } from "../../primitives/Card/Card";

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
 * and its label is a small card: who proposed it, in `awaiting_review`'s
 * amber, the relation, and Accept and Reject under it. **The answer is given
 * where the relation lands** — the owner, 29 Sep 2026: a queue in the corner
 * was disconnected from the edge it was about. Nothing here draws or deletes a
 * relation on its own; accepting one is the caller's `onDecide`.
 */

export type StudioWhiteboardNode = {
  id: string;
  /** Where the node sits, in the whiteboard's own coordinates. */
  position: { x: number; y: number };
  node: StudioNodeOf & { title: string; facts?: readonly string[] };
};

/**
 * A node still being written, drawn where it will land — the owner's note of
 * 1 Oct 2026. **Bridge's and nobody else's** until the caller sends it: it is
 * never reported as moved or selected, and the field is the caller's.
 */
export type StudioWhiteboardDraft = {
  /** A fresh id per draft, so a second draft is a fresh field rather than the first one re-kinded. */
  id: string;
  kind: StudioNodeKind;
  position: { x: number; y: number };
  /** The field. Focused once the board has drawn the card. */
  field: ReactNode;
  pending?: boolean;
};

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
  /** A node was put down somewhere new, by pointer or by arrow key. */
  onNodeMoved?: (id: string, position: { x: number; y: number }) => void;
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
  /** The node a rail press put down, with its field in it. Absent is none. */
  draft?: StudioWhiteboardDraft | null;
  /**
   * What sits over the board's top-right corner — a Studio's Run control.
   * **Never the field behind a rail press**: that is the draft, on the board
   * where the node will land (the owner, 1 Oct 2026). Nor the relations
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

type BoardNodeData = StudioWhiteboardNode["node"];
type DraftNodeData = { kind: StudioNodeKind; field: ReactNode; pending: boolean };
type BoardNode = Node<BoardNodeData, "studio"> | Node<DraftNodeData, "draft">;
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
  return (
    <>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <StudioNode {...data} selected={selected} />
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
 */
function DraftNodeView({ data, width }: NodeProps<Node<DraftNodeData, "draft">>) {
  const at = useRef<HTMLDivElement>(null);
  const shown = (width ?? 0) > 0;
  useEffect(() => {
    if (shown) at.current?.querySelector<HTMLElement>("textarea, input")?.focus();
  }, [shown]);
  return (
    <div ref={at}>
      <StudioNodeDraft kind={data.kind} pending={data.pending}>
        {data.field}
      </StudioNodeDraft>
    </div>
  );
}

/**
 * A proposed relation's label: who proposed it, the relation, and the answer.
 * **Its buttons are named by the whole sentence**, `Accept: Note A same as
 * Note B`, since a board can hold several proposals and a bare Accept names
 * none of them. `nodrag nopan` are React Flow's own: a press here is a press,
 * not the start of a pan.
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
  return (
    <Card
      className="armada-studio-edge__proposal nodrag nopan"
      style={at}
      role="group"
      aria-label={`${proposer}: ${said}`}
    >
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

const NODE_TYPES = { studio: BoardNodeView, draft: DraftNodeView };
const EDGE_TYPES = { studio: BoardEdgeView };

function toBoardNode({ id, position, node }: StudioWhiteboardNode): BoardNode {
  return { id, position, type: "studio", data: node, ariaLabel: studioNodeLabel(node) };
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
    draggable: false,
    selectable: false,
    focusable: false,
  };
}

/**
 * What the caller gave, over what React Flow keeps per node — where it was put,
 * its measured size, whether it is selected. A node's kind, title and state
 * always come from the caller, so a Finding that freezes redraws in place.
 */
function merged(
  given: readonly StudioWhiteboardNode[],
  draft: StudioWhiteboardDraft | null,
  kept: readonly BoardNode[],
): BoardNode[] {
  const byId = new Map(kept.map((node) => [node.id, node]));
  const fresh = [...given.map(toBoardNode), ...(draft === null ? [] : [toDraftNode(draft)])];
  return fresh.map((node) => {
    const held = byId.get(node.id);
    return held === undefined ? node : ({ ...held, data: node.data, ariaLabel: node.ariaLabel } as BoardNode);
  });
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
  const nodes = useMemo(() => merged(given, draft, kept), [given, draft, kept]);

  const onNodesChange = useCallback(
    (changes: NodeChange<BoardNode>[]) => {
      setKept((current) => applyNodeChanges(changes, merged(given, draft, current)));
      // `dragging: false` is a node put down: a drag ending, or an arrow key.
      for (const change of changes) {
        if (readOnly) break;
        if (change.type === "position" && change.dragging === false && change.position) {
          if (change.id !== draft?.id) onNodeMoved?.(change.id, change.position);
        }
      }
    },
    [given, draft, onNodeMoved, readOnly],
  );

  const edges = useMemo<BoardEdge[]>(() => {
    const titles = new Map(given.map(({ id, node }) => [id, `${STUDIO_NODE_KIND[node.kind]} ${node.title}`]));
    const titleOf = (id: string) => titles.get(id) ?? id;
    const placed = new Map(nodes.map((node) => [node.id, node]));
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
      aside={children}
    >
      {nodeBar}
    </GraphCanvas>
  );
}

export function StudioWhiteboard(props: StudioWhiteboardProps) {
  return <Board {...props} />;
}
