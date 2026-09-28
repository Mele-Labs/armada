import {
  Panel,
  Position,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Edge,
  type EdgeTypes,
  type FitViewOptions,
  type Node,
  type NodeTypes,
  type OnNodesChange,
} from "@xyflow/react";
import { useCallback, useState, type ReactNode } from "react";

import { Maximize } from "lucide-react";

import { GraphCanvasRailGroup, type GraphCanvasRailAct } from "./GraphCanvasRail";

/**
 * The graph surface itself — pan, zoom, fit, selection and the controls that
 * drive them. `docs/contracts/design-system.md`, Hard rules, names React Flow
 * the one sanctioned graph surface and the two things it draws.
 *
 * **Extracted from `StudioWhiteboard`, never copied out of it** (`#1539`).
 * **Placement is the caller's, always**: a Studio remembers where a person put
 * a node, a Job's workflow computes placement from step order, and this holds
 * no positions and reports no moves.
 *
 * **Nothing inside is React Flow's own.** Every node and edge is the caller's
 * type, the controls are `Button`, and `GraphCanvas.css` sets every variable
 * `base.css` would paint with to a token.
 */

export type GraphCanvasProps<N extends Node, E extends Edge> = {
  /**
   * The surface's own class, so a surface can paint what only it has — a
   * Studio's edge labels, a workflow's returning loop. `armada-graph-canvas`
   * is always on the element beside it.
   */
  surface: string;
  /** What the graph is, read to somebody who cannot see it. */
  label: string;
  nodes: N[];
  edges: E[];
  /** Stable module constants in the caller. A fresh object per render is a React Flow update loop. */
  nodeTypes: NodeTypes;
  edgeTypes: EdgeTypes;
  /** Absent means nothing in this graph moves, which is the workflow canvas. */
  onNodesChange?: OnNodesChange<N>;
  nodesDraggable?: boolean;
  /** Held down, a second node joins the selection rather than replacing it. */
  multiSelectionKeyCode?: string[];
  onSelectionChange?: (ids: readonly string[]) => void;
  fitView?: boolean;
  fitViewOptions?: FitViewOptions;
  /**
   * How far out a person — or a fit — may zoom. **React Flow's own floor is
   * 0.5**, which silently clamps `fitView`: a run wider than twice its frame
   * is fitted to a scale it cannot reach and drawn clipped at both ends.
   */
  minZoom?: number;
  /**
   * The rail's own groups, above the view group this draws — what a person puts
   * on the canvas. `GraphCanvasRailGroup` is what goes here, one per group, and
   * `docs/contracts/iconography.md`, *The canvas rail*, is the rule.
   *
   * **Drawn inside the graph, like `children`**, so a rail act may read the
   * viewport it places something into. Absent is a canvas with nothing to
   * place, which still carries the view group.
   */
  rail?: ReactNode;
  /**
   * A group under the view group, for a control only one surface has — the
   * workflow canvas's *Stay on the running step*. It is neither a tool nor a
   * zoom, and the rail lets a surface say so rather than making every surface
   * carry the same rows.
   */
  railBelow?: ReactNode;
  /**
   * What the surface draws over the top-right corner — the relations waiting on
   * a person, and whatever a selection cannot hover over itself. The canvas
   * decides nothing.
   */
  aside?: ReactNode;
  /**
   * Mounted inside the graph, where React Flow's own hooks resolve. For a
   * surface that has to read or write the viewport — the workflow canvas stays
   * on the running step this way.
   */
  children?: ReactNode;
};

/** Enough of a placed node to find its centre. Both surfaces hang edges this way. */
type Placed = {
  position: { x: number; y: number };
  measured?: { width?: number; height?: number };
};

/**
 * The facing sides of two nodes: across when they are further apart
 * horizontally than vertically, otherwise up or down. A fixed right-to-left
 * pair looped every edge whose target sat left of or above its source.
 */
export function facingSides(source: Placed | undefined, target: Placed | undefined) {
  const centre = (node: Placed | undefined) => ({
    x: (node?.position.x ?? 0) + (node?.measured?.width ?? 0) / 2,
    y: (node?.position.y ?? 0) + (node?.measured?.height ?? 0) / 2,
  });
  const from = centre(source);
  const to = centre(target);
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const [out, into] =
    Math.abs(dx) >= Math.abs(dy)
      ? dx >= 0 ? [Position.Right, Position.Left] : [Position.Left, Position.Right]
      : dy >= 0 ? [Position.Bottom, Position.Top] : [Position.Top, Position.Bottom];
  return { sourceHandle: `s-${out}`, targetHandle: `t-${into}` };
}

/** The four sides a node hangs an edge from. Nothing connects by hand, so none is drawn. */
export const GRAPH_CANVAS_SIDES = [Position.Left, Position.Right, Position.Top, Position.Bottom] as const;

/** React Flow's default descriptions offer delete, which neither surface ever does. */
const ARIA = {
  "node.a11yDescription.default": "Press Enter or Space to select a node, then the arrow keys to move it.",
  "node.a11yDescription.keyboardDisabled": "Press Enter or Space to select a node.",
  "edge.a11yDescription.default": "Press Enter or Space to select an edge.",
  "node.a11yDescription.ariaLiveMessage": ({ direction }: { direction: string }) => `Moved the node ${direction}.`,
};

/** `−` and `+`, the signs the boards draw. Not glyphs: `iconography.md` defaults to none. */
const SIGN = { in: "+", out: "−" } as const;

/** What the view group is, read to somebody who cannot see it. */
const VIEW_LABEL = "How you are looking at this";

/**
 * How a person looks at the canvas: out, in, and the whole of it. **Every
 * canvas has these and the rail draws them for all four**, which is why they
 * are here rather than handed in — the owner's note of 28 Sep 2026 took them
 * out of the bottom-right corner and put them under the tools.
 */
function ViewGroup() {
  const flow = useReactFlow();
  const acts: GraphCanvasRailAct[] = [
    { id: "out", name: "Zoom out", sign: SIGN.out, onPress: () => void flow.zoomOut() },
    { id: "in", name: "Zoom in", sign: SIGN.in, onPress: () => void flow.zoomIn() },
    { id: "fit", name: "Fit", icon: Maximize, onPress: () => void flow.fitView() },
  ];
  return <GraphCanvasRailGroup label={VIEW_LABEL} acts={acts} />;
}

function Surface<N extends Node, E extends Edge>({
  surface,
  label,
  nodes,
  edges,
  nodeTypes,
  edgeTypes,
  onNodesChange,
  nodesDraggable = false,
  multiSelectionKeyCode,
  onSelectionChange,
  fitView = true,
  fitViewOptions,
  minZoom,
  aside,
  children,
}: GraphCanvasProps<N, E>) {
  const onPicked = useCallback(
    ({ nodes: picked }: { nodes: N[] }) => onSelectionChange?.(picked.map((node) => node.id)),
    [onSelectionChange],
  );
  // **A canvas that opens empty asks for no fit** — the owner, 28 Sep 2026, on
  // the sketch pad. `fitView` is a queue React Flow arms and only disarms once
  // every node is measured, and `adoptUserNodes` reports an empty graph as not
  // initialized — so a blank pad kept the fit armed and the first box a person
  // added satisfied it, alone, at `maxZoom`. Measured: scale 1 to scale 2.
  //
  // Read once, because the prop is watched: turning it on when the first node
  // arrives would queue exactly the fit this refuses.
  const [fitsOnOpen] = useState(() => fitView && nodes.length > 0);

  return (
    <ReactFlow<N, E>
      className={`armada-graph-canvas ${surface}`}
      aria-label={label}
      colorMode="dark"
      nodes={nodes}
      edges={edges}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      onNodesChange={onNodesChange}
      onSelectionChange={onPicked}
      nodesConnectable={false}
      nodesDraggable={nodesDraggable}
      multiSelectionKeyCode={multiSelectionKeyCode}
      edgesReconnectable={false}
      deleteKeyCode={null}
      ariaLabelConfig={ARIA}
      fitView={fitsOnOpen}
      fitViewOptions={fitViewOptions}
      {...(minZoom === undefined ? {} : { minZoom })}
      // The attribution is a link out of the app, and no surface may navigate.
      proOptions={{ hideAttribution: true }}
    >
      {aside === undefined || aside === null ? null : (
        <Panel position="top-right" className="armada-graph-canvas__aside">
          {aside}
        </Panel>
      )}
      {children}
    </ReactFlow>
  );
}

/**
 * The graph, with the rail beside it.
 *
 * **The rail is a column of the frame and not a layer over the pane.** Drawn as
 * a React Flow `Panel` it covered whatever the fit put under it — the workflow
 * run's first card, measured 28 Sep 2026 — and every surface would have owed a
 * fit padding nobody could compute from a token. A column cannot overlap.
 *
 * It is still inside the provider, so a rail act reads the viewport it places
 * something into.
 */
export function GraphCanvas<N extends Node, E extends Edge>(props: GraphCanvasProps<N, E>) {
  const { rail, railBelow } = props;
  return (
    <div className="armada-graph-canvas-frame">
      <ReactFlowProvider>
        <div className="armada-graph-rail">
          {rail}
          <ViewGroup />
          {railBelow}
        </div>
        <div className="armada-graph-canvas-frame__pane">
          <Surface {...props} />
        </div>
      </ReactFlowProvider>
    </div>
  );
}
