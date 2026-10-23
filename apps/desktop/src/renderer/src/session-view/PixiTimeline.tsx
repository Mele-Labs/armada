// The Artifacts timeline drawn on a canvas, the third take beside the Spine and Lanes of `ArtifactTimeline.tsx`: the same Timeline, with
// the whole Job fitted to the panel, the time axis zoomed with the wheel or a pinch about the pointer and panned with a drag, branches as
// smooth curves, and each label given the room to the next thing on its track. The canvas (`pixi-timeline-stage.ts`) only draws. What is
// pressed, tabbed to, hovered and read aloud is a layer of buttons over it, one to a point, put where its picture is by script each time the
// view moves, as the map's are (`cockpit/deck/pixi/Hits.tsx`): each carries its name, its tooltip and its press, and the focused one is ringed
// on the canvas. `+` and `-` zoom, `0` fits again, the arrows walk the points and Enter opens one.

import { useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { Tooltip, timeSaid, useTimeFormat } from "@armada/components";

import { neighbour, type Timeline } from "./artifact-timeline";
import { BAND, LEFT, POINT, RIGHT, clamp, fitK, fitted, panned, placedOf, tracksOf, xOf, zoomed, type Row, type View } from "./pixi-timeline";
import { TimelineStage } from "./pixi-timeline-stage";
import "./artifact-timeline.css";
import "./pixi-timeline.css";

type At = { line: string; index: number };

const ARROWS = ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"] as const;
const isArrow = (key: string): key is (typeof ARROWS)[number] => (ARROWS as readonly string[]).includes(key);

/** How long the lines take to grow in, left to right. */
const GROW_MS = 400;
const STEP = 1.6;

export function PixiTimeline({ timeline }: { timeline: Timeline }) {
  const format = useTimeFormat();
  const scroller = useRef<HTMLDivElement>(null);
  const host = useRef<HTMLDivElement>(null);
  const stage = useRef<TimelineStage | undefined>(undefined);
  const spots = useRef(new Map<string, HTMLElement>());
  const origin = useRef<HTMLDivElement>(null);
  const [panel, setPanel] = useState(0);
  const [current, setCurrent] = useState<At | undefined>();
  const empty = timeline.lines.length === 0;

  const tracks = useMemo(() => tracksOf(timeline, panel), [timeline, panel]);
  const height = BAND + tracks.height;
  const clock = (ms: number) => timeSaid(ms, { hour12: format.hour12, seconds: false, date: false });

  // What the frame is drawn from, held where the wheel, the drag and the keys can reach it without drawing the buttons again.
  const view = useRef<View>({ t0: 0, k: 0 });
  const atFit = useRef(true);
  const reveal = useRef(window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 1 : 0);
  const focus = useRef<string | undefined>(undefined);
  const held = useRef({ timeline, tracks, panel, height, clock });
  held.current = { timeline, tracks, panel, height, clock };

  /** The canvas drawn at the view as it stands, and every button put where its point is. */
  const draw = () => {
    const now = held.current;
    stage.current?.set({ timeline: now.timeline, tracks: now.tracks, view: view.current, width: now.panel, height: now.height, focus: focus.current, reveal: reveal.current, clock: now.clock });
    for (const row of now.tracks.rows) {
      for (const [index, point] of row.line.points.entries()) {
        const spot = spots.current.get(`${row.line.key}@${index}`);
        if (spot !== undefined) spot.style.transform = `translate(${(xOf(view.current, point.t) - POINT / 2).toFixed(1)}px, ${(BAND + row.y - POINT / 2).toFixed(1)}px)`;
      }
    }
    if (origin.current !== null) origin.current.style.transform = `translate(${LEFT - 12 - POINT / 2}px, ${BAND + now.tracks.jobY - POINT / 2}px)`;
  };

  const move = (next: View) => {
    const kept = clamp(next, held.current.timeline, held.current.panel);
    view.current = kept;
    atFit.current = Math.abs(kept.k - fitK(held.current.timeline, held.current.panel)) < 1e-12;
    draw();
  };

  // The width of the panel, and again whenever it is resized.
  useLayoutEffect(() => {
    const el = scroller.current;
    if (el === null) return;
    const read = () => setPanel(el.clientWidth);
    read();
    if (typeof ResizeObserver === "undefined") return;
    const watch = new ResizeObserver(read);
    watch.observe(el);
    return () => watch.disconnect();
  }, [empty]);

  // The canvas.
  useLayoutEffect(() => {
    if (host.current === null) return;
    const made = TimelineStage.start(host.current);
    stage.current = made;
    draw();
    return () => {
      made.destroy();
      stage.current = undefined;
    };
  }, [empty]); // eslint-disable-line react-hooks/exhaustive-deps

  // A new width or a narrowed Job refits what was fitted, and keeps the zoom of what was not.
  useLayoutEffect(() => {
    view.current = atFit.current ? fitted(timeline, panel) : clamp(view.current, timeline, panel);
    draw();
  });

  // On first showing, the lines grow in from the left.
  const grown = useRef(false);
  useLayoutEffect(() => {
    if (panel === 0 || grown.current) return;
    grown.current = true;
    if (reveal.current >= 1) return;
    const start = performance.now();
    let frame = 0;
    const step = (now: number) => {
      const t = Math.min(1, (now - start) / GROW_MS);
      reveal.current = 1 - (1 - t) ** 3;
      draw();
      if (t < 1) frame = requestAnimationFrame(step);
    };
    frame = requestAnimationFrame(step);
    return () => {
      cancelAnimationFrame(frame);
      reveal.current = 1;
    };
  }, [panel]); // eslint-disable-line react-hooks/exhaustive-deps

  // The wheel zooms about the pointer, and a pinch (which a trackpad sends as a wheel with Control held) does too; sideways pans.
  useLayoutEffect(() => {
    const el = host.current;
    if (el === null) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      const at = event.clientX - el.getBoundingClientRect().left;
      const now = held.current;
      if (!event.ctrlKey && Math.abs(event.deltaX) > Math.abs(event.deltaY)) return move(panned(view.current, -event.deltaX, now.timeline, now.panel));
      move(zoomed(view.current, Math.exp(-event.deltaY * (event.ctrlKey ? 0.012 : 0.0035)), at, now.timeline, now.panel));
    };
    el.addEventListener("wheel", wheel, { passive: false });
    return () => el.removeEventListener("wheel", wheel);
  }, [empty]); // eslint-disable-line react-hooks/exhaustive-deps

  // A drag pans. One that moved is not a press: the button it began on is told so.
  const dragged = useRef(false);
  const press = (event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    dragged.current = false;
    const from = { x: event.clientX, t0: view.current.t0 };
    const hear = (move_: globalThis.PointerEvent) => {
      const dx = move_.clientX - from.x;
      if (Math.abs(dx) > 4) dragged.current = true;
      if (dragged.current) move({ k: view.current.k, t0: from.t0 - dx / view.current.k });
    };
    const done = () => {
      window.removeEventListener("pointermove", hear);
      window.removeEventListener("pointerup", done);
      window.removeEventListener("pointercancel", done);
    };
    window.addEventListener("pointermove", hear);
    window.addEventListener("pointerup", done);
    window.addEventListener("pointercancel", done);
  };

  const placed = useMemo(() => placedOf(tracks, timeline, panel), [tracks, timeline, panel]);
  const rowOf = (key: string): Row | undefined => tracks.rows.find((one) => one.line.key === key);
  const here: At | undefined = current !== undefined && placed.some((one) => one.line.key === current.line && current.index < one.xs.length) ? current : placed.length === 0 ? undefined : (() => {
    const top = [...placed].sort((a, b) => a.y - b.y || a.xs[0]! - b.xs[0]!)[0]!;
    return { line: top.line.key, index: 0 };
  })();

  /** Brings a point into the plot when it is off the edge, then focuses its button. */
  const goTo = (to: At) => {
    const row = rowOf(to.line);
    const point = row?.line.points[to.index];
    if (point === undefined) return;
    const x = xOf(view.current, point.t);
    if (x < LEFT || x > held.current.panel - RIGHT) move({ k: view.current.k, t0: point.t - (held.current.panel / 2 - LEFT) / view.current.k });
    spots.current.get(`${to.line}@${to.index}`)?.querySelector("button")?.focus();
  };

  const keys = (event: KeyboardEvent<HTMLElement>) => {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    const on = (event.target as HTMLElement).closest<HTMLElement>("[data-point]");
    const now = held.current;
    const about = on === null ? LEFT + (now.panel - LEFT - RIGHT) / 2 : xOf(view.current, rowOf(on.dataset.line!)!.line.points[Number(on.dataset.index)]!.t);
    if (event.key === "+" || event.key === "=") {
      event.preventDefault();
      return move(zoomed(view.current, STEP, about, now.timeline, now.panel));
    }
    if (event.key === "-" || event.key === "_") {
      event.preventDefault();
      return move(zoomed(view.current, 1 / STEP, about, now.timeline, now.panel));
    }
    if (event.key === "0") {
      event.preventDefault();
      return move(fitted(now.timeline, now.panel));
    }
    if (!isArrow(event.key) || on === null) return;
    const to = neighbour(placed, { line: on.dataset.line!, index: Number(on.dataset.index) }, event.key);
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") event.preventDefault();
    if (to === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    goTo(to);
  };

  if (empty) return null;
  return (
    <div
      className="at at--pixi"
      data-take="pixi"
      data-own-arrows=""
      data-owns-keys="0 t"
      role="group"
      aria-label="Timeline"
      onKeyDown={keys}
      onBlur={(event) => {
        if (event.currentTarget.contains(event.relatedTarget as Node | null)) return;
        focus.current = undefined;
        draw();
      }}
    >
      <div className="at__scroll" ref={scroller}>
        <div className="atp" ref={host} style={{ height }} onPointerDown={press}>
          <div className="atp__spot" ref={origin}>
            <Tooltip label="Job">
              <span className="atp__origin" role="img" aria-label="Job" />
            </Tooltip>
          </div>
          {tracks.rows.map((row) =>
            row.line.points.map((point, index) => {
              const key = `${row.line.key}@${index}`;
              const time = timeline.timed ? clock(point.t) : undefined;
              const mine = here?.line === row.line.key && here.index === index;
              const joins = index === 0 ? row.line.parents.flatMap((parent) => (parent.key === "job" || parent.said === undefined ? [] : [parent.said])) : [];
              return (
                <div key={key} className="atp__spot" ref={(element) => void (element === null ? spots.current.delete(key) : spots.current.set(key, element))}>
                  <Tooltip label={[row.line.title, index === 0 ? row.line.said : undefined, point.said, ...joins, time].filter((part) => part !== undefined).join(", ")}>
                    <button
                      type="button"
                      className="atp__point"
                      data-point=""
                      data-line={row.line.key}
                      data-index={index}
                      {...(mine ? { "data-layer-row": "" } : {})}
                      tabIndex={mine ? 0 : -1}
                      aria-label={[point.said, row.line.title, time].filter((part) => part !== undefined).join(", ")}
                      aria-disabled={point.open === undefined || undefined}
                      onFocus={() => {
                        setCurrent({ line: row.line.key, index });
                        focus.current = key;
                        draw();
                      }}
                      onClick={() => {
                        if (!dragged.current) point.open?.();
                      }}
                    />
                  </Tooltip>
                </div>
              );
            }),
          )}
        </div>
      </div>
    </div>
  );
}
