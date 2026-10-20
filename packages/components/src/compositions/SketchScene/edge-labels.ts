// Where an arrow's words sit. An arrow draws its words on a backing, and the backing slides along
// the arrow's own line until it clears every box and every other arrow's words. No layout library:
// a sketch has a dozen arrows, and greedy placement over a few candidate points is enough.

export type Rect = { x: number; y: number; w: number; h: number };
export type LabelMetrics = { charW: number; lineH: number; pad: number };
export type LabelPlace = { lines: string[]; x: number; y: number; w: number; h: number; lineH: number; pad: number };

/** Characters to a line, and lines to a label. What does not fit is cut and the full text stays in the accessible name. */
export const LABEL_CHARS = 20;
export const LABEL_LINES = 2;

export function wrapLabel(text: string): string[] {
  const words = text.trim().split(/\s+/).flatMap((word) => {
    const parts: string[] = [];
    for (let at = 0; at < word.length; at += LABEL_CHARS) parts.push(word.slice(at, at + LABEL_CHARS));
    return parts;
  });
  const lines: string[] = [];
  let cut = false;
  for (const word of words) {
    const last = lines.length - 1;
    if (last >= 0 && lines[last]!.length + 1 + word.length <= LABEL_CHARS) lines[last] += ` ${word}`;
    else if (lines.length < LABEL_LINES) lines.push(word);
    else {
      cut = true;
      break;
    }
  }
  if (cut) {
    const last = lines.length - 1;
    lines[last] = `${lines[last]!.slice(0, LABEL_CHARS - 1).trimEnd()}…`;
  }
  return lines;
}

const NUMBER = /-?\d+(?:\.\d+)?(?:e-?\d+)?/g;

/** The point `t` of the way along a path: a React Flow bezier (`M sx,sy C c1 c2 tx,ty`), or a routed way of runs and rounded corners. */
export function pathAt(path: string, t: number): { x: number; y: number } | undefined {
  if (path.includes("C")) {
    const n = path.match(NUMBER)?.map(Number);
    if (n === undefined || n.length < 8) return undefined;
    const [x0, y0, x1, y1, x2, y2, x3, y3] = n as [number, number, number, number, number, number, number, number];
    const u = 1 - t;
    const a = u * u * u;
    const b = 3 * u * u * t;
    const c = 3 * u * t * t;
    const d = t * t * t;
    return { x: a * x0 + b * x1 + c * x2 + d * x3, y: a * y0 + b * y1 + c * y2 + d * y3 };
  }
  // A routed way: its runs end where each command ends. A rounded corner is short, so it is read as its chord.
  const points = [...path.matchAll(/[MLQ]([^MLQ]*)/g)].flatMap((command) => {
    const n = command[1]?.match(NUMBER)?.map(Number) ?? [];
    return n.length < 2 ? [] : [{ x: n[n.length - 2]!, y: n[n.length - 1]! }];
  });
  if (points.length === 0) return undefined;
  const lengths = points.slice(1).map((one, at) => Math.hypot(one.x - points[at]!.x, one.y - points[at]!.y));
  let left = lengths.reduce((sum, one) => sum + one, 0) * t;
  for (const [at, length] of lengths.entries()) {
    if (left <= length && length > 0) {
      const a = points[at]!;
      const b = points[at + 1]!;
      return { x: a.x + ((b.x - a.x) * left) / length, y: a.y + ((b.y - a.y) * left) / length };
    }
    left -= length;
  }
  return points[points.length - 1];
}

const overlap = (a: Rect, b: Rect) => {
  const w = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
  const h = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
  return w > 0 && h > 0 ? w * h : 0;
};

/** Middle first, then outwards either side of it. */
const ALONG = [0.5, 0.44, 0.56, 0.38, 0.62, 0.32, 0.68, 0.26, 0.74, 0.2, 0.8, 0.14, 0.86];

export function placeLabels(
  edges: readonly { id: string; label: string | undefined; path: string }[],
  boxes: readonly Rect[],
  m: LabelMetrics,
): Map<string, LabelPlace> {
  const placed = new Map<string, LabelPlace>();
  const taken: Rect[] = [];
  for (const edge of edges) {
    if (edge.label === undefined || edge.label.trim() === "") continue;
    const lines = wrapLabel(edge.label);
    const w = Math.max(...lines.map((one) => one.length)) * m.charW + m.pad * 2;
    const h = lines.length * m.lineH + m.pad * 2;
    let best: LabelPlace | undefined;
    let bestCost = Number.POSITIVE_INFINITY;
    for (const [index, t] of ALONG.entries()) {
      const at = pathAt(edge.path, t);
      if (at === undefined) continue;
      const rect = { x: at.x - w / 2, y: at.y - h / 2, w, h };
      // A hit on a box costs more than one on another label, and distance from the middle breaks ties.
      const cost = boxes.reduce((sum, one) => sum + overlap(rect, one) * 4, 0) + taken.reduce((sum, one) => sum + overlap(rect, one), 0) + index;
      if (cost < bestCost) {
        bestCost = cost;
        best = { lines, x: at.x, y: at.y, w, h, lineH: m.lineH, pad: m.pad };
      }
      if (cost === index) break;
    }
    if (best === undefined) continue;
    placed.set(edge.id, best);
    taken.push({ x: best.x - best.w / 2, y: best.y - best.h / 2, w: best.w, h: best.h });
  }
  return placed;
}
