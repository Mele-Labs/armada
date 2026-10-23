// The Pixi take of the Artifacts timeline, as arithmetic: which track each artifact stands on, and how time maps to the canvas as it is zoomed
// and panned. The same `Timeline` as the other takes draw (`artifact-timeline.ts`); only the drawing differs. Tracks are laid out once at the
// fitted zoom and kept, since zooming in only draws everything further apart, and a label is cut to the room it has at the zoom it is drawn at.
//
// A branch leaves its parent in a curve over `STEM` pixels and arrives level on its own track, so the way out is the one place it can cross
// anything. A line is put on the first track on a side where its own extent is free and so is every track its branch passes through; where no
// track on either side is, it takes a new one. A label sits on the far side of its line from the Job, so the branch that reaches the line never
// runs through the line's own label.

import type { Placed, Timeline, TimelineLine } from "./artifact-timeline";

export const ROW = 52;
export const BAND = 36;
export const POINT = 24;
export const STEM = 44;
/** Left of where time begins, for the Job's own mark; right of where it ends. */
export const LEFT = 48;
export const RIGHT = 24;
const GAP = 12;
const CHAR = 7;
const WANT = 110;
const MINUTE = 60_000;
/** A tick's label needs this much room. */
const TICK_ROOM = 72;
const STEPS = [1, 5, 15, 30, 60, 120, 240].map((one) => one * MINUTE);

export type Side = "up" | "down";
export type Row = {
  line: TimelineLine;
  side: Side;
  track: number;
  /** The line's y from the top of the plot, below the band. */
  y: number;
  /** The parent it branches from: the Job's own line, or another row's artifact. */
  parent: { job: true } | { job?: undefined; row: string };
  /** The tracks its branch passes through on its way, for the labels there to give way to it. */
  passes: { side: Side; track: number }[];
};

export type Tracks = { rows: Row[]; up: number; down: number; jobY: number; height: number };

const natural = (title: string) => Math.min(title.length, 40) * CHAR + 8;

/** The y of a line on a track: the Job's row between, labels on the far side of their lines. */
const yOf = (side: Side, track: number, up: number): number => (side === "up" ? (up - 1 - track) * ROW + 36 : (up + 1 + track) * ROW + 16);

type Span = { a: number; b: number };

export function tracksOf(timeline: Timeline, panel: number): Tracks {
  const span = timeline.end - timeline.start;
  const k = span === 0 ? 0 : Math.max(0, (panel - LEFT - RIGHT) / span);
  const x = (t: number) => LEFT + (t - timeline.start) * k;
  const taken: Record<Side, Span[][]> = { up: [], down: [] };
  const free = (side: Side, track: number, a: number, b: number) => !(taken[side][track] ?? []).some((one) => one.a < b && a < one.b);
  const claim = (side: Side, track: number, a: number, b: number) => void (taken[side][track] ??= []).push({ a, b });
  const placed = new Map<string, { side: Side; track: number }>();
  const made: Omit<Row, "y">[] = [];

  for (const [index, line] of timeline.lines.entries()) {
    const first = x(line.points[0]!.t);
    const from = first - POINT / 2;
    const to = Math.max(x(line.points[line.points.length - 1]!.t) + POINT / 2, from + POINT + Math.min(natural(line.title), WANT));
    const above = line.parents.map((one) => (one.key === "job" ? undefined : placed.get(one.key))).find((one) => one !== undefined);
    const owner = line.parents.find((one) => one.key !== "job" && placed.has(one.key))?.key;
    const stem = { a: first - STEM, b: first + POINT / 2 };
    const sides: Side[] = above === undefined ? ["up", "down"] : [above.side];
    const choices = sides.map((side) => {
      const count = taken[side].length;
      for (let track = 0; track <= count; track++) {
        if (above !== undefined && track === above.track) continue;
        const from0 = above === undefined ? 0 : Math.min(track, above.track) + 1;
        const to0 = above === undefined ? track : Math.max(track, above.track);
        const through = Array.from({ length: Math.max(0, to0 - from0) }, (_, i) => from0 + i);
        if (free(side, track, from, to) && through.every((one) => free(side, one, stem.a, stem.b))) return { side, track, through, clean: true };
      }
      const from0 = above === undefined ? 0 : Math.min(count, above.track) + 1;
      const to0 = above === undefined ? count : Math.max(count, above.track);
      return { side, track: count, through: Array.from({ length: Math.max(0, to0 - from0) }, (_, i) => from0 + i), clean: false };
    });
    const best = choices.sort((a, b) => Number(b.clean) - Number(a.clean) || a.track - b.track || (index % 2 === 0 ? (a.side === "up" ? -1 : 1) : a.side === "down" ? -1 : 1))[0]!;
    claim(best.side, best.track, from, to);
    for (const through of best.through) claim(best.side, through, stem.a, stem.b);
    placed.set(line.key, { side: best.side, track: best.track });
    made.push({ line, side: best.side, track: best.track, parent: owner === undefined ? { job: true } : { row: owner }, passes: best.through.map((track) => ({ side: best.side, track })) });
  }
  const up = Math.max(0, ...made.filter((one) => one.side === "up").map((one) => one.track + 1));
  const down = Math.max(0, ...made.filter((one) => one.side === "down").map((one) => one.track + 1));
  return {
    rows: made.map((one) => ({ ...one, y: yOf(one.side, one.track, up) })),
    up,
    down,
    jobY: up * ROW + 26,
    height: (up + 1 + down) * ROW + 16,
  };
}

// ---- Time on the canvas ------------------------------------------------------------------------------------------------------------

/** What time is drawn at: the instant at the left edge of the plot, and the pixels to a unit. */
export type View = { t0: number; k: number };

/** Pixels to a unit when the whole Job is across the plot. */
export const fitK = (timeline: Timeline, width: number): number => {
  const span = timeline.end - timeline.start;
  return span === 0 ? 0 : Math.max(0, (width - LEFT - RIGHT) / span);
};

export const fitted = (timeline: Timeline, width: number): View => ({ t0: timeline.start, k: fitK(timeline, width) });

/** The most it zooms: a minute is wide enough to read. Untimed, a step of the order. */
const kMax = (timeline: Timeline, width: number): number => Math.max(fitK(timeline, width), timeline.timed ? 140 / MINUTE : 160);

/** The view kept to the Job: never zoomed out past the whole of it, and never panned off either end. */
export function clamp(view: View, timeline: Timeline, width: number): View {
  const floor = fitK(timeline, width);
  const k = Math.min(kMax(timeline, width), Math.max(floor, view.k));
  if (k === 0) return { t0: timeline.start, k };
  const room = (width - LEFT - RIGHT) / k;
  return { t0: Math.min(timeline.end - room, Math.max(timeline.start, view.t0)), k };
}

/** Zoomed by `factor` about the instant under `at` (an x on the canvas), which stays where it is. */
export function zoomed(view: View, factor: number, at: number, timeline: Timeline, width: number): View {
  const under = view.t0 + (at - LEFT) / view.k;
  const k = view.k * factor;
  return clamp({ t0: under - (at - LEFT) / k, k }, timeline, width);
}

export const panned = (view: View, dx: number, timeline: Timeline, width: number): View => clamp({ t0: view.t0 - dx / view.k, k: view.k }, timeline, width);

export const xOf = (view: View, t: number): number => LEFT + (t - view.t0) * view.k;

/** Ticks at round local clock times: the smallest step, from a minute to hours, that leaves each label room. Untimed, none. */
export function ticksFor(timeline: Timeline, view: View, width: number): { t: number; x: number }[] {
  if (!timeline.timed || view.k === 0) return [];
  const step = STEPS.find((one) => one * view.k >= TICK_ROOM) ?? STEPS[STEPS.length - 1]!;
  const zone = new Date(timeline.start).getTimezoneOffset() * MINUTE;
  const left = view.t0 - 12 / view.k;
  const right = view.t0 + (width - LEFT) / view.k;
  const out: { t: number; x: number }[] = [];
  for (let t = Math.ceil((left - zone) / step) * step + zone; t <= right; t += step) {
    const x = xOf(view, t);
    if (x >= LEFT - 12 && x <= width - 12) out.push({ t, x });
  }
  return out;
}

/** The fitted points as `neighbour` reads them: the arrows go by where things are drawn at the fit, which is the order at any zoom. */
export function placedOf(tracks: Tracks, timeline: Timeline, width: number): Placed[] {
  const x = (t: number) => xOf(fitted(timeline, width), t);
  return tracks.rows.map((row) => ({ line: row.line, y: row.y, xs: row.line.points.map((point) => x(point.t)), from: x(row.line.points[0]!.t) - POINT / 2, to: x(row.line.points[row.line.points.length - 1]!.t) + POINT / 2, title: natural(row.line.title) }));
}

/** The room a title has: to the next thing on its track or the branch that crosses it, never past the edge. */
export function roomFor(row: Row, tracks: Tracks, view: View, width: number, labelX: number): number {
  let edge = width - RIGHT;
  for (const other of tracks.rows) {
    const x = xOf(view, other.line.points[0]!.t);
    if (other !== row && other.side === row.side && other.track === row.track && x - POINT / 2 > labelX) edge = Math.min(edge, x - POINT / 2 - GAP);
    if (other !== row && other.passes.some((one) => one.side === row.side && one.track === row.track) && x - STEM > labelX) edge = Math.min(edge, x - STEM - GAP);
  }
  return edge - labelX;
}
