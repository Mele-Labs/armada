import { Position, getBezierPath } from "@xyflow/react";
import { useId } from "react";

import { facingSides } from "../GraphCanvas/GraphCanvas";
import { pathOf } from "../SketchPad/Ink";
import type { SketchBox, SketchLine, SketchStroke } from "../SketchPad/SketchPad";

/**
 * Sketch preview — a drawing from the pad, read-only and fitted to the space
 * it is given. What a Studio's Sketch node draws on the board.
 *
 * **The pad's look, without the pad.** A box is the pad's raised card with its
 * words in a sunken well, a join is React Flow's bezier between the two facing
 * sides with the closed arrow, a stroke is the pen's own curve and a picture
 * sits at its place and size. One SVG, so a board of Sketches is not a board
 * of nested canvases each with a viewport of its own.
 */
export type SketchPreviewProps = {
  /** What the drawing is, read to somebody who cannot see it. */
  label: string;
  boxes: readonly SketchBox[];
  lines: readonly SketchLine[];
  strokes: readonly SketchStroke[];
  pictures: readonly SketchPreviewPicture[];
};

/**
 * One picture on the drawing. `src` is the caller's `blob:` once its bytes
 * are read; absent draws the space it will take, so nothing moves when it lands.
 */
export type SketchPreviewPicture = {
  id: string;
  x: number;
  y: number;
  width: number;
  height: number;
  src?: string;
};

/**
 * A box's footprint in pad units. **The width is `--w-sketch-box`'s**, which
 * the pad draws every box at; the height is a box holding two rows, the pad's
 * `rows={2}`. Numbers here because a `viewBox` is fitted in JavaScript.
 */
const BOX = { width: 240, height: 76 };

/** Room round the drawing's outermost edge, in pad units. */
const MARGIN = 24;

type Rect = { x: number; y: number; width: number; height: number };

const SIDE: Record<string, Position> = {
  left: Position.Left,
  right: Position.Right,
  top: Position.Top,
  bottom: Position.Bottom,
};

function middleOf(rect: Rect, side: Position): { x: number; y: number } {
  switch (side) {
    case Position.Left:
      return { x: rect.x, y: rect.y + rect.height / 2 };
    case Position.Right:
      return { x: rect.x + rect.width, y: rect.y + rect.height / 2 };
    case Position.Top:
      return { x: rect.x + rect.width / 2, y: rect.y };
    default:
      return { x: rect.x + rect.width / 2, y: rect.y + rect.height };
  }
}

/** The bezier between two parts' facing sides, as the pad's edge draws it. */
function joinPath(from: Rect, to: Rect): string {
  const placed = (rect: Rect) => ({
    position: { x: rect.x, y: rect.y },
    measured: { width: rect.width, height: rect.height },
  });
  const { sourceHandle, targetHandle } = facingSides(placed(from), placed(to));
  const out = SIDE[sourceHandle.slice(2)] ?? Position.Right;
  const into = SIDE[targetHandle.slice(2)] ?? Position.Left;
  const start = middleOf(from, out);
  const end = middleOf(to, into);
  const [path] = getBezierPath({
    sourceX: start.x,
    sourceY: start.y,
    sourcePosition: out,
    targetX: end.x,
    targetY: end.y,
    targetPosition: into,
  });
  return path;
}

/** The smallest box holding everything drawn, with room round it. */
function bounds(rects: readonly Rect[], strokes: readonly SketchStroke[]): Rect {
  const xs: number[] = [];
  const ys: number[] = [];
  for (const rect of rects) {
    xs.push(rect.x, rect.x + rect.width);
    ys.push(rect.y, rect.y + rect.height);
  }
  for (const stroke of strokes) {
    for (const point of stroke.points) {
      xs.push(point.x);
      ys.push(point.y);
    }
  }
  if (xs.length === 0) return { x: 0, y: 0, width: 1, height: 1 };
  const x = Math.min(...xs) - MARGIN;
  const y = Math.min(...ys) - MARGIN;
  return { x, y, width: Math.max(...xs) + MARGIN - x, height: Math.max(...ys) + MARGIN - y };
}

export function SketchPreview({ label, boxes, lines, strokes, pictures }: SketchPreviewProps) {
  // An id per drawing, since a board holds several and a marker is found by id.
  const arrow = `sketch-arrow-${useId().replace(/[^a-zA-Z0-9_-]/g, "")}`;
  const placed = new Map<string, Rect>([
    ...boxes.map((box) => [box.id, { x: box.x, y: box.y, ...BOX }] as const),
    ...pictures.map((one) => [one.id, one] as const),
  ]);
  const view = bounds([...placed.values()], strokes);
  return (
    <svg
      className="armada-sketch-preview"
      role="img"
      aria-label={label}
      viewBox={`${String(view.x)} ${String(view.y)} ${String(view.width)} ${String(view.height)}`}
      preserveAspectRatio="xMidYMid meet"
    >
      <defs>
        <marker
          id={arrow}
          viewBox="0 0 10 10"
          refX="10"
          refY="5"
          markerWidth="6"
          markerHeight="6"
          orient="auto-start-reverse"
        >
          <path className="armada-sketch-preview__arrow" d="M 0 0 L 10 5 L 0 10 z" />
        </marker>
      </defs>
      {pictures.map((one) =>
        one.src === undefined ? (
          <rect
            key={one.id}
            className="armada-sketch-preview__unread"
            x={one.x}
            y={one.y}
            width={one.width}
            height={one.height}
          />
        ) : (
          <image
            key={one.id}
            href={one.src}
            x={one.x}
            y={one.y}
            width={one.width}
            height={one.height}
            preserveAspectRatio="xMidYMid meet"
          />
        ),
      )}
      {lines.map((line) => {
        const from = placed.get(line.from);
        const to = placed.get(line.to);
        if (from === undefined || to === undefined) return null;
        return (
          <path
            key={line.id}
            className="armada-sketch-preview__join"
            d={joinPath(from, to)}
            markerEnd={`url(#${arrow})`}
          />
        );
      })}
      {boxes.map((box) => (
        <foreignObject key={box.id} x={box.x} y={box.y} width={BOX.width} height={BOX.height}>
          <div className="armada-sketch-preview__box">
            <p className="armada-sketch-preview__well">{box.body}</p>
          </div>
        </foreignObject>
      ))}
      {strokes.map((stroke) => (
        <path key={stroke.id} className="armada-sketch-preview__stroke" d={pathOf(stroke.points)} />
      ))}
    </svg>
  );
}
