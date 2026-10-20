// Where a sketch's boxes go once they are measured. **An agent places boxes for the size the tool tells it
// (240 by 90), and a box grows to its words**, so a taller or wider box ran into the next one. This keeps the
// agent's rows and columns, moves a row down or a column across only as far as a box it would meet needs, and
// moves nothing that already has room. A box still touching another after that moves down until it clears.
//
// **A column moves for the boxes it would meet, not for the widest box anywhere left of it.** Moving every
// column right of a widened one pushed boxes in other rows apart for nothing, and the fit shrank the words.

import type { Rect } from "./edge-labels";

/** A box as the agent placed it, at its measured size. `ground` is a group or a lane, which holds boxes. */
export type Box = Rect & { id: string; ground?: boolean };

/** How far apart two agent coordinates may be and still be one column or row. */
const SAME_LINE = 40;

type Axis = { at: "x" | "y"; size: "w" | "h"; across: "x" | "y"; span: "w" | "h" };
const ROWS: Axis = { at: "y", size: "h", across: "x", span: "w" };
const COLUMNS: Axis = { at: "x", size: "w", across: "y", span: "h" };

/** The agent's lines along one axis: boxes whose starts are within `SAME_LINE` of the line's first. */
function linesOf(boxes: readonly Box[], axis: Axis): Box[][] {
  const lines: Box[][] = [];
  for (const one of [...boxes].sort((a, b) => a[axis.at] - b[axis.at])) {
    const line = lines[lines.length - 1];
    if (line !== undefined && one[axis.at] - line[0]![axis.at] <= SAME_LINE) line.push(one);
    else lines.push([one]);
  }
  return lines;
}

const meets = (a: Box, b: Box, axis: Axis, gap: number) => a[axis.across] < b[axis.across] + b[axis.span] + gap && b[axis.across] < a[axis.across] + a[axis.span] + gap;

/** Each line moved on as far as the boxes before it that it would meet need, and the whole line together. */
function spread(boxes: readonly Box[], axis: Axis, gap: number): Box[] {
  const done: Box[] = [];
  for (const line of linesOf(boxes, axis)) {
    let shift = 0;
    for (const one of line) for (const before of done) if (meets(one, before, axis, gap)) shift = Math.max(shift, before[axis.at] + before[axis.size] + gap - one[axis.at]);
    done.push(...line.map((one) => ({ ...one, [axis.at]: one[axis.at] + shift })));
  }
  return done;
}

const touching = (a: Rect, b: Rect, gap: number) => a.x < b.x + b.w + gap && b.x < a.x + a.w + gap && a.y < b.y + b.h + gap && b.y < a.y + a.h + gap;

/** Every box's settled rectangle, by id. A group or a lane is grown round the boxes it held. */
export function settle(boxes: readonly Box[], gap: number): Map<string, Rect> {
  const solid = boxes.filter((one) => one.ground !== true);
  const settled = new Map<string, Rect>();

  // Rows down first, then columns across, then anything still touching goes below what it touches.
  const moved = spread(spread(solid, ROWS, gap), COLUMNS, gap).sort((a, b) => a.y - b.y || a.x - b.x);
  const down: Box[] = [];
  for (const one of moved) {
    for (let clash = down.find((other) => touching(one, other, gap)); clash !== undefined; clash = down.find((other) => touching(one, other, gap))) {
      one.y = clash.y + clash.h + gap;
    }
    down.push(one);
    settled.set(one.id, { x: one.x, y: one.y, w: one.w, h: one.h });
  }

  for (const ground of boxes.filter((one) => one.ground === true)) {
    const held = solid.filter((one) => {
      const cx = one.x + one.w / 2;
      const cy = one.y + one.h / 2;
      return cx >= ground.x && cx <= ground.x + ground.w && cy >= ground.y && cy <= ground.y + ground.h;
    });
    if (held.length === 0) {
      settled.set(ground.id, { x: ground.x, y: ground.y, w: ground.w, h: ground.h });
      continue;
    }
    // The margins it had round what it held, kept; what it held now decides where it is and how big.
    const now = held.map((one) => settled.get(one.id)!);
    const left = Math.min(...held.map((one) => one.x - ground.x));
    const top = Math.min(...held.map((one) => one.y - ground.y));
    const right = Math.max(gap / 2, Math.min(...held.map((one) => ground.x + ground.w - one.x - one.w)));
    const bottom = Math.max(gap / 2, Math.min(...held.map((one) => ground.y + ground.h - one.y - one.h)));
    const x = Math.min(...now.map((one) => one.x)) - left;
    const y = Math.min(...now.map((one) => one.y)) - top;
    const w = Math.max(ground.w, Math.max(...now.map((one) => one.x + one.w)) + right - x);
    const h = Math.max(ground.h, Math.max(...now.map((one) => one.y + one.h)) + bottom - y);
    settled.set(ground.id, { x, y, w, h });
  }
  return settled;
}
