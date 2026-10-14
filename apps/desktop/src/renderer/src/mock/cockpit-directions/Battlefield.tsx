// The flight deck's map, drawn as a battlefield seen from above at a slight angle. Each repository is a territory:
// a base on its near edge whose landing pads are its bays, and its main as the objective on the far edge, a flag.
// Each Job and Session is a drone hovering on a dotted beam over a ground disc, a Job a large X quad and a Session
// a small plus quad. State is the drone and its frame, never words: rotors turn while it works, a drone that is idle
// or done is landed with its rotors still, a beacon pings from one that holds a call, an amber ring breathes round
// one that is stuck, a red flare burns on one whose Check fails. Pull requests are payloads on a dotted route to the
// objective. The activity log drives the scene (`deck-field.ts` says which event moves what). Calm at rest: only
// what is working or calling moves, with CSS animation alone and no loop in script. Mock only. proto/cockpit-flight-deck.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { Crosshair, Minus, Plus } from "lucide-react";
import { Button, Tooltip } from "@armada/components";

import { ALONG, STAGE_SAID, rand, TRAIL_MS, arcAt, arcOf, bigOf, cameraOf, layoutOf, type DroneIn, type Placed, type Play, type Point, type RepoIn, type Size, type Territory } from "./deck-field";

export type PadIn = { key: string; slot: number; bay: string; word: string; repo: string; holder?: string };
export type PrIn = { key: string; number: number; stage: keyof typeof ALONG; root: string; owner?: string };
export type GoalIn = { root: string; said: string; red: boolean };

const HOVER = 30;

type Tip = { said: string; title: string; left: number; top: number };

const rank = (one: DroneIn, selected: string | undefined, hot: string | undefined) => (one.key === selected ? 0 : one.key === hot ? 1 : one.hue === "bad" ? 2 : 3);

const short = (title: string, room: number) => (title.length > room ? `${title.slice(0, room - 1)}…` : title);

/** Four corner brackets round a box: the lock-on reticle. */
function bracketsOf(w: number, top: number, bottom: number): string {
  const leg = 9;
  return [
    [-1, top, 1],
    [1, top, 1],
    [1, bottom, -1],
    [-1, bottom, -1],
  ]
    .map(([sx, y, sy]) => `M${sx! * w},${y! + sy! * leg} V${y} H${sx! * (w - leg)}`)
    .join(" ");
}

// ---- The drone -----------------------------------------------------------------------------------------

/** One rotor, drawn as a squashed disc so it reads at an angle; its blades turn inside it. */
function Rotor({ x, y, r }: { x: number; y: number; r: number }) {
  return (
    <>
      <g transform={`translate(${x} ${y}) scale(1 0.38)`}>
        <circle className="bf-disc" r={r} />
        <g className="bf-blades">
          <line x1={-r} x2={r} vectorEffect="non-scaling-stroke" />
          <line y1={-r} y2={r} vectorEffect="non-scaling-stroke" />
        </g>
      </g>
      <circle className="bf-hub" cx={x} cy={y} r={2} />
    </>
  );
}

/** A Job: the large X quad. Its camera nose is toward the viewer. */
function JobGlyph() {
  return (
    <g className="bf-glyph" data-kind="job">
      <path className="bf-arm" d="M-17,-7 L-4,-1 M17,-7 L4,-1 M-15,9 L-4,2 M15,9 L4,2" />
      <Rotor x={-21} y={-8} r={11} />
      <Rotor x={21} y={-8} r={11} />
      <path className="bf-leg" d="M-7,6 V11 M7,6 V11" />
      <rect className="bf-hull" x={-10} y={-6.5} width={20} height={13} rx={5} />
      <rect className="bf-plate" x={-5.5} y={-3.6} width={11} height={5} rx={2} />
      <circle className="bf-lens" cx={0} cy={7.6} r={2.7} />
      <circle className="bf-led" cx={0} cy={-0.8} r={1.5} />
      <Rotor x={-17} y={10} r={12} />
      <Rotor x={17} y={10} r={12} />
    </g>
  );
}

/** A Session: the small plus quad, a round pod between four short arms. */
function SessionGlyph() {
  return (
    <g className="bf-glyph" data-kind="session">
      <path className="bf-arm" d="M-13,0 L-4,0 M13,0 L4,0 M0,-8 L0,-3 M0,8 L0,3" />
      <Rotor x={-15} y={0} r={7} />
      <Rotor x={15} y={0} r={7} />
      <Rotor x={0} y={-10} r={6} />
      <circle className="bf-hull" r={6.2} />
      <circle className="bf-plate" r={3.2} />
      <circle className="bf-led" r={1.4} />
      <Rotor x={0} y={10} r={7} />
    </g>
  );
}

function Drone({
  drone,
  at,
  from,
  index,
  big,
  selected,
  hot,
  labelled,
  muted,
  away,
  play,
  onSelect,
  onTip,
}: {
  drone: DroneIn;
  at: Placed;
  /** Where it lifted from, relative to where it is, while it is launching. */
  from: Point | undefined;
  index: number;
  big: number;
  selected: boolean;
  hot: boolean;
  labelled: boolean;
  muted: boolean;
  away: boolean;
  play: Play | undefined;
  onSelect: () => void;
  onTip: (tip: Tip | undefined, target?: SVGGElement) => void;
}) {
  const wide = drone.kind === "job" ? 26 : 20;
  const said = `${drone.kind === "job" ? "Job" : "Session"}, ${drone.said}`;
  const show = (event: { currentTarget: SVGGElement }) => onTip({ said, title: drone.title, left: 0, top: 0 }, event.currentTarget);
  return (
    <g className="bf-place" style={{ transform: `translate(${at.x}px, ${at.y}px)` }}>
      <g
        className="bf-launch"
        data-launching={play?.kind === "launch" || undefined}
        style={from === undefined ? undefined : ({ "--lx": `${from.x}px`, "--ly": `${from.y}px`, "--wait": `${play?.delay ?? 0}ms` } as CSSProperties)}
      >
        <g className="bf-settle" style={{ "--i": Math.min(index, 14) } as CSSProperties}>
          <g
            className="bf-drone"
            role="option"
            tabIndex={0}
            aria-selected={selected}
            aria-label={`${drone.title}, ${said}`}
            data-key={drone.key}
            data-kind={drone.kind}
            data-hue={drone.hue}
            data-hover={drone.hover || undefined}
            data-spin={drone.spin || undefined}
            data-stuck={drone.stuck || undefined}
            data-ping={drone.ping || undefined}
            data-flare={drone.flare || undefined}
            data-selected={selected || undefined}
            data-muted={muted || undefined}
            data-away={away || undefined}
            data-hot={hot || undefined}
            onClick={onSelect}
            onKeyDown={(event) => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onSelect();
              }
            }}
            onMouseEnter={show}
            onMouseLeave={() => onTip(undefined)}
            onFocus={show}
            onBlur={() => onTip(undefined)}
          >
            <g transform={`scale(${(at.s * big).toFixed(3)})`}>
              {/* The ground: a pool of light where it works, the rings that say how it stands, the disc it stands on. */}
              <ellipse className="bf-pool" rx={wide * 2.3} ry={wide * 0.9} />
              {drone.ping ? (
                <>
                  <ellipse className="bf-ping" rx={wide} ry={wide * 0.4} />
                  <ellipse className="bf-ping" data-late rx={wide} ry={wide * 0.4} />
                </>
              ) : null}
              {drone.stuck ? <ellipse className="bf-stall" rx={wide * 1.35} ry={wide * 0.55} /> : null}
              <ellipse className="bf-spot" rx={wide} ry={wide * 0.4} />
              <ellipse className="bf-rim" rx={wide} ry={wide * 0.4} />
              {selected ? (
                <ellipse className="bf-ticks" rx={wide * 1.9} ry={wide * 0.76} />
              ) : null}
              <line className="bf-beam" x1={0} x2={0} y1={0} y2={-HOVER} />
              {/* The air: its shadow, the drone, the antenna of one that is calling, the flare of one that is failing. */}
              <g className="bf-air">
                <ellipse className="bf-shadow" rx={wide * 0.7} ry={wide * 0.26} />
                <g className="bf-lift">
                  {drone.flare ? (
                    <g className="bf-flare" aria-hidden="true">
                      <circle r={wide * 1.15} cy={-2} />
                      <path d="M-9,-16 L-13,-27 M0,-19 L0,-33 M9,-16 L13,-27" />
                    </g>
                  ) : null}
                  {drone.kind === "job" ? <JobGlyph /> : <SessionGlyph />}
                  {drone.ping ? (
                    <g className="bf-mast" aria-hidden="true">
                      <line x1={0} y1={-7} x2={0} y2={-20} />
                      <circle className="bf-tip" cy={-21.5} r={2.6} />
                    </g>
                  ) : null}
                </g>
              </g>
              {selected ? (
                <g className="bf-reticle" aria-hidden="true">
                  <path d={bracketsOf(wide + 14, drone.hover ? -HOVER - 36 : -38, wide * 0.4 + 6)} />
                </g>
              ) : null}
              <rect className="bf-hit" x={-wide - 12} y={-HOVER - 34} width={(wide + 12) * 2} height={HOVER + 34 + wide * 0.4 + 18} />
            </g>
            {labelled ? (
              <text className="bf-label" y={wide * 0.4 * at.s * big + 22} textAnchor="middle">
                {short(drone.title, 22)}
              </text>
            ) : null}
          </g>
        </g>
      </g>
    </g>
  );
}

// ---- The scene's plain parts ---------------------------------------------------------------------------

/** The ground's own lines: a grid that narrows toward the far edge, fading as it goes. */
function Grid({ size }: { size: Size }) {
  const { width: w, height: h } = size;
  const vanish = { x: w / 2, y: -h * 0.9 };
  const down: string[] = [];
  for (let i = -14; i <= 14; i += 1) {
    const bottom = { x: w / 2 + i * (w / 11), y: h };
    const t = (0 - bottom.y) / (vanish.y - bottom.y);
    down.push(`M${bottom.x.toFixed(1)},${bottom.y} L${(bottom.x + (vanish.x - bottom.x) * t).toFixed(1)},0`);
  }
  const across: string[] = [];
  for (let k = 0; k < 16; k += 1) {
    const y = h * (1 - 1.05 * Math.pow(k / 16, 1.75));
    if (y > 0) across.push(`M0,${y.toFixed(1)} H${w}`);
  }
  return (
    <g className="bf-grid" aria-hidden="true">
      <path d={down.join(" ")} />
      <path d={across.join(" ")} />
    </g>
  );
}

function Base({ territory, onTip }: { territory: Territory; onTip: (tip: Tip | undefined, target?: SVGGElement) => void }) {
  const { base } = territory;
  const left = base.x - base.w / 2;
  const hx = left + 36;
  return (
    <g
      className="bf-base"
      onMouseEnter={(event) => onTip({ said: "Repository", title: territory.name, left: 0, top: 0 }, event.currentTarget)}
      onMouseLeave={() => onTip(undefined)}
    >
      <ellipse className="bf-slab" cx={base.x} cy={base.y + 4} rx={base.w / 2 + 12} ry={34} />
      <ellipse className="bf-slab" data-inner cx={base.x} cy={base.y + 4} rx={base.w / 2 - 8} ry={24} />
      <g transform={`translate(${hx} ${base.y - 2})`}>
        <polygon className="bf-hangar" data-face="top" points="-26,-8 0,-18 26,-8 0,2" />
        <polygon className="bf-hangar" data-face="left" points="-26,-8 0,2 0,16 -26,6" />
        <polygon className="bf-hangar" data-face="right" points="26,-8 0,2 0,16 26,6" />
        <path className="bf-mast-line" d="M18,-12 V-30" />
        <circle className="bf-mast-dot" cx={18} cy={-31} r={1.8} />
      </g>
      <text className="bf-name" x={base.x} y={base.y + 56} textAnchor="middle">
        {territory.name}
      </text>
    </g>
  );
}

function Goal({
  at,
  goal,
  capture,
  onPress,
  onTip,
}: {
  at: Point;
  goal: GoalIn;
  capture: Play | undefined;
  onPress: () => void;
  onTip: (tip: Tip | undefined, target?: SVGGElement) => void;
}) {
  const said = goal.said;
  return (
    <g
      className="bf-goal"
      data-red={goal.red || undefined}
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={`${said}, objective`}
      transform={`translate(${at.x} ${at.y}) scale(1.35)`}
      onClick={onPress}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onPress();
        }
      }}
      onMouseEnter={(event) => onTip({ said: "Objective", title: said, left: 0, top: 0 }, event.currentTarget)}
      onMouseLeave={() => onTip(undefined)}
      onFocus={(event) => onTip({ said: "Objective", title: said, left: 0, top: 0 }, event.currentTarget)}
      onBlur={() => onTip(undefined)}
    >
      <ellipse className="bf-zone" rx={46} ry={17} />
      <ellipse className="bf-rim" rx={20} ry={7.5} />
      <line className="bf-pole" x1={0} y1={0} x2={0} y2={-52} />
      <path className="bf-pennant" d="M0,-52 L26,-45 L0,-37 Z" />
      <circle className="bf-beacon" cy={-54} r={2.6} />
      {capture === undefined ? null : (
        <g className="bf-capture" aria-hidden="true" style={{ "--d": `${capture.delay + 900}ms` } as CSSProperties}>
          <ellipse className="bf-flash" rx={30} ry={11} />
          <ellipse className="bf-shock" rx={22} ry={8} />
          <rect className="bf-column" x={-3} y={-70} width={6} height={70} />
        </g>
      )}
      <rect className="bf-hit" x={-46} y={-64} width={92} height={80} />
    </g>
  );
}

function Pad({ pad, at, selected, onPress, onTip }: { pad: PadIn; at: Point; selected: boolean; onPress: () => void; onTip: (tip: Tip | undefined, target?: SVGGElement) => void }) {
  const tip = { said: pad.word, title: `slot-${pad.slot}, ${pad.repo}`, left: 0, top: 0 };
  return (
    <g
      className="bf-pad"
      role="option"
      tabIndex={0}
      aria-selected={selected}
      aria-label={`slot-${pad.slot}, ${pad.repo}, ${pad.word}`}
      data-bay={pad.bay}
      data-selected={selected || undefined}
      transform={`translate(${at.x} ${at.y})`}
      onClick={onPress}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onPress();
        }
      }}
      onMouseEnter={(event) => onTip(tip, event.currentTarget)}
      onMouseLeave={() => onTip(undefined)}
      onFocus={(event) => onTip(tip, event.currentTarget)}
      onBlur={() => onTip(undefined)}
    >
      <ellipse className="bf-pad__ring" rx={14} ry={5.8} />
      <ellipse className="bf-pad__core" rx={7} ry={2.9} />
      <rect className="bf-hit" x={-16} y={-10} width={32} height={20} />
    </g>
  );
}

/** A pull request: a small crate on its route to the objective, further along the nearer it is to merging. */
function Payload({ at, from, pr, arriving, fresh, onPress, onTip }: { at: Point; from: Point; pr: PrIn; arriving: Play | undefined; fresh: Play | undefined; onPress: () => void; onTip: (tip: Tip | undefined, target?: SVGGElement) => void }) {
  const said = STAGE_SAID[pr.stage];
  const title = `#${pr.number}`;
  const offset = { x: from.x - at.x, y: from.y - at.y };
  return (
    <g
      className="bf-payload"
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={`Pull request ${title}, ${said}`}
      data-stage={pr.stage}
      transform={`translate(${at.x.toFixed(1)} ${at.y.toFixed(1)})`}
      onClick={onPress}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onPress();
        }
      }}
      onMouseEnter={(event) => onTip({ said, title: `Pull request ${title}`, left: 0, top: 0 }, event.currentTarget)}
      onMouseLeave={() => onTip(undefined)}
      onFocus={(event) => onTip({ said, title: `Pull request ${title}`, left: 0, top: 0 }, event.currentTarget)}
      onBlur={() => onTip(undefined)}
    >
      <g
        className="bf-crate"
        data-fresh={fresh !== undefined || undefined}
        data-arriving={arriving !== undefined || undefined}
        style={{ "--fx": `${offset.x}px`, "--fy": `${offset.y}px`, "--wait": `${(arriving ?? fresh)?.delay ?? 0}ms` } as CSSProperties}
      >
        <polygon className="bf-crate__box" points="0,-6.5 7.5,0 0,6.5 -7.5,0" />
        <circle className="bf-crate__dot" r={1.8} />
      </g>
      <circle className="bf-hit" r={13} />
    </g>
  );
}

// ---- The field -----------------------------------------------------------------------------------------

export function Battlefield({
  repos,
  drones,
  pads,
  goals,
  prs,
  trails,
  plays,
  selected,
  onSelect,
  onPad,
  onGoal,
  onPr,
}: {
  repos: readonly RepoIn[];
  drones: readonly DroneIn[];
  pads: ReadonlyMap<string, PadIn>;
  goals: ReadonlyMap<string, GoalIn>;
  prs: readonly PrIn[];
  trails: ReadonlyMap<string, number>;
  plays: readonly Play[];
  selected: string | undefined;
  onSelect: (key: string) => void;
  onPad: (key: string) => void;
  onGoal: (root: string) => void;
  onPr: (key: string) => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState<Size>({ width: 900, height: 560 });
  useLayoutEffect(() => {
    const one = host.current;
    if (one === null) return;
    const read = () => {
      const box = one.getBoundingClientRect();
      if (box.width > 0 && box.height > 0) setSize((was) => (Math.round(box.width) === was.width && Math.round(box.height) === was.height ? was : { width: Math.round(box.width), height: Math.round(box.height) }));
    };
    read();
    const watch = new ResizeObserver(read);
    watch.observe(one);
    return () => watch.disconnect();
  }, []);

  const big = bigOf(size);
  const layout = useMemo(() => layoutOf(repos, drones, size, big), [repos, drones, size, big]);
  const territory = new Map(layout.territories.map((one) => [one.root, one] as const));
  const padAt = new Map(layout.territories.flatMap((one) => one.pads.map((pad) => [pad.key, { x: pad.x, y: pad.y }] as const)));
  const baseOf = (root: string): Point => {
    const t = territory.get(root);
    return t === undefined ? { x: size.width / 2, y: size.height } : { x: t.base.x - t.base.w / 2 + 36, y: t.base.y + 8 };
  };
  const sorted = [...drones].sort((a, b) => (layout.at.get(a.key)?.y ?? 0) - (layout.at.get(b.key)?.y ?? 0));
  const groundOf = (key: string): Point | undefined => {
    const at = layout.at.get(key);
    return at === undefined ? undefined : { x: at.x, y: at.y };
  };
  const airOf = (drone: DroneIn): Point | undefined => {
    const at = layout.at.get(drone.key);
    return at === undefined ? undefined : { x: at.x, y: at.y - (drone.hover ? HOVER : 4) * at.s - 6 };
  };
  const droneBy = new Map(drones.map((one) => [one.key, one] as const));
  const playOf = (key: string, kind: Play["kind"]) => plays.find((one) => one.key === key && one.kind === kind);

  // The camera: the whole field, or the locked drone at closer range. Zoom is one more step in or out.
  const [zoom, setZoom] = useState(1);
  const [aim, setAim] = useState<Point | undefined>();
  const lock = selected === undefined ? undefined : layout.at.get(selected);
  const lockX = lock?.x;
  const lockY = lock?.y;
  const first = useRef(true);
  useLayoutEffect(() => {
    if (lockX === undefined || lockY === undefined) return;
    // The first frame is the whole field; the camera eases to the lock one frame later.
    const go = () => {
      setAim({ x: lockX, y: lockY - 18 });
      setZoom((was) => Math.max(was, 1.45));
    };
    if (first.current) {
      first.current = false;
      const frame = window.requestAnimationFrame(() => window.requestAnimationFrame(go));
      return () => window.cancelAnimationFrame(frame);
    }
    go();
  }, [selected, lockX, lockY]);
  // A dispatch or a merge that happens while he watches is shown whole: the camera backs out to the field.
  const arrival = plays.some((one) => one.id.startsWith("arrived:") || one.id.startsWith("merged:"));
  useEffect(() => {
    if (!arrival) return;
    setZoom(1);
    setAim(undefined);
  }, [arrival]);
  const centre = aim ?? { x: size.width / 2, y: size.height / 2 };
  const cam = cameraOf(size, zoom, centre);

  const [tip, setTip] = useState<Tip>();
  const [hot, setHot] = useState<string>();
  const showTip = (next: Tip | undefined, target?: SVGGElement) => {
    if (next === undefined || target === undefined) return setTip(undefined);
    const box = target.getBoundingClientRect();
    const base = host.current?.getBoundingClientRect();
    if (base !== undefined) setTip({ ...next, left: box.x - base.x + box.width / 2, top: box.y - base.y });
  };

  const locked = selected !== undefined && lock !== undefined;
  const [hotTile, setHotTile] = useState<string>();
  // The plots: a name for those the selected drone is over and those a calling drone is over, a lift for those a working
  // drone is over, an amber hatch for those two or more are changing.
  const clashAt = new Map(layout.clashes.map((one) => [one.path, one] as const));
  const labelled = new Set<string>();
  const lit = new Set<string>();
  for (const drone of drones) {
    if (!drone.shown) continue;
    for (const { tile } of layout.over.get(drone.key) ?? []) {
      if (drone.key === selected || drone.ping) labelled.add(tile.path);
      if (drone.spin) lit.add(tile.path);
    }
  }
  // Labels: the selected drone's, the one under the pointer, then those that need him and those failing, each only where it
  // meets no label already drawn, so no two collide. Any other is named by its tip.
  const labels = new Set<string>();
  const taken: { l: number; r: number; t: number; b: number }[] = [];
  const ranked = drones.filter((one) => one.shown && (one.key === selected || one.key === hot || one.hue !== "calm")).sort((a, b) => rank(a, selected, hot) - rank(b, selected, hot));
  for (const one of ranked) {
    const at = layout.at.get(one.key);
    if (at === undefined) continue;
    const wide = Math.min(22, one.title.length) * 6.7 + 10;
    const y = at.y + (one.kind === "job" ? 26 : 20) * 0.4 * at.s * big + 22;
    const box = { l: at.x - wide / 2, r: at.x + wide / 2, t: y - 13, b: y + 5 };
    const meets = (o: { l: number; r: number; t: number; b: number }) => box.l < o.r && box.r > o.l && box.t < o.b && box.b > o.t;
    const bodies = drones.some((other) => {
      const there = other.key === one.key ? undefined : layout.at.get(other.key);
      const w = (other.kind === "job" ? 34 : 26) * big * (there?.s ?? 1);
      return there !== undefined && meets({ l: there.x - w, r: there.x + w, t: there.y - (HOVER + 28) * big * there.s, b: there.y + 8 * big });
    });
    if (one.key !== selected && one.key !== hot && (bodies || taken.some(meets))) continue;
    taken.push(box);
    labels.add(one.key);
  }
  // Boxes of every drone's body, so a plot's name is not set under one.
  const bodies = drones.flatMap((one) => {
    const there = layout.at.get(one.key);
    if (there === undefined) return [];
    const w = (one.kind === "job" ? 34 : 26) * big * there.s;
    return [{ l: there.x - w, r: there.x + w, t: there.y - (HOVER + 28) * big * there.s, b: there.y + 8 * big }];
  });
  const landing = new Set(plays.filter((one) => one.kind === "capture" && one.number !== undefined).map((one) => one.number));
  return (
    <div ref={host} className="bf" data-locked={locked || undefined}>
      <svg className="bf-grain" aria-hidden="true" width="100%" height="100%">
        <filter id="bf-noise" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="1" seed="5" />
          <feColorMatrix values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 0.5 0" />
        </filter>
        <rect width="100%" height="100%" filter="url(#bf-noise)" />
      </svg>
      <div className="bf-world" style={{ transform: `translate(${cam.x.toFixed(1)}px, ${cam.y.toFixed(1)}px) scale(${zoom})` }}>
        <svg className="bf-field" width={size.width} height={size.height} viewBox={`0 0 ${size.width} ${size.height}`} role="listbox" aria-label="Map" data-locked={locked || undefined}>
          <defs>
            <pattern id="bf-hatch" width="8" height="8" patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
              <line x1="0" y1="0" x2="0" y2="8" className="bf-hatch-line" />
            </pattern>
            <linearGradient id="bf-fade" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0" stopColor="#fff" stopOpacity="0" />
              <stop offset="0.7" stopColor="#fff" stopOpacity="0.55" />
              <stop offset="1" stopColor="#fff" stopOpacity="1" />
            </linearGradient>
            <mask id="bf-fade-mask" maskUnits="userSpaceOnUse" x="0" y="0" width={size.width} height={size.height}>
              <rect width={size.width} height={size.height} fill="url(#bf-fade)" />
            </mask>
            {(["calm", "ask", "bad"] as const).map((hue) => (
              <radialGradient key={hue} id={`bf-pool-${hue}`}>
                <stop offset="0" className={`bf-stop-pool-${hue}`} stopOpacity={hue === "calm" ? 0.14 : 0.24} />
                <stop offset="1" className={`bf-stop-pool-${hue}`} stopOpacity="0" />
              </radialGradient>
            ))}
            <linearGradient id="bf-cone" x1="0" y1="0" x2="1" y2="0">
              <stop offset="0" className="bf-stop-cone-a" />
              <stop offset="1" className="bf-stop-cone-b" />
            </linearGradient>
          </defs>

          <g mask="url(#bf-fade-mask)">
            <Grid size={size} />
          </g>

          {layout.territories.map((one) => (
            <g key={one.root} className="bf-territory">
              <rect className="bf-front" aria-hidden="true" x={one.rect.x} y={one.rect.y} width={one.rect.w} height={one.rect.h} rx={26} />
              {one.tiles.map((tile) => {
                const clash = clashAt.get(tile.path);
                const inset = Math.min(tile.rect.w, tile.rect.h);
                const room = Math.max(4, Math.floor((tile.rect.w - 12) / 6.3));
                const box = { l: tile.rect.x + 5, r: tile.rect.x + 9 + Math.min(room, tile.path.length) * 6.4, t: tile.rect.y + 2, b: tile.rect.y + 18 };
                const meets = (o: { l: number; r: number; t: number; b: number }) => box.l < o.r && box.r > o.l && box.t < o.b && box.b > o.t;
                const named = hotTile === tile.path || (labelled.has(tile.path) && !bodies.some(meets) && !taken.some(meets));
                return (
                  <g
                    key={tile.path}
                    className="bf-plot"
                    style={{ "--tint": `${(3 + rand(tile.path, 1) * 3).toFixed(1)}%` } as CSSProperties}
                    data-lit={lit.has(tile.path) || undefined}
                    data-clash={clash !== undefined || undefined}
                    data-named={named || undefined}
                    onMouseEnter={(event) => {
                      setHotTile(tile.path);
                      showTip(clash === undefined ? { said: "Directory", title: tile.path, left: 0, top: 0 } : { said: "Write clash", title: `Both are changing ${tile.path}`, left: 0, top: 0 }, event.currentTarget);
                    }}
                    onMouseLeave={() => {
                      setHotTile(undefined);
                      showTip(undefined);
                    }}
                  >
                    <rect className="bf-plot__ground" x={tile.rect.x} y={tile.rect.y} width={tile.rect.w} height={tile.rect.h} rx={4} />
                    {inset > 52 ? <rect className="bf-plot__contour" x={tile.rect.x + 8} y={tile.rect.y + 8} width={tile.rect.w - 16} height={tile.rect.h - 16} rx={10} /> : null}
                    {inset > 100 ? <rect className="bf-plot__contour" x={tile.rect.x + 17} y={tile.rect.y + 17} width={tile.rect.w - 34} height={tile.rect.h - 34} rx={16} /> : null}
                    {clash === undefined ? null : <rect className="bf-plot__hatch" x={tile.rect.x} y={tile.rect.y} width={tile.rect.w} height={tile.rect.h} rx={4} fill="url(#bf-hatch)" />}
                    {named && tile.rect.w > 44 ? (
                      <text className="bf-plot__label" x={tile.rect.x + 7} y={tile.rect.y + 14}>
                        {short(tile.path, room)}
                      </text>
                    ) : null}
                  </g>
                );
              })}
              {one.groups.map((group) => (
                <rect key={group.dir} className="bf-group" aria-hidden="true" x={group.rect.x} y={group.rect.y} width={group.rect.w} height={group.rect.h} rx={6} />
              ))}
            </g>
          ))}

          {layout.territories.map((one) => (
            <Base key={one.root} territory={one} onTip={showTip} />
          ))}

          {/* What the drones left on the ground: the contrail of a dispatch, the tether to a bay it holds, the lines of a formation. */}
          <g className="bf-lines" aria-hidden="true">
            {drones.map((drone) => {
              const to = groundOf(drone.key);
              if (to === undefined) return null;
              const age = trails.get(drone.key);
              const pad = drone.slot === undefined ? undefined : padAt.get(drone.slot);
              const parent = drone.parent === undefined ? undefined : groundOf(drone.parent);
              const origin = pad ?? baseOf(drone.root);
              const lit = selected === drone.key || selected === drone.parent;
              const launching = playOf(drone.key, "launch");
              return (
                <g key={drone.key} data-dim={!drone.shown || undefined}>
                  {age === undefined ? null : <path className="bf-trail" pathLength={1} d={arcOf(origin, to, 0.1)} style={{ opacity: Math.max(0.05, 0.15 * (1 - age / TRAIL_MS)), ...(launching === undefined ? {} : ({ "--wait": `${launching.delay}ms` } as CSSProperties)) }} data-launching={launching !== undefined || undefined} />}
                  {pad === undefined ? null : <path className="bf-tether" d={`M${pad.x},${pad.y} L${to.x},${to.y}`} />}
                  {parent === undefined ? null : <path className="bf-formation" data-lit={lit || undefined} d={arcOf(parent, to, 0.12)} />}
                  {(layout.over.get(drone.key) ?? []).map(({ tile }, i) => {
                    const from = airOf(drone);
                    if (from === undefined) return null;
                    return (
                      <g key={tile.path} className="bf-touch" data-main={i === 0 || undefined} data-lit={selected === drone.key || undefined}>
                        <path d={`M${from.x.toFixed(1)},${from.y.toFixed(1)} L${tile.cx.toFixed(1)},${tile.cy.toFixed(1)}`} />
                        <ellipse cx={tile.cx} cy={tile.cy} rx={8} ry={3.2} />
                      </g>
                    );
                  })}
                </g>
              );
            })}
            {layout.clashes.map((clash) => {
              const a = groundOf(clash.keys[0]!);
              const b = groundOf(clash.keys[1]!);
              if (a === undefined || b === undefined) return null;
              const tip = { said: "Write clash", title: `Both are changing ${clash.path}`, left: 0, top: 0 };
              return (
                <g key={clash.path} className="bf-clash">
                  <path className="bf-clash__line" d={arcOf(a, b, 0.1)} />
                  <path className="bf-clash__hit" d={arcOf(a, b, 0.1)} onMouseEnter={(event) => showTip(tip, event.currentTarget as unknown as SVGGElement)} onMouseLeave={() => showTip(undefined)} />
                </g>
              );
            })}
          </g>

          {/* Plays on the ground: a radar cone turning on a drone running a Check, a tracer between a Session and its Job, a ring where something was said. */}
          <g className="bf-plays" aria-hidden="true">
            {plays.map((play) => {
              const own = play.key === undefined ? undefined : layout.at.get(play.key);
              if (own === undefined) return null;
              const drone = droneBy.get(play.key!)!;
              if (play.kind === "sweep") {
                return (
                  <g key={play.id} transform={`translate(${own.x} ${own.y}) scale(${own.s})`}>
                    <g transform="scale(1 0.42)">
                      <g className="bf-sweep" style={{ animationDelay: `${play.delay}ms` }}>
                        <path d="M0,0 L88,-44 A98,98 0 0 1 88,44 Z" fill="url(#bf-cone)" />
                      </g>
                    </g>
                  </g>
                );
              }
              if (play.kind === "tracer") {
                const to = drone.parent === undefined ? undefined : groundOf(drone.parent);
                if (to === undefined) return <ellipse key={play.id} className="bf-blip" cx={own.x} cy={own.y} rx={24} ry={9} style={{ animationDelay: `${play.delay}ms` }} />;
                return <path key={play.id} className="bf-tracer" pathLength={100} d={arcOf({ x: own.x, y: own.y }, to, 0.12)} style={{ animationDelay: `${play.delay}ms` }} />;
              }
              if (play.kind === "blip") return <ellipse key={play.id} className="bf-blip" cx={own.x} cy={own.y} rx={24} ry={9} style={{ animationDelay: `${play.delay}ms` }} />;
              return null;
            })}
          </g>

          {layout.territories.map((one) =>
            one.pads.map((pad) => {
              const info = pads.get(pad.key);
              return info === undefined ? null : <Pad key={pad.key} pad={info} at={pad} selected={false} onPress={() => onPad(pad.key)} onTip={showTip} />;
            }),
          )}

          {layout.territories.map((one) => {
            const goal = goals.get(one.root);
            if (goal === undefined || one.goal === undefined) return null;
            return <Goal key={one.root} at={one.goal} goal={goal} capture={plays.find((p) => p.kind === "capture" && (p.root === one.root || (p.number !== undefined && prs.some((pr) => pr.number === p.number && pr.root === one.root))))} onPress={() => onGoal(one.root)} onTip={showTip} />;
          })}

          {sorted.map((drone, index) => {
            const at = layout.at.get(drone.key);
            if (at === undefined) return null;
            const launch = playOf(drone.key, "launch");
            const pad = drone.slot === undefined ? undefined : padAt.get(drone.slot);
            const origin = pad ?? baseOf(drone.root);
            const on = selected === drone.key;
            const asks = drone.hue !== "calm";
            return (
              <Drone
                key={drone.key}
                drone={drone}
                at={at}
                from={launch === undefined ? undefined : { x: origin.x - at.x, y: origin.y - at.y }}
                index={index}
                big={big}
                selected={on}
                hot={hot === drone.key}
                labelled={labels.has(drone.key)}
                muted={drone.shown && locked && !on && !asks}
                away={!drone.shown}
                play={launch}
                onSelect={() => onSelect(drone.key)}
                onTip={(next, target) => {
                  setHot(next === undefined ? undefined : drone.key);
                  showTip(next, target);
                }}
              />
            );
          })}

          {/* Pull requests in the air: from the drone that owns one, or from the base where none does, toward the objective. */}
          <g className="bf-routes">
            {prs.map((pr) => {
              const goal = territory.get(pr.root)?.goal;
              if (goal === undefined) return null;
              const owner = pr.owner === undefined ? undefined : droneBy.get(pr.owner);
              const start = (owner === undefined ? undefined : airOf(owner)) ?? { ...baseOf(pr.root), y: baseOf(pr.root).y - 14 };
              const end = { x: goal.x, y: goal.y - 20 };
              const landed = pr.stage === "landed";
              if (landed && !landing.has(pr.number)) return null;
              const rest = arcAt(start, end, landed ? ALONG.queued : ALONG[pr.stage], 0.07);
              const arrival = plays.find((p) => p.kind === "capture" && p.number === pr.number);
              const opened = plays.find((p) => p.kind === "payload" && p.number === pr.number);
              return (
                <g key={pr.key} data-shown={owner?.shown ?? true} data-dim={owner !== undefined && !owner.shown ? true : undefined}>
                  {landed ? null : <path className="bf-route" data-stage={pr.stage} d={arcOf(start, end, 0.07)} />}
                  <Payload at={landed ? end : rest} from={landed ? rest : start} pr={pr} arriving={landed ? arrival : undefined} fresh={landed ? undefined : opened} onPress={() => onPr(pr.key)} onTip={showTip} />
                </g>
              );
            })}
          </g>
        </svg>
      </div>
      <div className="bf-haze" aria-hidden="true" />
      {tip === undefined ? null : (
        <div className="bf-hover" style={{ left: tip.left, top: tip.top }} role="tooltip">
          <span>{tip.said}</span>
          {tip.title}
        </div>
      )}
      <div className="bf-zoom">
        <Tooltip label="Zoom in" asChild>
          <Button variant="ghost" iconOnly aria-label="Zoom in" disabled={zoom >= 3} onClick={() => setZoom((was) => Math.min(3, was + 0.5))}>
            <Plus size={16} aria-hidden />
          </Button>
        </Tooltip>
        <Tooltip label="Zoom out" asChild>
          <Button
            variant="ghost"
            iconOnly
            aria-label="Zoom out"
            disabled={zoom <= 1}
            onClick={() => {
              setZoom((was) => Math.max(1, was - 0.5));
              if (zoom <= 1.5) setAim(undefined);
            }}
          >
            <Minus size={16} aria-hidden />
          </Button>
        </Tooltip>
        <Tooltip label="Locate the selection" asChild>
          <Button
            variant="ghost"
            iconOnly
            aria-label="Locate the selection"
            disabled={lock === undefined}
            onClick={() => {
              if (lock === undefined) return;
              setAim({ x: lock.x, y: lock.y - 18 });
              setZoom((was) => Math.max(was, 1.45));
            }}
          >
            <Crosshair size={16} aria-hidden />
          </Button>
        </Tooltip>
      </div>
    </div>
  );
}
