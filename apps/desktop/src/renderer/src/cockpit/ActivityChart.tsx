// The flight deck's Activity: a beeswarm with no words. Each event is a dot at its time over the last hour, the newest at the right, packed
// up and down a centre line so none overlap, in the colour of its tone and smaller and fainter where it is a coalesced read, edit or
// command. Hovering or focusing a dot shows the events near it in a card, itself first, and pressing it picks its event. Takes what
// the feed took, so the Session open narrows it.

import { useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type RefObject } from "react";
import { Stamp, Tooltip } from "@armada/components";
import type { DashboardTab } from "@armada/overview";
import type { MergeLineView } from "@armada/screens";

import { QUIET_KINDS, titleOf, type FleetEvent, type Subject } from "./fleet-events";
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
const NO_VIEWS: readonly MergeLineView[] = [];

const newestFirst = (a: FleetEvent, b: FleetEvent) => Date.parse(b.at) - Date.parse(a.at);

/** What a card says: its newest events, each as the stacked row draws one, the glyph in the event's colour. Nothing in it can be pressed. */
function Hour({ events, lit }: { events: readonly FleetEvent[]; lit: string | undefined }) {
  return (
    <span className="armada-tl__tip armada-tl__hour">
      {events.slice(0, MOST_CARD).map((event) => {
        const Kind = GLYPH[event.subject.kind];
        const Owed = event.tone === "ask" ? STANDING_MARK.ask.icon : event.tone === "bad" ? STANDING_MARK.bad.icon : undefined;
        return (
          <span key={event.id} className="armada-tl__moment" data-lit={lit === event.id || undefined} data-weight={QUIET_KINDS.has(event.kind) ? "quiet" : undefined} style={toneOf(event.tone)}>
            <Kind size={14} aria-hidden="true" className="armada-tl__kindmark" />
            <span className="armada-tl__title">{titleOf(event.subject)}</span>
            <Stamp still first="age" at={event.at} className="armada-tl__age" />
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

/** The events within `span` ms of `at`, newest first. */
const near = (events: readonly FleetEvent[], at: number, span: number): FleetEvent[] => events.filter((event) => Math.abs(Date.parse(event.at) - at) <= span).sort(newestFirst);

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

function Swarm({ events, now, lit, onPick }: { events: readonly FleetEvent[]; now: number; lit: string | undefined; onPick: (event: FleetEvent) => void }) {
  const plot = useRef<HTMLDivElement>(null);
  const { w, h } = useSize(plot);
  const [hot, setHot] = useState<string>();
  const ordered = useMemo(() => [...events].sort((a, b) => Date.parse(a.at) - Date.parse(b.at)), [events]);
  const laid = useMemo(() => {
    const margin = 6;
    // Half a pixel is the finest a screen draws, so a dot that has drifted less keeps the style it has instead of being rewritten.
    const xs = ordered.map((event) => Math.round((margin + (1 - Math.min(HOUR, Math.max(0, now - Date.parse(event.at))) / HOUR) * (w - 2 * margin)) * 2) / 2);
    // The dots shrink until they fit the height; below the smallest they are laid as close as they go.
    for (let d = 6; d >= 2; d *= 0.85) {
      const ys = pack(xs, d, h / 2 - d / 2);
      if (ys !== undefined) return { d, xs, ys, packed: true };
    }
    return { d: 2, xs, ys: pack(xs, 2, 4000)!.map((y) => Math.max(-h / 2 + 1, Math.min(h / 2 - 1, y))), packed: false };
  }, [ordered, now, w, h]);
  const held = ordered.find((event) => event.id === hot);
  const cluster = held === undefined ? [] : [held, ...near(events, Date.parse(held.at), NEAR).filter((event) => event.id !== held.id)];
  const over = (event: { target: EventTarget }) => setHot((event.target as HTMLElement).closest<HTMLElement>("[data-id]")?.dataset.id);
  const step = (event: KeyboardEvent<HTMLElement>) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    // By place in the field rather than a number on each dot, which every dot would be rewritten to keep as events come and go.
    const dots = [...event.currentTarget.querySelectorAll<HTMLElement>(".armada-tl__dot")];
    const at = dots.indexOf((event.target as HTMLElement).closest<HTMLElement>(".armada-tl__dot")!);
    const from = at === -1 ? dots.length - 1 : at;
    const to = Math.min(dots.length - 1, Math.max(0, from + (event.key === "ArrowLeft" ? -1 : 1)));
    event.preventDefault();
    dots[to]?.focus();
  };
  return (
    <div className="armada-tl__sized" ref={plot}>
      <Tooltip label={<Hour events={cluster} lit={lit} />} card beside asChild>
        <div className="armada-tl__field" style={{ ["--d" as string]: `${laid.d}px` }} onPointerOver={over} onFocusCapture={over} onKeyDown={step}>
          <i className="armada-tl__axis-line" aria-hidden="true" />
          {ordered.map((event, i) => {
            const quiet = QUIET_KINDS.has(event.kind);
            // The size and the row are the field's `--d` and a whole number of half-dots, so a dot is rewritten only where it moves along.
            const row = laid.ys[i]! / (laid.d * 0.5);
            return (
              <button
                key={event.id}
                type="button"
                className="armada-tl__dot"
                data-id={event.id}
                data-weight={quiet ? "quiet" : undefined}
                data-lit={lit === event.id || undefined}
                tabIndex={i === ordered.length - 1 ? 0 : -1}
                aria-label={`${titleOf(event.subject)}: ${event.says}`}
                style={{ ...toneOf(event.tone), left: laid.xs[i]!, ...(laid.packed ? { ["--row" as string]: Math.round(row) } : { top: h / 2 + laid.ys[i]! }) }}
                onClick={() => onPick(event)}
              />
            );
          })}
        </div>
      </Tooltip>
    </div>
  );
}

export function ActivityGraph({
  now,
  views = NO_VIEWS,
  filter,
  keep,
  onPick,
  lit,
  of,
}: {
  now: number;
  views?: readonly MergeLineView[];
  filter: DashboardTab;
  keep?: (subject: Subject) => boolean;
  /** Narrows the chart to this Session alone: the one open on the flight deck's stage. */
  of?: Of;
  /** A press on a dot picks its event. */
  onPick: (event: FleetEvent) => void;
  /** The event held lit, by id: it is marked on the chart. */
  lit?: string;
}) {
  const lanes = useLanes(now, views, filter, keep, of, MOST_GRAPH);
  const events = useMemo(() => lanes.flatMap((lane) => lane.events).filter((event) => now - Date.parse(event.at) < HOUR), [lanes, now]);
  return (
    <div className="armada-tl armada-tl__graph" role="group" aria-label="Last hour">
      <div className="armada-tl__plot">
        <Swarm events={events} now={now} lit={lit} onPick={onPick} />
      </div>
      <i className="armada-tl__now" aria-hidden="true" />
    </div>
  );
}
