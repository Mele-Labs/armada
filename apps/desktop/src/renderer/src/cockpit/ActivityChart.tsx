// The flight deck's Activity as a chart with no words, in three takes the owner swaps between: a stream (a smooth stacked area
// over the hour), a beeswarm (a dot for each event) and a radial (a dial, a spoke for each minute). The take is `?take=` on the page,
// or the name the walk carries (`?walk=activity-radial`), and the stream is the default. Hovering or focusing shows the events near
// that point in a card, and pressing picks the newest of them. Takes what the feed takes, so the Job or Session open narrows it.

import { useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent, type MouseEvent, type RefObject } from "react";
import { Tooltip } from "@armada/components";
import type { DashboardTab } from "@armada/overview";
import type { JobSummary } from "@armada/protocol";
import type { MergeLineView } from "@armada/screens";

import { QUIET_KINDS, ageOf, titleOf, type EventTone, type FleetEvent, type Subject } from "./fleet-events";
import { GLYPH, STANDING_MARK, toneOf, useLanes, type Of } from "./Timeline";
import "./activity-chart.css";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
/** The newest events the chart counts. */
const MOST_GRAPH = 500;
/** The newest events a card lists. A card cannot scroll, so a busier place is cut here. */
const MOST_CARD = 8;
/** How far from a point an event is still the point's: a minute and a half. */
const NEAR = 1.5 * MINUTE;
/** Bottom to top, or inner to outer: what is owed him and what is failing sits last, where it is read first. */
const STACK: readonly EventTone[] = ["idle", "ok", "run", "bad", "ask"];

const NO_VIEWS: readonly MergeLineView[] = [];

type Take = "stream" | "beeswarm" | "radial";

/** The take this page asks for: `?take=`, else the walk's own name, else the stream. */
function takeOf(): Take {
  const query = new URLSearchParams(window.location.search);
  const asked = query.get("take") ?? query.get("walk")?.replace(/^activity-/, "");
  return asked === "beeswarm" || asked === "radial" ? asked : "stream";
}

const newestFirst = (a: FleetEvent, b: FleetEvent) => Date.parse(b.at) - Date.parse(a.at);
const unit = (n: number) => Math.min(1, Math.max(0, n));

/** What a card says: its newest events, each as the stacked row draws one, the glyph in the event's colour. Nothing in it can be pressed. */
function Hour({ events, now, lit }: { events: readonly FleetEvent[]; now: number; lit: string | undefined }) {
  return (
    <span className="armada-tl__tip armada-tl__hour">
      {events.slice(0, MOST_CARD).map((event) => {
        const Kind = GLYPH[event.subject.kind];
        const Owed = event.tone === "ask" ? STANDING_MARK.ask.icon : event.tone === "bad" ? STANDING_MARK.bad.icon : undefined;
        return (
          <span key={event.id} className="armada-tl__moment" data-lit={lit === event.id || undefined} data-weight={QUIET_KINDS.has(event.kind) ? "quiet" : undefined} style={toneOf(event.tone)}>
            <Kind size={14} aria-hidden="true" className="armada-tl__kindmark" />
            <span className="armada-tl__title">{titleOf(event.subject)}</span>
            <time className="armada-tl__age" dateTime={event.at}>
              {ageOf(event.at, now)}
            </time>
            <span className="armada-tl__says">
              <span className="armada-tl__what">{event.says}</span>
              {event.detail === undefined ? null : <span className="armada-tl__detail">{event.detail}</span>}
              {Owed === undefined ? null : <Owed size={14} aria-hidden="true" className="armada-tl__flag" />}
            </span>
          </span>
        );
      })}
    </span>
  );
}

/** How big the plot is, as the take lays itself out in pixels. */
function useSize(ref: RefObject<HTMLElement | null>): { w: number; h: number } {
  const [size, set] = useState({ w: 274, h: 240 });
  useLayoutEffect(() => {
    const held = ref.current;
    if (held === null) return;
    const read = () => set((was) => (was.w === held.clientWidth && was.h === held.clientHeight ? was : { w: held.clientWidth || 274, h: held.clientHeight || 240 }));
    read();
    if (typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(read);
    watch.observe(held);
    return () => watch.disconnect();
  }, [ref]);
  return size;
}

type Shared = { events: readonly FleetEvent[]; now: number; lit: string | undefined; onPick: (event: FleetEvent) => void };

/** The events within `span` ms of `at`, newest first. */
const near = (events: readonly FleetEvent[], at: number, span: number): FleetEvent[] => events.filter((event) => Math.abs(Date.parse(event.at) - at) <= span).sort(newestFirst);

// ---- Stream --------------------------------------------------------------------------------------------

const BINS = 60;
const WIDE = 600;
const TALL = 100;

type Point = readonly [number, number];

/** A uniform cubic B-spline through the points as cubic curves: it keeps the layers' order, so stacked bands never cross. */
function basis(points: readonly Point[]): string {
  const p = [points[0]!, points[0]!, ...points, points.at(-1)!, points.at(-1)!];
  const f = (n: number) => n.toFixed(2);
  let d = "";
  for (let i = 0; i + 3 < p.length; i++) {
    const [a, b, c, e] = [p[i]!, p[i + 1]!, p[i + 2]!, p[i + 3]!];
    if (i === 0) d += `M${f((a[0] + 4 * b[0] + c[0]) / 6)} ${f((a[1] + 4 * b[1] + c[1]) / 6)}`;
    d += `C${f((2 * b[0] + c[0]) / 3)} ${f((2 * b[1] + c[1]) / 3)} ${f((b[0] + 2 * c[0]) / 3)} ${f((b[1] + 2 * c[1]) / 3)} ${f((b[0] + 4 * c[0] + e[0]) / 6)} ${f((b[1] + 4 * c[1] + e[1]) / 6)}`;
  }
  return d;
}

/** Each tone, loud and then quiet, from the bottom up. */
const LAYERS = STACK.flatMap((tone) => [false, true].map((quiet) => ({ tone, quiet })));

function Stream({ events, now, lit, onPick }: Shared) {
  const [at, setAt] = useState<number>();
  const [on, setOn] = useState(false);
  const minuteOf = (event: FleetEvent) => Math.min(BINS - 1, Math.max(0, BINS - 1 - Math.floor((now - Date.parse(event.at)) / MINUTE)));
  const { layers, top, y } = useMemo(() => {
    const counts = LAYERS.map(() => new Array<number>(BINS).fill(0));
    for (const event of events) {
      const quiet = QUIET_KINDS.has(event.kind);
      counts[LAYERS.findIndex((layer) => layer.tone === event.tone && layer.quiet === quiet)]![minuteOf(event)]! += 1;
    }
    // Running totals: the bottom of layer j is `sums[j]`, its top `sums[j + 1]`.
    const sums = [new Array<number>(BINS).fill(0)];
    for (const row of counts) sums.push(row.map((n, i) => sums.at(-1)![i]! + n));
    const tallest = Math.max(6, ...sums.at(-1)!);
    const y = (value: number) => TALL - (value / tallest) * TALL * 0.92;
    const x = (bin: number) => ((bin + 0.5) / BINS) * WIDE;
    const edge = (row: readonly number[]): Point[] => [[0, y(row[0]!)], ...row.map((value, bin): Point => [x(bin), y(value)]), [WIDE, y(row.at(-1)!)]];
    const layers = LAYERS.flatMap((layer, j) => {
      if (counts[j]!.every((n) => n === 0)) return [];
      const lower = basis(edge(sums[j]!).reverse()).replace(/^M/, "L");
      return [{ ...layer, d: `${basis(edge(sums[j + 1]!))}${lower}Z` }];
    });
    return { layers, top: sums, y };
  }, [events, now]);
  const reading = at ?? 1;
  const here = near(events, now - (1 - reading) * HOUR, NEAR);
  const mark = lit === undefined ? undefined : events.find((event) => event.id === lit);
  const marked = (() => {
    if (mark === undefined) return undefined;
    const bin = minuteOf(mark);
    const low = LAYERS.findIndex((layer) => layer.tone === mark.tone);
    return { left: ((bin + 0.5) / BINS) * 100, top: ((y(top[low]![bin]!) + y(top[low + 2]![bin]!)) / 2 / TALL) * 100 };
  })();
  const at0 = (event: MouseEvent<HTMLElement>) => {
    const box = event.currentTarget.getBoundingClientRect();
    return unit((event.clientX - box.left) / (box.width || 1));
  };
  const pick = (f: number) => {
    const hit = near(events, now - (1 - f) * HOUR, NEAR)[0];
    if (hit !== undefined) onPick(hit);
  };
  const keys = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      setAt(unit(reading + (event.key === "ArrowLeft" ? -1 : 1) / BINS));
      setOn(true);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      pick(reading);
    }
  };
  return (
    <Tooltip label={<Hour events={here} now={now} lit={lit} />} card beside asChild>
      <div className="armada-tl__stream" onPointerOver={(event) => (setAt(at0(event)), setOn(true))} onPointerMove={(event) => setAt(at0(event))} onMouseLeave={() => setOn(false)}>
        <div
          className="armada-tl__scrubber"
          role="slider"
          tabIndex={0}
          aria-label="Time"
          aria-orientation="horizontal"
          aria-valuemin={0}
          aria-valuemax={BINS}
          aria-valuenow={Math.round((1 - reading) * BINS)}
          aria-valuetext={`${Math.round((1 - reading) * BINS)} minutes ago`}
          onClick={(event) => pick(at0(event))}
          onKeyDown={keys}
          onFocus={() => setOn(true)}
          onBlur={() => setOn(false)}
        >
          <svg className="armada-tl__area" viewBox={`0 0 ${WIDE} ${TALL}`} preserveAspectRatio="none" aria-hidden="true">
            {layers.map((layer) => (
              <path key={`${layer.tone}:${layer.quiet}`} className="armada-tl__layer" data-weight={layer.quiet ? "quiet" : undefined} style={toneOf(layer.tone)} d={layer.d} />
            ))}
          </svg>
          {on ? <i className="armada-tl__scrub" style={{ left: `${reading * 100}%` }} aria-hidden="true" /> : null}
          {marked === undefined ? null : <i className="armada-tl__lit" style={{ left: `${marked.left}%`, top: `${marked.top}%` }} aria-hidden="true" />}
        </div>
      </div>
    </Tooltip>
  );
}

// ---- Beeswarm ------------------------------------------------------------------------------------------

/** Dots at their time, packed up and down a centre line so none overlap; `d` is the dot's size. Undefined where they do not fit in `half`. */
function pack(xs: readonly number[], d: number, half: number): number[] | undefined {
  const placed: { x: number; y: number }[] = [];
  const out: number[] = [];
  for (const x of xs) {
    const near = placed.filter((one) => x - one.x < d);
    let y: number | undefined;
    for (let k = 0; y === undefined && k * d * 0.5 <= half; k++) {
      for (const sign of k === 0 ? [1] : [1, -1]) {
        const candidate = sign * k * d * 0.5;
        if (near.every((one) => (one.x - x) ** 2 + (one.y - candidate) ** 2 >= d * d)) {
          y = candidate;
          break;
        }
      }
    }
    if (y === undefined) return undefined;
    placed.push({ x, y });
    out.push(y);
  }
  return out;
}

function Swarm({ events, now, lit, onPick }: Shared) {
  const plot = useRef<HTMLDivElement>(null);
  const { w, h } = useSize(plot);
  const [hot, setHot] = useState<string>();
  const ordered = useMemo(() => [...events].sort((a, b) => Date.parse(a.at) - Date.parse(b.at)), [events]);
  const laid = useMemo(() => {
    const margin = 6;
    const xs = ordered.map((event) => margin + (1 - Math.min(HOUR, Math.max(0, now - Date.parse(event.at))) / HOUR) * (w - 2 * margin));
    // The dots shrink until they fit the height; below the smallest they are laid as close as they go.
    for (let d = 6; d >= 2; d *= 0.85) {
      const ys = pack(xs, d, h / 2 - d / 2);
      if (ys !== undefined) return { d, xs, ys };
    }
    return { d: 2, xs, ys: pack(xs, 2, 4000)!.map((y) => Math.max(-h / 2 + 1, Math.min(h / 2 - 1, y))) };
  }, [ordered, now, w, h]);
  const held = ordered.find((event) => event.id === hot);
  const cluster = held === undefined ? [] : [held, ...near(events, Date.parse(held.at), NEAR).filter((event) => event.id !== held.id)];
  const over = (event: { target: EventTarget }) => setHot((event.target as HTMLElement).closest<HTMLElement>("[data-id]")?.dataset.id);
  const step = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    const from = Number((event.target as HTMLElement).closest<HTMLElement>("[data-i]")?.dataset.i ?? ordered.length - 1);
    const to = Math.min(ordered.length - 1, Math.max(0, from + (event.key === "ArrowLeft" ? -1 : 1)));
    event.preventDefault();
    event.currentTarget.querySelector<HTMLElement>(`[data-i="${to}"]`)?.focus();
  };
  return (
    <div className="armada-tl__sized" ref={plot}>
      <Tooltip label={<Hour events={cluster} now={now} lit={lit} />} card beside asChild>
        <div className="armada-tl__field" onPointerOver={over} onFocusCapture={over} onKeyDown={step}>
          <i className="armada-tl__axis-line" aria-hidden="true" />
          {ordered.map((event, i) => {
            const quiet = QUIET_KINDS.has(event.kind);
            const size = laid.d * (quiet ? 0.7 : 1);
            return (
              <button
                key={event.id}
                type="button"
                className="armada-tl__dot"
                data-id={event.id}
                data-i={i}
                data-weight={quiet ? "quiet" : undefined}
                data-lit={lit === event.id || undefined}
                tabIndex={i === ordered.length - 1 ? 0 : -1}
                aria-label={`${titleOf(event.subject)}: ${event.says}`}
                style={{ ...toneOf(event.tone), left: laid.xs[i]!, top: h / 2 + laid.ys[i]!, width: size, height: size }}
                onClick={() => onPick(event)}
              />
            );
          })}
        </div>
      </Tooltip>
    </div>
  );
}

// ---- Radial --------------------------------------------------------------------------------------------

function Dial({ events, now, lit, onPick }: Shared) {
  const plot = useRef<HTMLDivElement>(null);
  const { w, h } = useSize(plot);
  const [hot, setHot] = useState<number>();
  const minutes = useMemo(() => {
    const spokes = Array.from({ length: BINS }, () => [] as FleetEvent[]);
    for (const event of events) spokes[Math.min(BINS - 1, Math.max(0, Math.floor((now - Date.parse(event.at)) / MINUTE)))]!.push(event);
    for (const one of spokes) one.sort(newestFirst);
    return spokes;
  }, [events, now]);
  const outer = Math.min(w, h) / 2 - 2;
  const inner = outer * 0.28;
  const per = (outer - inner) / Math.max(5, ...minutes.map((one) => one.length));
  const width = Math.max(2, ((2 * Math.PI * inner) / BINS) * 0.7);
  const turn = (minute: number) => `rotate(${180 - ((minute + 0.5) / BINS) * 360}deg)`;
  const over = (event: { target: EventTarget }) => {
    const found = (event.target as HTMLElement).closest<HTMLElement>("[data-minute]")?.dataset.minute;
    setHot(found === undefined ? undefined : Number(found));
  };
  const here = hot === undefined ? [] : [...(minutes[hot - 1] ?? []), ...minutes[hot]!, ...(minutes[hot + 1] ?? [])].sort(newestFirst);
  const step = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    const from = (event.target as HTMLElement).closest<HTMLElement>("[data-minute]")?.dataset.minute;
    if (from === undefined) return;
    // Left is back in time, as it is on the other two takes.
    const wanted = event.key === "ArrowLeft" ? Number(from) + 1 : Number(from) - 1;
    const found = event.currentTarget.querySelector<HTMLElement>(`button[data-minute="${wanted}"]`) ?? [...event.currentTarget.querySelectorAll<HTMLElement>("button[data-minute]")].filter((one) => (event.key === "ArrowLeft" ? Number(one.dataset.minute) > Number(from) : Number(one.dataset.minute) < Number(from))).sort((a, b) => Math.abs(Number(a.dataset.minute) - wanted) - Math.abs(Number(b.dataset.minute) - wanted))[0];
    event.preventDefault();
    found?.focus();
  };
  const first = minutes.findIndex((one) => one.length > 0);
  return (
    <div className="armada-tl__sized" ref={plot}>
      <Tooltip label={<Hour events={here} now={now} lit={lit} />} card beside asChild>
        <div className="armada-tl__field" onPointerOver={over} onFocusCapture={over} onKeyDown={step}>
          <i className="armada-tl__ring" aria-hidden="true" style={{ width: inner * 2, height: inner * 2, marginLeft: -inner, marginTop: -inner }} />
          <i className="armada-tl__twelve" aria-hidden="true" style={{ top: `calc(50% - ${outer}px)` }} />
          {minutes
            .map((one, minute) => ({ one, minute }))
            .reverse()
            .map(({ one, minute }) => {
              const style = { width, marginLeft: -width / 2, transform: turn(minute) } as CSSProperties;
              if (one.length === 0) return <i key={minute} className="armada-tl__spoke" data-empty style={{ ...style, height: inner + 2 }} aria-hidden="true" />;
              const newest = one[0]!;
              return (
                <button
                  key={minute}
                  type="button"
                  className="armada-tl__spoke"
                  data-minute={minute}
                  data-lit={one.some((event) => event.id === lit) || undefined}
                  tabIndex={minute === first ? 0 : -1}
                  aria-label={`${titleOf(newest.subject)}: ${newest.says}`}
                  style={{ ...style, height: inner + one.length * per, paddingTop: inner }}
                  onClick={() => onPick(newest)}
                >
                  {STACK.flatMap((tone) =>
                    [false, true].flatMap((quiet) => {
                      const n = one.filter((event) => event.tone === tone && QUIET_KINDS.has(event.kind) === quiet).length;
                      return n === 0 ? [] : [<i key={`${tone}:${quiet}`} className="armada-tl__piece" data-weight={quiet ? "quiet" : undefined} style={{ ...toneOf(tone), height: n * per }} />];
                    }),
                  )}
                  {one.some((event) => event.id === lit) ? <i className="armada-tl__tipring" aria-hidden="true" /> : null}
                </button>
              );
            })}
        </div>
      </Tooltip>
    </div>
  );
}

// ---- The panel's chart ---------------------------------------------------------------------------------

export function ActivityGraph({
  now,
  jobs,
  views = NO_VIEWS,
  filter,
  keep,
  onPick,
  lit,
  of,
}: {
  now: number;
  jobs: readonly JobSummary[];
  views?: readonly MergeLineView[];
  filter: DashboardTab;
  keep?: (subject: Subject) => boolean;
  /** Narrows the chart to this Job or Session alone: the one open on the flight deck's stage. */
  of?: Of;
  /** A press picks the newest event near the point pressed. */
  onPick: (event: FleetEvent) => void;
  /** The event held lit, by id: it is marked on the chart. */
  lit?: string;
}) {
  const lanes = useLanes(now, jobs, views, filter, keep, of, MOST_GRAPH);
  const events = useMemo(() => lanes.flatMap((lane) => lane.events).filter((event) => now - Date.parse(event.at) < HOUR), [lanes, now]);
  const take = takeOf();
  const shared = { events, now, lit, onPick };
  return (
    <div className="armada-tl armada-tl__graph" role="group" aria-label="Last hour" data-chart={take}>
      <div className="armada-tl__plot">{take === "beeswarm" ? <Swarm {...shared} /> : take === "radial" ? <Dial {...shared} /> : <Stream {...shared} />}</div>
      {take === "radial" ? null : <i className="armada-tl__now" aria-hidden="true" />}
    </div>
  );
}
