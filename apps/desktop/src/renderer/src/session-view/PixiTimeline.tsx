// The Artifacts timeline drawn on a canvas, the third take beside the Spine and Lanes of `ArtifactTimeline.tsx`: the same Timeline, with
// the whole Job fitted to the panel, the time axis zoomed with a pinch or Control and the wheel about the pointer and panned with a drag,
// branches as smooth curves, and every line titled whole on its own track. The tracks are packed by the room their titles really need
// (`pixi-timeline.ts`), packed again whenever the zoom changes, and glide to their new places. Tracks taller than the panel scroll, with the
// time axis held at the top. The canvas (`pixi-timeline-stage.ts`) only draws. What is pressed, tabbed to, hovered and read aloud is a layer of
// buttons over it, one to a point, put where its picture is by script each time the view moves, as the map's are (`cockpit/deck/pixi/Hits.tsx`):
// each carries its name, its tooltip and its press, and the focused one is ringed on the canvas. `+` and `-` zoom, `0` fits again, the arrows
// walk the points and Enter opens one.

import { useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";
import { Tooltip, timeSaid, useTimeFormat } from "@armada/components";

import { neighbour, type Timeline, type TimelineLine } from "./artifact-timeline";
import { LEFT, POINT, RIGHT, clamp, fitK, fitted, panned, placedOf, tracksOf, ticksFor, xOf, zoomed, type Tracks, type View } from "./pixi-timeline";
import { TimelineStage, forgetWidths, titleWidth } from "./pixi-timeline-stage";
import "./artifact-timeline.css";
import "./pixi-timeline.css";

type At = { line: string; index: number };

const ARROWS = ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"] as const;
const isArrow = (key: string): key is (typeof ARROWS)[number] => (ARROWS as readonly string[]).includes(key);

/** How long the lines take to grow in, left to right, and the tracks to glide to their new places when the zoom packs them again. */
const GROW_MS = 400;
const MOVE_MS = 200;
const STEP = 1.6;

const reduced = () => window.matchMedia("(prefers-reduced-motion: reduce)").matches;

type Packed = { timeline: Timeline; k: number; fonts: number; tracks: Tracks };
type Glide = { from: Tracks; start: number };

export function PixiTimeline({ timeline }: { timeline: Timeline }) {
  const format = useTimeFormat();
  const scroller = useRef<HTMLDivElement>(null);
  const band = useRef<HTMLDivElement>(null);
  const host = useRef<HTMLDivElement>(null);
  const stage = useRef<TimelineStage | undefined>(undefined);
  const spots = useRef(new Map<string, HTMLElement>());
  const origin = useRef<HTMLDivElement>(null);
  const [panel, setPanel] = useState(0);
  const [current, setCurrent] = useState<At | undefined>();
  const [home, setHome] = useState<At | undefined>();
  const empty = timeline.lines.length === 0;
  const clock = (ms: number) => timeSaid(ms, { hour12: format.hour12, seconds: false, date: false });

  // What the frame is drawn from, held where the wheel, the drag, the keys and the glide can reach it without drawing the buttons again.
  const view = useRef<View>({ t0: 0, k: 0 });
  const atFit = useRef(true);
  const reveal = useRef(reduced() ? 1 : 0);
  const focus = useRef<string | undefined>(undefined);
  const held = useRef({ timeline, panel, clock });
  held.current = { timeline, panel, clock };

  // The packing, for the zoom it was made at, and the glide from the one before.
  const packed = useRef<Packed | undefined>(undefined);
  const glide = useRef<Glide | undefined>(undefined);
  const loop = useRef(0);
  const fonts = useRef(0);
  const family = useRef("sans-serif");
  const jobY = useRef<number | undefined>(undefined);
  const placedFirst = useRef(false);

  /** The tracks as they are drawn at this moment: part of the way from the old packing to the new while they glide. */
  const shown = (now = performance.now()): Tracks | undefined => {
    const have = packed.current;
    const going = glide.current;
    if (have === undefined) return undefined;
    if (going === undefined) return have.tracks;
    const t = Math.min(1, (now - going.start) / MOVE_MS);
    if (t >= 1) {
      glide.current = undefined;
      return have.tracks;
    }
    const e = 1 - (1 - t) ** 3;
    const before = new Map(going.from.rows.map((row) => [row.line.key, row.y] as const));
    const lerp = (a: number, b: number) => a + (b - a) * e;
    return {
      ...have.tracks,
      rows: have.tracks.rows.map((row) => ({ ...row, y: lerp(before.get(row.line.key) ?? row.y, row.y) })),
      jobY: lerp(going.from.jobY, have.tracks.jobY),
      height: lerp(going.from.height, have.tracks.height),
    };
  };

  /** Packs again if the zoom, the Job or the type has changed since the last time. */
  const repack = () => {
    const now = held.current;
    const k = view.current.k;
    const have = packed.current;
    if (have !== undefined && have.timeline === now.timeline && have.k === k && have.fonts === fonts.current) return;
    const before = have !== undefined && have.timeline === now.timeline && !reduced() ? shown() : undefined;
    const tracks = tracksOf(now.timeline, { t0: now.timeline.start, k }, (title) => titleWidth(family.current, title));
    packed.current = { timeline: now.timeline, k, fonts: fonts.current, tracks };
    glide.current = before === undefined ? undefined : { from: before, start: performance.now() };
    if (glide.current !== undefined) run();
    if (have === undefined || have.timeline !== now.timeline) {
      // The point nearest the Job's line is where the keyboard enters.
      const near = [...tracks.rows].sort((a, b) => Math.abs(a.y - tracks.jobY) - Math.abs(b.y - tracks.jobY) || xOf(view.current, a.line.points[0]!.t) - xOf(view.current, b.line.points[0]!.t))[0];
      setHome(near === undefined ? undefined : { line: near.line.key, index: 0 });
    }
  };

  /** Draws every frame until the tracks have arrived. */
  const run = () => {
    if (loop.current !== 0) return;
    const step = () => {
      loop.current = 0;
      draw();
      if (glide.current !== undefined) loop.current = requestAnimationFrame(step);
    };
    loop.current = requestAnimationFrame(step);
  };

  /** The canvas drawn at the view as it stands, the axis and every button put where its tick or point is. */
  const draw = () => {
    const now = held.current;
    if (now.panel === 0) return;
    repack();
    const tracks = shown();
    if (tracks === undefined) return;
    const at = view.current;
    const top = scroller.current;
    if (host.current !== null) host.current.style.height = `${tracks.height}px`;
    if (top !== null) {
      // The Job's line is in view when the tracks first show, and stays where it is on the screen as the tracks gather round it.
      if (!placedFirst.current) {
        placedFirst.current = true;
        top.scrollTop = Math.max(0, tracks.jobY - (top.clientHeight - (band.current?.offsetHeight ?? 0)) / 2);
      } else if (jobY.current !== undefined && top.scrollTop > 0 && tracks.jobY !== jobY.current) {
        top.scrollTop = Math.max(0, top.scrollTop + tracks.jobY - jobY.current);
      }
    }
    jobY.current = tracks.jobY;
    stage.current?.set({ timeline: now.timeline, tracks, view: at, width: now.panel, height: tracks.height, focus: focus.current, reveal: reveal.current });
    for (const row of tracks.rows) {
      for (const [index, point] of row.line.points.entries()) {
        const spot = spots.current.get(`${row.line.key}@${index}`);
        if (spot !== undefined) spot.style.transform = `translate(${(xOf(at, point.t) - POINT / 2).toFixed(1)}px, ${(row.y - POINT / 2).toFixed(1)}px)`;
      }
    }
    if (origin.current !== null) origin.current.style.transform = `translate(${LEFT - 12 - POINT / 2}px, ${(tracks.jobY - POINT / 2).toFixed(1)}px)`;
    const axis = band.current;
    if (axis !== null) {
      const ticks = ticksFor(now.timeline, at, now.panel);
      while (axis.children.length < ticks.length) {
        const one = document.createElement("span");
        one.className = "at__tick";
        axis.appendChild(one);
      }
      Array.from(axis.children).forEach((one, i) => {
        const tick = ticks[i];
        const span = one as HTMLElement;
        span.hidden = tick === undefined;
        if (tick === undefined) return;
        const said = now.clock(tick.t);
        if (span.textContent !== said) span.textContent = said;
        span.style.left = `${tick.x}px`;
      });
    }
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

  // The canvas, and the type it is measured in: titles are packed again once the page's fonts have come.
  useLayoutEffect(() => {
    if (host.current === null) return;
    family.current = getComputedStyle(host.current).fontFamily || "sans-serif";
    const made = TimelineStage.start(host.current);
    stage.current = made;
    let alive = true;
    void document.fonts.ready.then(() => {
      if (!alive) return;
      forgetWidths();
      fonts.current += 1;
      draw();
      made.redraw();
    });
    draw();
    return () => {
      alive = false;
      cancelAnimationFrame(loop.current);
      loop.current = 0;
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

  // A pinch (which a trackpad sends as a wheel with Control held), or the wheel with Control, Command or Option, zooms about the pointer;
  // sideways pans. A plain wheel is left to scroll the tracks.
  useLayoutEffect(() => {
    const el = host.current;
    if (el === null) return;
    const wheel = (event: WheelEvent) => {
      const now = held.current;
      if (event.ctrlKey || event.metaKey || event.altKey) {
        event.preventDefault();
        const at = event.clientX - el.getBoundingClientRect().left;
        return move(zoomed(view.current, Math.exp(-event.deltaY * (event.ctrlKey ? 0.012 : 0.0035)), at, now.timeline, now.panel));
      }
      if (Math.abs(event.deltaX) > Math.abs(event.deltaY)) {
        event.preventDefault();
        move(panned(view.current, -event.deltaX, now.timeline, now.panel));
      }
    };
    el.addEventListener("wheel", wheel, { passive: false });
    return () => el.removeEventListener("wheel", wheel);
  }, [empty]); // eslint-disable-line react-hooks/exhaustive-deps

  // A drag pans, along time and up and down the tracks. One that moved is not a press: the button it began on is told so.
  const dragged = useRef(false);
  const press = (event: PointerEvent<HTMLElement>) => {
    if (event.button !== 0) return;
    dragged.current = false;
    const from = { x: event.clientX, y: event.clientY, t0: view.current.t0, top: scroller.current?.scrollTop ?? 0 };
    const hear = (move_: globalThis.PointerEvent) => {
      const dx = move_.clientX - from.x;
      const dy = move_.clientY - from.y;
      if (Math.abs(dx) > 4 || Math.abs(dy) > 4) dragged.current = true;
      if (!dragged.current) return;
      if (scroller.current !== null) scroller.current.scrollTop = from.top - dy;
      move({ k: view.current.k, t0: from.t0 - dx / view.current.k });
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

  const lineOf = (key: string): TimelineLine | undefined => timeline.lines.find((one) => one.key === key);
  const exists = (to: At | undefined): to is At => to !== undefined && (lineOf(to.line)?.points.length ?? 0) > to.index;
  const here: At | undefined = exists(current) ? current : exists(home) ? home : empty ? undefined : { line: timeline.lines[0]!.key, index: 0 };
  const joinsOf = useMemo(() => new Map(timeline.lines.map((line) => [line.key, line.parents.flatMap((parent) => (parent.key === "job" || parent.said === undefined ? [] : [parent.said]))] as const)), [timeline]);

  /** Brings a point into the plot when it is off the edge, then focuses its button. */
  const goTo = (to: At) => {
    const point = lineOf(to.line)?.points[to.index];
    if (point === undefined) return;
    const x = xOf(view.current, point.t);
    if (x < LEFT || x > held.current.panel - RIGHT) move({ k: view.current.k, t0: point.t - (held.current.panel / 2 - LEFT) / view.current.k });
    spots.current.get(`${to.line}@${to.index}`)?.querySelector("button")?.focus();
  };

  const keys = (event: KeyboardEvent<HTMLElement>) => {
    if (event.altKey || event.ctrlKey || event.metaKey) return;
    const on = (event.target as HTMLElement).closest<HTMLElement>("[data-point]");
    const now = held.current;
    const under = on === null ? undefined : lineOf(on.dataset.line!)?.points[Number(on.dataset.index)];
    const about = under === undefined ? LEFT + (now.panel - LEFT - RIGHT) / 2 : xOf(view.current, under.t);
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
    if (!isArrow(event.key) || on === null || packed.current === undefined) return;
    const to = neighbour(placedOf(packed.current.tracks, view.current), { line: on.dataset.line!, index: Number(on.dataset.index) }, event.key);
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
        <div className="at__band" ref={band} aria-hidden="true" />
        <div className="atp" ref={host} onPointerDown={press}>
          <div className="atp__spot" ref={origin}>
            <Tooltip label="Job">
              <span className="atp__origin" role="img" aria-label="Job" />
            </Tooltip>
          </div>
          {timeline.lines.map((line) =>
            line.points.map((point, index) => {
              const key = `${line.key}@${index}`;
              const time = timeline.timed ? clock(point.t) : undefined;
              const mine = here?.line === line.key && here.index === index;
              const joins = index === 0 ? (joinsOf.get(line.key) ?? []) : [];
              return (
                <div key={key} className="atp__spot" ref={(element) => void (element === null ? spots.current.delete(key) : spots.current.set(key, element))}>
                  <Tooltip label={[line.title, index === 0 ? line.said : undefined, point.said, ...joins, time].filter((part) => part !== undefined).join(", ")}>
                    <button
                      type="button"
                      className="atp__point"
                      data-point=""
                      data-line={line.key}
                      data-index={index}
                      {...(mine ? { "data-layer-row": "" } : {})}
                      tabIndex={mine ? 0 : -1}
                      aria-label={[point.said, line.title, time].filter((part) => part !== undefined).join(", ")}
                      aria-disabled={point.open === undefined || undefined}
                      onFocus={() => {
                        setCurrent({ line: line.key, index });
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
