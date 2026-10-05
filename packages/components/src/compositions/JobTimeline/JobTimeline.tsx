import { ZoomIn, ZoomOut } from "lucide-react";
import { Fragment, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { CSSProperties, KeyboardEvent, PointerEvent as ReactPointerEvent } from "react";

import { JOB_STATUS } from "../../generated/vocabulary";
import { Button } from "../../primitives/Button/Button";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import { WaveJobCard } from "../WaveCanvas/WaveCanvas";
import type { WaveCanvasCard } from "../WaveCanvas/WaveCanvas";
import type { Window } from "./timeline";
import { clampWindow, following, laneRows, LEAST_WIDTH_MS, percentOf, reaches, spanOf, ticksOf, tickLabel, zoomed } from "./timeline";

/**
 * Jobs in lanes, one lane per family, on a time axis with a playhead a person drags. `#920`.
 *
 * **A family is a root Job and everything it dispatched, however deep.** Each Job is a dot and a
 * short title at the moment it started, with a thin line along its row for how long it ran. A
 * child branches off its dispatcher's row at the instant it was minted, by an edge that never
 * leaves the family's lane; a family's concurrent Jobs stack in rows inside it. Nothing crosses
 * from one lane into another, so 150 Jobs are as many lanes as the window holds.
 *
 * **The card is the card `WaveCanvas` draws, shown once, below the lanes**, for the Job a person
 * points at, focuses or presses: its acts are the list row's own.
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

/** A root Job and every Job under it, the root first. */
export type JobTimelineFamily = { root: string; members: readonly string[] };

export type JobTimelineProps = {
  bars: readonly JobTimelineBar[];
  dispatches: readonly JobTimelineDispatch[];
  /** One lane each, in the order drawn. */
  families: readonly JobTimelineFamily[];
  /** Jobs with no parent and no children on the board: a quiet lane of their own, under the families. */
  alone: readonly string[];
  /** Epoch milliseconds. Never later than `now`. */
  playhead: number;
  now: number;
  /** The playhead moved, by drag or key. */
  onPlayhead: (t: number) => void;
};

const ZOOM = 2;
const KEY_STEPS = 50;
/** The window opens on this share of the whole span, ending at now. */
const OPENS_ON = 4;
/** A node's dot and title, in pixels, for how much of a row it takes. */
const NODE_PX = 168;
/** How long a pointer rests on a node before the card follows it, so crossing the lanes shows nothing. */
const REST_MS = 90;

const tokenOf = (status: string | null) => `var(${JOB_STATUS[status ?? "queued"]?.statusToken ?? "--status-not-started"})`;
const verbOf = (status: string | null) => JOB_STATUS[status ?? "queued"]?.verb ?? status ?? "";

export function JobTimeline({ bars, dispatches, families, alone, playhead, now, onPlayhead }: JobTimelineProps) {
  const span = spanOf(bars, now);
  const [asked, setAsked] = useState<Window | null>(null);
  const first = { start: span.start + span.width - span.width / OPENS_ON, width: span.width / OPENS_ON };
  const win = clampWindow(asked ?? first, span);
  const track = useRef<HTMLDivElement>(null);
  const dock = useRef<HTMLDivElement>(null);
  const [trackPx, setTrackPx] = useState(960);
  const [shown, setShown] = useState<string | null>(null);
  const rest = useRef<number | undefined>(undefined);

  useLayoutEffect(() => {
    const el = track.current;
    if (el === null) return;
    const measure = () => el.getBoundingClientRect().width > 0 && setTrackPx(el.getBoundingClientRect().width);
    measure();
    const watch = new ResizeObserver(measure);
    watch.observe(el);
    return () => watch.disconnect();
  }, []);
  useEffect(() => () => window.clearTimeout(rest.current), []);

  const byId = new Map(bars.map((bar) => [bar.id, bar]));
  const parentOf = new Map(dispatches.map((one) => [one.child, one.parent]));
  const madeAt = new Map(dispatches.map((one) => [one.child, one.at]));
  const labelMs = (NODE_PX / trackPx) * win.width;

  const lane = (ids: readonly string[]) => {
    const here = ids.flatMap((id) => {
      const bar = byId.get(id);
      // One that began before the window keeps its title at the edge, over the line of how long it ran.
      return bar !== undefined && reaches(win, bar.from, bar.to) ? [{ ...bar, from: Math.max(bar.from, win.start), before: bar.from < win.start }] : [];
    });
    return { here, ...laneRows(here, parentOf, labelMs) };
  };
  const lanes = [
    ...families.map((one) => ({ key: one.root, root: byId.get(one.root), ...lane(one.members) })),
    ...(alone.length === 0 ? [] : [{ key: "alone", root: undefined, ...lane(alone) }]),
  ].filter((one) => one.here.length > 0);

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

  const point = (id: string) => {
    window.clearTimeout(rest.current);
    rest.current = window.setTimeout(() => setShown(id), REST_MS);
  };
  const open = (id: string) => {
    window.clearTimeout(rest.current);
    setShown(id);
    // Into the card, where its acts are: the card comes after the lanes, so Tab would cross every Job to reach it.
    requestAnimationFrame(() => dock.current?.querySelector("button")?.focus());
  };
  const away = (event: KeyboardEvent) => {
    if (event.key !== "Escape" || shown === null) return;
    event.preventDefault();
    const was = shown;
    setShown(null);
    (document.querySelector(`[data-timeline-job="${was}"]`) as HTMLElement | null)?.focus();
  };

  const card = shown === null ? undefined : byId.get(shown);

  const nodes = (here: readonly (JobTimelineBar & { before: boolean })[], rows: ReadonlyMap<string, number>) => {
    const held = new Set(here.map((bar) => bar.id));
    return (
      <>
        {here.flatMap((bar) => {
          const parent = parentOf.get(bar.id);
          const made = madeAt.get(bar.id);
          if (parent === undefined || made === undefined || !held.has(parent)) return [];
          const from = rows.get(parent)!;
          const to = rows.get(bar.id)!;
          const style = {
            left: `${percentOf(win, made)}%`,
            width: `${Math.max(percentOf(win, bar.from) - percentOf(win, made), 0)}%`,
            "--from": Math.min(from, to),
            "--rows": Math.abs(to - from),
          } as CSSProperties;
          return [
            <span
              key={`edge:${bar.id}`}
              className="armada-timeline__edge"
              data-down={to >= from || undefined}
              data-ghost={bar.status === null || undefined}
              role="img"
              aria-label={`${bar.card.title} dispatched from ${byId.get(parent)!.card.title}`}
              style={style}
            />,
          ];
        })}
        {here.map((bar) => {
          const style = { left: `${percentOf(win, bar.from)}%`, "--row": rows.get(bar.id), "--state": tokenOf(bar.status) } as CSSProperties;
          return (
            <Fragment key={bar.id}>
              {bar.status === null ? null : (
                <span
                  className="armada-timeline__span"
                  style={{ ...style, width: `${Math.max(percentOf(win, bar.to) - percentOf(win, bar.from), 0)}%` }}
                />
              )}
              <button
                type="button"
                className="armada-timeline__node"
                data-timeline-job={bar.id}
                data-shown={bar.id === shown || undefined}
                data-before={bar.before || undefined}
                data-ghost={bar.status === null || undefined}
                aria-label={`${bar.card.title}, ${verbOf(bar.status)}`}
                style={style}
                onPointerEnter={() => point(bar.id)}
                onPointerLeave={() => window.clearTimeout(rest.current)}
                onFocus={() => setShown(bar.id)}
                onClick={() => open(bar.id)}
              >
                <span className="armada-timeline__dot" />
                <span className="armada-timeline__label">{bar.card.title}</span>
              </button>
            </Fragment>
          );
        })}
      </>
    );
  };

  return (
    <div className="armada-timeline" onKeyDown={away}>
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
      <div className="armada-timeline__scroll" onPointerDown={(event) => event.target === event.currentTarget && setShown(null)}>
        <div className="armada-timeline__lanes">
          <div className="armada-timeline__ruler">
            <div className="armada-timeline__ruler-track" ref={track} onPointerDown={scrub}>
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
          <div className="armada-timeline__line" aria-hidden="true">
            <span style={{ left: `${head}%` }} />
          </div>
          {lanes.map((one) => (
            <section
              key={one.key}
              className="armada-timeline__lane"
              data-alone={one.root === undefined || undefined}
              aria-label={one.root === undefined ? "Not dispatched and dispatching nothing" : `${one.root.card.title}, and what it dispatched`}
              style={{ "--rows": one.count, "--state": tokenOf(one.root?.status ?? null) } as CSSProperties}
            >
              <header className="armada-timeline__family" data-ghost={one.root?.status === null || undefined}>
                {one.root === undefined ? null : (
                  <>
                    <span className="armada-timeline__family-title">{one.root.card.title}</span>
                    {one.root.card.handle === undefined ? null : <span className="armada-timeline__family-handle mono">{one.root.card.handle}</span>}
                  </>
                )}
              </header>
              <div className="armada-timeline__row-track">{nodes(one.here, one.rows)}</div>
            </section>
          ))}
        </div>
      </div>
      <div className="armada-timeline__dock" ref={dock}>
        {card === undefined ? null : <WaveJobCard card={{ ...card.card, status: card.status ?? "queued", dimmed: card.status === null }} />}
      </div>
    </div>
  );
}
