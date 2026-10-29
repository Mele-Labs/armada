import {
  Handle,
  Position,
  getBezierPath,
  useReactFlow,
  useStore,
  EdgeLabelRenderer,
  type Edge,
  type EdgeProps,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import { Ban, Check, CircleDot, Clock, GitCompare, ListPlus, MessageCircleQuestion, UserCheck, type LucideIcon } from "lucide-react";
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
} from "react";
import { createPortal } from "react-dom";

import { GRAPH_CANVAS_SIDES, GraphCanvas } from "../GraphCanvas/GraphCanvas";
import { GEO, HOME, isArrow, nearest, place, placeChips, type Rect } from "./layout";
import type { LaneNodeView, LaneView, RuleView } from "./model";

/**
 * The lanes of a Workspace or of a Job: the workflow's steps on top, a lane under it for each kind of thing a Job touches, and rules as
 * edges between lanes. **A mock's canvas**: the caller hands lanes and rules already worked out (`build.ts`), and this places them in
 * columns shared by every lane and draws them on `GraphCanvas` for the pan, zoom and fit.
 *
 * **What a node says is a glyph and its tooltip, never a phrase.** The state an instance stands in, the ones it went through, a step
 * that waits and a rule that fired are marks; the words are in the tooltip.
 *
 * **Drawn at the scale it is read**, one canvas unit a pixel, so a node's name is the kit's body size and never fitted down to a frame: the lanes
 * fill its height and the person pans across. A rule's chip sits in the gutter between lanes, where no node is (`layout.ts`).
 *
 * **Keys**: arrows step to the nearest node, `]` and `[` go to the next and the previous rule and pick it, `Enter` picks what has focus, `Esc` lets
 * go of the pick. (`R` and `Shift R` are the walk player's own, and the letter keys the Cockpit takes.) A tooltip is drawn outside the canvas, since a bubble
 * inside React Flow's viewport is scaled with the zoom, and is shown only while its node or chip is hovered or reached by key, held to it as the canvas moves.
 */

export type LanePick = { kind: "node"; id: string } | { kind: "rule"; id: string };

export type LaneCanvasProps = {
  /** What the canvas is, read to somebody who cannot see it. */
  label: string;
  lanes: readonly LaneView[];
  rules: readonly RuleView[];
  picked?: LanePick | undefined;
  onPick?: (pick: LanePick | undefined) => void;
};

/** A tooltip is held to the element it describes, and goes with it. */
type Tip = { lines: readonly string[]; on: Element };

type Held = {
  picked: LanePick | undefined;
  pick: (pick: LanePick | undefined) => void;
  current: string | undefined;
  setCurrent: (id: string) => void;
  node: (id: string, element: HTMLButtonElement | null) => void;
  chip: (key: string, element: HTMLButtonElement | null) => void;
  tip: (lines: readonly string[] | undefined, on?: Element, soon?: boolean) => void;
  /** A key was pressed in the canvas a moment ago, so focus that arrives now was moved by it and is read. */
  keyed: () => boolean;
};

const Hold = createContext<Held | undefined>(undefined);

type LaneData = { lane: LaneView };
type ItemData = { node: LaneNodeView; lane: LaneView };
type RuleData = { rule: RuleView; target: string; chip: { x: number; y: number } };
type MoveData = { back: boolean };
type CanvasNode = Node<LaneData, "lane"> | Node<ItemData, "item">;
type CanvasEdge = Edge<RuleData, "rule"> | Edge<MoveData, "move">;

const TIP_DELAY = 400;

/** What a node's mark is, in the words its tooltip reads. */
function markOf(node: LaneNodeView): { Glyph: LucideIcon; said: string } | undefined {
  if (node.skipped === true) return { Glyph: Ban, said: "Skipped" };
  if (node.waiting !== undefined) return { Glyph: Clock, said: node.waiting };
  if (node.lit === true) return { Glyph: CircleDot, said: "Now" };
  if (node.passed === true) return { Glyph: Check, said: "Went through" };
  return undefined;
}

function LaneBand({ data }: NodeProps<Node<LaneData, "lane">>) {
  const hold = useContext(Hold)!;
  const { lane } = data;
  return (
    <div className="armada-lane" data-kind={lane.kind} style={{ "--lane-head": `${GEO.head}px` } as CSSProperties}>
      <div
        className="armada-lane__head"
        onMouseEnter={(event) => lane.note !== undefined && hold.tip([lane.title, lane.note], event.currentTarget)}
        onMouseLeave={() => hold.tip(undefined)}
      >
        <span className="armada-lane__title">{lane.title}</span>
        <span className="armada-lane__source">{lane.source}</span>
      </div>
    </div>
  );
}

function ItemNode({ data }: NodeProps<Node<ItemData, "item">>) {
  const hold = useContext(Hold)!;
  const { node, lane } = data;
  const mine = hold.current === node.id;
  const picked = hold.picked?.kind === "node" && hold.picked.id === node.id;
  const mark = markOf(node);
  const name = `${lane.kind === "workflow" ? "Step" : lane.title}: ${node.label}${node.who === undefined ? "" : `, assigned to ${node.who}`}`;
  const lines = [node.label, ...(mark === undefined ? [] : [mark.said]), ...(node.facts ?? []), ...(node.who === undefined ? [] : [`Assigned to ${node.who}`]), ...(node.sub === undefined || node.waiting !== undefined ? [] : [node.sub])];
  return (
    <div
      className="armada-lane-item"
      data-role={node.role}
      data-lit={node.lit || undefined}
      data-passed={node.passed || undefined}
      data-waiting={node.waiting !== undefined || undefined}
      data-end={node.end || undefined}
      data-skipped={node.skipped || undefined}
      data-changed={node.changed !== undefined || undefined}
    >
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={false} />
      ))}
      <button
        type="button"
        ref={(element) => hold.node(node.id, element)}
        className="armada-lane-item__main nodrag nopan"
        data-node={node.id}
        data-picked={picked || undefined}
        tabIndex={mine ? 0 : -1}
        aria-label={name}
        aria-pressed={picked}
        aria-current={node.lit === true ? "step" : undefined}
        onFocus={(event) => {
          hold.setCurrent(node.id);
          if (event.currentTarget.matches(":focus-visible") && hold.keyed()) hold.tip(lines, event.currentTarget, true);
        }}
        onBlur={() => hold.tip(undefined)}
        onMouseEnter={(event) => hold.tip(lines, event.currentTarget)}
        onMouseLeave={() => hold.tip(undefined)}
        onClick={() => hold.pick(picked ? undefined : { kind: "node", id: node.id })}
      >
        <span className="armada-lane-item__mark" aria-hidden="true">
          {mark === undefined ? null : <mark.Glyph size={16} strokeWidth={2} />}
        </span>
        <span className="armada-lane-item__label">{node.label}</span>
        {node.sub === undefined && node.waiting === undefined ? null : <span className="armada-lane-item__sub">{node.waiting ?? node.sub}</span>}
        {node.who === undefined ? null : (
          <span className="armada-lane-item__who" aria-hidden="true">
            <UserCheck size={14} strokeWidth={2} />
            <span>{node.who}</span>
          </span>
        )}
      </button>
      {node.changed === undefined ? null : (
        <span
          className="armada-lane-item__changed"
          role="img"
          aria-label={`Changed on this Job: ${node.changed}`}
          onMouseEnter={(event) => hold.tip(["Changed on this Job", node.changed!], event.currentTarget, true)}
          onMouseLeave={() => hold.tip(undefined)}
        >
          <GitCompare size={14} strokeWidth={2} aria-hidden="true" />
        </span>
      )}
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`s-${side}`} id={`s-${side}`} type="source" position={side} isConnectable={false} />
      ))}
    </div>
  );
}

/** The mark on a rule's chip: how it ended on a Job, or how much it may do alone in the editor. Acts alone carries none. */
function chipMark(rule: RuleView): { Glyph: LucideIcon; said: string } | undefined {
  if (rule.proposed === true) return { Glyph: ListPlus, said: "Proposed" };
  if (rule.status === "acted") return { Glyph: Check, said: "Acted" };
  if (rule.status === "asked") return { Glyph: MessageCircleQuestion, said: "Asked" };
  if (rule.status === "waiting") return { Glyph: Clock, said: "Waiting" };
  if (rule.autonomy === "asks") return { Glyph: MessageCircleQuestion, said: "Asks" };
  if (rule.autonomy === "off") return { Glyph: Ban, said: "Off" };
  return undefined;
}

function RuleEdge(props: EdgeProps<Edge<RuleData, "rule">>) {
  const hold = useContext(Hold)!;
  const { rule, target, chip } = props.data!;
  const [path] = getBezierPath(props);
  const picked = hold.picked?.kind === "rule" && hold.picked.id === rule.rule;
  const mark = chipMark(rule);
  const tone = rule.proposed === true ? "proposed" : rule.status ?? "defined";
  const lines = [rule.label, ...(rule.did === undefined ? (mark === undefined ? [] : [mark.said]) : [rule.did]), ...(rule.meta === undefined ? [] : [rule.meta])];
  const name = `${rule.proposed === true ? "Proposed: " : ""}${rule.label}${rule.quiet === true ? `, ${target}` : ""}`;
  return (
    <>
      <g className="armada-lane-edge" data-tone={tone} data-autonomy={rule.autonomy} data-picked={picked || undefined}>
        <defs>
          <marker id={`head-${props.id}`} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
            <path className="armada-lane-edge__head" d="M0 0L10 5L0 10z" />
          </marker>
        </defs>
        <path id={props.id} d={path} fill="none" className="react-flow__edge-path armada-lane-edge__line" markerEnd={`url(#head-${props.id})`} />
      </g>
      <EdgeLabelRenderer>
        <button
          type="button"
          ref={(element) => hold.chip(props.id, element)}
          className="armada-lane-chip nodrag nopan"
          data-rule={rule.rule}
          data-tone={tone}
          data-autonomy={rule.autonomy}
          data-quiet={rule.quiet || undefined}
          data-picked={picked || undefined}
          tabIndex={-1}
          aria-label={name}
          aria-pressed={picked}
          style={{ transform: `translate(-50%, -50%) translate(${chip.x}px, ${chip.y}px)` }}
          onFocus={(event) => {
            if (event.currentTarget.matches(":focus-visible") && hold.keyed()) hold.tip(lines, event.currentTarget, true);
          }}
          onBlur={() => hold.tip(undefined)}
          onMouseEnter={(event) => hold.tip(lines, event.currentTarget)}
          onMouseLeave={() => hold.tip(undefined)}
          onClick={() => hold.pick(picked ? undefined : { kind: "rule", id: rule.rule })}
        >
          {mark === undefined ? null : <mark.Glyph className="armada-lane-chip__mark" size={14} strokeWidth={2} aria-hidden="true" />}
          {rule.quiet === true ? null : (
            <>
              <span className="armada-lane-chip__label">{rule.label}</span>
              {rule.meta === undefined ? null : <code className="armada-lane-chip__meta">{rule.meta}</code>}
            </>
          )}
        </button>
      </EdgeLabelRenderer>
    </>
  );
}

/** A way a state is left within its lane: a thin line, looping underneath when it goes back. */
function MoveEdge(props: EdgeProps<Edge<MoveData, "move">>) {
  const [path] = getBezierPath(props);
  return (
    <g className="armada-lane-move" data-back={props.data?.back || undefined}>
      <defs>
        <marker id={`head-${props.id}`} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path className="armada-lane-move__head" d="M0 0L10 5L0 10z" />
        </marker>
      </defs>
      <path d={path} fill="none" className="react-flow__edge-path armada-lane-move__line" markerEnd={`url(#head-${props.id})`} />
    </g>
  );
}

const NODE_TYPES = { lane: LaneBand, item: ItemNode };
const EDGE_TYPES = { rule: RuleEdge, move: MoveEdge };

/** Across when the target lies in the same lane; a way down or up the lanes leaves and enters at the top and the bottom. */
function sidesOf(from: Rect, to: Rect): { sourceHandle: string; targetHandle: string } {
  if (Math.abs(from.y - to.y) < 1) {
    return to.x >= from.x ? { sourceHandle: `s-${Position.Right}`, targetHandle: `t-${Position.Left}` } : { sourceHandle: `s-${Position.Bottom}`, targetHandle: `t-${Position.Bottom}` };
  }
  return to.y > from.y ? { sourceHandle: `s-${Position.Bottom}`, targetHandle: `t-${Position.Top}` } : { sourceHandle: `s-${Position.Top}`, targetHandle: `t-${Position.Bottom}` };
}

/**
 * The tooltip, in the document's body and measured there. **Held to the element it describes**: it is placed again every frame it is up, so a canvas that pans, a lane
 * that grows with its frame or a node that is gone takes the bubble with it, and one that has left the canvas is not drawn.
 */
function TipView({ tip, within }: { tip: Tip; within: HTMLElement | null }) {
  const bubble = useRef<HTMLSpanElement>(null);
  const [placed, setPlaced] = useState<{ left: number; top: number } | undefined>();
  useLayoutEffect(() => {
    let frame = 0;
    const settle = () => {
      const held = bubble.current;
      const at = tip.on.getBoundingClientRect();
      const frameAt = within?.getBoundingClientRect();
      const seen = tip.on.isConnected && (frameAt === undefined || (at.right > frameAt.left && at.left < frameAt.right && at.bottom > frameAt.top && at.top < frameAt.bottom));
      if (held !== null && seen) {
        const { width, height } = held.getBoundingClientRect();
        const gap = 6;
        const room = 8;
        const left = Math.round(Math.max(room, Math.min(at.left, window.innerWidth - width - room)));
        const below = at.bottom + gap;
        const top = Math.round(below + height + room <= window.innerHeight ? below : Math.max(room, at.top - height - gap));
        setPlaced((was) => (was !== undefined && was.left === left && was.top === top ? was : { left, top }));
      } else setPlaced((was) => (was === undefined ? was : undefined));
      frame = window.requestAnimationFrame(settle);
    };
    settle();
    return () => window.cancelAnimationFrame(frame);
  }, [tip, within]);
  return createPortal(
    <span ref={bubble} className="armada-lane-tip" aria-hidden="true" style={{ left: placed?.left ?? 0, top: placed?.top ?? 0, visibility: placed === undefined ? "hidden" : "visible" }}>
      {tip.lines.map((line, index) => (
        <span key={index} className="armada-lane-tip__line" data-first={index === 0 || undefined}>
          {line}
        </span>
      ))}
    </span>,
    document.body,
  );
}

/** Brings the node focus reached into the frame, at the zoom the person has. Nothing is current until something is focused, so opening pans nowhere. */
function Follows({ node, items }: { node: string | undefined; items: ReadonlyMap<string, Rect> }) {
  const flow = useReactFlow();
  const width = useStore((state) => state.width);
  const height = useStore((state) => state.height);
  useEffect(() => {
    const one = node === undefined ? undefined : items.get(node);
    if (one === undefined || width === 0) return;
    const { x, y, zoom } = flow.getViewport();
    const inset = 24;
    const left = one.x * zoom + x;
    const top = one.y * zoom + y;
    const right = (one.x + one.w) * zoom + x;
    const bottom = (one.y + one.h) * zoom + y;
    if (left >= inset && top >= inset && right <= width - inset && bottom <= height - inset) return;
    void flow.setCenter(one.x + one.w / 2, one.y + one.h / 2, { zoom, duration: 150 });
    // Only the node changing pans: a resize or a new set of lanes leaves the view where the person put it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [flow, node]);
  return null;
}

export function LaneCanvas({ label, lanes, rules, picked, onPick }: LaneCanvasProps) {
  const [current, setCurrent] = useState<string | undefined>();
  const [tip, setTip] = useState<Tip | undefined>();
  const timer = useRef<number | undefined>(undefined);
  const keyedAt = useRef(0);
  const frame = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState(0);
  const nodeButtons = useRef(new Map<string, HTMLButtonElement>());
  const chipButtons = useRef(new Map<string, HTMLButtonElement>());
  useEffect(() => () => window.clearTimeout(timer.current), []);

  // The lanes fill the frame's height, so the frame is measured and the lanes placed in it.
  useLayoutEffect(() => {
    const held = frame.current;
    if (held === null) return;
    const read = () => setHeight(held.clientHeight);
    read();
    const watch = new ResizeObserver(read);
    watch.observe(held);
    return () => watch.disconnect();
  }, []);

  const placement = useMemo(() => place(lanes, height), [lanes, height]);
  const chips = useMemo(() => placeChips(rules, placement), [rules, placement]);
  const first = lanes[0]?.nodes[0]?.id;
  const here = current !== undefined && placement.items.has(current) ? current : first;

  const hold = useMemo<Held>(
    () => ({
      picked,
      pick: (next) => onPick?.(next),
      current: here,
      setCurrent,
      node: (id, element) => void (element === null ? nodeButtons.current.delete(id) : nodeButtons.current.set(id, element)),
      chip: (key, element) => void (element === null ? chipButtons.current.delete(key) : chipButtons.current.set(key, element)),
      tip: (lines, on, soon = false) => {
        window.clearTimeout(timer.current);
        if (lines === undefined || on === undefined) return setTip(undefined);
        if (soon) return setTip({ lines, on });
        timer.current = window.setTimeout(() => setTip({ lines, on }), TIP_DELAY);
      },
      keyed: () => performance.now() - keyedAt.current < 300,
    }),
    [here, picked, onPick],
  );

  const nodes = useMemo<CanvasNode[]>(() => {
    const out: CanvasNode[] = [];
    for (const lane of lanes) {
      const band = placement.bands.get(lane.id)!;
      out.push({
        id: `lane:${lane.id}`,
        type: "lane",
        position: { x: band.x, y: band.y },
        width: band.w,
        height: band.h,
        draggable: false,
        selectable: false,
        focusable: false,
        zIndex: 0,
        data: { lane },
      });
      for (const node of lane.nodes) {
        const box = placement.items.get(node.id)!;
        out.push({
          id: node.id,
          type: "item",
          position: { x: box.x, y: box.y },
          width: box.w,
          height: box.h,
          draggable: false,
          selectable: true,
          focusable: false,
          zIndex: 1,
          data: { node, lane },
        });
      }
    }
    return out;
  }, [lanes, placement]);

  const edges = useMemo<CanvasEdge[]>(() => {
    const titleOf = new Map(lanes.map((lane) => [lane.id, lane.title] as const));
    const moves = lanes.flatMap((lane) =>
      lane.moves.flatMap((pair): CanvasEdge[] => {
        const from = placement.items.get(pair[0]);
        const to = placement.items.get(pair[1]);
        if (from === undefined || to === undefined) return [];
        return [{ id: `move:${pair[0]}>${pair[1]}`, source: pair[0], target: pair[1], type: "move", ...sidesOf(from, to), focusable: false, selectable: false, data: { back: to.x < from.x } }];
      }),
    );
    const drawn = rules.flatMap((rule): CanvasEdge[] => {
      const from = placement.items.get(rule.from);
      const to = placement.items.get(rule.to);
      const chip = chips.get(rule.id);
      if (from === undefined || to === undefined || chip === undefined) return [];
      return [
        {
          id: rule.id,
          source: rule.from,
          target: rule.to,
          type: "rule",
          ...sidesOf(from, to),
          focusable: false,
          selectable: false,
          data: { rule, target: titleOf.get(rule.to.split("/")[0]!) ?? rule.to, chip },
        },
      ];
    });
    return [...moves, ...drawn];
  }, [lanes, rules, placement, chips]);

  const focusNode = useCallback((id: string) => nodeButtons.current.get(id)?.focus({ preventScroll: true }), []);

  const keys = (event: KeyboardEvent<HTMLElement>) => {
    keyedAt.current = performance.now();
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    const target = event.target as HTMLElement;
    if (target.closest("input, textarea, select") !== null) return;
    if (event.key === "Escape" && picked !== undefined) {
      event.preventDefault();
      event.stopPropagation();
      onPick?.(undefined);
      return;
    }
    // `]` and `[` go round the rules in the order they are drawn, from the one focused or picked; each edge of a rule counts once. The rule reached is picked, so what
    // an inspector beside the canvas shows follows it, and its chip has focus.
    if (event.key === "]" || event.key === "[") {
      const ids = [...new Set(rules.filter((rule) => rule.quiet !== true).map((rule) => rule.rule))];
      if (ids.length === 0) return;
      event.preventDefault();
      const at = target.closest<HTMLElement>("[data-rule]")?.dataset.rule ?? (picked?.kind === "rule" ? picked.id : "");
      const now = ids.indexOf(at);
      const back = event.key === "[";
      const next = ids[now === -1 ? (back ? ids.length - 1 : 0) : (now + (back ? ids.length - 1 : 1)) % ids.length]!;
      const edge = rules.find((rule) => rule.rule === next && rule.quiet !== true)!;
      chipButtons.current.get(edge.id)?.focus({ preventScroll: true });
      onPick?.({ kind: "rule", id: next });
      return;
    }
    const node = target.closest<HTMLElement>("[data-node]")?.dataset.node;
    if (node === undefined || !isArrow(event.key)) return;
    const to = nearest(placement.items, node, event.key);
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") event.preventDefault();
    if (to === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    focusNode(to);
  };

  return (
    <Hold.Provider value={hold}>
      <div ref={frame} className="armada-lane-canvas" data-own-arrows="" role="group" aria-label={label} onKeyDown={keys}>
        <GraphCanvas<CanvasNode, CanvasEdge>
          surface="armada-lane-surface"
          label={label}
          nodes={nodes}
          edges={edges}
          nodeTypes={NODE_TYPES}
          edgeTypes={EDGE_TYPES}
          minZoom={0.3}
          home={HOME}
          panOnScroll
        >
          <Follows node={current} items={placement.items} />
        </GraphCanvas>
      </div>
      {tip === undefined ? null : <TipView tip={tip} within={frame.current} />}
    </Hold.Provider>
  );
}
