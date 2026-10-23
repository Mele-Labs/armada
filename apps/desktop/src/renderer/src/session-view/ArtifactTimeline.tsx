// The Artifacts layer's timeline, drawn in one of two takes of the same data (`artifact-timeline.ts`): a Spine, the Job's own line through the
// middle with every artifact branching off above and below it, and Lanes, a row to each kind with the joins drawn across them. A point is a
// version of an artifact and a press on it does what the list's row does. The arrows walk the points, left and right along a line and up and
// down between lines; Tab enters at one point and leaves from it.

import { useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { SquareTerminal } from "lucide-react";
import { Tooltip, timeSaid, useTimeFormat } from "@armada/components";

import { POINT, lanesOf, neighbour, spineOf, type Layout, type Placed, type Timeline } from "./artifact-timeline";
import "./artifact-timeline.css";

export type TimelineTake = "spine" | "lanes";

type At = { line: string; index: number };

const ARROWS = ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"] as const;
const isArrow = (key: string): key is (typeof ARROWS)[number] => (ARROWS as readonly string[]).includes(key);

/** The row a point sits on, first line's first point to last line's last, as the arrows would reach them. */
const firstOf = (placed: readonly Placed[]): At | undefined => {
  const top = [...placed].sort((a, b) => a.y - b.y || a.xs[0]! - b.xs[0]!)[0];
  return top === undefined ? undefined : { line: top.line.key, index: 0 };
};

export function ArtifactTimeline({ timeline, take }: { timeline: Timeline; take: TimelineTake }) {
  const format = useTimeFormat();
  const clock = (ms: number) => timeSaid(ms, { hour12: format.hour12, seconds: false, date: false });
  // The whole Job is laid across the panel's width, and again whenever the panel is resized.
  const scroller = useRef<HTMLDivElement>(null);
  const [panel, setPanel] = useState(0);
  const empty = timeline.lines.length === 0;
  useLayoutEffect(() => {
    const held = scroller.current;
    if (held === null) return;
    const read = () => setPanel(held.clientWidth);
    read();
    if (typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(read);
    watch.observe(held);
    return () => watch.disconnect();
  }, [empty]);
  const layout: Layout = useMemo(() => (take === "spine" ? spineOf(timeline, panel, clock) : lanesOf(timeline, panel, clock)), [timeline, take, panel, format.hour12]); // eslint-disable-line react-hooks/exhaustive-deps
  // Where points would sit too close to press, it scrolls, and opens at the latest end.
  useLayoutEffect(() => {
    if (layout.scrolls && scroller.current !== null) scroller.current.scrollLeft = scroller.current.scrollWidth;
  }, [layout.scrolls, take]);
  const [current, setCurrent] = useState<At | undefined>();
  const points = useRef(new Map<string, HTMLButtonElement>());
  const here = current !== undefined && layout.placed.some((one) => one.line.key === current.line && current.index < one.xs.length) ? current : firstOf(layout.placed);

  const walk = (event: KeyboardEvent<HTMLElement>) => {
    if (event.altKey || event.ctrlKey || event.metaKey || !isArrow(event.key)) return;
    const on = (event.target as HTMLElement).closest<HTMLElement>("[data-point]");
    if (on === null) return;
    const to = neighbour(layout.placed, { line: on.dataset.line!, index: Number(on.dataset.index) }, event.key);
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") event.preventDefault();
    if (to === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    points.current.get(`${to.line}@${to.index}`)?.focus();
  };

  if (timeline.lines.length === 0) return null;
  return (
    <div className="at" data-take={take} data-own-arrows="" role="group" aria-label="Timeline" onKeyDown={walk}>
      <div className="at__scroll" ref={scroller}>
        <div className="at__canvas" style={{ width: layout.width }}>
          <div className="at__band">
            {layout.ticks.map((tick) =>
              tick.said === undefined ? null : (
                <span key={tick.x} className="at__tick" style={{ left: tick.x }}>
                  {tick.said}
                </span>
              ),
            )}
          </div>
          <div className="at__plot" style={{ height: layout.height }}>
            {layout.lanes.map((lane) => (
              <div key={`${lane.id}:${lane.top}`} className="at__lane" style={{ top: lane.top, height: lane.height }}>
                <span className="at__lane-label">{lane.label}</span>
              </div>
            ))}
            <svg className="at__paths" width={layout.width} height={layout.height} aria-hidden="true">
              {layout.ticks.map((tick) => (
                <line key={tick.x} className="at__grid" x1={tick.x} x2={tick.x} y1={0} y2={layout.height} />
              ))}
              {layout.job === undefined ? null : <line className="at__job" x1={layout.job.from} x2={layout.job.to} y1={layout.job.y} y2={layout.job.y} />}
              {layout.placed.map((one) => (
                <line key={one.line.key} className="at__run" x1={one.xs[0]} x2={one.xs[one.xs.length - 1]} y1={one.y} y2={one.y} />
              ))}
              {layout.paths.map((path) => (
                <path key={path.key} className="at__branch" data-join={path.said === undefined ? undefined : ""} d={path.d} />
              ))}
            </svg>
            {layout.job === undefined ? null : (
              <div className="at__spot" style={{ left: layout.job.from - POINT / 2, top: layout.job.y - POINT / 2 }}>
                <Tooltip label="Job">
                  <span className="at__origin" role="img" aria-label="Job">
                    <SquareTerminal size={12} strokeWidth={2} aria-hidden />
                  </span>
                </Tooltip>
              </div>
            )}
            {layout.paths.map((path) =>
              path.said === undefined ? null : (
                <span key={path.key} className="at__join" style={{ left: path.at.x + 6, top: path.at.y - 8 }}>
                  {path.said}
                </span>
              ),
            )}
            {layout.placed.map((one) => (
              <Line key={one.line.key} one={one} here={here} points={points.current} clock={clock} timed={timeline.timed} onAt={setCurrent} />
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

/** One artifact: its name over its line, and a point for each version of it. */
function Line({ one, here, points, clock, timed, onAt }: { one: Placed; here: At | undefined; points: Map<string, HTMLButtonElement>; clock: (ms: number) => string; timed: boolean; onAt: (at: At) => void }) {
  const { line } = one;
  const Kind = line.Glyph;
  return (
    <>
      <span className="at__title" style={{ left: one.xs[0]! - POINT / 2, top: one.y - POINT / 2 - 18, maxWidth: one.title }}>
        {line.title}
      </span>
      {line.points.map((point, index) => {
        const key = `${line.key}@${index}`;
        const time = timed ? clock(point.t) : undefined;
        const mine = here?.line === line.key && here.index === index;
        const Glyph = index === 0 ? Kind : point.Glyph;
        return (
          <div key={key} className="at__spot" style={{ left: one.xs[index]! - POINT / 2, top: one.y - POINT / 2 }}>
            <Tooltip label={[line.title, index === 0 ? line.said : undefined, point.said, time].filter((part) => part !== undefined).join(", ")}>
              <button
                type="button"
                ref={(element) => void (element === null ? points.delete(key) : points.set(key, element))}
                className="at__point"
                data-point=""
                data-line={line.key}
                data-index={index}
                data-first={index === 0 || undefined}
                data-plain={Glyph === undefined || undefined}
                data-tone={point.tone}
                {...(mine ? { "data-layer-row": "" } : {})}
                tabIndex={mine ? 0 : -1}
                aria-label={[point.said, line.title, time].filter((part) => part !== undefined).join(", ")}
                aria-disabled={point.open === undefined || undefined}
                onFocus={() => onAt({ line: line.key, index })}
                onClick={point.open}
              >
                {Glyph === undefined ? <span className="at__dot" aria-hidden="true" /> : <Glyph size={12} strokeWidth={2} aria-hidden="true" />}
              </button>
            </Tooltip>
          </div>
        );
      })}
    </>
  );
}
