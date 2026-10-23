// The Pixi take of the Artifacts timeline, as arithmetic: which track each artifact stands on, and how time maps to the canvas as it is zoomed
// and panned. The same `Timeline` as the other takes draw (`artifact-timeline.ts`); only the drawing differs.
//
// **Labels win over compactness.** A line takes the room it really needs on its track, its first point to the end of its full title (cut past
// `MOST_TITLE`), measured with the canvas's own type, and lines are packed greedily in time order onto the first track on a side, nearest the Job,
// where that room is free and so is the way its branch takes to get there. Zooming changes how wide a title is in time, so it packs again and
// lines fold back towards the Job's line.
//
// A branch leaves its parent in a curve over `STEM` pixels and arrives level on its own track. A title sits beside its line's first point, on the
// far side of the line from the Job, so the branch that reaches the line never runs through the line's own title. A branch may cross another
// line, which the canvas draws it faint behind, but is kept from crossing another line's title where a track allows.

import type { Placed, Timeline, TimelineLine } from "./artifact-timeline";

export const ROW = 52;
export const POINT = 24;
export const STEM = 44;
/** Left of where time begins, for the Job's own mark; right of where it ends. */
export const LEFT = 48;
export const RIGHT = 24;
const GAP = 12;
/** A title is shown whole up to this, and cut past it. */
export const MOST_TITLE = 240;
/** From a line's first point to where its title begins, and from where it ends to the next thing on the track. */
const TO_TITLE = POINT / 2 + 4;
const MINUTE = 60_000;
/** A tick's label needs this much room. */
const TICK_ROOM = 72;
const STEPS = [1, 5, 15, 30, 60, 120, 240].map((one) => one * MINUTE);

export type Side = "up" | "down";
export type Row = {
  line: TimelineLine;
  side: Side;
  track: number;
  /** The line's y from the top of the plot. */
  y: number;
  /** The parent it branches from: the Job's own line, or another row's artifact. */
  parent: { job: true } | { job?: undefined; row: string };
  /** Where its title stands as packed (the plot's left at the Job's start): its left edge, its width, and whether it hangs left of the first point. */
  label: { x: number; w: number; flip: boolean };
};

export type Tracks = { rows: Row[]; up: number; down: number; jobY: number; height: number };

/** The y of a line on a track: the Job's row between, labels on the far side of their lines. */
const yOf = (side: Side, track: number, up: number): number => (side === "up" ? (up - 1 - track) * ROW + 36 : (up + 1 + track) * ROW + 16);

type Span = { a: number; b: number };
/** What a line holds on its track: all of it, and the part its title is written in. */
type Foot = Span & { title: Span };

const overlap = (one: Span, two: Span) => one.a < two.b && two.a < one.b;

/**
 * Packs every line onto a track, in the order they appeared. A line takes the first track, nearest the Job, where the room its title really needs is
 * free and where the branch that carries it there crosses no one else's title. The branch may cross another line, which is drawn behind it.
 * `view.k` is the zoom it is packed at, so zooming packs again; panning moves everything together and changes nothing.
 */
export function tracksOf(timeline: Timeline, view: View, measure: (title: string) => number): Tracks {
  const x = (t: number) => xOf(view, t);
  const reach = x(timeline.end) + RIGHT;
  const feet: Record<Side, Foot[][]> = { up: [], down: [] };
  const stems: Record<Side, Span[][]> = { up: [], down: [] };
  const placed = new Map<string, { side: Side; track: number }>();
  const made: Omit<Row, "y">[] = [];

  for (const [index, line] of timeline.lines.entries()) {
    const first = x(line.points[0]!.t);
    const last = x(line.points[line.points.length - 1]!.t);
    const w = Math.min(measure(line.title), MOST_TITLE);
    // A title that would run past the end of the Job hangs left of its first point instead, level with it.
    const flip = first + TO_TITLE + w + GAP > reach && first - TO_TITLE - w - GAP > LEFT - POINT;
    const label = flip ? { x: first + POINT / 2 - w, w, flip } : { x: first + TO_TITLE, w, flip };
    const title = flip ? { a: label.x - GAP, b: first + POINT / 2 } : { a: label.x, b: label.x + w + GAP };
    const whole: Foot = { a: Math.min(first - POINT / 2, title.a), b: Math.max(last + POINT / 2, title.b), title };
    const owner = line.parents.find((one) => one.key !== "job" && placed.has(one.key))?.key;
    const above = owner === undefined ? undefined : placed.get(owner);
    const stem: Span = { a: first - STEM, b: first };

    const sides: Side[] = above === undefined ? ["up", "down"] : [above.side];
    const choices = sides.map((side) => {
      const count = Math.max(feet[side].length, stems[side].length);
      const between = (track: number) => {
        const low = above === undefined ? 0 : Math.min(track, above.track) + 1;
        const high = above === undefined ? track : Math.max(track, above.track);
        return Array.from({ length: Math.max(0, high - low) }, (_, i) => low + i);
      };
      // Another line's title is in the way of this branch, on a track it passes or on this one before the line begins.
      const blocked = (track: number) =>
        [...between(track), track].some((at) => (feet[side][at] ?? []).some((one) => overlap(one.title, at === track ? { a: stem.a, b: whole.a } : stem)));
      const room = (track: number) =>
        !(feet[side][track] ?? []).some((one) => one.a < whole.b + GAP && whole.a < one.b + GAP) && !(stems[side][track] ?? []).some((one) => overlap(one, title));
      for (let track = 0; track <= count; track++) {
        if (above !== undefined && track === above.track) continue;
        if (room(track) && !blocked(track)) return { side, track, through: between(track), clean: true };
      }
      return { side, track: count, through: between(count), clean: false };
    });
    const best = choices.sort((a, b) => Number(b.clean) - Number(a.clean) || a.track - b.track || (index % 2 === 0 ? (a.side === "up" ? -1 : 1) : a.side === "down" ? -1 : 1))[0]!;
    (feet[best.side][best.track] ??= []).push(whole);
    for (const through of best.through) (stems[best.side][through] ??= []).push(stem);
    placed.set(line.key, { side: best.side, track: best.track });
    made.push({ line, side: best.side, track: best.track, parent: owner === undefined ? { job: true } : { row: owner }, label });
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

/** The points as `neighbour` reads them: the arrows go by where things are drawn now. */
export function placedOf(tracks: Tracks, view: View): Placed[] {
  return tracks.rows.map((row) => ({
    line: row.line,
    y: row.y,
    xs: row.line.points.map((point) => xOf(view, point.t)),
    from: xOf(view, row.line.points[0]!.t) - POINT / 2,
    to: xOf(view, row.line.points[row.line.points.length - 1]!.t) + POINT / 2,
    title: row.label.w,
  }));
}
