// The arithmetic under `JobTimeline`: which row a bar sits in, which part of the time axis is
// showing, and where its ticks fall. Apart from the component so it is tested as arithmetic.

/** A stretch of time, in epoch milliseconds. */
export type Window = { start: number; width: number };

/** The least a window may show, so a zoom never ends in a single instant. */
export const LEAST_WIDTH_MS = 60_000;

/** The span every bar fits in, with a little air at the end so a live bar is not on the edge. */
export function spanOf(bars: readonly { from: number; to: number }[], now: number): Window {
  if (bars.length === 0) return { start: now - LEAST_WIDTH_MS, width: LEAST_WIDTH_MS };
  const start = Math.min(...bars.map((bar) => bar.from));
  const end = Math.max(now, ...bars.map((bar) => bar.to));
  return { start, width: Math.max(end - start, LEAST_WIDTH_MS) };
}

/** `win` kept inside `span` and no narrower than `LEAST_WIDTH_MS`. */
export function clampWindow(win: Window, span: Window): Window {
  const width = Math.min(Math.max(win.width, LEAST_WIDTH_MS), span.width);
  const start = Math.min(Math.max(win.start, span.start), span.start + span.width - width);
  return { start, width };
}

/** `win` made `factor` times narrower about `focus`, which keeps its place on screen. */
export function zoomed(win: Window, span: Window, factor: number, focus: number): Window {
  const width = Math.min(Math.max(win.width / factor, LEAST_WIDTH_MS), span.width);
  const at = (focus - win.start) / win.width;
  return clampWindow({ start: focus - at * width, width }, span);
}

/** `win`, moved the least it takes for `t` to be inside it with a margin. */
export function following(win: Window, span: Window, t: number): Window {
  const margin = win.width * 0.05;
  if (t < win.start + margin) return clampWindow({ ...win, start: t - margin }, span);
  if (t > win.start + win.width - margin) return clampWindow({ ...win, start: t - win.width + margin }, span);
  return win;
}

/** Where `t` falls across `win`, as a percentage. Outside it, below 0 or above 100. */
export function percentOf(win: Window, t: number): number {
  return ((t - win.start) / win.width) * 100;
}

/** Whether anything from `from` to `to` is inside `win`. */
export function reaches(win: Window, from: number, to: number): boolean {
  return to >= win.start && from <= win.start + win.width;
}

export type Placed = { id: string; from: number; to: number; parent?: string };

/**
 * One row per bar so that no two in a row overlap, in the fewest rows greedy placement finds.
 *
 * **Placed in the order they start**, so a parent is always placed before what it dispatched, and
 * a child takes its parent's row when that is free: a chain reads straight across. Otherwise the
 * highest row that is free, which keeps the rows at the top full and the tail short.
 */
export function packRows(bars: readonly Placed[], gap = 0): Map<string, number> {
  const rowOf = new Map<string, number>();
  const freeFrom: number[] = [];
  for (const bar of [...bars].sort((a, b) => a.from - b.from || a.to - b.to || a.id.localeCompare(b.id))) {
    const free = (row: number) => freeFrom[row] === undefined || freeFrom[row]! + gap <= bar.from;
    const parent = bar.parent === undefined ? undefined : rowOf.get(bar.parent);
    let row = parent !== undefined && free(parent) ? parent : freeFrom.findIndex((_, at) => free(at));
    if (row === -1) row = freeFrom.length;
    freeFrom[row] = bar.to;
    rowOf.set(bar.id, row);
  }
  return rowOf;
}

const STEPS_MS = [
  60_000, 300_000, 900_000, 1_800_000, 3_600_000, 3 * 3_600_000, 6 * 3_600_000, 12 * 3_600_000, 86_400_000,
  7 * 86_400_000,
];

/** Tick instants across `win`, on round times, about `across` of them. */
export function ticksOf(win: Window, across = 8): number[] {
  const step = STEPS_MS.find((one) => win.width / one <= across) ?? STEPS_MS[STEPS_MS.length - 1]!;
  const out: number[] = [];
  // Local midnight is the origin for a day or more, so a tick lands on a day and not on 01:00.
  const offset = step >= 86_400_000 ? new Date(win.start).getTimezoneOffset() * 60_000 : 0;
  for (let t = Math.ceil((win.start - offset) / step) * step + offset; t <= win.start + win.width; t += step) out.push(t);
  return out;
}

/** A tick's label: the time of day, or the date once ticks are a day or more apart. */
export function tickLabel(t: number, win: Window): string {
  const at = new Date(t);
  return win.width > 3 * 86_400_000
    ? at.toLocaleDateString([], { month: "short", day: "numeric" })
    : at.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", hour12: false });
}
