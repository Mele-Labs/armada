import {
  Handle,
  NodeToolbar,
  Position,
  getBezierPath,
  useConnection,
  useReactFlow,
  useStore,
  EdgeLabelRenderer,
  type Connection,
  type Edge,
  type EdgeProps,
  type Node,
  type NodeProps,
} from "@xyflow/react";
import { Ban, Check, CircleDot, Clock, Drone, GitCompare, ListPlus, MapPin, MessageCircleQuestion, MessageSquareText, Split, SquarePen, SquarePlus, Trash2, UserCheck, type LucideIcon } from "lucide-react";
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

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { GRAPH_CANVAS_SIDES, GraphCanvas } from "../GraphCanvas/GraphCanvas";
import { GEO, HOME, MIN_ZOOM, isArrow, nearest, panTo, place, placeChips, withToolbar, type Placement, type Rect } from "./layout";
import { REMOVAL_WAITS } from "./model";
import type { LaneNodeView, LaneView, NodeKind, RuleView } from "./model";

/**
 * The lanes of a Workspace or of a Job: the workflow's steps on top, a lane under it for each lifecycle the workflow uses, and rules as
 * edges between lanes. **A mock's canvas**: the caller hands lanes and rules already worked out (`build.ts`), and this places them in
 * columns shared by every lane and draws them on `GraphCanvas` for the pan, zoom and fit.
 *
 * **What a node says is a glyph and its tooltip, never a phrase.** Where an instance stands, the states it went through, a step that waits and a rule that fired are marks; the
 * words are in the tooltip. **Where an instance stands is a place and not an act**: a static pin on a heavier neutral frame. The running hue and the working glyph are for the step
 * the agent is working on, and for nothing else.
 *
 * **Drawn at the scale it is read**, one canvas unit a pixel, so a node's name is the kit's body size and never fitted down to a frame: the lanes
 * fill its height and the person pans across. A rule's chip sits in the gutter between lanes, where no node is (`layout.ts`). **The canvas follows**: when the lanes change it pans
 * to what changed, with the earliest of it at the left; as it opens it shows where the Job stands; and it pans to a rule that is picked and to any node or chip that takes focus
 * by key or hover. The lane heads stand over the left of the frame, so a pan never takes a lane's name away. The zoom is the person's and nothing here changes it.
 *
 * **Every node is the shape of what it is** (`NodeKind`): a step a rounded box, a state the tracker or the forge reads a pill, a wait or a watch a hexagon, a Job another Job started a card with the drone glyph, and
 * where the workflow starts and ends small circles; a rule's chip leads with a diamond. Each kind has its own faint tint from `kinds.css`, which is never a status: what a node stands in is its frame and its mark.
 * A wait is reached by a dashed edge, a transition by a solid one.
 *
 * **Edited by hand where `editing` is given.** What is picked shows a toolbar over it: Edit, Tell an agent, Add after and Delete; a hovered node shows connectors, and one dragged
 * to another node is `onConnect`. Edit takes the person to the picked thing's fields in the side panel, which are live; the first change on Armada's workflow makes the person's own copy, and nothing here says so.
 *
 * **Keys**: arrows step to the nearest node, `]` and `[` go to the next and the previous rule and pick it, `Enter` picks what has focus, `Esc` lets
 * go of the pick. With something picked and editing on, `E` edits it, `T` tells an agent, `A` adds after it, `Backspace` deletes it, and `C` starts a connector from a node, which
 * the next node picked ends. (`R` and `Shift R` are the walk player's own, and the letter keys the Cockpit takes.) A tooltip is drawn outside the canvas, since a bubble
 * inside React Flow's viewport is scaled with the zoom, and is shown only while its node or chip is hovered or reached by key, held to it as the canvas moves.
 */

export type LanePick = { kind: "node"; id: string } | { kind: "rule"; id: string } | { kind: "lane"; id: string };

/** What the person may do by hand to the lanes. Every act takes what is picked; a connector takes the two nodes it joins. */
export type LaneEditing = {
  /** Whether the pick can be deleted where the canvas is drawn: a tab deletes only what it added. Absent, anything can. */
  canDelete?: (pick: LanePick) => boolean;
  /** Whether the pick can be told to an agent. Absent, anything can. */
  canTell?: (pick: LanePick) => boolean;
  /** Whether a node can have another added after it. Absent, any can. */
  canAdd?: (pick: LanePick) => boolean;
  /** Whether a connector can join the two nodes, each by its id. A pair that cannot is offered no landing. Absent, any two can. */
  canConnect?: (from: string, to: string) => boolean;
  /** Takes the person to the picked thing's fields, which the side panel draws live. */
  onEdit: (pick: LanePick) => void;
  onTell: (pick: LanePick) => void;
  onAddAfter: (pick: LanePick) => void;
  onDelete: (pick: LanePick) => void;
  onConnect: (from: string, to: string) => void;
};

export type LaneCanvasProps = {
  /** What the canvas is, read to somebody who cannot see it. */
  label: string;
  lanes: readonly LaneView[];
  rules: readonly RuleView[];
  picked?: LanePick | undefined;
  onPick?: (pick: LanePick | undefined) => void;
  /** Absent, the lanes are read and not edited. */
  editing?: LaneEditing | undefined;
  /** What is drawn over the canvas's right edge, an overlay panel: the canvas pans to keep what it shows clear of it. */
  coveredBy?: HTMLElement | null | undefined;
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
  editing: LaneEditing | undefined;
  /** The lane drawn first, whose nodes carry their toolbar underneath since the frame ends above them. */
  topLane: string | undefined;
  /** The node a connector was started from by key, which the next node pressed ends. */
  connecting: string | undefined;
  connect: (to: string) => void;
  /** Pans to bring nodes and chips (by id) into view. `follow` is for what changed, which is put at the left; without it the pan is the least that shows them. */
  reveal: (ids: readonly string[], follow?: boolean) => void;
};

const Hold = createContext<Held | undefined>(undefined);

type LaneData = { lane: LaneView };
type ItemData = { node: LaneNodeView; lane: LaneView };
type RuleData = { rule: RuleView; target: string; chip: { x: number; y: number } };
type MoveData = { back: boolean; wait: boolean };
type CanvasNode = Node<LaneData, "lane"> | Node<ItemData, "item">;
type CanvasEdge = Edge<RuleData, "rule"> | Edge<MoveData, "move">;

const TIP_DELAY = 400;

/** What a node's mark is, in the words its tooltip reads. */
function markOf(node: LaneNodeView): { Glyph: LucideIcon; said: string } | undefined {
  if (node.skipped === true) return { Glyph: Ban, said: "Skipped" };
  if (node.ended === true) return { Glyph: Ban, said: "Ended" };
  if (node.waiting !== undefined) return { Glyph: Clock, said: "Waiting" };
  if (node.lit === true) return { Glyph: CircleDot, said: "Working" };
  if (node.here === true) return { Glyph: MapPin, said: "Now" };
  if (node.passed === true) return { Glyph: Check, said: "Went through" };
  return undefined;
}

/** What a mark on a node or a rule says of the levels it differs in: `Differs in my-store, store-api`. */
const differsSaid = (names: readonly string[]): string => `Differs in ${names.join(", ")}`;

/** The acts a toolbar offers after Edit, with their glyphs and the keys that do them. */
const ACTS = [
  { id: "tell", said: "Tell an agent", Glyph: MessageSquareText, key: "T" },
  { id: "add", said: "Add after", Glyph: SquarePlus, key: "A" },
  { id: "delete", said: "Delete", Glyph: Trash2, key: "⌫" },
] as const;

/**
 * The toolbar over what is picked: Edit as a button that says so, then Tell an agent, Add after and Delete, each a glyph with its tooltip and key; a rule has no Add after. Edit goes to the fields in the side
 * panel, which are live: there is no mode to enter, and the first change on Armada's workflow makes the person's own copy without saying so.
 */
function Toolbar({ pick, name, rule }: { pick: LanePick; name: string; rule: boolean }) {
  const { editing } = useContext(Hold)!;
  if (editing === undefined) return null;
  const run = { tell: editing.onTell, add: editing.onAddAfter, delete: editing.onDelete };
  return (
    <div className="armada-lane-toolbar nodrag nopan" role="toolbar" aria-label={`Edit ${name}`}>
      <Tooltip label="Edit" shortcut="E">
        <Button type="button" variant="ghost" size="sm" aria-label={`Edit ${name}`} onClick={() => editing.onEdit(pick)}>
          <SquarePen size={16} strokeWidth={2} aria-hidden="true" />
          Edit
        </Button>
      </Tooltip>
      {ACTS.filter((one) => !(rule && one.id === "add") && !(one.id === "add" && editing.canAdd?.(pick) === false) && !(one.id === "delete" && editing.canDelete?.(pick) === false) && !(one.id === "tell" && editing.canTell?.(pick) === false)).map((one) => (
        <Tooltip key={one.id} label={one.said} shortcut={one.key}>
          <Button type="button" variant="ghost" size="sm" iconOnly aria-label={`${one.said} ${name}`} onClick={() => run[one.id](pick)}>
            <one.Glyph size={16} strokeWidth={2} aria-hidden="true" />
          </Button>
        </Tooltip>
      ))}
    </div>
  );
}

/** The lane's own line, under its nodes. Its head is drawn over the left of the frame (`Heads`), so it stays on show as the canvas pans. */
function LaneBand({ data }: NodeProps<Node<LaneData, "lane">>) {
  return <div className="armada-lane" data-kind={data.lane.kind} />;
}

/** The lane heads, over the left of the frame at the lanes' own heights: a title and where its states are read from, which a pan to a later column never takes away. */
function Heads({ lanes, placement }: { lanes: readonly LaneView[]; placement: Placement }) {
  const hold = useContext(Hold)!;
  const transform = useStore((state) => state.transform);
  const ty = transform[1];
  const zoom = transform[2];
  return (
    <div className="armada-lane-heads">
      {lanes.map((lane) => {
        const band = placement.bands.get(lane.id)!;
        // A lifecycle's head is picked where the lanes are edited: its fields are the lifecycle's name and its deleting.
        const pickable = hold.editing !== undefined && lane.kind === "lifecycle";
        const picked = hold.picked?.kind === "lane" && hold.picked.id === lane.id;
        const words = (
          <>
            <span className="armada-lane__title">{lane.title}</span>
            <span className="armada-lane__source">{lane.ended === true ? "Ended" : lane.source}</span>
          </>
        );
        return (
          <div
            key={lane.id}
            className="armada-lane__head"
            data-kind={lane.kind}
            data-picked={picked || undefined}
            data-ended={lane.ended || undefined}
            style={{ top: ty + band.y * zoom, height: band.h * zoom, width: GEO.head }}
            onMouseEnter={(event) => lane.note !== undefined && hold.tip([lane.title, lane.note], event.currentTarget)}
            onMouseLeave={() => hold.tip(undefined)}
          >
            {pickable ? (
              <button type="button" className="armada-lane__pick nodrag nopan" aria-pressed={picked} onClick={() => hold.pick(picked ? undefined : { kind: "lane", id: lane.id })}>
                {words}
              </button>
            ) : (
              words
            )}
            {lane.pending === true ? (
              <span className="armada-lane__pending">
                <Tooltip label={REMOVAL_WAITS}>
                  <span role="img" aria-label={REMOVAL_WAITS}>
                    <Clock size={14} strokeWidth={2} aria-hidden="true" />
                  </span>
                </Tooltip>
              </span>
            ) : null}
          </div>
        );
      })}
    </div>
  );
}

/** The outline each kind is drawn in. */
type Shape = "box" | "pill" | "hexagon" | "card" | "dot";
const SHAPE: Readonly<Record<NodeKind, Shape>> = { step: "box", wait: "hexagon", watch: "hexagon", tracker: "pill", forge: "pill", document: "pill", job: "card", start: "dot", end: "dot" };

/** The handles an edge joins a node by, on each side: those it lands on, drawn under the node, and those it leaves by, drawn over it so a connector can be started from them. */
function Handles({ node, lane, connectable, part }: { node: LaneNodeView; lane: LaneView; connectable: boolean; part: "target" | "source" }) {
  return part === "target" ? (
    <>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle key={`t-${side}`} id={`t-${side}`} type="target" position={side} isConnectable={connectable} />
      ))}
      {connectable ? <Handle id="t-node" className="armada-lane-item__drop" type="target" position={Position.Top} /> : null}
    </>
  ) : (
    <>
      {GRAPH_CANVAS_SIDES.map((side) => (
        <Handle
          key={`s-${side}`}
          id={`s-${side}`}
          type="source"
          position={side}
          isConnectable={connectable}
          {...(connectable && side === Position.Right ? { role: "button", tabIndex: -1, "aria-label": `Connect from ${lane.kind === "workflow" ? "step" : lane.title} ${node.label}` } : {})}
        />
      ))}
    </>
  );
}

function ItemNode({ data }: NodeProps<Node<ItemData, "item">>) {
  const hold = useContext(Hold)!;
  const { node, lane } = data;
  const mine = hold.current === node.id;
  const picked = hold.picked?.kind === "node" && hold.picked.id === node.id;
  const mark = markOf(node);
  const shape = SHAPE[node.kind];
  const connectable = hold.editing !== undefined && shape !== "dot";
  const dropping = useConnection((one) => one.inProgress && one.fromNode.id !== node.id);
  const name = `${lane.kind === "workflow" ? "Step" : lane.title}: ${node.label}${node.who === undefined ? "" : `, assigned to ${node.who}`}${node.ended === true ? ", ended" : ""}`;
  const lines = [
    node.label,
    ...(node.waiting ?? (mark === undefined ? [] : [mark.said])),
    ...(node.facts ?? []),
    ...(node.who === undefined ? [] : [`Assigned to ${node.who}`]),
    ...(node.differs === undefined ? [] : [differsSaid(node.differs)]),
    ...(node.sub === undefined || node.waiting !== undefined ? [] : [node.sub]),
  ];
  // Where the workflow starts and ends are circles with nothing to pick: a name on hover, and the edge that joins them.
  if (shape === "dot") {
    return (
      <div className="armada-lane-item" data-kind={node.kind} data-shape={shape} data-passed={node.passed || undefined} data-here={node.here || undefined}>
        <Handles node={node} lane={lane} connectable={false} part="target" />
        <Handles node={node} lane={lane} connectable={false} part="source" />
        <span
          className="armada-lane-item__dot"
          role="img"
          aria-label={node.label}
          onMouseEnter={(event) => hold.tip([node.label], event.currentTarget)}
          onMouseLeave={() => hold.tip(undefined)}
        />
      </div>
    );
  }
  return (
    <div
      className="armada-lane-item"
      data-role={node.role}
      data-kind={node.kind}
      data-shape={shape}
      data-connectable={connectable || undefined}
      data-connecting={hold.connecting === node.id || undefined}
      data-dropping={dropping || undefined}
      data-lit={node.lit || undefined}
      data-here={node.here || undefined}
      data-passed={node.passed || undefined}
      data-waiting={node.waiting !== undefined || undefined}
      data-end={node.end || undefined}
      data-skipped={node.skipped || undefined}
      data-ended={node.ended || undefined}
      data-changed={node.changed !== undefined || undefined}
    >
      <Handles node={node} lane={lane} connectable={connectable} part="target" />
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
          if (hold.keyed()) hold.reveal([node.id]);
          if (event.currentTarget.matches(":focus-visible") && hold.keyed()) hold.tip(lines, event.currentTarget, true);
        }}
        onBlur={() => hold.tip(undefined)}
        onMouseEnter={(event) => {
          hold.reveal([node.id]);
          hold.tip(lines, event.currentTarget);
        }}
        onMouseLeave={() => hold.tip(undefined)}
        onClick={() => (hold.connecting !== undefined && hold.connecting !== node.id ? hold.connect(node.id) : hold.pick(picked ? undefined : { kind: "node", id: node.id }))}
      >
        {shape === "hexagon" ? (
          <svg className="armada-lane-item__shape" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
            <polygon points="8,0 92,0 100,50 92,100 8,100 0,50" />
          </svg>
        ) : null}
        <span className="armada-lane-item__mark" aria-hidden="true">
          {shape === "card" ? <Drone size={16} strokeWidth={2} /> : mark === undefined ? null : <mark.Glyph size={16} strokeWidth={2} />}
        </span>
        <span className="armada-lane-item__label">{node.label}</span>
        {node.waiting !== undefined ? (
          <span className="armada-lane-item__sub">
            {node.waiting.map((line) => (
              <span key={line}>{line}</span>
            ))}
          </span>
        ) : node.sub === undefined ? null : (
          <span className="armada-lane-item__sub">{node.sub}</span>
        )}
        {node.who === undefined ? null : (
          <span className="armada-lane-item__who" aria-hidden="true">
            <UserCheck size={14} strokeWidth={2} />
            <span>{node.who}</span>
          </span>
        )}
        {shape !== "card" || mark === undefined ? null : (
          <span className="armada-lane-item__status" aria-hidden="true">
            <mark.Glyph size={14} strokeWidth={2} />
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
      {node.differs === undefined ? null : (
        <span
          className="armada-lane-item__differs"
          data-beside={node.changed !== undefined || undefined}
          role="img"
          aria-label={differsSaid(node.differs)}
          onMouseEnter={(event) => hold.tip([differsSaid(node.differs!)], event.currentTarget, true)}
          onMouseLeave={() => hold.tip(undefined)}
        >
          <Split size={14} strokeWidth={2} aria-hidden="true" />
        </span>
      )}
      <Handles node={node} lane={lane} connectable={connectable} part="source" />
      <NodeToolbar isVisible={picked && hold.editing !== undefined} position={lane.id === hold.topLane ? Position.Bottom : Position.Top} offset={10}>
        <Toolbar pick={{ kind: "node", id: node.id }} name={`${lane.kind === "workflow" ? "step" : lane.title} ${node.label}`} rule={false} />
      </NodeToolbar>
    </div>
  );
}

/** The mark on a rule's chip: how it ended on a Job, or how much it may do alone in the editor. Acts alone carries none. */
function chipMark(rule: RuleView): { Glyph: LucideIcon; said: string } | undefined {
  if (rule.pending === true) return { Glyph: Clock, said: REMOVAL_WAITS };
  if (rule.proposed === true) return { Glyph: ListPlus, said: "Proposed" };
  if (rule.needs !== undefined) return { Glyph: MessageCircleQuestion, said: "Waiting on you" };
  if (rule.status === "acted") return { Glyph: Check, said: "Acted" };
  if (rule.status === "asked") return { Glyph: MessageCircleQuestion, said: "Asked" };
  if (rule.status === "waiting") return { Glyph: Clock, said: "Waiting" };
  if (rule.status === "skipped") return { Glyph: Ban, said: "Skipped" };
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
  const lines = [
    rule.label,
    ...(rule.needs !== undefined ? ["Waiting on you", ...(rule.did ?? [])] : rule.did === undefined ? (mark === undefined ? [] : [mark.said]) : rule.did),
    ...(rule.meta === undefined ? [] : [rule.meta]),
    ...(rule.differs === undefined ? [] : [differsSaid(rule.differs)]),
  ];
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
          data-needs={rule.needs !== undefined || undefined}
          data-picked={picked || undefined}
          tabIndex={-1}
          aria-label={name}
          aria-pressed={picked}
          style={{ transform: `translate(-50%, -50%) translate(${chip.x}px, ${chip.y}px)` }}
          onFocus={(event) => {
            if (hold.keyed()) hold.reveal([props.id]);
            if (event.currentTarget.matches(":focus-visible") && hold.keyed()) hold.tip(lines, event.currentTarget, true);
          }}
          onBlur={() => hold.tip(undefined)}
          onMouseEnter={(event) => {
            hold.reveal([props.id]);
            hold.tip(lines, event.currentTarget);
          }}
          onMouseLeave={() => hold.tip(undefined)}
          onClick={() => hold.pick(picked ? undefined : { kind: "rule", id: rule.rule })}
        >
          <span className="armada-lane-chip__diamond" aria-hidden="true">
            <svg viewBox="0 0 24 24">
              <polygon points="12,1 23,12 12,23 1,12" />
            </svg>
            {mark === undefined ? null : <mark.Glyph className="armada-lane-chip__mark" size={12} strokeWidth={2} />}
          </span>
          {rule.quiet === true ? null : (
            <>
              <span className="armada-lane-chip__label">{rule.label}</span>
              {rule.meta === undefined ? null : <code className="armada-lane-chip__meta">{rule.meta}</code>}
            </>
          )}
          {rule.differs === undefined ? null : <Split className="armada-lane-chip__differs" size={14} strokeWidth={2} aria-hidden="true" />}
        </button>
        {picked && hold.editing !== undefined && rule.quiet !== true && rule.proposed !== true ? (
          <div className="armada-lane-toolbar-at" style={{ transform: `translate(-50%, -100%) translate(${chip.x}px, ${chip.y - 22}px)` }}>
            <Toolbar pick={{ kind: "rule", id: rule.rule }} name={`rule ${rule.label}`} rule />
          </div>
        ) : null}
      </EdgeLabelRenderer>
    </>
  );
}

/** A way a state is left within its lane: a thin solid line, looping underneath when it goes back, and dashed where it leads to a wait. */
function MoveEdge(props: EdgeProps<Edge<MoveData, "move">>) {
  const [path] = getBezierPath(props);
  return (
    <g className="armada-lane-move" data-back={props.data?.back || undefined} data-wait={props.data?.wait || undefined}>
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
  if (Math.abs(from.y + from.h / 2 - (to.y + to.h / 2)) < 1) {
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

/** A pan asked for: the rectangles to bring into view, and whether they are what changed. */
type Pan = { n: number; rects: readonly Rect[]; follow: boolean };

/** Pans the canvas as asked, at the zoom the person has. It waits for the frame's size, so a request made as the canvas opens is kept until it can be met. */
function Pans({ pan, content, covered }: { pan: Pan | undefined; content: { width: number; height: number }; covered: number }) {
  const flow = useReactFlow();
  const width = useStore((state) => state.width);
  const height = useStore((state) => state.height);
  const done = useRef(0);
  useEffect(() => {
    if (pan === undefined || pan.n === done.current || width === 0 || height === 0) return;
    done.current = pan.n;
    const next = panTo(pan.rects, flow.getViewport(), { width: Math.max(1, width - covered), height }, content, pan.follow);
    if (next !== undefined) void flow.setViewport(next);
    // The request is what runs it: a resize or new lanes leave the view where the person put it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [flow, pan, width, height, covered]);
  return null;
}

/** What a node or a chip is drawn as, so a change to it can be told from one to something beside it. */
const markOfNode = (node: LaneNodeView): string => [node.lit, node.here, node.passed, node.waiting, node.skipped, node.who, node.changed, node.differs, node.sub].map(String).join("|");
const markOfRule = (rule: RuleView): string => [rule.status, rule.meta, rule.proposed, rule.autonomy, rule.did, rule.needs, rule.differs].map(String).join("|");

export function LaneCanvas({ label, lanes, rules, picked, onPick, editing, coveredBy }: LaneCanvasProps) {
  const [current, setCurrent] = useState<string | undefined>();
  const [connecting, setConnecting] = useState<string | undefined>();
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
  // Where the workflow starts and ends are circles nothing reaches: the first node to focus is the first step.
  const ends = useMemo(() => new Set(lanes.flatMap((lane) => lane.nodes.filter((node) => node.kind === "start" || node.kind === "end").map((node) => node.id))), [lanes]);
  const first = lanes[0]?.nodes.find((node) => !ends.has(node.id))?.id;
  const here = current !== undefined && placement.items.has(current) ? current : first;

  // **The canvas follows what it is asked to show.** A request names nodes and chips, and the pan brings them into view at the zoom the person has (`Pans`).
  const [pan, setPan] = useState<Pan | undefined>();
  const asked = useRef(0);
  const geometry = useRef({ placement, chips, toolbar: editing !== undefined, top: lanes[0]?.id });
  geometry.current = { placement, chips, toolbar: editing !== undefined, top: lanes[0]?.id };
  /** `toolbar` names the ones that stand under a toolbar, which is brought into view with them. */
  const reveal = useCallback((ids: readonly string[], follow = false, toolbar: readonly string[] = []) => {
    const { placement: laid, chips: placed, toolbar: editable, top } = geometry.current;
    const rects: Rect[] = [];
    for (const id of ids) {
      const item = laid.items.get(id);
      const chip = placed.get(id);
      const room = editable && toolbar.includes(id);
      if (item !== undefined) rects.push(room ? withToolbar(item, id.startsWith(`${top}/`) ? "below" : "above") : item);
      else if (chip !== undefined) {
        const rect = { x: chip.x - chip.w / 2, y: chip.y - chip.h / 2, w: chip.w, h: chip.h };
        rects.push(room ? withToolbar(rect, "above") : rect);
      }
    }
    if (rects.length === 0) return;
    asked.current += 1;
    setPan({ n: asked.current, rects, follow });
  }, []);

  // When the lanes change, what changed is brought into view with the earliest of it at the left, and as they open, where the Job stands: the states it is in, what waits, what was
  // changed on it and the rules between them. A rule's chip and the nodes it joins go together, so the context for a chip is on show with it.
  const drawn = useRef<ReadonlyMap<string, string> | undefined>(undefined);
  useEffect(() => {
    const now = new Map<string, string>();
    for (const lane of lanes) for (const node of lane.nodes) now.set(node.id, markOfNode(node));
    for (const rule of rules) now.set(rule.id, markOfRule(rule));
    const before = drawn.current;
    drawn.current = now;
    // A step the Job skips is marked where it stands, but nothing happens there: the view goes to what does.
    const standing = new Set(lanes.flatMap((lane) => lane.nodes.filter((node) => node.skipped !== true && (node.lit === true || node.here === true || node.waiting !== undefined || node.changed !== undefined)).map((node) => node.id)));
    const ids = new Set<string>();
    for (const rule of rules) {
      const fresh = before === undefined ? rule.proposed === true || standing.has(rule.from) || standing.has(rule.to) : before.get(rule.id) !== now.get(rule.id);
      if (fresh) for (const id of [rule.id, rule.from, rule.to]) ids.add(id);
    }
    // A node added to a lane that was there is a change. A node of a lane that is itself new is one only where something stands on it: the lane is shown at the states that are held, not every one it has.
    for (const lane of lanes) {
      const laneWas = before !== undefined && [...before.keys()].some((key) => key.startsWith(`${lane.id}/`));
      for (const node of lane.nodes) {
        if (node.skipped === true) continue;
        const was = before?.get(node.id);
        if (before === undefined ? standing.has(node.id) : was === undefined ? laneWas || standing.has(node.id) : was !== now.get(node.id)) ids.add(node.id);
      }
    }
    if (ids.size > 0) reveal([...ids], true);
  }, [lanes, rules, reveal]);

  // What is picked is brought into view with its toolbar: a rule with its chips and the nodes it joins. It is a pan, so the canvas keeps its width and the panel covers only what is past it.
  const revealPick = () => {
    if (picked === undefined) return;
    if (picked.kind === "lane") return;
    if (picked.kind === "node") return reveal([picked.id], false, [picked.id]);
    const edges = rules.filter((rule) => rule.rule === picked.id);
    reveal(edges.flatMap((rule) => [rule.id, rule.from, rule.to]), false, edges.map((rule) => rule.id));
  };
  useEffect(() => {
    revealPick();
    // Only a change of what is picked pans.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [picked?.kind, picked?.id]);

  // **What an overlay covers is not the canvas.** The panel slides in over the right edge, so once it has settled the width it takes is read, and the pick is brought into what is left.
  const [covered, setCovered] = useState(0);
  useEffect(() => {
    const sheet = coveredBy?.closest<HTMLElement>("[data-side]");
    if (coveredBy === null || coveredBy === undefined || sheet === null || sheet === undefined) return setCovered(0);
    let live = true;
    const read = () => {
      const held = frame.current;
      if (!live || held === null) return;
      setCovered(Math.max(0, Math.round(held.getBoundingClientRect().right - sheet.getBoundingClientRect().left)));
    };
    void Promise.allSettled(sheet.getAnimations().map((one) => one.finished)).then(read);
    return () => {
      live = false;
    };
  }, [coveredBy]);
  const seen = useRef(0);
  useEffect(() => {
    if (covered === seen.current) return;
    seen.current = covered;
    if (covered > 0) revealPick();
    // The width it covers is what asks again.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [covered]);

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
      editing,
      topLane: lanes[0]?.id,
      connecting,
      connect: (to) => {
        if (connecting !== undefined && editing?.canConnect?.(connecting, to) !== false) editing?.onConnect(connecting, to);
        setConnecting(undefined);
      },
      reveal,
    }),
    [here, picked, onPick, reveal, editing, connecting, lanes],
  );

  // A connector dragged from one node's handle to another's.
  const connect = useCallback((joined: Connection) => editing?.canConnect?.(joined.source, joined.target) !== false && editing?.onConnect(joined.source, joined.target), [editing]);
  const joinable = useCallback((joined: Connection) => editing?.canConnect?.(joined.source, joined.target) !== false, [editing]);
  const connectable = editing !== undefined;

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
    const kindOf = new Map(lanes.flatMap((lane) => lane.nodes.map((node) => [node.id, node.kind] as const)));
    const moves = lanes.flatMap((lane) =>
      lane.moves.flatMap((pair): CanvasEdge[] => {
        const from = placement.items.get(pair[0]);
        const to = placement.items.get(pair[1]);
        if (from === undefined || to === undefined) return [];
        const waits = kindOf.get(pair[1]);
        return [{ id: `move:${pair[0]}>${pair[1]}`, source: pair[0], target: pair[1], type: "move", ...sidesOf(from, to), focusable: false, selectable: false, data: { back: to.x < from.x, wait: waits === "wait" || waits === "watch" } }];
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
    if (event.key === "Escape" && connecting !== undefined) {
      event.preventDefault();
      event.stopPropagation();
      setConnecting(undefined);
      return;
    }
    if (event.key === "Escape" && picked !== undefined) {
      event.preventDefault();
      event.stopPropagation();
      onPick?.(undefined);
      return;
    }
    // What is picked is edited by key too: `E` edits it, `T` tells an agent, `A` adds after it, `Backspace` deletes it, and `C` starts a connector from a node, which the next node
    // pressed ends.
    if (editing !== undefined && picked !== undefined) {
      const removable = editing.canDelete?.(picked) !== false ? editing.onDelete : undefined;
      const act = { e: editing.onEdit, t: editing.canTell?.(picked) === false ? undefined : editing.onTell, a: picked.kind === "node" && editing.canAdd?.(picked) !== false ? editing.onAddAfter : undefined, Backspace: removable, Delete: removable }[event.key] as ((pick: LanePick) => void) | undefined;
      if (act !== undefined) {
        event.preventDefault();
        act(picked);
        return;
      }
      if (event.key === "c" && picked.kind === "node") {
        event.preventDefault();
        setConnecting(picked.id);
        return;
      }
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
    const to = nearest(placement.items, node, event.key, ends);
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
          minZoom={MIN_ZOOM}
          home={HOME}
          panOnScroll
          {...(connectable ? { onConnect: connect, isValidConnection: joinable } : {})}
        >
          <Pans pan={pan} covered={covered} content={{ width: placement.width, height: placement.height }} />
          <Heads lanes={lanes} placement={placement} />
        </GraphCanvas>
      </div>
      {tip === undefined ? null : <TipView tip={tip} within={frame.current} />}
    </Hold.Provider>
  );
}
