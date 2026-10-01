// A Studio's Sketch on the pad, and back — the owner's call of 1 Oct 2026: a
// Studio's Sketch and the dispatch composer's pad are one drawing.
//
// **Two spellings of one drawing, and the seam is here.** The wire says
// `boxes`; the pad's draft says `shapes`, and holds a picture as a `blob:`
// rather than as the frame Fleet kept. Which pictures are already kept is held
// beside the drawing, by id, because that is what a save sends instead of
// bytes — a kept picture is named, never re-sent.

import type { SketchDrawing, SketchToKeep } from "@armada/protocol";

import type { Drawing } from "./draft/sketch";

/**
 * A Sketch's drawing as the pad holds it. **A kept picture's `src` is `""`**:
 * its bytes are the board's, read by `frameKey`, and the pad is handed them as
 * they land rather than a copy taken once.
 */
export function padOf(drawing: SketchDrawing): Drawing {
  return {
    shapes: drawing.boxes.map(({ id, x, y, body }) => ({ id, x, y, body })),
    joins: drawing.joins.map(({ id, from, to }) => ({ id, from, to })),
    strokes: drawing.strokes.map(({ id, points }) => ({ id, points: points.map(({ x, y }) => ({ x, y })) })),
    pictures: drawing.pictures.map(({ id, x, y, width, height }) => ({ id, x, y, width, height, src: "" })),
  };
}

/** The pictures a Sketch already keeps, by id — what a save names rather than sends. */
export function keptOf(drawing: SketchDrawing): ReadonlySet<string> {
  return new Set(drawing.pictures.map((one) => one.id));
}

/**
 * The drawing with every place rounded to a whole pad unit, which is what
 * Fleet keeps. The pad rounds a stroke and a paste itself; a box dragged by
 * React Flow can land between two.
 */
function rounded(drawing: Drawing): Drawing {
  const at = ({ x, y }: { x: number; y: number }) => ({ x: Math.round(x), y: Math.round(y) });
  return {
    shapes: drawing.shapes.map((one) => ({ ...one, ...at(one) })),
    joins: drawing.joins,
    strokes: drawing.strokes.map((one) => ({ ...one, points: one.points.map(at) })),
    pictures: drawing.pictures.map((one) => ({
      ...one,
      ...at(one),
      width: Math.round(one.width),
      height: Math.round(one.height),
    })),
  };
}

/**
 * What a drawing is, with nothing a `blob:` says — so a pad closed as it was
 * opened is told apart from one somebody drew on, and writes nothing.
 */
export function drawnAs(drawing: Drawing): string {
  const { shapes, joins, strokes, pictures } = rounded(drawing);
  return JSON.stringify({
    shapes,
    joins,
    strokes,
    pictures: pictures.map(({ id, x, y, width, height }) => ({ id, x, y, width, height })),
  });
}

/**
 * The drawing as main keeps it: a kept picture by its id alone, a new one
 * with its bytes. `bytesOf` reads a `blob:` back, and is the caller's because
 * fetching is.
 */
export async function toKeep(
  drawing: Drawing,
  kept: ReadonlySet<string>,
  bytesOf: (src: string) => Promise<Uint8Array>,
): Promise<SketchToKeep> {
  const { shapes, joins, strokes, pictures } = rounded(drawing);
  return {
    boxes: shapes.map(({ id, x, y, body }) => ({ id, x, y, body })),
    joins: joins.map(({ id, from, to }) => ({ id, from, to })),
    strokes: strokes.map(({ id, points }) => ({ id, points: [...points] })),
    pictures: await Promise.all(
      pictures.map(async ({ id, x, y, width, height, src }) =>
        kept.has(id) ? { id, x, y, width, height } : { id, x, y, width, height, bytes: await bytesOf(src) },
      ),
    ),
  };
}
