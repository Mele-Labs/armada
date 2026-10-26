import { BaseEdge, Handle, applyNodeChanges, useNodesInitialized, useReactFlow, useStore, type Edge, type EdgeProps, type Node, type NodeChange, type NodeProps, type ReactFlowInstance } from "@xyflow/react";
import {
  Activity, Bot, Box, Clock, Cpu, Eye, File, GitBranch, GitMerge, Globe, Hammer, HardDrive, Layers, Lock, Package, Pause, Pencil, Play, Rocket, RotateCw, Scale,
  ExternalLink, ScrollText, Server, Settings, ShieldCheck, SquarePlus, Terminal, Trash2, Undo2, Webhook, Wrench, X, Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent, type ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";

import { Button } from "../../primitives/Button/Button";
import type { DropdownMenuEntry } from "../../primitives/DropdownMenu/DropdownMenu";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { GRAPH_CANVAS_SIDES, GraphCanvas, clearOf } from "../GraphCanvas/GraphCanvas";
import { GraphCanvasNodeAct, GraphCanvasNodeBar, GraphCanvasRailGroup, type GraphCanvasRailAct } from "../GraphCanvas/GraphCanvasRail";
import { Ink, type SketchPoint } from "../SketchPad/Ink";
import type { DocumentPlace } from "../ViewerNotes/compose";
import { NoteNumber, useViewerNotes, type ViewerNotesScope } from "../ViewerNotes/ViewerNotes";
import { labelSize, placeLabels, type LabelMetrics, type LabelPlace, type Rect } from "./edge-labels";
import { sketchHtml } from "./export-html";
import { sceneMermaid } from "./export-mermaid";
import { screenMarkup, screenStyles } from "./export-screen";
import { DEFAULT_METRICS, EXPORT_TOKENS, SANS, frameOf, sketchSvg, stepPlan, type ExportGeometry, type ExportMetrics, type SketchSvgInput } from "./export-svg";
import { MERMAID_COPIED, savedSaid, sketchFileName, type SketchShareFile } from "./share";
import { copyWith, saveWith, svgToPng, useSketchShareHost } from "./SketchShare";
import { roundedPath, routeAll } from "./route";
import { holdsStep, screenGuess, screenSteps, screenWidth } from "./screen";
import { settle } from "./settle";
import { ScreenView } from "./ScreenView";
import {
  CHANGE_SAID, NO_MARKS, SCENE_COLORS, cssMs, diffScenes, markJoin, markRemove, nextMarkId, parseScene, partLabel, withMarks,
  type Scene, type SceneChange, type SceneEdge, type SceneIcon, type SceneMarks, type SceneNode, type SceneShape,
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
  /** What the sketch is called: the name of a file it is saved as, and the heading of the page it is shared as. */
  title?: string;
  /** A press on a part opens a small ask about it; this hears what was asked. */
  onAsk?: (about: { id: string; label: string }, said: string) => void;
  /**
   * What the owner did to it, kept by the host: lines, boxes and joins of his, and parts of the
   * Drone's he struck out. His parts are drawn in his colour. Absent offers no tools.
   */
  marks?: SceneMarks;
  onMarks?: (marks: SceneMarks) => void;
  /**
   * The same drawing shrunk into whatever box it is given, for a card: no rail, no keys, no steps, no
   * asking, and nothing in it to press or tab to. It is fitted again when its box changes size.
   */
  preview?: boolean;
  /**
   * Plays the steps as it opens and loops them, from the first again each time the scene is a different one. With
   * reduced motion it plays nothing, and the rail's press steps one at a time.
   */
  autoplay?: boolean;
  /** For a `preview`: the steps loop while this is true and the drawing is still while it is not. Never with reduced motion. */
  playing?: boolean;
  /**
   * Where notes on the sketch are held, and the Job they go to. A right press on a part, or on the parts a Shift drag
   * boxed, offers Add note; each held note's number stands on its parts. Absent takes none, and Shift drag is React Flow's.
   */
  notes?: ViewerNotesScope;
  /** Set where the sketch is embedded in a Job's document: a note on its parts is then about that place in the page. */
  within?: DocumentPlace;
  /** Parts a sent note named, by id: picked when it opens, so they stand out and a right press notes them again. */
  marked?: readonly string[];
  /**
   * Filled with a function that draws the sketch as it stands as static SVG markup, the way Save as web page does and with the theme's values
   * written in. For a host that builds a file of its own around the sketch. Emptied when the sketch goes; the function gives nothing where the scene does not draw.
   */
  exporter?: { current: (() => string | undefined) | null };
};

const ICON: Record<SceneIcon, LucideIcon> = {
  activity: Activity, bot: Bot, box: Box, clock: Clock, cpu: Cpu, eye: Eye, file: File, "git-branch": GitBranch, "git-merge": GitMerge, globe: Globe,
  hammer: Hammer, "hard-drive": HardDrive, layers: Layers, lock: Lock, package: Package, rocket: Rocket, scale: Scale, "scroll-text": ScrollText,
  server: Server, settings: Settings, "shield-check": ShieldCheck, terminal: Terminal, webhook: Webhook, wrench: Wrench, zap: Zap,
};

type NodeData = {
  node: SceneNode;
  change: SceneChange | undefined;
  dim: boolean;
  current: boolean;
  /** The step playing, for a screen that holds it in one of its elements. */
  inner: number | undefined;
  asked: boolean;
  mine: boolean;
  struck: boolean;
  joining: boolean;
  /** The notes on it: held, and the one being written. */
  notes: readonly { n: number; writing: boolean; said: string }[];
  /** Where a group or a lane was grown to, round the boxes it holds once they are moved apart. */
  frame: { w: number; h: number } | undefined;
  onBody: (body: string) => void;
};
type SceneRfNode = Node<NodeData, "scene">;
type EdgeData = { path: string; place: LabelPlace | undefined; edge: SceneEdge; change: SceneChange | undefined; dim: boolean; active: boolean; still: boolean; asked: boolean; mine: boolean; struck: boolean; onPress: (id: string) => void };
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

/** The outlines a box's frame cannot draw: drawn in a 100-unit square stretched to the box. A pill and a circle are the frame's own corners. */
const OUTLINE: Partial<Record<SceneShape, ReactNode>> = {
  diamond: <polygon points="50,0 100,50 50,100 0,50" />,
  hexagon: <polygon points="12,0 88,0 100,50 88,100 12,100 0,50" />,
  cylinder: (
    <>
      <path d="M0 10 A50 10 0 0 1 100 10 V90 A50 10 0 0 1 0 90 Z" />
      <path d="M0 10 A50 10 0 0 0 100 10" />
    </>
  ),
};

/** A box in an outline of its own: no title band, its glyph, title and words centred. */
function Shaped({ node }: { node: SceneNode }) {
  const Glyph = node.icon === undefined ? undefined : ICON[node.icon];
  const outline = node.shape === undefined ? undefined : OUTLINE[node.shape];
  return (
    <>
      {outline === undefined ? null : (
        <svg className="armada-scene-node__outline" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden>
          {outline}
        </svg>
      )}
      <div className="armada-scene-node__shaped">
        {node.title === undefined && Glyph === undefined ? null : (
          <span className="armada-scene-node__name">
            {Glyph === undefined || node.icon === undefined ? null : (
              <Tooltip label={sentence(node.icon)}>
                <span className="armada-scene-node__icon" role="img" aria-label={sentence(node.icon)}>
                  <Glyph size={12} strokeWidth={2} aria-hidden />
                </span>
              </Tooltip>
            )}
            {node.title}
          </span>
        )}
        {node.body === undefined ? null : <pre className="armada-scene-node__body">{node.body}</pre>}
      </div>
    </>
  );
}

function SceneNodeView({ data }: NodeProps<SceneRfNode>) {
  const { node, change, dim, current, inner, asked, mine, struck, joining, notes, frame, onBody } = data;
  const shape = node.kind === "box" && !mine && node.shape !== "box" ? node.shape : undefined;
  return (
    <>
      <Sides type="target" />
      <div
        className="armada-scene-node"
        data-kind={node.kind}
        data-wire={node.wire}
        data-shape={shape}
        data-color={node.color}
        data-change={change}
        data-dim={dim || undefined}
        data-current={current || undefined}
        data-asked={asked || undefined}
        data-mine={mine || undefined}
        data-struck={struck || undefined}
        data-joining={joining || undefined}
        // **`w` and `h` are floors, not clips**: a box grows to hold its words. A group and a lane have none, so take the size they were grown to.
        style={
          node.screen !== undefined
            ? { width: node.w ?? screenWidth(node.screen) }
            : GROUND.has(node.kind)
              ? { width: frame?.w ?? node.w, height: frame?.h ?? node.h }
              : { ...(node.w === undefined ? {} : { minWidth: node.w }), ...(node.h === undefined ? {} : { minHeight: node.h }) }
        }
      >
        {mine ? (
          // `nodrag nopan`: typing and selecting words must not move the box or the view.
          <div className="armada-scene-node__own nodrag nopan">
            <Textarea rows={2} value={node.body ?? ""} aria-label="The words in your box" onChange={(event) => onBody(event.target.value)} />
          </div>
        ) : node.screen !== undefined ? (
          <>
            {node.title === undefined ? null : <span className="armada-scene-node__caption">{node.title}</span>}
            <ScreenView screen={node.screen} at={inner} />
          </>
        ) : node.kind === "wire" ? (
          <Wire node={node} />
        ) : shape !== undefined ? (
          <Shaped node={node} />
        ) : node.kind === "label" ? (
          <span className="armada-scene-node__label">{node.title ?? node.body}</span>
        ) : (
          <>
            <Head node={node} />
            {node.body === undefined ? null : <pre className="armada-scene-node__body">{node.body}</pre>}
          </>
        )}
      </div>
      {notes.length === 0 ? null : (
        <span className="armada-scene-node__notes">
          {notes.map((one) => (
            <NoteNumber key={one.n} n={one.n} writing={one.writing} said={one.said} />
          ))}
        </span>
      )}
      <Sides type="source" />
    </>
  );
}

/** An arrow on the way `route` found for it, which `SketchScene` works out from where the boxes settled. */
function SceneEdgeView({ id, data }: EdgeProps<SceneRfEdge>) {
  if (data === undefined) return null;
  const { path, edge, place, change, dim, active, still, asked, mine, struck, onPress } = data;
  const said = `${mine ? "Yours, " : struck ? "Struck out, " : change === undefined ? "" : `${CHANGE_SAID[change]}, `}${edge.label ?? "Arrow"}`;
  const at = place;
  return (
    <g className="armada-scene-edge" data-color={edge.color} data-change={change} data-dim={dim || undefined} data-active={active || undefined} data-asked={asked || undefined} data-mine={mine || undefined} data-struck={struck || undefined} data-still={still || undefined} data-flow={edge.flow !== false || undefined}>
      <defs>
        <marker id={`head-${id}`} viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse">
          <path className="armada-scene-edge__head" d="M0 0L10 5L0 10z" />
        </marker>
      </defs>
      <BaseEdge id={id} path={path} markerEnd={`url(#head-${id})`} className="armada-scene-edge__line" />
      {active ? <circle className="armada-scene-edge__dot" style={{ offsetPath: `path("${path}")` }} /> : null}
      {at === undefined || edge.label === undefined ? null : (
        // The words may be cut to fit, so the arrow's name carries all of them.
        <g className="armada-scene-edge__words" role="img" aria-label={edge.label}>
          <rect className="armada-scene-edge__backing" x={at.x - at.w / 2} y={at.y - at.h / 2} width={at.w} height={at.h} />
          {at.lines.map((line, index) => (
            <text key={line + String(index)} className="armada-scene-edge__label" x={at.x} y={at.y - at.h / 2 + at.pad + (index + 0.5) * at.lineH}>
              {line}
            </text>
          ))}
        </g>
      )}
      {still ? null : (
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
      )}
    </g>
  );
}

const NODE_TYPES = { scene: SceneNodeView };
const EDGE_TYPES = { flow: SceneEdgeView };
// **Never past full size**: React Flow's own ceiling is 2, so a sketch of three boxes was fitted at twice the size of its words.
const FIT = { padding: 0.1, maxZoom: 1 };
const PREVIEW_FIT = { padding: 0.04, maxZoom: 1 };
const PAN = 80;

const tokenPx = (name: string, fallback: number) => {
  const read = typeof document === "undefined" ? NaN : Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name));
  return Number.isNaN(read) ? fallback : read;
};
/** The label's own type, read from the tokens it is drawn in. */
const labelMetrics = (): LabelMetrics => ({ charW: tokenPx("--text-xs", 14) * SANS, lineH: tokenPx("--leading-xs", 20), pad: tokenPx("--space-1", 4) });
/** The room kept between boxes, and the gutters the arrows run in. */
const gapPx = () => tokenPx("--space-8", 32);

const SHARE_ENTRIES: DropdownMenuEntry[] = [
  { kind: "item", id: "html", label: "Save as web page" },
  { kind: "item", id: "image", label: "Save as image" },
  { kind: "item", id: "mermaid", label: "Copy as Mermaid" },
];

const reduced = () => typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
/** A step is as long as the pulse token: the dot crosses its edge in most of it and the rest holds the node lit. */
const pulseMs = () => {
  const read = typeof document === "undefined" ? undefined : cssMs(getComputedStyle(document.documentElement).getPropertyValue("--duration-pulse"));
  return read ?? 1600;
};

function Capture({ into, extent, preview }: { into: { current: ReactFlowInstance | null }; extent: string; preview: boolean }) {
  const flow = useReactFlow();
  // A preview sits in a box that changes size as the window and its neighbours do, and keeps fitting it.
  const box = useStore((state) => (preview ? `${String(state.width)}x${String(state.height)}` : ""));
  // Fitted once the nodes are measured and moved apart, because a fit made before frames boxes of no size
  // where they will not stay, and again when the Drone's own scene grows or shrinks. The owner's boxes never move the view.
  const measured = useNodesInitialized();
  const seen = useRef<string | undefined>(undefined);
  useEffect(() => {
    const wanted = `${extent}|${box}`;
    if (!measured || extent === "" || seen.current === wanted) return;
    const first = seen.current === undefined;
    seen.current = wanted;
    void flow.fitView({ ...(preview ? PREVIEW_FIT : FIT), duration: first || preview || reduced() ? 0 : 250 });
  }, [measured, extent, box, preview, flow]);
  useEffect(() => {
    into.current = flow;
    return () => {
      into.current = null;
    };
  }, [flow, into]);
  return null;
}

const GROUND = new Set(["group", "lane"]);

/** A body's words as the browser broke them into lines: which line each character fell on, read from where it was drawn. */
function linesOf(body: HTMLElement): string[] | undefined {
  const text = body.firstChild;
  if (!(text instanceof Text)) return undefined;
  const lines: string[] = [];
  const range = document.createRange();
  let line = "";
  let top = Number.NaN;
  for (let at = 0; at < text.length; at += 1) {
    const char = text.data.charAt(at);
    if (char === "\n") {
      lines.push(line);
      line = "";
      top = Number.NaN;
      continue;
    }
    range.setStart(text, at);
    range.setEnd(text, at + 1);
    const rect = range.getClientRects()[0];
    if (rect !== undefined && !Number.isNaN(top) && rect.top - top > rect.height / 2) {
      lines.push(line.trimEnd());
      line = "";
      top = Number.NaN;
    }
    if (rect !== undefined && Number.isNaN(top)) top = rect.top;
    line += char;
  }
  lines.push(line.trimEnd());
  return lines;
}

/** What the caller built, over what React Flow kept per node: its size and whether it is picked. */
function overlay(kept: readonly SceneRfNode[], fresh: readonly SceneRfNode[]): SceneRfNode[] {
  const held = new Map(kept.map((one) => [one.id, one]));
  return fresh.map((one) => {
    const was = held.get(one.id);
    return was === undefined ? one : { ...was, ...one, measured: was.measured, selected: was.selected };
  });
}
const JOINS_THE_PICK = ["Meta", "Control"];
const NO_NOTES: NodeData["notes"] = [];

export function SketchScene({ scene: given, against, title, onAsk, marks, onMarks, preview = false, autoplay = false, playing = false, notes: noteScope, within, marked, exporter }: SketchSceneProps) {
  const parsed = useMemo(() => parseScene(given), [given]);
  const before = useMemo(() => (against === undefined ? undefined : parseScene(against)), [against]);
  const diff = useMemo(() => (parsed === undefined || before === undefined ? undefined : diffScenes(before, parsed)), [parsed, before]);
  const base: Scene | undefined = diff?.scene ?? parsed;
  const mark = marks ?? NO_MARKS;
  const composed = useMemo(() => (base === undefined ? undefined : withMarks(base, mark)), [base, mark]);
  const shown = composed?.scene;
  const [asked, setAsked] = useState<string | undefined>(undefined);
  const [stepAt, setStepAt] = useState<number | undefined>(undefined);
  const [pen, setPen] = useState(false);
  const [picked, setPicked] = useState<readonly string[]>([]);
  const [joining, setJoining] = useState<string | undefined>(undefined);
  const flow = useRef<ReactFlowInstance | null>(null);
  const figure = useRef<HTMLElement>(null);
  const host = useSketchShareHost();
  const drawnAs = useRef<(() => string) | undefined>(undefined);
  useEffect(() => {
    if (exporter === undefined) return undefined;
    exporter.current = () => drawnAs.current?.();
    return () => {
      exporter.current = null;
    };
  }, [exporter]);
  const viewer = useViewerNotes(preview ? undefined : noteScope, figure);
  // A Shift drag's box, in the figure's own pixels, while it is drawn.
  const [boxing, setBoxing] = useState<{ x0: number; y0: number; x1: number; y1: number } | undefined>(undefined);
  // Where it started, read by the moves and the release however soon they come after the press.
  const boxFrom = useRef<{ x: number; y: number } | undefined>(undefined);
  // Each part's notes, by its id.
  const noted = useMemo(() => {
    const by = new Map<string, { n: number; writing: boolean; said: string }[]>();
    for (const one of viewer.marks) {
      const parts = one.about.kind === "sketch" ? one.about.parts : one.about.kind === "document" && one.about.target.kind === "part" && one.about.target.sketch === (title ?? "Sketch") ? one.about.target.parts : [];
      for (const part of parts) by.set(part.id, [...(by.get(part.id) ?? []), { n: one.n, writing: one.writing, said: one.text }]);
    }
    return by;
  }, [viewer.marks, title]);
  // **A step is a number a node, an arrow or an element of a screen carries**, and what shares a number plays together.
  const steps = useMemo(
    () =>
      [
        ...new Set(
          [
            ...(shown?.nodes ?? []).flatMap((one) => [one.step, ...(one.screen === undefined ? [] : screenSteps(one.screen))]),
            ...(shown?.edges ?? []).map((one) => one.step),
          ].filter((one): one is number => one !== undefined),
        ),
      ].sort((a, b) => a - b),
    [shown],
  );
  const loops = autoplay || (preview && playing);
  const sceneKey = useMemo(() => JSON.stringify(given), [given]);

  // Opening on a scene starts its steps where the host asks for it, and a different scene starts over.
  const first = steps.length > 0 ? steps[0] : undefined;
  useEffect(() => {
    setStepAt(loops && first !== undefined && !reduced() ? 0 : undefined);
  }, [loops, first, sceneKey]);

  useEffect(() => {
    if (stepAt === undefined || reduced()) return;
    const timer = setTimeout(() => setStepAt(stepAt + 1 < steps.length ? stepAt + 1 : loops ? 0 : undefined), pulseMs());
    return () => clearTimeout(timer);
  }, [stepAt, steps.length, loops]);

  const edit = (next: SceneMarks) => onMarks?.(next);
  const onBody = (id: string, body: string) => edit({ ...mark, nodes: mark.nodes.map((one) => (one.id === id ? { ...one, body } : one)) });
  const at = stepAt === undefined ? undefined : steps[stepAt];
  // What this step lights: its own nodes, and where its arrows go.
  const lit = useMemo(() => {
    const ids = new Set<string>();
    if (at === undefined) return ids;
    for (const one of shown?.nodes ?? []) if (one.step === at) ids.add(one.id);
    for (const one of shown?.edges ?? []) if (one.step === at) ids.add(one.to);
    return ids;
  }, [at, shown]);
  // **React Flow holds what a node measures and whether it is picked**, and a graph that is handed its
  // nodes anew each render keeps neither unless it applies the changes itself, as the pad does.
  const [kept, setKept] = useState<SceneRfNode[]>([]);
  const sizes = kept.map((one) => `${one.id}:${String(one.measured?.width)}x${String(one.measured?.height)}`).join("|");
  // Where every box and arrow goes, from what the boxes measured: the Drone's boxes moved apart until none
  // touches another, and each arrow routed round them. Keyed on their sizes, so a step lighting a node moves nothing.
  const layout = useMemo(() => {
    const measured = new Map(kept.map((one) => [one.id, one.measured]));
    const gap = gapPx();
    const all = shown?.nodes ?? [];
    const sizeOf = (node: SceneNode) => {
      if (GROUND.has(node.kind)) return { w: node.w ?? 0, h: node.h ?? 0 };
      const held = measured.get(node.id);
      const guess = node.screen === undefined ? { w: node.w ?? 240, h: node.h ?? 90 } : screenGuess(node.screen, node.w);
      return { w: held?.width ?? guess.w, h: held?.height ?? guess.h };
    };
    // Settled once every box has a size of its own; until then they stand where the Drone put them.
    // The owner's own boxes stay where he put them.
    const ready = all.length > 0 && all.every((one) => GROUND.has(one.kind) || measured.get(one.id)?.width !== undefined);
    const drones = all.filter((one) => !(composed?.mine.has(one.id) ?? false));
    const metrics = labelMetrics();
    // An arrow with words keeps room for them between the two boxes it joins.
    const joins = (shown?.edges ?? []).flatMap((one) => (one.label === undefined || one.label.trim() === "" ? [] : [{ from: one.from, to: one.to, ...labelSize(one.label, metrics) }]));
    const rects = new Map<string, Rect>(ready ? settle(drones.map((one) => ({ id: one.id, x: one.x, y: one.y, ...sizeOf(one), ground: GROUND.has(one.kind) })), gap, joins) : []);
    for (const one of all) if (!rects.has(one.id)) rects.set(one.id, { x: one.x, y: one.y, ...sizeOf(one) });
    const solid = all.filter((one) => !GROUND.has(one.kind));
    const ids = (keep: (one: SceneNode) => boolean) => new Set(all.filter(keep).map((one) => one.id));
    const ways = routeAll(rects, shown?.edges ?? [], gap, ids((one) => GROUND.has(one.kind)), ids((one) => one.shape !== undefined && one.shape !== "box" && one.shape !== "cylinder"));
    const drawn = (shown?.edges ?? []).flatMap((edge) => {
      const way = ways.get(edge.id);
      return way === undefined ? [] : [{ id: edge.id, label: edge.label, path: roundedPath(way, gap / 4) }];
    });
    // Where each arrow ends: its head, which no arrow's words may cover.
    const heads = [...ways.values()].map((way) => ({ x: way[way.length - 1]!.x - gap / 4, y: way[way.length - 1]!.y - gap / 4, w: gap / 2, h: gap / 2 }));
    const places = placeLabels(drawn, solid.map((one) => rects.get(one.id)!), metrics, heads);
    // What the view is fitted to: the Drone's boxes where they settled, rounded so a pixel's change does not refit.
    const round = (n: number) => String(Math.round(n / 20) * 20);
    const ours = drones.map((one) => rects.get(one.id)!);
    const extent = !ready || ours.length === 0 ? "" : [Math.min(...ours.map((r) => r.x)), Math.min(...ours.map((r) => r.y)), Math.max(...ours.map((r) => r.x + r.w)), Math.max(...ours.map((r) => r.y + r.h))].map(round).join(",");
    return { extent, rects, paths: new Map(drawn.map((one) => [one.id, one.path])), places };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `sizes` stands for what `kept` measured
  }, [shown, composed, sizes]);
  const places = layout.places;
  const fresh = useMemo<SceneRfNode[]>(
    () =>
      (shown?.nodes ?? []).map((node) => {
        const change = diff?.nodes.get(node.id);
        const mine = composed?.mine.has(node.id) ?? false;
        const struck = composed?.struck.has(node.id) ?? false;
        // A screen that holds the step in one of its elements rings that element and stands the rest of itself back, and does not dim.
        const inner = at !== undefined && node.screen !== undefined && !lit.has(node.id) && holdsStep(node.screen, at) ? at : undefined;
        const name = mine ? (node.body?.trim() === "" || node.body === undefined ? "Your box" : `Your box: ${node.body}`) : partLabel(shown!, node.id);
        const rect = layout.rects.get(node.id);
        return {
          id: node.id,
          type: "scene",
          position: { x: rect?.x ?? node.x, y: rect?.y ?? node.y },
          data: {
            node,
            change,
            // A group or a lane is the ground the steps happen on, and does not stand back.
            dim: (diff !== undefined && change === "same") || (at !== undefined && !lit.has(node.id) && inner === undefined && !GROUND.has(node.kind)),
            current: lit.has(node.id),
            inner,
            asked: node.id === asked,
            mine,
            struck,
            joining: node.id === joining,
            notes: noted.get(node.id) ?? NO_NOTES,
            frame: GROUND.has(node.kind) && rect !== undefined ? { w: rect.w, h: rect.h } : undefined,
            onBody: (body: string) => onBody(node.id, body),
          },
          draggable: false,
          ...(GROUND.has(node.kind) ? { zIndex: -1 } : {}),
          ariaLabel: `${mine ? "" : struck ? "Struck out, " : change === undefined ? "" : `${CHANGE_SAID[change]}, `}${name}`,
        };
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps -- `onBody` closes over `mark`, which `composed` already tracks
    [shown, diff, at, lit, asked, joining, composed, layout, noted],
  );
  const latest = useRef(fresh);
  latest.current = fresh;
  const nodes = useMemo(() => overlay(kept, fresh), [kept, fresh]);
  const onNodesChange = useCallback((changes: NodeChange<SceneRfNode>[]) => setKept((held) => applyNodeChanges(changes, overlay(held, latest.current))), []);
  const markedKey = (marked ?? []).join("\n");
  const laidOut = fresh.length > 0;
  const wasMarked = useRef(false);
  useEffect(() => {
    if (!laidOut) return;
    if (markedKey === "" && !wasMarked.current) return;
    wasMarked.current = markedKey !== "";
    const ids = markedKey === "" ? [] : markedKey.split("\n");
    onNodesChange(latest.current.map((one) => ({ type: "select" as const, id: one.id, selected: ids.includes(one.id) })));
  }, [markedKey, sceneKey, laidOut, onNodesChange]);
  const edges = useMemo<SceneRfEdge[]>(
    () =>
      (shown?.edges ?? []).flatMap((edge) => {
        const path = layout.paths.get(edge.id);
        if (path === undefined) return [];
        const change = diff?.edges.get(edge.id);
        return [
          {
            id: edge.id,
            type: "flow",
            source: edge.from,
            target: edge.to,
            // Over the boxes, so no box hides where an arrow goes. A picked box still rises over it.
            zIndex: 1,
            data: {
              path,
              edge,
              place: places.get(edge.id),
              change,
              dim: (diff !== undefined && change === "same") || (at !== undefined && edge.step !== at),
              active: at !== undefined && edge.step === at,
              still: preview,
              asked: edge.id === asked,
              mine: composed?.mine.has(edge.id) ?? false,
              struck: composed?.struck.has(edge.id) ?? false,
              onPress: (id: string) => setAsked((was) => (was === id ? undefined : id)),
            },
          },
        ];
      }),
    [shown, diff, at, asked, composed, preview, places, layout],
  );

  if (shown === undefined || base === undefined) return <figure className="armada-sketch" data-preview={preview || undefined} role="group" aria-label="Sketch" />;

  const strokes = [...(shown.strokes ?? []), ...mark.strokes];
  const draw = (points: readonly SketchPoint[]) => edit({ ...mark, strokes: [...mark.strokes, { id: `i${String(mark.strokes.length + 1)}`, points }] });

  /** A note about `ids`, or about the parts picked where none is named. */
  const aboutParts = (ids: readonly string[]) => {
    if (ids.length === 0) return undefined;
    const parts = ids.map((id) => ({ id, label: partLabel(shown, id) }));
    const sketch = title ?? "Sketch";
    return { parts, about: within === undefined ? { kind: "sketch" as const, title: sketch, parts } : { kind: "document" as const, place: within, target: { kind: "part" as const, sketch, parts } } };
  };
  /** The part a press landed on, and what a note from there is about: the picked parts where it is one of them. */
  const pointedAt = (target: EventTarget | null) => {
    const id = target instanceof Element ? target.closest<HTMLElement>(".react-flow__node")?.dataset.id : undefined;
    return aboutParts(id === undefined ? picked : picked.includes(id) ? picked : [id]);
  };
  /** The menu at `at`, about what `target` points at; the box opens clear of the parts it is about. */
  const noteMenu = (at: { x: number; y: number }, target: EventTarget | null) => {
    const about = pointedAt(target);
    const rects = (about?.parts ?? []).flatMap((one) => {
      const drawn = figure.current?.querySelector(`.react-flow__node[data-id="${CSS.escape(one.id)}"]`);
      return drawn == null ? [] : [drawn.getBoundingClientRect()];
    });
    const anchor =
      rects.length === 0
        ? undefined
        : { left: Math.min(...rects.map((r) => r.left)), top: Math.min(...rects.map((r) => r.top)), right: Math.max(...rects.map((r) => r.right)), bottom: Math.max(...rects.map((r) => r.bottom)) };
    viewer.menu(at, about?.about, anchor);
  };

  const onKeyDown = (event: KeyboardEvent) => {
    const target = event.target as HTMLElement;
    if (target instanceof HTMLTextAreaElement || target instanceof HTMLInputElement) return;
    viewer.onKeyDown(event);
    if (viewer.on && (event.key === "ContextMenu" || (event.key === "F10" && event.shiftKey))) {
      event.preventDefault();
      const node = target.closest(".react-flow__node") ?? (picked.length === 0 ? null : figure.current?.querySelector(`.react-flow__node[data-id="${CSS.escape(picked[picked.length - 1]!)}"]`));
      const box = (node ?? target).getBoundingClientRect();
      noteMenu({ x: box.left, y: box.bottom }, node ?? target);
      return;
    }
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


  /** The drawing as it stands, in the theme's colours now: what every export is made from. `steps` is for a page that plays them. */
  const exportInput = (steps: boolean): SketchSvgInput => {
    const style = getComputedStyle(figure.current ?? document.documentElement);
    const tokens = Object.fromEntries(EXPORT_TOKENS.map((name) => [name, style.getPropertyValue(name).trim()]));
    const px = (name: string, fallback: number) => {
      const read = Number.parseFloat(tokens[name] ?? "");
      return Number.isNaN(read) ? fallback : read;
    };
    const d = DEFAULT_METRICS;
    const metrics: ExportMetrics = {
      textXs: px("--text-xs", d.textXs), textSm: px("--text-sm", d.textSm), leadingXs: px("--leading-xs", d.leadingXs), leadingSm: px("--leading-sm", d.leadingSm),
      space1: px("--space-1", d.space1), space2: px("--space-2", d.space2), space3: px("--space-3", d.space3), edge: px("--edge-active", d.edge),
      radiusSm: px("--radius-sm", d.radiusSm), radiusMd: px("--radius-md", d.radiusMd), pad: px("--space-6", d.pad),
    };
    const icons: Record<string, string> = {};
    for (const one of shown.nodes) {
      if (one.icon === undefined || icons[one.icon] !== undefined) continue;
      const Glyph = ICON[one.icon];
      icons[one.icon] = renderToStaticMarkup(<Glyph size={12} strokeWidth={2} />).replace(/^<svg[^>]*>/, "").replace(/<\/svg>$/, "");
    }
    // Where each screen's sheet starts under its title, in the node's own units and not the zoom's.
    const sheetTop = (node: SceneNode): number => {
      const drawn = figure.current?.querySelector(`.react-flow__node[data-id="${CSS.escape(node.id)}"] .armada-scene-node`);
      const sheet = drawn?.querySelector(".armada-screen");
      if (!(drawn instanceof HTMLElement) || !(sheet instanceof HTMLElement) || drawn.offsetHeight === 0) return node.title === undefined ? 0 : metrics.leadingSm + metrics.space2;
      return ((sheet.getBoundingClientRect().top - drawn.getBoundingClientRect().top) * drawn.offsetHeight) / drawn.getBoundingClientRect().height;
    };
    const screens = shown.nodes.some((one) => one.screen !== undefined) ? screenMarkup(shown, (id) => sheetTop(shown.nodes.find((one) => one.id === id)!), steps, screenStyles()) : undefined;
    const geometry: ExportGeometry = { rects: layout.rects, paths: layout.paths, places };
    const bodyLines: Record<string, string[]> = {};
    for (const one of shown.nodes) {
      const body = figure.current?.querySelector(`.react-flow__node[data-id="${CSS.escape(one.id)}"] .armada-scene-node__body`);
      const lines = body instanceof HTMLElement ? linesOf(body) : undefined;
      if (lines !== undefined) bodyLines[one.id] = lines;
    }
    return {
      scene: shown,
      bodyLines,
      geometry,
      metrics,
      tokens,
      icons,
      ...(screens === undefined ? {} : { screens }),
      state: { change: new Map([...(diff?.nodes ?? []), ...(diff?.edges ?? [])]), mine: composed?.mine ?? new Set(), struck: composed?.struck ?? new Set() },
      title: title ?? "Sketch",
      standalone: true,
    };
  };

  drawnAs.current = () => sketchSvg({ ...exportInput(false), standalone: false });

  /** Told either way, since a save or a copy is otherwise silent. */
  const share = async (id: string) => {
    const name = title ?? "Sketch";
    const input = exportInput(id === "html");
    const save = async (files: readonly SketchShareFile[]) => {
      const first = files[0]!;
      try {
        const saved = await saveWith(host, files);
        const said = savedSaid(first.name, saved);
        if (said !== undefined) host.said?.(said);
      } catch {
        host.said?.(`${first.name} was not saved.`);
      }
    };
    if (id === "mermaid") {
      try {
        // Each colour as the theme holds it now, since Mermaid draws in values and not in tokens.
        const style = getComputedStyle(figure.current ?? document.documentElement);
        const hues = Object.fromEntries(SCENE_COLORS.map((one) => [one, style.getPropertyValue(`--sketch-${one}`).trim()]).filter(([, value]) => value !== ""));
        await copyWith(host, sceneMermaid(shown, hues));
        host.copied?.(MERMAID_COPIED);
      } catch {
        host.said?.("The Mermaid text did not copy.");
      }
    } else if (id === "html") {
      const pulse = pulseMs() > 0 ? pulseMs() : 1600;
      const html = sketchHtml({ svg: sketchSvg({ ...input, standalone: false }), title: name, tokens: input.tokens, steps: stepPlan(shown), pulse });
      await save([{ kind: "html", name: sketchFileName(title, "html"), data: html }]);
    } else if (id === "image") {
      const svg = sketchSvg(input);
      const frame = frameOf(shown, input.geometry, input.metrics);
      try {
        const png = await svgToPng(svg, frame.w, frame.h);
        await save([
          { kind: "png", name: sketchFileName(title, "png"), data: png },
          { kind: "svg", name: sketchFileName(title, "svg"), data: svg },
        ]);
      } catch {
        host.said?.(`${sketchFileName(title, "png")} was not saved.`);
      }
    }
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
      name: stepAt === undefined ? "Play the steps" : reduced() ? "Next step" : "Stop the steps",
      icon: stepAt !== undefined && !reduced() ? Pause : Play,
      pressed: stepAt !== undefined,
      disabled: steps.length === 0,
      why: "This sketch has no order to play.",
      onPress: () => setStepAt(stepAt === undefined ? 0 : reduced() ? (stepAt + 1 < steps.length ? stepAt + 1 : undefined) : undefined),
    },
    {
      id: "restart",
      name: "Restart the steps",
      icon: RotateCw,
      disabled: steps.length === 0,
      why: "This sketch has no order to play.",
      onPress: () => setStepAt(0),
    },
    {
      id: "share",
      name: "Share",
      icon: ExternalLink,
      menu: {
        entries: SHARE_ENTRIES,
        onSelect: (id: string) => void share(id),
      },
    },
  ];

  // **A Shift drag boxes parts to note**, in place of React Flow's own, which picks only what it holds whole and asks for
  // the Shift key's own press first. Any part the box touches is picked; the ground it stands on is not.
  const local = (event: { clientX: number; clientY: number }) => {
    const rect = figure.current!.getBoundingClientRect();
    return { x: event.clientX - rect.left, y: event.clientY - rect.top };
  };
  const boxDown = (event: ReactPointerEvent<HTMLElement>) => {
    if (!viewer.on || pen || !event.shiftKey || event.button !== 0) return;
    event.stopPropagation();
    event.preventDefault();
    event.currentTarget.setPointerCapture?.(event.pointerId);
    const at = local(event);
    boxFrom.current = at;
    setBoxing({ x0: at.x, y0: at.y, x1: at.x, y1: at.y });
  };
  const boxMove = (event: ReactPointerEvent<HTMLElement>) => {
    const from = boxFrom.current;
    if (from === undefined) return;
    const at = local(event);
    setBoxing({ x0: from.x, y0: from.y, x1: at.x, y1: at.y });
  };
  const boxUp = (event: ReactPointerEvent<HTMLElement>) => {
    const from = boxFrom.current;
    if (from === undefined) return;
    boxFrom.current = undefined;
    const end = local(event);
    const rect = figure.current!.getBoundingClientRect();
    const box = { left: rect.left + Math.min(from.x, end.x), right: rect.left + Math.max(from.x, end.x), top: rect.top + Math.min(from.y, end.y), bottom: rect.top + Math.max(from.y, end.y) };
    setBoxing(undefined);
    const ids = [...(figure.current?.querySelectorAll<HTMLElement>(".react-flow__node") ?? [])].flatMap((one) => {
      const id = one.dataset.id;
      const drawn = shown.nodes.find((node) => node.id === id);
      if (id === undefined || drawn === undefined || GROUND.has(drawn.kind)) return [];
      const at = one.getBoundingClientRect();
      return at.right >= box.left && at.left <= box.right && at.bottom >= box.top && at.top <= box.bottom ? [id] : [];
    });
    onNodesChange(nodes.map((one) => ({ type: "select" as const, id: one.id, selected: ids.includes(one.id) })));
  };

  const part = asked === undefined ? undefined : { id: asked, label: partLabel(shown, asked) };
  const isEdge = part !== undefined && shown.edges.some((one) => one.id === part.id);
  const lifted = picked.length > 0 && picked.every((id) => composed?.struck.has(id));

  return (
    <figure
      ref={figure}
      className="armada-sketch"
      role="group"
      aria-label="Sketch"
      data-playing={at !== undefined || undefined}
      {...(viewer.on
        ? {
            onPointerDownCapture: boxDown,
            // The canvas pans on the mouse's own down, which a pointer's held default keeps from coming; one that comes anyway is kept from it too.
            onMouseDownCapture: (event: ReactMouseEvent<HTMLElement>) => {
              if (boxFrom.current !== undefined || (event.shiftKey && event.button === 0 && !pen)) event.stopPropagation();
            },
            onPointerMove: boxMove,
            onPointerUp: boxUp,
            onContextMenu: (event: ReactMouseEvent<HTMLElement>) => {
              event.preventDefault();
              noteMenu({ x: event.clientX, y: event.clientY }, event.target);
            },
          }
        : {})}
      {...(preview
        ? { "data-preview": true, inert: true }
        : { "aria-keyshortcuts": `+ - 0 ArrowLeft ArrowRight ArrowUp ArrowDown${viewer.on ? " ContextMenu Shift+F10 Meta+Enter" : ""}`, tabIndex: 0, onKeyDown })}
    >
      <GraphCanvas<SceneRfNode, SceneRfEdge>
        surface="armada-sketch-scene"
        label="Drawing"
        nodes={nodes}
        edges={edges}
        nodeTypes={NODE_TYPES}
        edgeTypes={EDGE_TYPES}
        onNodesChange={onNodesChange}
        fitViewOptions={preview ? PREVIEW_FIT : FIT}
        minZoom={preview ? 0.02 : 0.2}
        multiSelectionKeyCode={JOINS_THE_PICK}
        onSelectionChange={setPicked}
        still={preview}
        {...(preview
          ? {}
          : {
              onNodePress: press,
              onPanePress: () => {
                setAsked(undefined);
                setJoining(undefined);
              },
              rail: <GraphCanvasRailGroup label="What you can do with the sketch" acts={acts} />,
            })}
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
        <Capture into={flow} extent={layout.extent} preview={preview} />
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
      {boxing === undefined ? null : (
        <span
          className="armada-sketch__box"
          style={{ left: Math.min(boxing.x0, boxing.x1), top: Math.min(boxing.y0, boxing.y1), width: Math.abs(boxing.x1 - boxing.x0), height: Math.abs(boxing.y1 - boxing.y0) }}
        />
      )}
      {viewer.layer}
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
