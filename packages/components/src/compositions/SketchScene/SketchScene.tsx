import { BaseEdge, Handle, getBezierPath, useNodesInitialized, useReactFlow, type Edge, type EdgeProps, type Node, type NodeProps, type ReactFlowInstance } from "@xyflow/react";
import {
  Activity, Bot, Box, Clock, Cpu, Eye, File, GitBranch, GitMerge, Globe, Hammer, HardDrive, Layers, Lock, Package, Pause, Pencil, Play, Rocket, Scale,
  ScrollText, Server, Settings, ShieldCheck, Terminal, Undo2, Webhook, Wrench, X, Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { GRAPH_CANVAS_SIDES, GraphCanvas, facingSides } from "../GraphCanvas/GraphCanvas";
import { GraphCanvasRailGroup, type GraphCanvasRailAct } from "../GraphCanvas/GraphCanvasRail";
import { Ink, type SketchPoint, type SketchStroke } from "../SketchPad/Ink";
import { diffScenes, parseScene, partLabel, type Scene, type SceneChange, type SceneEdge, type SceneIcon, type SceneNode } from "./scene";

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
  /** What the owner drew on top, kept by the host. Absent draws no pen. */
  ink?: readonly SketchStroke[];
  onInk?: (ink: readonly SketchStroke[]) => void;
};

const ICON: Record<SceneIcon, LucideIcon> = {
  activity: Activity, bot: Bot, box: Box, clock: Clock, cpu: Cpu, eye: Eye, file: File, "git-branch": GitBranch, "git-merge": GitMerge, globe: Globe,
  hammer: Hammer, "hard-drive": HardDrive, layers: Layers, lock: Lock, package: Package, rocket: Rocket, scale: Scale, "scroll-text": ScrollText,
  server: Server, settings: Settings, "shield-check": ShieldCheck, terminal: Terminal, webhook: Webhook, wrench: Wrench, zap: Zap,
};

const CHANGE_SAID: Record<SceneChange, string> = { added: "Added", removed: "Removed", changed: "Changed", same: "Unchanged" };

type NodeData = { node: SceneNode; change: SceneChange | undefined; dim: boolean; current: boolean; asked: boolean };
type SceneRfNode = Node<NodeData, "scene">;
type EdgeData = { edge: SceneEdge; change: SceneChange | undefined; dim: boolean; asked: boolean; onPress: (id: string) => void };
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
  const { node, change, dim, current, asked } = data;
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
        style={node.w === undefined || node.h === undefined ? undefined : { width: node.w, height: node.h }}
      >
        {node.kind === "wire" ? (
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
  const { edge, change, dim, asked, onPress } = data;
  const said = `${change === undefined ? "" : `${CHANGE_SAID[change]}, `}${edge.label ?? "Arrow"}`;
  return (
    <g className="armada-scene-edge" data-change={change} data-dim={dim || undefined} data-asked={asked || undefined} data-flow={edge.flow !== false || undefined}>
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

function Capture({ into }: { into: { current: ReactFlowInstance | null } }) {
  const flow = useReactFlow();
  // Fitted once the nodes are measured, because a fit made before them frames boxes of no size.
  const measured = useNodesInitialized();
  const fitted = useRef(false);
  useEffect(() => {
    if (!measured || fitted.current) return;
    fitted.current = true;
    void flow.fitView(FIT);
  }, [measured, flow]);
  useEffect(() => {
    into.current = flow;
    return () => {
      into.current = null;
    };
  }, [flow, into]);
  return null;
}

const GROUND = new Set(["group", "lane"]);

export function SketchScene({ scene: given, against, onAsk, ink, onInk }: SketchSceneProps) {
  const parsed = useMemo(() => parseScene(given), [given]);
  const before = useMemo(() => (against === undefined ? undefined : parseScene(against)), [against]);
  const diff = useMemo(() => (parsed === undefined || before === undefined ? undefined : diffScenes(before, parsed)), [parsed, before]);
  const shown: Scene | undefined = diff?.scene ?? parsed;
  const [asked, setAsked] = useState<string | undefined>(undefined);
  const [playing, setPlaying] = useState<number | undefined>(undefined);
  const [pen, setPen] = useState(false);
  const flow = useRef<ReactFlowInstance | null>(null);
  const steps = useMemo(() => (shown?.nodes ?? []).filter((one) => one.step !== undefined).sort((a, b) => (a.step ?? 0) - (b.step ?? 0)), [shown]);

  useEffect(() => {
    if (playing === undefined || reduced()) return;
    const timer = setTimeout(() => setPlaying(playing + 1 < steps.length ? playing + 1 : undefined), STEP_MS);
    return () => clearTimeout(timer);
  }, [playing, steps.length]);

  const current = playing === undefined ? undefined : steps[playing]?.id;
  const nodes = useMemo<SceneRfNode[]>(
    () =>
      (shown?.nodes ?? []).map((node) => {
        const change = diff?.nodes.get(node.id);
        return {
          id: node.id,
          type: "scene",
          position: { x: node.x, y: node.y },
          data: { node, change, dim: (diff !== undefined && change === "same") || (current !== undefined && node.id !== current), current: node.id === current, asked: node.id === asked },
          draggable: false,
          ...(GROUND.has(node.kind) ? { zIndex: -1 } : {}),
          ariaLabel: `${change === undefined ? "" : `${CHANGE_SAID[change]}, `}${partLabel(shown!, node.id)}`,
        };
      }),
    [shown, diff, current, asked],
  );
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
        data: { edge, change, dim: (diff !== undefined && change === "same") || (current !== undefined && edge.to !== current), asked: edge.id === asked, onPress: (id: string) => setAsked((was) => (was === id ? undefined : id)) },
      };
    });
  }, [shown, diff, current, asked]);

  if (shown === undefined) return <figure className="armada-sketch" role="group" aria-label="Sketch" />;

  const strokes = [...(shown.strokes ?? []), ...(ink ?? [])];
  const draw = (points: readonly SketchPoint[]) => onInk?.([...(ink ?? []), { id: `i${String((ink ?? []).length + 1)}`, points }]);

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

  const acts: GraphCanvasRailAct[] = [
    ...(onInk === undefined
      ? []
      : [
          { id: "draw", name: "Draw", icon: Pencil, pressed: pen, why: "Drag on the sketch to draw. Press again to stop.", onPress: () => setPen(!pen) },
          { id: "undo", name: "Undo", icon: Undo2, disabled: (ink ?? []).length === 0, why: "Nothing has been drawn by hand.", onPress: () => onInk([...(ink ?? [])].slice(0, -1)) },
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

  return (
    // eslint-disable-next-line jsx-a11y/no-noninteractive-tabindex -- the keys below act on the canvas it holds
    <figure className="armada-sketch" role="group" aria-label="Sketch" aria-keyshortcuts="+ - 0 ArrowLeft ArrowRight ArrowUp ArrowDown" tabIndex={0} onKeyDown={onKeyDown}>
      <GraphCanvas<SceneRfNode, SceneRfEdge>
        surface="armada-sketch-scene"
        label="Drawing"
        nodes={nodes}
        edges={edges}
        nodeTypes={NODE_TYPES}
        edgeTypes={EDGE_TYPES}
        fitViewOptions={FIT}
        minZoom={0.2}
        onNodePress={(id) => setAsked((was) => (was === id ? undefined : id))}
        onPanePress={() => setAsked(undefined)}
        rail={<GraphCanvasRailGroup label="What you can do with the sketch" acts={acts} />}
        aside={part === undefined || onAsk === undefined ? undefined : <AskBox key={part.id} part={part} onAsk={onAsk} onClose={() => setAsked(undefined)} />}
      >
        <Capture into={flow} />
        <Ink strokes={strokes} pen={pen && onInk !== undefined} onDraw={draw} />
      </GraphCanvas>
    </figure>
  );
}

/** A small box to ask about the part pressed. Enter sends; Shift and Enter is a new line. */
function AskBox({ part, onAsk, onClose }: { part: { id: string; label: string }; onAsk: NonNullable<SketchSceneProps["onAsk"]>; onClose: () => void }) {
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
        <Tooltip label="Close">
          <Button variant="ghost" size="sm" aria-label="Close ask" onClick={onClose} type="button">
            <X size={16} strokeWidth={2} aria-hidden />
          </Button>
        </Tooltip>
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
