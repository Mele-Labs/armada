import { ZoomIn, ZoomOut } from "lucide-react";
import { useMemo, useRef, useState } from "react";
import type { KeyboardEvent, PointerEvent as ReactPointerEvent } from "react";

import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { WaveCanvas } from "../WaveCanvas/WaveCanvas";
import type { WaveCanvasCard, WaveCanvasEdge, WaveCanvasNode } from "../WaveCanvas/WaveCanvas";
import type { Window } from "./timeline";
import { clampWindow, following, LEAST_WIDTH_MS, packRows, percentOf, reaches, spanOf, ticksOf, tickLabel, zoomed } from "./timeline";

/**
 * Jobs as the graph view's cards, placed left to right by when they started, with a playhead on
 * the time axis a person drags. An edge runs from the Job that dispatched another to it. `#920`.
 *
 * **The cards are `WaveCanvas`'s own**, with their acts, and so are the edges: this only decides
 * where each sits. Cards that would overlap in time share a row only when they do not touch, and
 * only the cards that start inside the window are drawn, so 150 Jobs are as many as the window
 * holds. The window zooms.
 */
export type JobTimelineBar = {
  /** The Job's id. */
  id: string;
  /** The card, less its state: that is the playhead's to say. */
  card: Omit<WaveCanvasCard, "status" | "dimmed">;
  /** The state the Job was in at the playhead, a registered `job_status` wire value. Absent before it existed. */
  status: string | null;
  /** Epoch milliseconds: first started, or created where it never started. */
  from: number;
  /** Epoch milliseconds: ended, or now where it is live. */
  to: number;
};

/** One Job dispatching another. */
export type JobTimelineDispatch = {
  /** The Job whose Drone dispatched it. */
  parent: string;
  child: string;
  /** Epoch milliseconds the child was minted. */
  at: number;
};

export type JobTimelineProps = {
  bars: readonly JobTimelineBar[];
  dispatches: readonly JobTimelineDispatch[];
  /** Epoch milliseconds. Never later than `now`. */
  playhead: number;
  now: number;
  /** The playhead moved, by drag or key. */
  onPlayhead: (t: number) => void;
};

const ZOOM = 2;
const KEY_STEPS = 50;
/** The window is this wide on the canvas, in its own units; a card is `--w-workflow-node`. */
const CANVAS_W = 2400;
const CARD_W = 260;
const CARD_GAP = 24;
const DOWN = 132;
/** The window opens on this share of the whole span, ending at now. */
const OPENS_ON = 4;

export function JobTimeline({ bars, dispatches, playhead, now, onPlayhead }: JobTimelineProps) {
  const span = spanOf(bars, now);
  const [asked, setAsked] = useState<Window | null>(null);
  const first = { start: span.start + span.width - span.width / OPENS_ON, width: span.width / OPENS_ON };
  const win = clampWindow(asked ?? first, span);
  const track = useRef<HTMLDivElement>(null);

  const { nodes, edges } = useMemo(() => {
    const perMs = CANVAS_W / win.width;
    const cardMs = (CARD_W + CARD_GAP) / perMs;
    const shown = bars.filter((bar) => reaches(win, bar.from, bar.from));
    const parentOf = new Map(dispatches.map((one) => [one.child, one.parent]));
    const rows = packRows(shown.map((bar) => ({ id: bar.id, from: bar.from, to: bar.from + cardMs, parent: parentOf.get(bar.id) })));
    const held = new Set(shown.map((bar) => bar.id));
    const titles = new Map(shown.map((bar) => [bar.id, bar.card.title]));
    const nodes: WaveCanvasNode[] = shown.map((bar) => ({
      id: `job:${bar.id}`,
      position: { x: (bar.from - win.start) * perMs, y: rows.get(bar.id)! * DOWN },
      card: { ...bar.card, status: bar.status ?? "queued", dimmed: playhead < bar.from },
    }));
    const edges: WaveCanvasEdge[] = dispatches
      .filter((one) => held.has(one.parent) && held.has(one.child))
      .map((one) => ({
        id: `${one.parent}>${one.child}`,
        source: `job:${one.parent}`,
        target: `job:${one.child}`,
        said: `${titles.get(one.child)} dispatched from ${titles.get(one.parent)}`,
      }));
    return { nodes, edges };
  }, [bars, dispatches, win.start, win.width, playhead]);

  const move = (t: number) => {
    const at = Math.min(Math.max(t, span.start), now);
    onPlayhead(at);
    setAsked((was) => following(clampWindow(was ?? first, span), span, at));
  };
  const timeAt = (clientX: number) => {
    const box = track.current!.getBoundingClientRect();
    return win.start + ((clientX - box.left) / box.width) * win.width;
  };
  // Listened for on the window: a drag follows the pointer off the handle and out of the track.
  const scrub = (event: ReactPointerEvent) => {
    event.preventDefault();
    move(timeAt(event.clientX));
    const over = (next: PointerEvent) => move(timeAt(next.clientX));
    const done = () => {
      window.removeEventListener("pointermove", over);
      window.removeEventListener("pointerup", done);
    };
    window.addEventListener("pointermove", over);
    window.addEventListener("pointerup", done);
  };
  const press = (event: KeyboardEvent) => {
    const step = win.width / KEY_STEPS;
    const to: Record<string, number> = {
      ArrowLeft: playhead - step,
      ArrowRight: playhead + step,
      PageUp: playhead - step * 10,
      PageDown: playhead + step * 10,
      Home: span.start,
      End: now,
    };
    const at = to[event.key];
    if (at === undefined) return;
    event.preventDefault();
    move(at);
  };
  const zoom = (factor: number) =>
    setAsked(zoomed(win, span, factor, playhead >= win.start && playhead <= win.start + win.width ? playhead : win.start + win.width / 2));
  const head = Math.min(Math.max(percentOf(win, playhead), 0), 100);

  return (
    <div className="armada-timeline">
      <div className="armada-timeline__bar">
        <Tooltip label="Playhead">
          <time className="armada-timeline__at" dateTime={new Date(playhead).toISOString()}>
            {new Date(playhead).toLocaleString([], { month: "short", day: "numeric", hour: "2-digit", minute: "2-digit", hour12: false })}
          </time>
        </Tooltip>
        <Tooltip label="More time">
          <Button variant="ghost" size="sm" iconOnly aria-label="More time" disabled={win.width >= span.width} onClick={() => zoom(1 / ZOOM)}>
            <ZoomOut size={16} />
          </Button>
        </Tooltip>
        <Tooltip label="Less time">
          <Button variant="ghost" size="sm" iconOnly aria-label="Less time" disabled={win.width <= LEAST_WIDTH_MS} onClick={() => zoom(ZOOM)}>
            <ZoomIn size={16} />
          </Button>
        </Tooltip>
      </div>
      <div className="armada-timeline__canvas">
        <WaveCanvas nodes={nodes} edges={edges} label="Jobs by when they started, and what dispatched what" />
      </div>
      <div className="armada-timeline__ruler" ref={track} onPointerDown={scrub}>
        {ticksOf(win).map((t) => (
          <span key={t} className="armada-timeline__tick" style={{ left: `${percentOf(win, t)}%` }}>
            {tickLabel(t, win)}
          </span>
        ))}
        <div
          className="armada-timeline__playhead"
          style={{ left: `${head}%` }}
          role="slider"
          tabIndex={0}
          aria-label="Playhead"
          aria-valuemin={span.start}
          aria-valuemax={now}
          aria-valuenow={playhead}
          aria-valuetext={new Date(playhead).toLocaleString()}
          onPointerDown={scrub}
          onKeyDown={press}
        />
      </div>
    </div>
  );
}
