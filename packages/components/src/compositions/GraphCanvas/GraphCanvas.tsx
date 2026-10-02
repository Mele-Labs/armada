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
import { useCallback, useMemo, useState, type ReactNode } from "react";

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
   * A press on empty canvas, at that point in the graph's own coordinates —
   * where a person puts down a kind they armed on the rail. **A press and not
   * a drag**: React Flow reports no click for a pan, so dragging the canvas
   * while a kind is armed still pans.
   */
  onPanePress?: (at: { x: number; y: number }) => void;
  /**
   * A press on a node, at that point in the graph's own coordinates — a Studio
   * puts an armed kind down inside the Zone that was pressed. A drag is no
   * press, as `onPanePress` says.
   */
  onNodePress?: (nodeId: string, at: { x: number; y: number }) => void;
  /** A kind is armed: the canvas draws a crosshair, which says the next press puts it down. */
  placing?: boolean;
  /**
   * A surface's own control over the top-right corner — a Studio's Run.
   * **Never the field behind a rail press**: that is put down on the canvas
   * itself (the owner, 1 Oct 2026).
   *
   * **It drew two things until 28 Sep 2026 and said which it was for neither**,
   * which is the whole of the owner's *why is this showing when I have nothing
   * selected*. A queue that waits on a person and a field that waits on the
   * press a person just made are not one panel. Since 29 Sep 2026 what waits on
   * a person is answered on the board, where it is drawn.
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

/** How many times a placement steps on before it gives up and stacks. */
const MOST_NUDGES = 24;

/**
 * Where a new node lands, given the point the person is looking at: that
 * point, or a step down and across from it while something already sits there.
 *
 * **Placing at the middle of the viewport and nothing else stacks.** Two nodes
 * added in a row landed on one spot and the second hid the first — unnoticed
 * while an empty canvas still had `fitView` armed, since that accidental fit
 * moved the viewport between the two presses. Taking it away uncovered this.
 */
export function clearOf(
  taken: readonly { x: number; y: number }[],
  at: { x: number; y: number },
  step: number,
): { x: number; y: number } {
  let { x, y } = at;
  // On top of, rather than beside: corners closer than a step apart in both
  // axes. The walk is bounded — a board that crowded wants a person moving a
  // card, not a canvas hunting for room forever.
  const under = () => taken.some((one) => Math.abs(one.x - x) < step && Math.abs(one.y - y) < step);
  for (let tried = 0; tried < MOST_NUDGES && under(); tried += 1) {
    x += step;
    y += step;
  }
  return { x: Math.round(x), y: Math.round(y) };
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

/** Nothing to draw. A surface hands `undefined` for a slot it has nothing in. */
const noPanel = (slot: ReactNode): boolean => slot === undefined || slot === null;

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
  onPanePress,
  onNodePress,
  placing = false,
  aside,
  children,
}: GraphCanvasProps<N, E>) {
  const flow = useReactFlow();
  // **A caller that applies no changes still gets its nodes' sizes kept.**
  // React Flow reads a node's size off the node it is handed and draws a node
  // without one hidden, until its resize observer measures it again — which it
  // asks for only when that node's own render sees it go from measured to not.
  // The Job's run is rebuilt on every render and took no `onNodesChange`, so
  // every render hid every card for a frame, and one built before the first
  // measurement and taken in after it (a guide card opening, a live Job's next
  // read) wiped that measurement before any card drew it. Nothing asked again,
  // and the owner's first real Job drew an empty canvas (1 Oct 2026) where the
  // mock, rendered once, drew every step. Keeping the sizes is the part of
  // `applyNodeChanges` such a caller needs; one that applies its own changes
  // hands `measured` back itself and is left alone.
  const [sizes, setSizes] = useState<ReadonlyMap<string, { width: number; height: number }>>(new Map());
  const keepSizes = useCallback<OnNodesChange<N>>((changes) => {
    setSizes((held) => {
      let next: Map<string, { width: number; height: number }> | undefined;
      for (const change of changes) {
        if (change.type !== "dimensions" || change.dimensions === undefined) continue;
        const was = held.get(change.id);
        if (was?.width === change.dimensions.width && was.height === change.dimensions.height) continue;
        next ??= new Map(held);
        next.set(change.id, change.dimensions);
      }
      return next ?? held;
    });
  }, []);
  const sized = useMemo(
    () =>
      onNodesChange !== undefined
        ? nodes
        : nodes.map((node) => {
            const measured = sizes.get(node.id);
            return node.measured !== undefined || measured === undefined ? node : { ...node, measured };
          }),
    [nodes, onNodesChange, sizes],
  );
  const onPaneClick = useCallback(
    (event: { clientX: number; clientY: number }) =>
      onPanePress?.(flow.screenToFlowPosition({ x: event.clientX, y: event.clientY })),
    [flow, onPanePress],
  );
  const onNodeClick = useCallback(
    (event: { clientX: number; clientY: number }, node: N) =>
      onNodePress?.(node.id, flow.screenToFlowPosition({ x: event.clientX, y: event.clientY })),
    [flow, onNodePress],
  );
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
      data-placing={placing || undefined}
      aria-label={label}
      colorMode="dark"
      nodes={sized}
      edges={edges}
      nodeTypes={nodeTypes}
      edgeTypes={edgeTypes}
      onNodesChange={onNodesChange ?? keepSizes}
      onSelectionChange={onPicked}
      {...(onPanePress === undefined ? {} : { onPaneClick })}
      {...(onNodePress === undefined ? {} : { onNodeClick })}
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
      {/* One corner, because the other one is Helm's dock. */}
      {noPanel(aside) ? null : (
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
