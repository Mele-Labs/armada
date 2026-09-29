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
import { useCallback, useMemo, useState, type ReactNode } from "react";

import { GRAPH_CANVAS_SIDES, GraphCanvas, clearOf, facingSides } from "../GraphCanvas/GraphCanvas";
import { STUDIO_NODE_KIND, StudioNode, studioNodeLabel, type StudioNodeOf } from "../StudioNode/StudioNode";

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
 * never says which kind an edge is. A relation nobody has answered is dashed
 * *and* says so under its label, in `awaiting_review`'s amber — the one thing
 * on the board waiting on a person, and a dash alone did not say it was
 * waiting or that anything had proposed it. Nothing here draws or deletes a
 * relation — that is a person's act.
 */

export type StudioWhiteboardNode = {
  id: string;
  /** Where the node sits, in the whiteboard's own coordinates. */
  position: { x: number; y: number };
  node: StudioNodeOf & { title: string; facts?: readonly string[] };
};

export type StudioEdgeRelation = "same_as" | "blocks" | "answers";

export type StudioWhiteboardEdge = {
  id: string;
  source: string;
  target: string;
} & (
  | { kind: "produced" }
  /** Proposed by Helm or a scout, and not yet accepted by a person. */
  | { kind: StudioEdgeRelation; proposed: boolean }
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
   * The field behind whatever the rail opened, over the board's top-right
   * corner. **This and nothing else** — it drew the relations waiting on a
   * person too, and a panel doing two jobs with neither of them named is what
   * the owner read on 28 Sep 2026 as a card showing when nothing was selected.
   * What waits on a person is `waiting`.
   */
  children?: ReactNode;
  /**
   * What is waiting on a person, under the field and always under it. It comes
   * and goes with what the Studio holds rather than with anything the person
   * is doing, so it says so on its own face.
   */
  waiting?: ReactNode;
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

/**
 * What a proposed relation says under its own label, on the board. Sentence
 * case and lower than the label, because it is the standing rather than the
 * relation — nothing here changes what a kind means.
 */
const EDGE_PROPOSED = "proposed, waiting on you";

/** The relation's label, as `studio.md`, Edges, names it. Produced has none. */
export const STUDIO_EDGE_LABEL: Readonly<Record<StudioEdgeRelation, string>> = {
  same_as: "same as",
  blocks: "blocks",
  answers: "answers",
};

type BoardNodeData = StudioWhiteboardNode["node"];
type BoardNode = Node<BoardNodeData, "studio">;
type BoardEdgeData = { label: string | null; proposed: boolean };
type BoardEdge = Edge<BoardEdgeData, "studio">;

/**
 * The card, with an anchor for each end of an edge on every side. Which side
 * an edge takes is chosen from where the two nodes sit — `facingSides`.
 * Nothing connects by hand, so none is drawn.
 */
function BoardNodeView({ data, selected }: NodeProps<BoardNode>) {
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

function BoardEdgeView(props: EdgeProps<BoardEdge>) {
  const [path, labelX, labelY] = getBezierPath(props);
  const label = props.data?.label ?? null;
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
          <span
            className="armada-studio-edge__label"
            style={{ transform: `translate(-50%, -50%) translate(${labelX}px, ${labelY}px)` }}
          >
            {label}
            {/* **A dashed line is not a sentence.** It was the whole of what
                said a relation had been proposed, and the owner read the label
                beside it as a fact the Studio had decided — *where did same as
                come from*. The second line names the two things the dash could
                not: that somebody proposed it, and that it is his to answer. */}
            {props.data?.proposed !== true ? null : (
              <span className="armada-studio-edge__waiting">{EDGE_PROPOSED}</span>
            )}
          </span>
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

const NODE_TYPES = { studio: BoardNodeView };
const EDGE_TYPES = { studio: BoardEdgeView };

function toBoardNode({ id, position, node }: StudioWhiteboardNode): BoardNode {
  return { id, position, type: "studio", data: node, ariaLabel: studioNodeLabel(node) };
}

/**
 * What the caller gave, over what React Flow keeps per node — where it was put,
 * its measured size, whether it is selected. A node's kind, title and state
 * always come from the caller, so a Finding that freezes redraws in place.
 */
function merged(given: readonly StudioWhiteboardNode[], kept: readonly BoardNode[]): BoardNode[] {
  const byId = new Map(kept.map((node) => [node.id, node]));
  return given.map((entry) => {
    const fresh = toBoardNode(entry);
    const held = byId.get(entry.id);
    return held === undefined ? fresh : { ...held, data: fresh.data, ariaLabel: fresh.ariaLabel };
  });
}

function edgeLabel(edge: StudioWhiteboardEdge, titleOf: (id: string) => string): string {
  const said = edge.kind === "produced" ? "produced" : STUDIO_EDGE_LABEL[edge.kind];
  const proposed = edge.kind !== "produced" && edge.proposed ? `, ${EDGE_PROPOSED}` : "";
  return `${titleOf(edge.source)} ${said} ${titleOf(edge.target)}${proposed}`;
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
  children,
  waiting,
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
  const nodes = useMemo(() => merged(given, kept), [given, kept]);

  const onNodesChange = useCallback(
    (changes: NodeChange<BoardNode>[]) => {
      setKept((current) => applyNodeChanges(changes, merged(given, current)));
      // `dragging: false` is a node put down: a drag ending, or an arrow key.
      for (const change of changes) {
        if (readOnly) break;
        if (change.type === "position" && change.dragging === false && change.position) {
          onNodeMoved?.(change.id, change.position);
        }
      }
    },
    [given, onNodeMoved, readOnly],
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
      },
    }));
  }, [given, givenEdges, nodes]);

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
      waiting={waiting}
    >
      {nodeBar}
    </GraphCanvas>
  );
}

export function StudioWhiteboard(props: StudioWhiteboardProps) {
  return <Board {...props} />;
}
