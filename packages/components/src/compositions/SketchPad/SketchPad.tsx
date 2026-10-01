import {
  Handle,
  MarkerType,
  applyNodeChanges,
  type Edge,
  type FitViewOptions,
  type Node,
  type NodeChange,
  type NodeProps,
} from "@xyflow/react";
import { Pencil, SquarePlus, Trash2, Undo2 } from "lucide-react";
import { useCallback, useMemo, useRef, useState } from "react";

import { Ink, type SketchPoint, type SketchStroke } from "./Ink";
import { Paste, useMiddle } from "./Paste";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { GRAPH_CANVAS_SIDES, GraphCanvas, facingSides } from "../GraphCanvas/GraphCanvas";
import { GraphCanvasNodeAct, GraphCanvasNodeBar, GraphCanvasRailGroup } from "../GraphCanvas/GraphCanvasRail";
import type { GraphCanvasRailAct } from "../GraphCanvas/GraphCanvasRail";

/**
 * Sketch pad — boxes, the lines between them, and what a person draws by hand,
 * beside a prompt. `docs/journeys/dispatch-a-job.md`, The sketch.
 *
 * **What a prompt cannot say** (`#1547`): when what somebody means is a shape,
 * they type a paragraph describing a picture and the Drone reads the paragraph.
 *
 * **`GraphCanvas` is the graph half** (`#1539`) — no second canvas and no
 * drawing library. A box is words and a place, the way a Studio's `Sketch` node
 * is `{ body: String }`: no colour, no size, nothing to pick. The pen is this
 * surface's alone and no other canvas inherits it: `Ink`, and `Paste` for ⌘V.
 *
 * **Nothing here stages anything.** What goes out is a PNG Bridge writes; every
 * edit is reported to the caller, so the drawing survives a switch to Write.
 */
export type SketchPadProps = {
  /** What the picture is, read to somebody who cannot see it. */
  label: string;
  boxes: readonly SketchBox[];
  lines: readonly SketchLine[];
  /** Everything drawn by hand, in the order it was drawn. */
  strokes: readonly SketchStroke[];
  /** Everything pasted onto the pad as a picture. */
  pictures: readonly SketchPicture[];
  /**
   * A box put down somewhere new, by pointer or by arrow key. Reported when it
   * lands, never while it travels.
   */
  onMove: (id: string, at: { x: number; y: number }) => void;
  /** The words in one box, as they are typed. */
  onBody: (id: string, body: string) => void;
  /**
   * A box added, in the pad's own coordinates, holding `body` — empty from Add
   * a box, the text from a paste. **The caller mints the id.**
   */
  onAdd: (at: { x: number; y: number }, body: string) => void;
  /**
   * A picture pasted onto the pad, already scaled to sit on it, placed by its
   * top-left corner. **The caller mints the id**, as it does for a box.
   */
  onPicture: (picture: SketchPictureLanding, at: { x: number; y: number }) => void;
  /** Whatever is selected, taken off. Never called with nothing selected. */
  onRemove: (ids: readonly string[]) => void;
  /** Two things joined, boxes or pictures. Offered only while exactly two are selected. */
  onJoin: (from: string, to: string) => void;
  /**
   * A line drawn by hand, in the pad's own coordinates. **The caller mints the
   * id**, as it does for a box. Reported when the pointer lifts, never during.
   */
  onDraw: (points: readonly SketchPoint[]) => void;
  /**
   * The last line drawn, taken back. Never called with nothing drawn by hand.
   * It takes strokes and nothing else — a box comes off under Remove.
   */
  onUndo: () => void;
  /**
   * What a person said about the picture. **Beside the pad and not in a box**:
   * it is about the whole sketch, and a box holding it would be read as part
   * of the shape.
   */
  said: string;
  onSaid: (said: string) => void;
  /**
   * The Studio node this was made from. **Absent is a pad opened blank**, which
   * is a real answer (`draft/sketch.ts`), so it draws no line rather than an
   * empty one.
   */
  from?: string;
  /** Nothing may be drawn while the connection is not live. */
  disabled?: boolean;
};

/** One box: where a person put it, and the words in it. */
export type SketchBox = { id: string; x: number; y: number; body: string };

/** One box joined to another, in the direction it was drawn. */
export type SketchLine = { id: string; from: string; to: string };

/**
 * One picture pasted onto the pad: where it sits, the size it is drawn at, and
 * a `blob:` address for its bytes — what the window's `img-src` draws.
 */
export type SketchPicture = SketchPictureLanding & { id: string; x: number; y: number };

/** What a paste hands the caller, before it has an id or a place. */
export type SketchPictureLanding = { src: string; width: number; height: number };

/** The pen's own two, re-exported so a caller reads one module's props. */
export type { SketchPoint, SketchStroke } from "./Ink";

/** What the field beside the pad asks for. Never a Wh- opener. */
const SAID_LABEL = "About this sketch";

const SAID_PLACEHOLDER = "What the picture is meant to show.";

/** What an empty box is called, for somebody who cannot see it. */
const EMPTY_BOX = "An empty box";

/** A box's own field, which carries no visible label — the box is the label. */
const BOX_LABEL = "The words in this box";

/** What a box holds, shown rather than described. A phrase, never a paragraph. */
const BOX_PLACEHOLDER = "A panel, a read, a step";

/** Where the picture was made, before the node it was made from. */
const MADE_IN_A_STUDIO = "From a Studio";

/** What a pasted picture is called, for somebody who cannot see it. */
const A_PICTURE = "A pasted picture";

type BoxNode = Node<{ body: string; onBody: (body: string) => void; disabled: boolean }, "sketch">;
type PictureNode = Node<SketchPictureLanding, "picture">;
type PadNode = BoxNode | PictureNode;
type PadEdge = Edge<Record<string, never>, "default">;

/** A handle on each side, in and out, so a line meets whichever side faces the other end. */
function Sides({ type }: { type: "source" | "target" }) {
  const prefix = type === "source" ? "s" : "t";
  return GRAPH_CANVAS_SIDES.map((side) => (
    <Handle
      key={`${prefix}-${side}`}
      id={`${prefix}-${side}`}
      type={type}
      position={side}
      isConnectable={false}
    />
  ));
}

/**
 * One box. The card is the grip and the well inside it is `nodrag`, so a drag
 * starting in the words selects text rather than moving the box out from under
 * the cursor.
 */
function BoxView({ data, selected }: NodeProps<BoxNode>) {
  return (
    <>
      <Sides type="target" />
      <div className="armada-sketch-box" aria-current={selected || undefined}>
        {/* `nodrag` is read by walking up from whatever the pointer hit, so the
            wrapper carries it and the whole well is exempt. */}
        <div className="nodrag">
          <Textarea
            rows={2}
            value={data.body}
            placeholder={BOX_PLACEHOLDER}
            disabled={data.disabled}
            aria-label={BOX_LABEL}
            onChange={(event) => data.onBody(event.target.value)}
          />
        </div>
      </div>
      <Sides type="source" />
    </>
  );
}

/**
 * One pasted picture, at the size it landed at. **The whole of it is the
 * grip**, because there is nothing in it to type into; `draggable` is off so a
 * drag moves the picture rather than starting the browser's own drag of an
 * image.
 */
function PictureView({ data, selected }: NodeProps<PictureNode>) {
  return (
    <>
      <Sides type="target" />
      <div className="armada-sketch-picture" aria-current={selected || undefined}>
        <img src={data.src} alt="" width={data.width} height={data.height} draggable={false} />
      </div>
      <Sides type="source" />
    </>
  );
}

const NODE_TYPES = { sketch: BoxView, picture: PictureView };
const EDGE_TYPES = {};

/**
 * Room around a fitted picture. **A tenth rather than React Flow's default**,
 * so a picture fitted edge to edge still has air between its outermost box and
 * the pane's own edge — and the bar hovering over a box picked there has
 * somewhere to sit.
 *
 * **Only on open, and only where there is a picture.** `GraphCanvas` reads
 * `fitView` once and refuses it on an empty canvas; a blank pad that kept the
 * fit armed zoomed onto the first box added, which is what the owner read on
 * 28 Sep 2026.
 */
const FIT: FitViewOptions = { padding: 0.1 };

/**
 * Held down, a second box joins the selection rather than replacing it — which
 * is how a person picks the two they are joining. Named rather than left to
 * React Flow's platform read, for the two reasons `StudioWhiteboard` states.
 */
const JOINS_THE_SELECTION = ["Meta", "Control"];

/** How many boxes a join takes. Two, and the control says so while it is off. */
const A_JOIN_TAKES = 2;

/** What the rail is, and what hovers over a picked box, read to somebody who cannot see them. */
const RAIL_LABEL = "What you can draw";

const BOX_ACTS_LABEL = "What you can do with the boxes you picked";

/**
 * The rail down the pad's leading edge — the pen, the line it takes back, and a
 * box added. **The acts on a box are not here**: they hover over the box, which
 * is `BoxActs` below.
 */
function PadRail({
  onAdd,
  pen,
  onPen,
  drawn,
  onUndo,
  disabled,
}: {
  onAdd: (at: { x: number; y: number }, body: string) => void;
  pen: boolean;
  onPen: (pen: boolean) => void;
  drawn: boolean;
  onUndo: () => void;
  disabled: boolean;
}) {
  const middle = useMiddle();

  const acts: GraphCanvasRailAct[] = [
    {
      id: "draw",
      name: "Draw",
      icon: Pencil,
      pressed: pen,
      why: "Drag on the pad to draw. Press again to stop.",
      onPress: () => onPen(!pen),
    },
    {
      id: "undo",
      name: "Undo",
      icon: Undo2,
      disabled: !drawn,
      why: "Nothing has been drawn by hand.",
      onPress: onUndo,
    },
    // The pen is a mode, so a box somebody is about to type in puts it down.
    {
      id: "add",
      name: "Add a box",
      icon: SquarePlus,
      onPress: () => {
        onPen(false);
        onAdd(middle(), "");
      },
    },
  ];
  return <GraphCanvasRailGroup label={RAIL_LABEL} acts={acts} disabled={disabled} />;
}

/**
 * The acts on the boxes a person picked, hovering over them.
 *
 * **Remove draws `trash-2` and Join draws its word** — `docs/contracts/`
 * `iconography.md`, *The node bar*. The mint for *remove this* serves the
 * Studio, the pad and the attachment chip at once; nothing in the registry
 * means *join two boxes*. The Studio's own bar draws the same control, so the
 * two read as one vocabulary.
 */
function BoxActs({
  picked,
  onRemove,
  onJoin,
  disabled,
}: {
  picked: readonly string[];
  onRemove: (ids: readonly string[]) => void;
  onJoin: (from: string, to: string) => void;
  disabled: boolean;
}) {
  const joinable = picked.length === A_JOIN_TAKES;
  return (
    <GraphCanvasNodeBar label={BOX_ACTS_LABEL} nodeIds={picked}>
      <GraphCanvasNodeAct
        name="Join"
        disabled={disabled || !joinable}
        why="Pick two boxes to join them."
        onPress={() => onJoin(picked[0]!, picked[1]!)}
      />
      <GraphCanvasNodeAct
        name="Remove"
        icon={Trash2}
        danger
        disabled={disabled}
        onPress={() => onRemove(picked)}
      />
    </GraphCanvasNodeBar>
  );
}

/** One box as React Flow holds it, rebuilt from what the caller gave. */
function toPadNode(box: SketchBox, props: SketchPadProps): BoxNode {
  return {
    id: box.id,
    position: { x: box.x, y: box.y },
    type: "sketch",
    data: {
      body: box.body,
      onBody: (body: string) => props.onBody(box.id, body),
      disabled: props.disabled ?? false,
    },
    ariaLabel: box.body.trim() === "" ? EMPTY_BOX : `Box: ${box.body}`,
  };
}

/** One picture as React Flow holds it. */
function toPictureNode(picture: SketchPicture): PictureNode {
  return {
    id: picture.id,
    position: { x: picture.x, y: picture.y },
    type: "picture",
    data: { src: picture.src, width: picture.width, height: picture.height },
    ariaLabel: A_PICTURE,
  };
}

/**
 * Everything the caller gave, as React Flow holds it — **the pictures, then
 * the boxes**, so a box is drawn over a picture and never hidden under one.
 * A screenshot is what a box is about, and the box is the part with words.
 */
function given(props: SketchPadProps): PadNode[] {
  return [...props.pictures.map(toPictureNode), ...props.boxes.map((box) => toPadNode(box, props))];
}

/**
 * What the caller gave, over what React Flow keeps per box — where it sits,
 * what it measured, whether it is selected. The words are always the caller's,
 * so a box typed into redraws in place.
 *
 * **The caller's boxes are merged in before any change is applied**, or a box
 * added after mount is never in the list React Flow measures and stays at
 * `visibility: hidden` forever. `StudioWhiteboard` folds the same way.
 */
function merged(kept: readonly PadNode[], props: SketchPadProps): PadNode[] {
  const byId = new Map(kept.map((node) => [node.id, node]));
  return given(props).map((fresh) => {
    const held = byId.get(fresh.id);
    return held === undefined
      ? fresh
      : ({ ...held, data: fresh.data, ariaLabel: fresh.ariaLabel } as PadNode);
  });
}

export function SketchPad(props: SketchPadProps) {
  const { label, boxes, lines, strokes, pictures, onMove, onAdd, onPicture, onRemove, onJoin } =
    props;
  const { onDraw, onUndo, said, onSaid, from } = props;
  const disabled = props.disabled ?? false;
  // Placement is the pad's to hold between moves, the way the whiteboard holds
  // it; the caller hears each one through `onMove` and keeps it in the draft.
  const [kept, setKept] = useState<PadNode[]>(() => given(props));
  const [picked, setPicked] = useState<readonly string[]>([]);
  // Which of the two the pointer does. A mode the pad holds and never reports:
  // it dies with the surface, and nothing outside it is a picture.
  const [pen, setPen] = useState(false);
  const nodes = merged(kept, props);
  const drawing = pen && !disabled;
  // The frame a paste is listened for on. It takes focus from a press on the
  // pane, which is otherwise not focusable, so ⌘V after a click lands here.
  const frame = useRef<HTMLDivElement>(null);

  const onNodesChange = useCallback(
    (changes: NodeChange<PadNode>[]) => {
      setKept((current) => applyNodeChanges(changes, merged(current, props)));
      // `dragging: false` is a box put down: a drag ending, or an arrow key.
      for (const change of changes) {
        if (change.type === "position" && change.dragging === false && change.position) {
          onMove(change.id, change.position);
        }
      }
    },
    [props, onMove],
  );

  // Hung off the boxes rather than off `nodes`, which is a fresh array every
  // render — the sides an edge leaves from are decided by where the two sit.
  // A picture's size is known, so its centre is too.
  const edges = useMemo<PadEdge[]>(() => {
    const placed = new Map<string, { position: { x: number; y: number }; measured?: { width: number; height: number } }>([
      ...boxes.map((box) => [box.id, { position: { x: box.x, y: box.y } }] as const),
      ...pictures.map(
        (one) =>
          [
            one.id,
            { position: { x: one.x, y: one.y }, measured: { width: one.width, height: one.height } },
          ] as const,
      ),
    ]);
    return lines.map((line) => ({
      id: line.id,
      source: line.from,
      target: line.to,
      ...facingSides(placed.get(line.from), placed.get(line.to)),
      ariaLabel: `A line from ${line.from} to ${line.to}`,
      markerEnd: { type: MarkerType.ArrowClosed },
    }));
  }, [lines, boxes, pictures]);

  return (
    <div className="armada-sketch-pad">
      <div className="armada-sketch-pad__canvas" ref={frame} tabIndex={-1}>
        <GraphCanvas<PadNode, PadEdge>
          surface="armada-sketch-pad__graph"
          label={label}
          nodes={nodes}
          edges={edges}
          nodeTypes={NODE_TYPES}
          edgeTypes={EDGE_TYPES}
          onNodesChange={onNodesChange}
          onSelectionChange={setPicked}
          nodesDraggable={!disabled}
          multiSelectionKeyCode={JOINS_THE_SELECTION}
          fitViewOptions={FIT}
          rail={
            <PadRail
              onAdd={onAdd}
              pen={drawing}
              onPen={setPen}
              drawn={strokes.length > 0}
              onUndo={onUndo}
              disabled={disabled}
            />
          }
        >
          <Ink strokes={strokes} pen={drawing} onDraw={onDraw} />
          {disabled ? null : <Paste pad={frame} onAdd={onAdd} onPicture={onPicture} />}
          {/* Not while the pen is down: the layer that catches a stroke covers
              the whole canvas, so a bar drawn under it is one nothing can
              press. Every rail act puts the pen away, which is the way back. */}
          {drawing ? null : (
            <BoxActs picked={picked} onRemove={onRemove} onJoin={onJoin} disabled={disabled} />
          )}
        </GraphCanvas>
        {/* A blank canvas under the controls says nothing about what it is for,
            and this is the one moment with no picture to read instead. */}
        {boxes.length > 0 || strokes.length > 0 || pictures.length > 0 ? null : (
          <p className="armada-sketch-pad__empty" role="note">
            Nothing is drawn yet. Add a box and write what it is, join the boxes that feed each
            other, or draw on the pad by hand.
          </p>
        )}
      </div>
      <Textarea
        label={SAID_LABEL}
        rows={2}
        value={said}
        placeholder={SAID_PLACEHOLDER}
        disabled={disabled}
        onChange={(event) => onSaid(event.target.value)}
      />
      {/* **`a Studio` is said once, and it is said here** — the owner, 28 Sep
          2026, reading this line against the chip below it. Two facts, not
          one: this says which node the picture was made from, the chip says
          the request carries a picture. The phrase they shared moved up to the
          one that is about provenance, and the chip keeps its name alone. */}
      {from === undefined ? null : (
        <p className="armada-sketch-pad__from">
          <span className="armada-sketch-pad__made">{MADE_IN_A_STUDIO}</span>{" "}
          <span className="mono">{from}</span>
        </p>
      )}
    </div>
  );
}
