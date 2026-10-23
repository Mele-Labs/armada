import {
  BaseEdge,
  Handle,
  Position,
  getBezierPath,
  useReactFlow,
  useStore,
  type Edge,
  type EdgeProps,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
} from "react";
import { createPortal } from "react-dom";
import type { LucideIcon } from "lucide-react";

import { GRAPH_CANVAS_SIDES, GraphCanvas } from "../GraphCanvas/GraphCanvas";

/**
 * A Job's Artifacts as a graph of how it grew: each thing hangs off what produced it.
 * The caller works out what and where (`session-view/artifact-graph.ts`); this draws it,
 * on `GraphCanvas` for the pan, zoom and fit, and owns the keys.
 *
 * **One node to an artifact, its versions a row of marks inside it.** A mark is a button of
 * its own, so a node is a button and a row beside it, never a button in a button.
 *
 * **Tooltips are drawn outside the canvas.** A bubble inside React Flow's viewport is scaled
 * with the zoom and cut to the node it hangs off; these are measured against the window and
 * wrap at a width that holds a whole title.
 */

export type ArtifactGraphMark = {
  key: string;
  Glyph: LucideIcon;
  tone?: "passed" | "failed" | "pending" | "done";
  /** The mark's accessible name. */
  name: string;
  /** What its tooltip reads, one line each. */
  tip: readonly string[];
  open?: () => void;
};

export type ArtifactGraphNode = {
  id: string;
  /** Top-left, in the canvas's own coordinates. */
  x: number;
  y: number;
  width: number;
  height: number;
  kind: "message" | "plan" | "group" | "artifact";
  Glyph: LucideIcon;
  /** The node's accessible name. */
  name: string;
  title: string;
  /** Mono and quiet, after the title, only where it helps. */
  sub?: string;
  tip: readonly string[];
  /** Not what the search matched. */
  dim: boolean;
  marks: readonly ArtifactGraphMark[];
  /** Absent for a node with nowhere to go. */
  open?: () => void;
};

export type ArtifactGraphEdge = {
  id: string;
  source: string;
  target: string;
  /** `again` is a later version that followed something other than what the first did: drawn thin. */
  kind: "from" | "join" | "again";
  /** What the line says, for its tooltip and to somebody who cannot see it. */
  name: string;
};

type Tip = { lines: readonly string[]; at: { left: number; top: number; bottom: number } };

type Held = {
  current: string | undefined;
  setCurrent: (id: string) => void;
  node: (id: string, element: HTMLButtonElement | null) => void;
  mark: (key: string, element: HTMLButtonElement | null) => void;
  tip: (lines: readonly string[] | undefined, at?: Tip["at"], soon?: boolean) => void;
};

const Hold = createContext<Held | undefined>(undefined);

type NodeData = { node: ArtifactGraphNode };
type EdgeData = { edge: ArtifactGraphEdge };
type CanvasNode = Node<NodeData, "artifact">;
type CanvasEdge = Edge<EdgeData, "artifact">;

/** How long a hover waits before its tooltip, as the primitive does. */
const TIP_DELAY = 400;

const rectOf = (element: Element): Tip["at"] => {
  const { left, top, bottom } = element.getBoundingClientRect();
  return { left, top, bottom };
};

function NodeView({ data }: NodeProps<CanvasNode>) {
  const hold = useContext(Hold)!;
  const { node } = data;
  const { Glyph } = node;
  const mine = hold.current === node.id;
  return (
    <div className="armada-artifact-node" data-kind={node.kind} data-dim={node.dim || undefined}>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <button
        type="button"
        ref={(element) => hold.node(node.id, element)}
        className="armada-artifact-node__main nodrag nopan"
        data-node={node.id}
        {...(mine ? { "data-layer-row": "" } : {})}
        tabIndex={mine ? 0 : -1}
        aria-label={node.name}
        aria-disabled={node.open === undefined || undefined}
        onFocus={(event) => {
          hold.setCurrent(node.id);
          if (event.currentTarget.matches(":focus-visible")) hold.tip(node.tip, rectOf(event.currentTarget), true);
        }}
        onBlur={() => hold.tip(undefined)}
        onMouseEnter={(event) => hold.tip(node.tip, rectOf(event.currentTarget))}
        onMouseLeave={() => hold.tip(undefined)}
        onClick={node.open}
      >
        <Glyph className="armada-artifact-node__glyph" size={16} strokeWidth={2} aria-hidden="true" />
        <span className="armada-artifact-node__title">{node.title}</span>
        {node.sub === undefined ? null : <code className="armada-artifact-node__sub">{node.sub}</code>}
      </button>
      {node.marks.length === 0 ? null : (
        <div className="armada-artifact-node__marks" role="group" aria-label={`Versions of ${node.title}`}>
          {node.marks.map((mark) => {
            const MarkGlyph = mark.Glyph;
            return (
              <button
                key={mark.key}
                type="button"
                ref={(element) => hold.mark(mark.key, element)}
                className="armada-artifact-mark nodrag nopan"
                data-mark={mark.key}
                data-of={node.id}
                data-tone={mark.tone}
                tabIndex={-1}
                aria-label={mark.name}
                aria-disabled={mark.open === undefined || undefined}
                onFocus={(event) => {
                  hold.setCurrent(node.id);
                  if (event.currentTarget.matches(":focus-visible")) hold.tip(mark.tip, rectOf(event.currentTarget), true);
                }}
                onBlur={() => hold.tip(undefined)}
                onMouseEnter={(event) => hold.tip(mark.tip, rectOf(event.currentTarget))}
                onMouseLeave={() => hold.tip(undefined)}
                onClick={mark.open}
              >
                <MarkGlyph size={12} strokeWidth={2} aria-hidden="true" />
              </button>
            );
          })}
        </div>
      )}
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`s-${side}`} id={`s-${side}`} type="source" position={side} isConnectable={false} />
      ))}
    </div>
  );
}

function EdgeView(props: EdgeProps<CanvasEdge>) {
  const hold = useContext(Hold)!;
  const edge = props.data!.edge;
  const [path] = getBezierPath(props);
  return (
    <>
      <BaseEdge id={props.id} path={path} className={`armada-artifact-edge armada-artifact-edge--${edge.kind}`} interactionWidth={0} />
      {/* The hit area is its own wide path: the line itself is too thin to hover. */}
      <path
        d={path}
        className="armada-artifact-edge__hit"
        fill="none"
        onMouseEnter={(event) => hold.tip([edge.name], { left: event.clientX, top: event.clientY, bottom: event.clientY })}
        onMouseLeave={() => hold.tip(undefined)}
      />
    </>
  );
}

const NODE_TYPES = { artifact: NodeView };
const EDGE_TYPES = { artifact: EdgeView };

/** The tooltip, in the document's body and measured there. */
function TipView({ tip }: { tip: Tip }) {
  const bubble = useRef<HTMLSpanElement>(null);
  const [placed, setPlaced] = useState<{ left: number; top: number } | undefined>();
  useLayoutEffect(() => {
    const held = bubble.current;
    if (held === null) return;
    const { width, height } = held.getBoundingClientRect();
    const gap = 6;
    const room = 8;
    const left = Math.max(room, Math.min(tip.at.left, window.innerWidth - width - room));
    const below = tip.at.bottom + gap;
    const top = below + height + room <= window.innerHeight ? below : Math.max(room, tip.at.top - height - gap);
    setPlaced({ left, top });
  }, [tip]);
  return createPortal(
    <span ref={bubble} className="armada-artifact-tip" aria-hidden="true" style={{ left: placed?.left ?? 0, top: placed?.top ?? 0, visibility: placed === undefined ? "hidden" : "visible" }}>
      {tip.lines.map((line, index) => (
        <span key={index} className="armada-artifact-tip__line" data-first={index === 0 || undefined}>
          {line}
        </span>
      ))}
    </span>,
    document.body,
  );
}

type Arrow = "ArrowLeft" | "ArrowRight" | "ArrowUp" | "ArrowDown";
const isArrow = (key: string): key is Arrow => key === "ArrowLeft" || key === "ArrowRight" || key === "ArrowUp" || key === "ArrowDown";

/**
 * The node an arrow reaches: the nearest whose centre lies that way, a node off to the side costing twice what one straight on does.
 * Edges are not followed, so a node joined to nothing near it is still reached.
 */
function nearest(nodes: readonly ArtifactGraphNode[], from: string, arrow: Arrow): string | undefined {
  const here = nodes.find((one) => one.id === from);
  if (here === undefined) return undefined;
  const cx = here.x + here.width / 2;
  const cy = here.y + here.height / 2;
  let best: { id: string; cost: number } | undefined;
  for (const one of nodes) {
    if (one.id === from) continue;
    const dx = one.x + one.width / 2 - cx;
    const dy = one.y + one.height / 2 - cy;
    const along = arrow === "ArrowRight" ? dx : arrow === "ArrowLeft" ? -dx : arrow === "ArrowDown" ? dy : -dy;
    const across = arrow === "ArrowRight" || arrow === "ArrowLeft" ? Math.abs(dy) : Math.abs(dx);
    // Straight ahead only: a node level with this one is not "down".
    if (along <= 0 || across > along * 2) continue;
    const cost = along + across * 2;
    if (best === undefined || cost < best.cost) best = { id: one.id, cost };
  }
  return best?.id;
}

/** Brings the node focus reached into the frame, at the zoom the person has. Nothing is current until something is focused, so opening pans nowhere. */
function Follows({ node, nodes }: { node: string | undefined; nodes: readonly ArtifactGraphNode[] }) {
  const flow = useReactFlow();
  const width = useStore((state) => state.width);
  const height = useStore((state) => state.height);
  useEffect(() => {
    const one = nodes.find((it) => it.id === node);
    if (one === undefined || width === 0) return;
    const { x, y, zoom } = flow.getViewport();
    const inset = 24;
    const left = one.x * zoom + x;
    const top = one.y * zoom + y;
    const right = (one.x + one.width) * zoom + x;
    const bottom = (one.y + one.height) * zoom + y;
    if (left >= inset && top >= inset && right <= width - inset && bottom <= height - inset) return;
    void flow.setCenter(one.x + one.width / 2, one.y + one.height / 2, { zoom, duration: 150 });
    // Only the node changing pans: a resize or a new search leaves the view where the person put it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [flow, node]);
  return null;
}

export function ArtifactGraph({ nodes: given, edges: givenEdges, label }: { nodes: readonly ArtifactGraphNode[]; edges: readonly ArtifactGraphEdge[]; /** What the graph is, read to somebody who cannot see it. */ label: string }) {
  const [current, setCurrent] = useState<string | undefined>();
  const [tip, setTip] = useState<Tip | undefined>();
  const timer = useRef<number | undefined>(undefined);
  const nodeButtons = useRef(new Map<string, HTMLButtonElement>());
  const markButtons = useRef(new Map<string, HTMLButtonElement>());
  useEffect(() => () => window.clearTimeout(timer.current), []);
  // Where Tab lands before anything has been: the node at the start, top of the first column.
  const first = [...given].sort((a, b) => a.x - b.x || a.y - b.y)[0]?.id;
  const here = current !== undefined && given.some((one) => one.id === current) ? current : first;

  const hold = useMemo<Held>(
    () => ({
      current: here,
      setCurrent,
      node: (id, element) => void (element === null ? nodeButtons.current.delete(id) : nodeButtons.current.set(id, element)),
      mark: (key, element) => void (element === null ? markButtons.current.delete(key) : markButtons.current.set(key, element)),
      tip: (lines, at, soon = false) => {
        window.clearTimeout(timer.current);
        if (lines === undefined || at === undefined) return setTip(undefined);
        if (soon) return setTip({ lines, at });
        timer.current = window.setTimeout(() => setTip({ lines, at }), TIP_DELAY);
      },
    }),
    [here],
  );

  const nodes = useMemo<CanvasNode[]>(
    () =>
      given.map((one) => ({
        id: one.id,
        type: "artifact" as const,
        position: { x: one.x, y: one.y },
        width: one.width,
        height: one.height,
        draggable: false,
        // `selectable` is what gives the node pointer events at all; nothing is drawn for a selected node.
        selectable: true,
        focusable: false,
        data: { node: one },
      })),
    [given],
  );

  const edges = useMemo<CanvasEdge[]>(() => {
    const at = new Map(given.map((one) => [one.id, one] as const));
    return givenEdges.flatMap((edge): CanvasEdge[] => {
      const from = at.get(edge.source);
      const to = at.get(edge.target);
      if (from === undefined || to === undefined) return [];
      // Across when the target lies to the right; a line that has to come back round leaves and enters underneath.
      const forward = to.x >= from.x + from.width;
      return [
        {
          id: edge.id,
          source: edge.source,
          target: edge.target,
          type: "artifact" as const,
          sourceHandle: forward ? `s-${Position.Right}` : `s-${Position.Bottom}`,
          targetHandle: forward ? `t-${Position.Left}` : `t-${Position.Bottom}`,
          ariaLabel: edge.name,
          focusable: false,
          data: { edge },
        },
      ];
    });
  }, [given, givenEdges]);

  const focusNode = useCallback((id: string) => nodeButtons.current.get(id)?.focus(), []);

  const keys = (event: KeyboardEvent<HTMLElement>) => {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    const target = event.target as HTMLElement;
    const mark = target.closest<HTMLElement>("[data-mark]");
    const node = target.closest<HTMLElement>("[data-node]")?.dataset.node ?? mark?.dataset.of;
    if (node === undefined) return;
    const own = given.find((one) => one.id === node);
    const stepped = (to: number) => {
      const list = own?.marks ?? [];
      const at = mark === null ? -1 : list.findIndex((one) => one.key === mark.dataset.mark);
      const next = list[Math.min(list.length - 1, Math.max(0, at + to))];
      if (next === undefined) return;
      event.preventDefault();
      markButtons.current.get(next.key)?.focus();
    };
    // From the node, `.` enters its marks at the first and `,` at the last; inside them they step.
    if (event.key === ".") return stepped(1);
    if (event.key === ",") return stepped(mark === null ? Infinity : -1);
    if (event.key === "Escape" && mark !== null) {
      event.preventDefault();
      focusNode(node);
      return;
    }
    if (!isArrow(event.key)) return;
    // Along a row of marks, left and right step it.
    if (mark !== null && (event.key === "ArrowLeft" || event.key === "ArrowRight")) return stepped(event.key === "ArrowRight" ? 1 : -1);
    const to = nearest(given, node, event.key);
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") event.preventDefault();
    if (to === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    focusNode(to);
  };

  return (
    <Hold.Provider value={hold}>
      <div className="armada-artifact-graph" data-own-arrows="" role="group" aria-label={label} onKeyDown={keys}>
        <GraphCanvas<CanvasNode, CanvasEdge>
          surface="armada-artifact-canvas"
          label={label}
          nodes={nodes}
          edges={edges}
          nodeTypes={NODE_TYPES}
          edgeTypes={EDGE_TYPES}
          minZoom={0.2}
          fitViewOptions={FIT}
        >
          <Follows node={current} nodes={given} />
        </GraphCanvas>
      </div>
      {tip === undefined ? null : <TipView tip={tip} />}
    </Hold.Provider>
  );
}

const FIT = { maxZoom: 1, padding: 0.08 };
