import { BaseEdge, Handle, applyNodeChanges, getBezierPath, useNodesInitialized, useReactFlow, type Edge, type EdgeProps, type Node, type NodeChange, type NodeProps, type ReactFlowInstance } from "@xyflow/react";
import {
  Activity, Bot, Box, Clock, Cpu, Eye, File, GitBranch, GitMerge, Globe, Hammer, HardDrive, Layers, Lock, Package, Pause, Pencil, Play, Rocket, Scale,
  ScrollText, Server, Settings, ShieldCheck, SquarePlus, Terminal, Trash2, Undo2, Webhook, Wrench, X, Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { GRAPH_CANVAS_SIDES, GraphCanvas, clearOf, facingSides } from "../GraphCanvas/GraphCanvas";
import { GraphCanvasNodeAct, GraphCanvasNodeBar, GraphCanvasRailGroup, type GraphCanvasRailAct } from "../GraphCanvas/GraphCanvasRail";
import { Ink, type SketchPoint } from "../SketchPad/Ink";
import {
  NO_MARKS, diffScenes, extentOf, markJoin, markRemove, nextMarkId, parseScene, partLabel, withMarks,
  type Scene, type SceneChange, type SceneEdge, type SceneIcon, type SceneMarks, type SceneNode,
} from "./scene";

/**
 * A scene, drawn in Armada's own frames: a header band over a mono body, state carried by the
 * frame. **Pan, zoom and fit are `GraphCanvas`'s** (wheel and pinch, drag, the rail's three, and
 * the keys below); the pen is the sketch pad's own `Ink`. A scene that does not validate draws an
 * empty frame.
 *
 * With `against`, the scene is read as a change to it: what it adds is green, what it drops is red,
 * what it alters is amber, and what it leaves is dimmed.
 */
export type SketchSceneProps = {
  /** What a Drone or the pad supplied. Validated here; anything that fails draws nothing. */
  scene: unknown;
  /** The current-state scene this one changes. */
  against?: unknown;
  /** A press on a part opens a small ask about it; this hears what was asked. */
  onAsk?: (about: { id: string; label: string }, said: string) => void;
  /**
   * What the owner did to it, kept by the host: lines, boxes and joins of his, and parts of the
   * Drone's he struck out. His parts are drawn in his colour. Absent offers no tools.
   */
  marks?: SceneMarks;
  onMarks?: (marks: SceneMarks) => void;
};

const ICON: Record<SceneIcon, LucideIcon> = {
  activity: Activity, bot: Bot, box: Box, clock: Clock, cpu: Cpu, eye: Eye, file: File, "git-branch": GitBranch, "git-merge": GitMerge, globe: Globe,
  hammer: Hammer, "hard-drive": HardDrive, layers: Layers, lock: Lock, package: Package, rocket: Rocket, scale: Scale, "scroll-text": ScrollText,
  server: Server, settings: Settings, "shield-check": ShieldCheck, terminal: Terminal, webhook: Webhook, wrench: Wrench, zap: Zap,
};

const CHANGE_SAID: Record<SceneChange, string> = { added: "Added", removed: "Removed", changed: "Changed", same: "Unchanged" };

type NodeData = {
  node: SceneNode;
  change: SceneChange | undefined;
  dim: boolean;
  current: boolean;
  asked: boolean;
  mine: boolean;
  struck: boolean;
  joining: boolean;
  onBody: (body: string) => void;
};
type SceneRfNode = Node<NodeData, "scene">;
type EdgeData = { edge: SceneEdge; change: SceneChange | undefined; dim: boolean; asked: boolean; mine: boolean; struck: boolean; onPress: (id: string) => void };
type SceneRfEdge = Edge<EdgeData, "flow">;

function Sides({ type }: { type: "source" | "target" }) {
  const prefix = type === "source" ? "s" : "t";
  return GRAPH_CANVAS_SIDES.map((side) => <Handle key={`${prefix}-${side}`} id={`${prefix}-${side}`} type={type} position={side} isConnectable={false} />);
}

const sentence = (name: string) => name.charAt(0).toUpperCase() + name.slice(1).replace(/-/g, " ");

function Head({ node }: { node: SceneNode }) {
  const Glyph = node.icon === undefined ? undefined : ICON[node.icon];
  if (node.title === undefined && Glyph === undefined) return null;
  return (
    <div className="armada-scene-node__head">
      {Glyph === undefined || node.icon === undefined ? null : (
        <Tooltip label={sentence(node.icon)}>
          <span className="armada-scene-node__icon" role="img" aria-label={sentence(node.icon)}>
            <Glyph size={12} strokeWidth={2} aria-hidden />
          </span>
        </Tooltip>
      )}
      {node.title === undefined ? null : <span className="armada-scene-node__title">{node.title}</span>}
      {node.lang === undefined ? null : <span className="armada-scene-node__lang">{node.lang}</span>}
    </div>
  );
}

function Wire({ node }: { node: SceneNode }) {
  const part = node.wire ?? "card";
  return (
    <div className="armada-scene-wire" data-part={part}>
      {part === "list" || part === "text" || part === "card" ? (
        <>
          {node.title === undefined ? null : <span className="armada-scene-wire__title">{node.title}</span>}
          <span className="armada-scene-wire__line" />
          <span className="armada-scene-wire__line" />
          {part === "text" ? null : <span className="armada-scene-wire__line" data-short />}
        </>
      ) : (
        <span className="armada-scene-wire__title">{node.title}</span>
      )}
    </div>
  );
}

function SceneNodeView({ data }: NodeProps<SceneRfNode>) {
  const { node, change, dim, current, asked, mine, struck, joining, onBody } = data;
  return (
    <>
      <Sides type="target" />
      <div
        className="armada-scene-node"
        data-kind={node.kind}
        data-wire={node.wire}
        data-change={change}
        data-dim={dim || undefined}
        data-current={current || undefined}
        data-asked={asked || undefined}
        data-mine={mine || undefined}
        data-struck={struck || undefined}
        data-joining={joining || undefined}
        style={node.w === undefined || node.h === undefined ? undefined : { width: node.w, height: node.h }}
      >
        {mine ? (
          // `nodrag nopan`: typing and selecting words must not move the box or the view.
          <div className="armada-scene-node__own nodrag nopan">
            <Textarea rows={2} value={node.body ?? ""} aria-label="The words in your box" onChange={(event) => onBody(event.target.value)} />
          </div>
        ) : node.kind === "wire" ? (
          <Wire node={node} />
        ) : node.kind === "label" ? (
          <span className="armada-scene-node__label">{node.title ?? node.body}</span>
        ) : (
          <>
            <Head node={node} />
            {node.body === undefined ? null : <pre className="armada-scene-node__body">{node.body}</pre>}
          </>
        )}
      </div>
      <Sides type="source" />
    </>
  );
}

function SceneEdgeView({ id, data, sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition }: EdgeProps<SceneRfEdge>) {
  const [path, labelX, labelY] = getBezierPath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition });
  if (data === undefined) return null;
  const { edge, change, dim, asked, mine, struck, onPress } = data;
  const said = `${mine ? "Yours, " : struck ? "Struck out, " : change === undefined ? "" : `${CHANGE_SAID[change]}, `}${edge.label ?? "Arrow"}`;
  return (
    <g className="armada-scene-edge" data-change={change} data-dim={dim || undefined} data-asked={asked || undefined} data-mine={mine || undefined} data-struck={struck || undefined} data-flow={edge.flow !== false || undefined}>
      <defs>
        <marker id={`head-${id}`} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
          <path className="armada-scene-edge__head" d="M0 0L10 5L0 10z" />
        </marker>
      </defs>
      <BaseEdge id={id} path={path} markerEnd={`url(#head-${id})`} className="armada-scene-edge__line" />
      {edge.label === undefined ? null : (
        <text className="armada-scene-edge__label" x={labelX} y={labelY}>
          {edge.label}
        </text>
      )}
      <path
        className="armada-scene-edge__hit"
        d={path}
        role="button"
        tabIndex={0}
        aria-label={said}
        onClick={() => onPress(edge.id)}
        onKeyDown={(event) => {
          if (event.key === "Enter" || event.key === " ") onPress(edge.id);
        }}
      />
    </g>
  );
}

const NODE_TYPES = { scene: SceneNodeView };
const EDGE_TYPES = { flow: SceneEdgeView };
const FIT = { padding: 0.1 };
const PAN = 80;

const reduced = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
const STEP_MS = 1400;

function Capture({ into, extent }: { into: { current: ReactFlowInstance | null }; extent: string }) {
  const flow = useReactFlow();
  // Fitted once the nodes are measured, because a fit made before them frames boxes of no size, and
  // again when the Drone's own scene grows or shrinks. The owner's boxes never move the view.
  const measured = useNodesInitialized();
  const seen = useRef<string | undefined>(undefined);
  useEffect(() => {
    if (!measured || seen.current === extent) return;
    const first = seen.current === undefined;
    seen.current = extent;
    void flow.fitView({ ...FIT, duration: first || reduced() ? 0 : 250 });
  }, [measured, extent, flow]);
  useEffect(() => {
    into.current = flow;
    return () => {
      into.current = null;
    };
  }, [flow, into]);
  return null;
}

const GROUND = new Set(["group", "lane"]);

/** What the caller built, over what React Flow kept per node: its size and whether it is picked. */
function overlay(kept: readonly SceneRfNode[], fresh: readonly SceneRfNode[]): SceneRfNode[] {
  const held = new Map(kept.map((one) => [one.id, one]));
  return fresh.map((one) => {
    const was = held.get(one.id);
    return was === undefined ? one : { ...was, ...one, measured: was.measured, selected: was.selected };
  });
}
const JOINS_THE_PICK = ["Meta", "Control"];

export function SketchScene({ scene: given, against, onAsk, marks, onMarks }: SketchSceneProps) {
  const parsed = useMemo(() => parseScene(given), [given]);
  const before = useMemo(() => (against === undefined ? undefined : parseScene(against)), [against]);
  const diff = useMemo(() => (parsed === undefined || before === undefined ? undefined : diffScenes(before, parsed)), [parsed, before]);
  const base: Scene | undefined = diff?.scene ?? parsed;
  const mark = marks ?? NO_MARKS;
  const composed = useMemo(() => (base === undefined ? undefined : withMarks(base, mark)), [base, mark]);
  const shown = composed?.scene;
  const [asked, setAsked] = useState<string | undefined>(undefined);
  const [playing, setPlaying] = useState<number | undefined>(undefined);
  const [pen, setPen] = useState(false);
  const [picked, setPicked] = useState<readonly string[]>([]);
  const [joining, setJoining] = useState<string | undefined>(undefined);
  const flow = useRef<ReactFlowInstance | null>(null);
  const figure = useRef<HTMLElement>(null);
  const steps = useMemo(() => (shown?.nodes ?? []).filter((one) => one.step !== undefined).sort((a, b) => (a.step ?? 0) - (b.step ?? 0)), [shown]);
  const extent = base === undefined ? "" : extentOf(base);

  useEffect(() => {
    if (playing === undefined || reduced()) return;
    const timer = setTimeout(() => setPlaying(playing + 1 < steps.length ? playing + 1 : undefined), STEP_MS);
    return () => clearTimeout(timer);
  }, [playing, steps.length]);

  const edit = (next: SceneMarks) => onMarks?.(next);
  const onBody = (id: string, body: string) => edit({ ...mark, nodes: mark.nodes.map((one) => (one.id === id ? { ...one, body } : one)) });
  const current = playing === undefined ? undefined : steps[playing]?.id;
  const fresh = useMemo<SceneRfNode[]>(
    () =>
      (shown?.nodes ?? []).map((node) => {
        const change = diff?.nodes.get(node.id);
        const mine = composed?.mine.has(node.id) ?? false;
        const struck = composed?.struck.has(node.id) ?? false;
        const name = mine ? (node.body?.trim() === "" || node.body === undefined ? "Your box" : `Your box: ${node.body}`) : partLabel(shown!, node.id);
        return {
          id: node.id,
          type: "scene",
          position: { x: node.x, y: node.y },
          data: {
            node,
            change,
            dim: (diff !== undefined && change === "same") || (current !== undefined && node.id !== current),
            current: node.id === current,
            asked: node.id === asked,
            mine,
            struck,
            joining: node.id === joining,
            onBody: (body: string) => onBody(node.id, body),
          },
          draggable: false,
          ...(GROUND.has(node.kind) ? { zIndex: -1 } : {}),
          ariaLabel: `${mine ? "" : struck ? "Struck out, " : change === undefined ? "" : `${CHANGE_SAID[change]}, `}${name}`,
        };
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `onBody` closes over `mark`, which `composed` already tracks
    [shown, diff, current, asked, joining, composed],
  );
  // **React Flow holds what a node measures and whether it is picked**, and a graph that is handed its
  // nodes anew each render keeps neither unless it applies the changes itself, as the pad does.
  const [kept, setKept] = useState<SceneRfNode[]>([]);
  const latest = useRef(fresh);
  latest.current = fresh;
  const nodes = useMemo(() => overlay(kept, fresh), [kept, fresh]);
  const onNodesChange = useCallback((changes: NodeChange<SceneRfNode>[]) => setKept((held) => applyNodeChanges(changes, overlay(held, latest.current))), []);
  const edges = useMemo<SceneRfEdge[]>(() => {
    const placed = new Map((shown?.nodes ?? []).map((node) => [node.id, { position: { x: node.x, y: node.y }, measured: { width: node.w ?? 200, height: node.h ?? 80 } }]));
    return (shown?.edges ?? []).map((edge) => {
      const change = diff?.edges.get(edge.id);
      return {
        id: edge.id,
        type: "flow",
        source: edge.from,
        target: edge.to,
        ...facingSides(placed.get(edge.from), placed.get(edge.to)),
        data: {
          edge,
          change,
          dim: (diff !== undefined && change === "same") || (current !== undefined && edge.to !== current),
          asked: edge.id === asked,
          mine: composed?.mine.has(edge.id) ?? false,
          struck: composed?.struck.has(edge.id) ?? false,
          onPress: (id: string) => setAsked((was) => (was === id ? undefined : id)),
        },
      };
    });
  }, [shown, diff, current, asked, composed]);

  if (shown === undefined || base === undefined) return <figure className="armada-sketch" role="group" aria-label="Sketch" />;

  const strokes = [...(shown.strokes ?? []), ...mark.strokes];
  const draw = (points: readonly SketchPoint[]) => edit({ ...mark, strokes: [...mark.strokes, { id: `i${String(mark.strokes.length + 1)}`, points }] });

  const onKeyDown = (event: KeyboardEvent) => {
    const target = event.target as HTMLElement;
    if (target instanceof HTMLTextAreaElement || target instanceof HTMLInputElement) return;
    const instance = flow.current;
    if (instance === null) return;
    const moved = (dx: number, dy: number) => {
      const at = instance.getViewport();
      void instance.setViewport({ ...at, x: at.x + dx, y: at.y + dy });
    };
    if (event.key === "+" || event.key === "=") void instance.zoomIn();
    else if (event.key === "-") void instance.zoomOut();
    else if (event.key === "0") void instance.fitView(FIT);
    else if (event.key === "ArrowLeft" && target.getAttribute("role") !== "button") moved(PAN, 0);
    else if (event.key === "ArrowRight" && target.getAttribute("role") !== "button") moved(-PAN, 0);
    else if (event.key === "ArrowUp" && target.getAttribute("role") !== "button") moved(0, PAN);
    else if (event.key === "ArrowDown" && target.getAttribute("role") !== "button") moved(0, -PAN);
  };

  /** Where a new box lands: the middle of what is in view, stepped to the nearest place no box already holds. */
  const middle = () => {
    const pane = figure.current?.querySelector(".react-flow")?.getBoundingClientRect();
    const instance = flow.current;
    if (pane === undefined || instance === null) return { x: 0, y: 0 };
    const centre = instance.screenToFlowPosition({ x: (pane.left + pane.right) / 2 - 120, y: (pane.top + pane.bottom) / 2 - 40 });
    const held = instance.getNodes().filter((one) => !GROUND.has((one.data as NodeData).node.kind)).map((one) => ({ x: one.position.x, y: one.position.y, w: one.measured?.width ?? 240, h: one.measured?.height ?? 90 }));
    const free = (at: { x: number; y: number }) => held.every((one) => at.x + 240 + 20 <= one.x || at.x >= one.x + one.w + 20 || at.y + 90 + 20 <= one.y || at.y >= one.y + one.h + 20);
    for (const dy of [0, 1, -1, 2, -2, 3, -3]) {
      for (const dx of [0, 1, -1, 2, -2]) {
        const at = { x: Math.round(centre.x + dx * 260), y: Math.round(centre.y + dy * 110) };
        if (free(at)) return at;
      }
    }
    return clearOf(held, centre, 40);
  };

  const press = (id: string) => {
    if (joining !== undefined && id !== joining) {
      edit(markJoin(base, mark, joining, id));
      setJoining(undefined);
      return;
    }
    // His own boxes are for editing and joining, not for asking the Drone about.
    setAsked((was) => (was === id || composed?.mine.has(id) ? undefined : id));
  };

  const acts: GraphCanvasRailAct[] = [
    ...(onMarks === undefined
      ? []
      : [
          { id: "draw", name: "Draw", icon: Pencil, pressed: pen, why: "Drag on the sketch to draw. Press again to stop.", onPress: () => setPen(!pen) },
          { id: "undo", name: "Undo", icon: Undo2, disabled: mark.strokes.length === 0, why: "Nothing has been drawn by hand.", onPress: () => edit({ ...mark, strokes: mark.strokes.slice(0, -1) }) },
          {
            id: "add",
            name: "Add a box",
            icon: SquarePlus,
            onPress: () => {
              setPen(false);
              setAsked(undefined);
              setJoining(undefined);
              const at = middle();
              edit({ ...mark, nodes: [...mark.nodes, { id: nextMarkId(mark), kind: "box", x: at.x, y: at.y, body: "" }] });
            },
          },
        ]),
    {
      id: "play",
      name: playing === undefined ? "Play the steps" : reduced() ? "Next step" : "Stop the steps",
      icon: playing !== undefined && !reduced() ? Pause : Play,
      pressed: playing !== undefined,
      disabled: steps.length === 0,
      why: "This sketch has no order to play.",
      onPress: () => setPlaying(playing === undefined ? 0 : reduced() ? (playing + 1 < steps.length ? playing + 1 : undefined) : undefined),
    },
  ];

  const part = asked === undefined ? undefined : { id: asked, label: partLabel(shown, asked) };
  const isEdge = part !== undefined && shown.edges.some((one) => one.id === part.id);
  const lifted = picked.length > 0 && picked.every((id) => composed?.struck.has(id));

  return (
    // eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- the keys below act on the canvas it holds
    <figure ref={figure} className="armada-sketch" role="group" aria-label="Sketch" aria-keyshortcuts="+ - 0 ArrowLeft ArrowRight ArrowUp ArrowDown" tabIndex={0} onKeyDown={onKeyDown}>
      <GraphCanvas<SceneRfNode, SceneRfEdge>
        surface="armada-sketch-scene"
        label="Drawing"
        nodes={nodes}
        edges={edges}
        nodeTypes={NODE_TYPES}
        edgeTypes={EDGE_TYPES}
        onNodesChange={onNodesChange}
        fitViewOptions={FIT}
        minZoom={0.2}
        multiSelectionKeyCode={JOINS_THE_PICK}
        onSelectionChange={setPicked}
        onNodePress={press}
        onPanePress={() => {
          setAsked(undefined);
          setJoining(undefined);
        }}
        rail={<GraphCanvasRailGroup label="What you can do with the sketch" acts={acts} />}
        aside={
          part === undefined || onAsk === undefined ? undefined : (
            <AskBox
              key={part.id}
              part={part}
              onAsk={onAsk}
              onClose={() => setAsked(undefined)}
              {...(onMarks === undefined || !isEdge
                ? {}
                : {
                    onRemove: () => {
                      edit(markRemove(mark, [part.id]));
                      setAsked(undefined);
                    },
                    removal: composed?.struck.has(part.id) ? "Put back" : "Remove",
                  })}
            />
          )
        }
      >
        <Capture into={flow} extent={extent} />
        <Ink strokes={strokes} pen={pen && onMarks !== undefined} onDraw={draw} />
        {onMarks === undefined || pen || picked.length === 0 ? null : (
          <GraphCanvasNodeBar label="What you can do with what you picked" nodeIds={picked}>
            <GraphCanvasNodeAct
              name="Join"
              disabled={picked.length > 2}
              why="Pick one box or two to join them."
              onPress={() => {
                if (picked.length === 2) edit(markJoin(base, mark, picked[0]!, picked[1]!));
                else setJoining(picked[0]);
              }}
            />
            <GraphCanvasNodeAct
              name={lifted ? "Put back" : "Remove"}
              icon={lifted ? Undo2 : Trash2}
              danger={!lifted}
              onPress={() => edit(markRemove(mark, picked))}
            />
          </GraphCanvasNodeBar>
        )}
      </GraphCanvas>
    </figure>
  );
}

/** A small box to ask about the part pressed. Enter sends; Shift and Enter is a new line. */
function AskBox({
  part,
  onAsk,
  onClose,
  onRemove,
  removal,
}: {
  part: { id: string; label: string };
  onAsk: NonNullable<SketchSceneProps["onAsk"]>;
  onClose: () => void;
  onRemove?: () => void;
  removal?: string;
}) {
  const [said, setSaid] = useState("");
  const send = () => {
    if (said.trim() === "") return;
    onAsk(part, said.trim());
    onClose();
  };
  return (
    <form
      className="armada-scene-ask"
      aria-label={`Ask about ${part.label}`}
      onSubmit={(event) => {
        event.preventDefault();
        send();
      }}
    >
      <div className="armada-scene-ask__head">
        <span className="armada-scene-ask__about">{part.label}</span>
        <span className="armada-scene-ask__acts">
          {onRemove === undefined || removal === undefined ? null : (
            <Tooltip label={removal}>
              <Button variant="ghost" size="sm" iconOnly aria-label={removal} onClick={onRemove} type="button">
                {removal === "Remove" ? <Trash2 size={16} strokeWidth={2} aria-hidden /> : <Undo2 size={16} strokeWidth={2} aria-hidden />}
              </Button>
            </Tooltip>
          )}
          <Tooltip label="Close">
            <Button variant="ghost" size="sm" aria-label="Close ask" onClick={onClose} type="button">
              <X size={16} strokeWidth={2} aria-hidden />
            </Button>
          </Tooltip>
        </span>
      </div>
      <Textarea
        label={`Ask about ${part.label}`}
        rows={2}
        value={said}
        autoFocus
        onChange={(event) => setSaid(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && !event.shiftKey) {
            event.preventDefault();
            send();
          }
        }}
      />
      <Button size="sm" type="submit" disabled={said.trim() === ""}>
        Ask
      </Button>
    </form>
  );
}
