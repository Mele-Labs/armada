// The flight deck's map, a battlefield on a tilted plane in perspective. Each repository is a territory laid out as its own
// directories: every directory a slab standing on the ground, higher where drones work over it and tallest where two of
// them clash. A base sits on the near edge, its landing pads the bays, and the repository's main is the objective on the far
// edge, a flag. Each Job and Session is a drone flying at altitude over the directories it changes, on a vertical beam from
// its shadow, a Job a large X quad and a Session a small plus quad. State is the drone and its frame, never words: rotors turn
// while it works, a drone that is idle or done is low with its rotors still, a beacon pings from one that holds a call, an
// amber ring breathes round one that is stuck, a red flare burns on one whose Check fails. Pull requests are payloads on a
// lifted route to the objective. The activity log drives the scene (`deck-field.ts` says which event moves what).
//
// The 3D is CSS alone: a plane in `perspective`, tilted with rotateX and a little rotateZ, its parts placed with translate3d.
// Drones, labels and flags are billboards, counter-rotated so they stand upright and read. The camera, the slabs and the
// drones' flights are transitions and keyframes of transform and opacity; no loop runs in script. The lifted lines are drawn on
// one layer above the slabs and shifted by the height they stand for. Calm at rest: only what is working or calling moves.
// Mock only. proto/cockpit-flight-deck.

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { Crosshair, Minus, Plus } from "lucide-react";
import { Button, Tooltip } from "@armada/components";

import { ALONG, CAM, FLAT, SLAB, STAGE_SAID, TRAIL_MS, alongOf, bigOf, heightsOf, layoutOf, rand, type DroneIn, type P3, type Placed, type Play, type Point, type RepoIn, type Size, type Territory, type Tile } from "./deck-field";

export type PadIn = { key: string; slot: number; bay: string; word: string; repo: string; holder?: string };
export type PrIn = { key: string; number: number; stage: keyof typeof ALONG; root: string; owner?: string };
export type GoalIn = { root: string; said: string; red: boolean };

/**
 * The height of the layer the lifted lines are drawn on, just above the tallest slab. And the picture of a drone: a box
 * standing upright round it. Two planes that cross are split and drawn in pieces, and a piece can lose to the wrong one, so a
 * drone flies with the foot of that box clear of every slab and ground beneath it (`clearance`), plus a margin for a slab that
 * rises a little when something touches it.
 */
const LAYER = SLAB.high + 2;
const BOX = { x: -68, y: -58, w: 136, h: 96 };
const FOOT = 0.81 * (BOX.y + BOX.h);
const MARGIN = 10;
/** The least height a pull request flies at: its picture clear of every slab. */
const ROUTE = SLAB.high + 22;
const SIN = Math.sin((CAM.wide * Math.PI) / 180);

type Tip = { said: string; title: string; left: number; top: number };
type Target = Element;

const vars = (values: Record<string, string | number>) => values as CSSProperties;
const px = (value: number) => `${value.toFixed(1)}px`;
const rank = (one: DroneIn, selected: string | undefined, hot: string | undefined) => (one.key === selected ? 0 : one.key === hot ? 1 : one.hue === "bad" ? 2 : 3);
const short = (title: string, room: number) => (title.length > room ? `${title.slice(0, room - 1)}…` : title);
const onPlane = (p: Point): Point => ({ x: p.x, y: p.y * FLAT });
const keyed = (event: { key: string; preventDefault: () => void }, act: () => void) => {
  if (event.key === "Enter" || event.key === " ") {
    event.preventDefault();
    act();
  }
};

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

/** Where a point at some height is drawn on the lifted layer, so that it shows where it stands at the camera's pitch. */
function liftOf(pitch: number): (p: P3) => Point {
  const tan = Math.tan((pitch * Math.PI) / 180);
  const sin = Math.sin((CAM.yaw * Math.PI) / 180);
  const cos = Math.cos((CAM.yaw * Math.PI) / 180);
  return (p) => {
    const a = (p.z - LAYER) * tan;
    return { x: p.x - a * sin, y: p.y - a * cos };
  };
}

/**
 * A flat picture standing in the scene, upright toward the viewer, its origin on the anchor. The billboard is sized round
 * the picture and turns about the anchor: a box of no size, or a picture drawn off its own origin, is clipped by the layer. The viewBox starts at zero so that a transform-origin of 0 0 in the CSS is the anchor.
 */
function Sprite({ x, y, w, h, label, children }: { x: number; y: number; w: number; h: number; label?: boolean; children: React.ReactNode }) {
  return (
    <div className={label === undefined ? "bf-bb" : "bf-bb bf-tag"} style={{ left: x, top: y, width: w, height: h, transformOrigin: `${-x}px ${-y}px` }}>
      <svg className="bf-sprite" width={w} height={h} viewBox={`0 0 ${w} ${h}`} aria-hidden="true">
        <g transform={`translate(${-x} ${-y})`}>{children}</g>
      </svg>
    </div>
  );
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
  floor,
  alt,
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
  /** Where it stands on the plane. */
  at: Point;
  /** The height of the plot under it, where its shadow lies. */
  floor: number;
  /** How high it flies: its centre above the plane. */
  alt: number;
  /** Where it lifted from, relative to where it stands, and how high that was, while it is launching. */
  from: P3 | undefined;
  index: number;
  big: number;
  selected: boolean;
  hot: boolean;
  labelled: boolean;
  muted: boolean;
  away: boolean;
  play: Play | undefined;
  onSelect: () => void;
  onTip: (tip: Tip | undefined, target?: Target) => void;
}) {
  const wide = drone.kind === "job" ? 26 : 20;
  const said = `${drone.kind === "job" ? "Job" : "Session"}, ${drone.said}`;
  const show = (event: { currentTarget: Element }) => onTip({ said, title: drone.title, left: 0, top: 0 }, event.currentTarget);
  const launching = play?.kind === "launch" && from !== undefined;
  return (
    <div
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
      data-launching={launching || undefined}
      style={vars({
        transform: `translate3d(${px(at.x)}, ${px(at.y)}, 0px)`,
        "--alt": alt,
        "--floor": floor,
        "--beam": Math.max(0, alt - FOOT - floor),
        "--r": `${(wide * 0.85 * big).toFixed(1)}px`,
        "--i": Math.min(index, 14),
        "--wait": `${play?.delay ?? 0}ms`,
        ...(launching ? { "--lx": px(from.x), "--ly": px(from.y), "--zs": from.z } : {}),
      })}
      onClick={onSelect}
      onKeyDown={(event) => keyed(event, onSelect)}
      onMouseEnter={show}
      onMouseLeave={() => onTip(undefined)}
      onFocus={show}
      onBlur={() => onTip(undefined)}
    >
      {/* The ground under it, lying on the plot: a pool of light where it works or calls, the shadow, the disc, the rings that say how it stands. */}
      <div className="bf-gnd" aria-hidden="true">
        <i className="bf-pool" />
        {drone.ping ? (
          <>
            <i className="bf-ping" />
            <i className="bf-ping" data-late />
          </>
        ) : null}
        {drone.stuck ? <i className="bf-stall" /> : null}
        <i className="bf-shadow" />
        <i className="bf-spot" />
        <i className="bf-rim" />
        {selected ? <i className="bf-ticks" /> : null}
      </div>
      <div className="bf-beam3" aria-hidden="true" />
      {/* The air: the drone on its altitude, a billboard so it stands upright. */}
      <div className="bf-alt">
        <Sprite x={BOX.x} y={BOX.y} w={BOX.w} h={BOX.h}>
          <g transform={`scale(${big})`}>
            {drone.flare ? (
              <g className="bf-flare">
                <path d="M-9,-16 L-13,-27 M0,-19 L0,-33 M9,-16 L13,-27" />
              </g>
            ) : null}
            {drone.kind === "job" ? <JobGlyph /> : <SessionGlyph />}
            {drone.ping ? (
              <g className="bf-mast">
                <line x1={0} y1={-7} x2={0} y2={-20} />
                <circle className="bf-tip" cy={-21.5} r={2.6} />
              </g>
            ) : null}
            {selected ? (
              <g className="bf-reticle">
                <path d={bracketsOf(wide + 14, -34, 22)} />
              </g>
            ) : null}
          </g>
          <rect className="bf-hit" x={-(wide + 12) * big} y={-34 * big} width={(wide + 12) * 2 * big} height={56 * big} />
        </Sprite>
        {/* Its name, a picture of its own above it, so that it is clear of everything the drone's own box is. */}
        {labelled ? (
          <Sprite x={-110} y={-Math.round(24 * big + (drone.flare ? 26 : drone.ping ? 14 : 4) + (selected ? 14 : 0) + 16)} w={220} h={24} label>
            <text className="bf-label" y={-Math.round(24 * big + (drone.flare ? 26 : drone.ping ? 14 : 4) + (selected ? 14 : 0))} textAnchor="middle">
              {short(drone.title, 22)}
            </text>
          </Sprite>
        ) : null}
      </div>
    </div>
  );
}

// ---- The scene's plain parts ---------------------------------------------------------------------------

/** A plot of the repository: a slab on the ground, its top lit, two sides darker, its height set by what works over it. */
function Slab({
  tile,
  height,
  lit,
  clash,
  named,
  bump,
  room,
  onEnter,
  onLeave,
}: {
  tile: Tile;
  height: number;
  lit: boolean;
  clash: boolean;
  named: boolean;
  bump: { wait: number; amp: number } | undefined;
  room: number;
  onEnter: (event: { currentTarget: Element }) => void;
  onLeave: () => void;
}) {
  const { rect } = tile;
  const inset = Math.min(rect.w, rect.h * FLAT);
  return (
    <div
      className="bf-slab"
      data-lit={lit || undefined}
      data-clash={clash || undefined}
      data-bump={bump !== undefined || undefined}
      style={vars({ left: rect.x, top: rect.y * FLAT, width: rect.w, height: rect.h * FLAT, "--h": height.toFixed(2), "--tint": `${(5 + rand(tile.path, 1) * 3).toFixed(1)}%`, ...(bump === undefined ? {} : { "--wait": `${bump.wait}ms`, "--amp": bump.amp }) })}
      onMouseEnter={onEnter}
      onMouseLeave={onLeave}
    >
      <div className="bf-top">{inset > 60 ? <span className="bf-contour" /> : null}</div>
      <div className="bf-face" data-side="south" />
      <div className="bf-face" data-side="west" />
      {named && rect.w > 44 ? (
        <div className="bf-plotname">
          <div className="bf-bb">{short(tile.path, room)}</div>
        </div>
      ) : null}
    </div>
  );
}

/** The compound on the ground: two plates, where the pads are set. */
function Base({ territory, onTip }: { territory: Territory; onTip: (tip: Tip | undefined, target?: Target) => void }) {
  const { base } = territory;
  const centre = onPlane({ x: base.x, y: base.y + 4 });
  const width = base.w + 24;
  return (
    <g
      className="bf-base"
      onMouseEnter={(event) => onTip({ said: "Repository", title: territory.name, left: 0, top: 0 }, event.currentTarget)}
      onMouseLeave={() => onTip(undefined)}
    >
      <rect className="bf-slab-base" x={centre.x - width / 2} y={centre.y - 27 * FLAT} width={width} height={54 * FLAT} rx={36} />
      <rect className="bf-slab-base" data-inner x={centre.x - width / 2 + 16} y={centre.y - 19 * FLAT} width={width - 32} height={38 * FLAT} rx={26} />
    </g>
  );
}

/** What stands on the compound: the hangar, and the repository's name. */
function Hangar({ territory }: { territory: Territory }) {
  const { base } = territory;
  const centre = onPlane({ x: base.x, y: base.y + 4 });
  const hangar = onPlane({ x: base.x - base.w / 2 + 36, y: base.y - 2 });
  return (
    <>
      <div className="bf-at" style={{ transform: `translate3d(${px(hangar.x)}, ${px(hangar.y)}, 24px)` }} aria-hidden="true">
        <Sprite x={-40} y={-50} w={80} h={70}>
          <polygon className="bf-hangar" data-face="top" points="-26,-8 0,-18 26,-8 0,2" />
          <polygon className="bf-hangar" data-face="left" points="-26,-8 0,2 0,16 -26,6" />
          <polygon className="bf-hangar" data-face="right" points="26,-8 0,2 0,16 26,6" />
          <path className="bf-mast-line" d="M18,-12 V-30" />
          <circle className="bf-mast-dot" cx={18} cy={-31} r={1.8} />
        </Sprite>
      </div>
      <div className="bf-at" style={{ transform: `translate3d(${px(centre.x)}, ${px(centre.y + 30 * FLAT)}, 0px)` }} aria-hidden="true">
        <div className="bf-bb bf-name">{territory.name}</div>
      </div>
    </>
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
  onTip: (tip: Tip | undefined, target?: Target) => void;
}) {
  const said = goal.said;
  const tip = { said: "Objective", title: said, left: 0, top: 0 };
  return (
    <div
      className="bf-goal"
      data-red={goal.red || undefined}
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={`${said}, objective`}
      style={{ transform: `translate3d(${px(at.x)}, ${px(at.y)}, 1px)` }}
      onClick={onPress}
      onKeyDown={(event) => keyed(event, onPress)}
      onMouseEnter={(event) => onTip(tip, event.currentTarget)}
      onMouseLeave={() => onTip(undefined)}
      onFocus={(event) => onTip(tip, event.currentTarget)}
      onBlur={() => onTip(undefined)}
    >
      <i className="bf-zone" />
      <i className="bf-goal-rim" />
      {capture === undefined ? null : (
        <>
          <i className="bf-flash" style={vars({ "--d": `${capture.delay + 900}ms` })} />
          <i className="bf-quake" style={vars({ "--d": `${capture.delay + 900}ms` })} />
          <i className="bf-quake" data-late style={vars({ "--d": `${capture.delay + 1250}ms` })} />
          <div className="bf-column" style={vars({ "--d": `${capture.delay + 900}ms` })} />
        </>
      )}
      <Sprite x={-70} y={-112} w={140} h={112}>
        <g transform="scale(1.35)">
          <line className="bf-pole" x1={0} y1={0} x2={0} y2={-52} />
          <path className="bf-pennant" d="M0,-52 L26,-45 L0,-37 Z" />
          <circle className="bf-beacon" cy={-54} r={2.6} />
        </g>
        <rect className="bf-hit" x={-62} y={-92} width={124} height={92} />
      </Sprite>
    </div>
  );
}

function Pad({ pad, at, onPress, onTip }: { pad: PadIn; at: Point; onPress: () => void; onTip: (tip: Tip | undefined, target?: Target) => void }) {
  const tip = { said: pad.word, title: `slot-${pad.slot}, ${pad.repo}`, left: 0, top: 0 };
  return (
    <g
      className="bf-pad"
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={`slot-${pad.slot}, ${pad.repo}, ${pad.word}`}
      data-bay={pad.bay}
      transform={`translate(${at.x} ${at.y})`}
      onClick={onPress}
      onKeyDown={(event) => keyed(event, onPress)}
      onMouseEnter={(event) => onTip(tip, event.currentTarget)}
      onMouseLeave={() => onTip(undefined)}
      onFocus={(event) => onTip(tip, event.currentTarget)}
      onBlur={() => onTip(undefined)}
    >
      <circle className="bf-pad__ring" r={13} />
      <circle className="bf-pad__core" r={6.5} />
      <circle className="bf-hit" r={17} />
    </g>
  );
}

/** A pull request: a small crate flying its route to the objective, further along the nearer it is to merging. */
function Payload({ at, from, pr, arriving, fresh, onPress, onTip }: { at: P3; from: P3; pr: PrIn; arriving: Play | undefined; fresh: Play | undefined; onPress: () => void; onTip: (tip: Tip | undefined, target?: Target) => void }) {
  const said = STAGE_SAID[pr.stage];
  const title = `#${pr.number}`;
  const tip = { said, title: `Pull request ${title}`, left: 0, top: 0 };
  const hop = arriving ?? fresh;
  return (
    <div
      className="bf-payload"
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={`Pull request ${title}, ${said}`}
      data-stage={pr.stage}
      style={{ transform: `translate3d(${px(at.x)}, ${px(at.y)}, ${px(at.z)})` }}
      onClick={onPress}
      onKeyDown={(event) => keyed(event, onPress)}
      onMouseEnter={(event) => onTip(tip, event.currentTarget)}
      onMouseLeave={() => onTip(undefined)}
      onFocus={(event) => onTip(tip, event.currentTarget)}
      onBlur={() => onTip(undefined)}
    >
      <div
        className="bf-hop"
        data-fresh={fresh !== undefined || undefined}
        data-arriving={arriving !== undefined || undefined}
        style={vars({ "--fx": px(from.x - at.x), "--fy": px(from.y - at.y), "--fz": from.z - at.z, "--wait": `${hop?.delay ?? 0}ms` })}
      >
        <Sprite x={-20} y={-20} w={40} h={40}>
          <polygon className="bf-crate__box" points="0,-6.5 7.5,0 0,6.5 -7.5,0" />
          <circle className="bf-crate__dot" r={1.8} />
          <circle className="bf-hit" r={13} />
        </Sprite>
      </div>
    </div>
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
  const plane = { width: size.width, height: size.height * FLAT };
  const territory = new Map(layout.territories.map((one) => [one.root, one] as const));
  const padAt = new Map(layout.territories.flatMap((one) => one.pads.map((pad) => [pad.key, onPlane(pad)] as const)));
  const baseOf = (root: string): P3 => {
    const t = territory.get(root);
    const at = onPlane(t === undefined ? { x: size.width / 2, y: size.height } : { x: t.base.x - t.base.w / 2 + 36, y: t.base.y + 8 });
    return { ...at, z: 20 };
  };
  const heights = heightsOf(layout, drones, plays);
  const tiles = layout.territories.flatMap((one) => one.tiles);
  const floorAt = (at: Placed) => {
    const under = tiles.find((t) => at.x >= t.rect.x && at.x <= t.rect.x + t.rect.w && at.y >= t.rect.y && at.y <= t.rect.y + t.rect.h);
    return under === undefined ? 0 : (heights.get(under.path) ?? 0);
  };
  // How high it flies: clear of the tallest slab within reach of its box and of the ground lying under its neighbours, and higher while it works.
  const altOf = (drone: DroneIn) => {
    const at = layout.at.get(drone.key);
    if (at === undefined) return 0;
    const reach = tiles.filter((t) => t.rect.x < at.x + 170 && t.rect.x + t.rect.w > at.x - 170 && t.rect.y < at.y + 90 && t.rect.y + t.rect.h > at.y - 60);
    const clear = Math.max(floorAt(at), ...reach.map((t) => heights.get(t.path) ?? 0));
    return clear + FOOT + MARGIN + (drone.hover ? Math.round(18 * big) : 0);
  };
  const droneBy = new Map(drones.map((one) => [one.key, one] as const));
  const flying = (drone: DroneIn): P3 | undefined => {
    const at = layout.at.get(drone.key);
    return at === undefined ? undefined : { x: at.x, y: at.y * FLAT, z: altOf(drone) };
  };
  const sorted = [...drones].sort((a, b) => (layout.at.get(a.key)?.y ?? 0) - (layout.at.get(b.key)?.y ?? 0));
  const playOf = (key: string, kind: Play["kind"]) => plays.find((one) => one.key === key && one.kind === kind);

  // The camera: the whole field, or the locked drone closer and lower. Zoom is the camera's distance, one more step in or out.
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
      setAim({ x: lockX, y: lockY - 30 });
      setZoom((was) => Math.max(was, 1.7));
    };
    if (first.current) {
      first.current = false;
      const frame = window.requestAnimationFrame(() => window.requestAnimationFrame(go));
      return () => window.cancelAnimationFrame(frame);
    }
    go();
  }, [selected, lockX, lockY]);
  // A dispatch or a merge that happens while he watches is shown whole: the camera pulls back a little, the event plays, it returns.
  const arrival = plays.some((one) => one.id.startsWith("arrived:") || one.id.startsWith("merged:"));
  const [pulled, setPulled] = useState(false);
  useEffect(() => {
    if (!arrival) return;
    setPulled(true);
    const timer = window.setTimeout(() => setPulled(false), 3600);
    return () => window.clearTimeout(timer);
  }, [arrival]);
  // At zoom 1 the camera stands far enough back to hold the whole field; an arrival backs it off a little more.
  const eye = pulled ? 0.76 : zoom * 0.86;
  const pitch = !pulled && zoom > 1.2 ? CAM.near : CAM.wide;
  const clamp = (value: number, half: number, whole: number) => (half * 2 >= whole ? whole / 2 : Math.min(whole - half, Math.max(half, value)));
  const look = pulled || aim === undefined ? { x: size.width / 2, y: size.height * 0.6 } : aim;
  const ax = clamp(look.x, size.width / 2 / eye, plane.width);
  const ay = clamp(look.y * FLAT, plane.height / 2 / eye, plane.height);
  // The lifted lines are hidden while the pitch eases, since they are drawn for the pitch it is going to.
  const [easing, setEasing] = useState(false);
  const eased = useRef(pitch);
  useEffect(() => {
    if (eased.current === pitch) return;
    eased.current = pitch;
    setEasing(true);
    const timer = window.setTimeout(() => setEasing(false), 1200);
    return () => window.clearTimeout(timer);
  }, [pitch]);
  const lifted = liftOf(pitch);
  const curve = (a: P3, b: P3, rise: number, n = 18) => {
    const points = Array.from({ length: n + 1 }, (_, i) => lifted(alongOf(a, b, rise, i / n)));
    return `M${points.map((p) => `${p.x.toFixed(1)},${p.y.toFixed(1)}`).join(" L")}`;
  };

  const [tip, setTip] = useState<Tip>();
  const [hot, setHot] = useState<string>();
  const showTip = (next: Tip | undefined, target?: Target) => {
    if (next === undefined || target === undefined) return setTip(undefined);
    const box = target.getBoundingClientRect();
    const base = host.current?.getBoundingClientRect();
    if (base !== undefined) setTip({ ...next, left: box.x - base.x + box.width / 2, top: box.y - base.y });
  };

  const locked = selected !== undefined && lock !== undefined;
  const [hotTile, setHotTile] = useState<string>();
  // The plots: a name for those the selected drone is over and those a calling drone is over, a lift for those a working
  // drone is over, an amber slab for those two or more are changing.
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
  // Where each drone shows on the screen, near enough to keep one name off another: the plane stands out of the screen by its tilt.
  const shownAt = (drone: DroneIn) => {
    const at = layout.at.get(drone.key);
    return at === undefined ? undefined : { at, up: altOf(drone) * SIN };
  };
  const bodies = drones.flatMap((one) => {
    const there = shownAt(one);
    if (there === undefined) return [];
    const w = (one.kind === "job" ? 34 : 26) * big;
    return [{ key: one.key, l: there.at.x - w, r: there.at.x + w, t: there.at.y - there.up - 30 * big, b: there.at.y - there.up + 22 * big }];
  });
  // Labels: the selected drone's, the one under the pointer, then those that need him and those failing, each only where it
  // meets no label already drawn, so no two collide. Any other is named by its tip.
  const labels = new Set<string>();
  const taken: { l: number; r: number; t: number; b: number }[] = [];
  const ranked = drones.filter((one) => one.shown && (one.key === selected || one.key === hot || one.hue !== "calm")).sort((a, b) => rank(a, selected, hot) - rank(b, selected, hot));
  for (const one of ranked) {
    const there = shownAt(one);
    if (there === undefined) continue;
    const wide = Math.min(22, one.title.length) * 6.7 + 10;
    const y = there.at.y - there.up - 24 * big - (one.flare ? 26 : one.ping ? 14 : 4);
    const box = { l: there.at.x - wide / 2, r: there.at.x + wide / 2, t: y - 12, b: y + 5 };
    const meets = (o: { l: number; r: number; t: number; b: number }) => box.l < o.r && box.r > o.l && box.t < o.b && box.b > o.t;
    if (one.key !== selected && one.key !== hot && (bodies.some((other) => other.key !== one.key && meets(other)) || taken.some(meets))) continue;
    taken.push(box);
    labels.add(one.key);
  }
  // The slabs that rise when something touches them: a Check, a turn, a launch under a drone; and the wave a merge sends over its territory.
  const bump = new Map<string, { wait: number; amp: number }>();
  for (const play of plays) {
    if ((play.kind === "launch" || play.kind === "sweep" || play.kind === "tracer") && play.key !== undefined) {
      for (const { tile } of layout.over.get(play.key) ?? []) if (!bump.has(tile.path)) bump.set(tile.path, { wait: play.delay + (play.kind === "launch" ? 900 : 250), amp: 8 });
    }
    if (play.kind === "capture") {
      const root = play.root ?? prs.find((pr) => pr.number === play.number)?.root;
      const home = root === undefined ? undefined : territory.get(root);
      if (home?.goal === undefined) continue;
      for (const tile of home.tiles) bump.set(tile.path, { wait: Math.round(play.delay + 900 + Math.hypot(tile.cx - home.goal.x, (tile.cy - home.goal.y) * FLAT) * 1.1), amp: 6 });
    }
  }
  const landing = new Set(plays.filter((one) => one.kind === "capture" && one.number !== undefined).map((one) => one.number));
  const cam = `rotateX(${pitch}deg) rotateZ(${CAM.yaw}deg) scale3d(${eye}, ${eye}, ${eye}) translate3d(${px(-ax)}, ${px(-ay)}, 0px)`;

  return (
    <div ref={host} className="bf" data-locked={locked || undefined} style={vars({ "--pitch": `${pitch}deg`, "--yaw": `${CAM.yaw}deg`, perspective: `${Math.round(size.width * 2.8)}px` })}>
      <svg className="bf-grain" aria-hidden="true" width="100%" height="100%">
        <filter id="bf-noise" x="0" y="0" width="100%" height="100%">
          <feTurbulence type="fractalNoise" baseFrequency="0.9" numOctaves="1" seed="5" />
          <feColorMatrix values="0 0 0 0 1  0 0 0 0 1  0 0 0 0 1  0 0 0 0.5 0" />
        </filter>
        <rect width="100%" height="100%" filter="url(#bf-noise)" />
      </svg>
      <div className="bf-sway">
        <div className="bf-cam" style={{ transform: cam }}>
          <div className="bf-plane" role="listbox" aria-label="Map" style={{ width: plane.width, height: plane.height }}>
            <div className="bf-floor" style={{ left: -plane.width * 0.6, top: -plane.height * 0.9, width: plane.width * 2.2, height: plane.height * 2.1 }} />

            {/* The ground itself: each territory's front line, the border round each top directory, the bases and their pads. */}
            <svg className="bf-ground" width={plane.width} height={plane.height}>
              {layout.territories.map((one) => (
                <g key={one.root} className="bf-territory">
                  <rect className="bf-front" aria-hidden="true" x={one.rect.x} y={one.rect.y * FLAT} width={one.rect.w} height={one.rect.h * FLAT} rx={30} />
                  {one.groups.map((group) => (
                    <rect key={group.dir} className="bf-group" aria-hidden="true" x={group.rect.x} y={group.rect.y * FLAT} width={group.rect.w} height={group.rect.h * FLAT} rx={8} />
                  ))}
                </g>
              ))}
              {layout.territories.map((one) => (
                <Base key={one.root} territory={one} onTip={showTip} />
              ))}
              {layout.territories.map((one) =>
                one.pads.map((pad) => {
                  const info = pads.get(pad.key);
                  return info === undefined ? null : <Pad key={pad.key} pad={info} at={onPlane(pad)} onPress={() => onPad(pad.key)} onTip={showTip} />;
                }),
              )}
            </svg>
            {layout.territories.map((one) => (
              <Hangar key={one.root} territory={one} />
            ))}

            {/* The slabs. */}
            {layout.territories.map((one) =>
              one.tiles.map((tile) => {
                const room = Math.max(4, Math.floor((tile.rect.w - 12) / 6.3));
                const there = (heights.get(tile.path) ?? 0) * SIN;
                const box = { l: tile.rect.x + 5, r: tile.rect.x + 9 + Math.min(room, tile.path.length) * 6.4, t: tile.rect.y - there - 16, b: tile.rect.y - there };
                const meets = (o: { l: number; r: number; t: number; b: number }) => box.l < o.r && box.r > o.l && box.t < o.b && box.b > o.t;
                const named = hotTile === tile.path || (labelled.has(tile.path) && !bodies.some(meets) && !taken.some(meets));
                const clash = clashAt.get(tile.path);
                return (
                  <Slab
                    key={tile.path}
                    tile={tile}
                    height={heights.get(tile.path) ?? 0}
                    lit={lit.has(tile.path)}
                    clash={clash !== undefined}
                    named={named}
                    bump={bump.get(tile.path)}
                    room={room}
                    onEnter={(event) => {
                      setHotTile(tile.path);
                      showTip(clash === undefined ? { said: "Directory", title: tile.path, left: 0, top: 0 } : { said: "Write clash", title: `Both are changing ${tile.path}`, left: 0, top: 0 }, event.currentTarget);
                    }}
                    onLeave={() => {
                      setHotTile(undefined);
                      showTip(undefined);
                    }}
                  />
                );
              }),
            )}

            {/* Plays on the ground: the cone of a Check turning on a drone, a ring where something was said, a ring from a plot that was touched. */}
            {plays.map((play) => {
              const own = play.key === undefined ? undefined : layout.at.get(play.key);
              const drone = play.key === undefined ? undefined : droneBy.get(play.key);
              if (own === undefined || drone === undefined) return null;
              const at = onPlane(own);
              const z = floorAt(own) + 1.5;
              const place = { transform: `translate3d(${px(at.x)}, ${px(at.y)}, ${px(z)})` };
              if (play.kind === "sweep") {
                return (
                  <div key={play.id} className="bf-at" style={place} aria-hidden="true">
                    <i className="bf-cone" style={{ animationDelay: `${play.delay}ms` }} />
                  </div>
                );
              }
              if (play.kind === "blip" || (play.kind === "tracer" && (drone.parent === undefined || layout.at.get(drone.parent) === undefined))) {
                return (
                  <div key={play.id} className="bf-at" style={place} aria-hidden="true">
                    <i className="bf-ripple" style={{ animationDelay: `${play.delay}ms` }} />
                  </div>
                );
              }
              return null;
            })}
            {plays.flatMap((play) => {
              if ((play.kind !== "launch" && play.kind !== "sweep" && play.kind !== "tracer") || play.key === undefined) return [];
              const main = layout.over.get(play.key)?.[0]?.tile;
              if (main === undefined) return [];
              const at = onPlane({ x: main.cx, y: main.cy });
              return [
                <div key={`${play.id}:tile`} className="bf-at" style={{ transform: `translate3d(${px(at.x)}, ${px(at.y)}, ${px((heights.get(main.path) ?? 0) + 1.5)})` }} aria-hidden="true">
                  <i className="bf-ripple" data-tile style={{ animationDelay: `${play.delay + (play.kind === "launch" ? 1000 : 300)}ms` }} />
                </div>,
              ];
            })}

            {/* The lifted lines: what drones leave in the air, the tether to a bay it holds, the beams to the plots it changes, the formation, the clash, the routes. */}
            <svg className="bf-lift" data-easing={easing || undefined} width={plane.width} height={plane.height} style={{ transform: `translateZ(${LAYER}px)` }} aria-hidden="true">
              {drones.map((drone) => {
                const to = flying(drone);
                if (to === undefined) return null;
                const age = trails.get(drone.key);
                const pad = drone.slot === undefined ? undefined : padAt.get(drone.slot);
                const origin: P3 = pad === undefined ? baseOf(drone.root) : { ...pad, z: 2 };
                const parent = drone.parent === undefined ? undefined : droneBy.get(drone.parent);
                const above = parent === undefined ? undefined : flying(parent);
                const on = selected === drone.key || selected === drone.parent;
                const launching = playOf(drone.key, "launch");
                return (
                  <g key={drone.key} data-dim={!drone.shown || undefined}>
                    {age === undefined ? null : <path className="bf-trail" pathLength={1} d={curve(origin, to, 46)} style={{ opacity: Math.max(0.05, 0.2 * (1 - age / TRAIL_MS)), ...(launching === undefined ? {} : vars({ "--wait": `${launching.delay}ms` })) }} data-launching={launching !== undefined || undefined} />}
                    {pad === undefined ? null : <path className="bf-tether" d={curve({ ...pad, z: 2 }, to, 30)} />}
                    {above === undefined ? null : <path className="bf-formation" data-lit={on || undefined} d={curve(to, above, 26)} />}
                    {(layout.over.get(drone.key) ?? []).map(({ tile }, i) => {
                      const end = onPlane({ x: tile.cx, y: tile.cy });
                      const top: P3 = { ...end, z: heights.get(tile.path) ?? 0 };
                      const mark = lifted(top);
                      return (
                        <g key={tile.path} className="bf-touch" data-main={i === 0 || undefined} data-lit={selected === drone.key || undefined}>
                          <path d={curve(to, top, 0)} />
                          <ellipse cx={mark.x} cy={mark.y} rx={8} ry={8 * Math.cos((pitch * Math.PI) / 180)} />
                        </g>
                      );
                    })}
                  </g>
                );
              })}
              {layout.clashes.map((clash) => {
                const a = droneBy.get(clash.keys[0]!);
                const b = droneBy.get(clash.keys[1]!);
                const from = a === undefined ? undefined : flying(a);
                const to = b === undefined ? undefined : flying(b);
                if (from === undefined || to === undefined) return null;
                const tipped = { said: "Write clash", title: `Both are changing ${clash.path}`, left: 0, top: 0 };
                return (
                  <g key={clash.path} className="bf-clash">
                    <path className="bf-clash__line" d={curve(from, to, 22)} />
                    <path className="bf-clash__hit" d={curve(from, to, 22)} onMouseEnter={(event) => showTip(tipped, event.currentTarget)} onMouseLeave={() => showTip(undefined)} />
                  </g>
                );
              })}
              {plays.map((play) => {
                if (play.kind !== "tracer" || play.key === undefined) return null;
                const drone = droneBy.get(play.key);
                const from = drone === undefined ? undefined : flying(drone);
                const parent = drone?.parent === undefined ? undefined : droneBy.get(drone.parent);
                const to = parent === undefined ? undefined : flying(parent);
                if (from === undefined || to === undefined) return null;
                return <path key={play.id} className="bf-tracer" pathLength={100} d={curve(from, to, 34)} style={{ animationDelay: `${play.delay}ms` }} />;
              })}
              {prs.map((pr) => {
                const home = territory.get(pr.root);
                if (home?.goal === undefined || (pr.stage === "landed" && !landing.has(pr.number))) return null;
                const owner = pr.owner === undefined ? undefined : droneBy.get(pr.owner);
                const start = (owner === undefined ? undefined : flying(owner)) ?? { ...baseOf(pr.root), z: ROUTE };
                const end: P3 = { ...onPlane(home.goal), z: ROUTE };
                return pr.stage === "landed" ? null : <path key={pr.key} className="bf-route" data-stage={pr.stage} data-dim={(owner !== undefined && !owner.shown) || undefined} d={curve(start, end, 34)} />;
              })}
            </svg>

            {layout.territories.map((one) => {
              const goal = goals.get(one.root);
              if (goal === undefined || one.goal === undefined) return null;
              const capture = plays.find((p) => p.kind === "capture" && (p.root === one.root || (p.number !== undefined && prs.some((pr) => pr.number === p.number && pr.root === one.root))));
              return <Goal key={one.root} at={onPlane(one.goal)} goal={goal} capture={capture} onPress={() => onGoal(one.root)} onTip={showTip} />;
            })}

            {sorted.map((drone, index) => {
              const at = layout.at.get(drone.key);
              if (at === undefined) return null;
              const launch = playOf(drone.key, "launch");
              const pad = drone.slot === undefined ? undefined : padAt.get(drone.slot);
              const origin = pad === undefined ? baseOf(drone.root) : { ...pad, z: 4 };
              const floor = floorAt(at);
              const on = selected === drone.key;
              const asks = drone.hue !== "calm";
              const here = onPlane(at);
              return (
                <Drone
                  key={drone.key}
                  drone={drone}
                  at={here}
                  floor={floor}
                  alt={altOf(drone)}
                  from={launch === undefined ? undefined : { x: origin.x - here.x, y: origin.y - here.y, z: origin.z }}
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

            {/* Pull requests in the air, on the route from the drone that owns one, or from the base where none does, to the objective. */}
            {prs.map((pr) => {
              const home = territory.get(pr.root);
              if (home?.goal === undefined) return null;
              const owner = pr.owner === undefined ? undefined : droneBy.get(pr.owner);
              const start = (owner === undefined ? undefined : flying(owner)) ?? { ...baseOf(pr.root), z: ROUTE };
              const end: P3 = { ...onPlane(home.goal), z: ROUTE };
              const landed = pr.stage === "landed";
              if (landed && !landing.has(pr.number)) return null;
              const rest = alongOf(start, end, 34, landed ? ALONG.queued : ALONG[pr.stage]);
              const arrival = plays.find((p) => p.kind === "capture" && p.number === pr.number);
              const opened = plays.find((p) => p.kind === "payload" && p.number === pr.number);
              return (
                <div key={pr.key} className="bf-routing" data-dim={(owner !== undefined && !owner.shown) || undefined}>
                  <Payload at={landed ? end : rest} from={landed ? rest : start} pr={pr} arriving={landed ? arrival : undefined} fresh={landed ? undefined : opened} onPress={() => onPr(pr.key)} onTip={showTip} />
                </div>
              );
            })}
          </div>
        </div>
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
              setAim({ x: lock.x, y: lock.y - 30 });
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
