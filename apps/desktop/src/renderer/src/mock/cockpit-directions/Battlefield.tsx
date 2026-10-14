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

import { useEffect, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent as ReactKeyboardEvent, type MouseEvent as ReactMouseEvent, type PointerEvent as ReactPointerEvent } from "react";
import { Crosshair, Megaphone, Minus, Plus, ShieldCheck, ShieldX } from "lucide-react";
import { Button, Tooltip } from "@armada/components";

import { aimAt, camera } from "./deck-model";
import { GLYPH as DOING_GLYPH, MOST_LOOPS, WORD as DOING_WORD } from "./deck-doing";
import { Callout, type Anchor, type Glyph, type Tip } from "./Callout";
import { STAGE } from "./shared";
import { Sprite, keyed, onPlane, px, vars } from "./bf-parts";
import { ALONG, CAM, FLAT, SLAB, STAGE_SAID, TRAIL_MS, alongOf, bigOf, heightsOf, layoutOf, rand, type DroneIn, type P3, type Placed, type Play, type Point, type RepoIn, type Size, type Territory, type Tile } from "./deck-field";

export type PadIn = { key: string; slot: number; bay: string; word: string; repo: string; Glyph: Glyph; holder?: string };
/** What a drone's callout says beyond the drone itself: its state as the list draws it, the call it holds, what it last did and when, its last hour. */
export type DroneFacts = { state: { Glyph: Glyph; said: string; tone: "calm" | "ask" | "bad" }; call?: string; last?: { says: string; ago: string }; ticks: readonly number[]; bay?: { slot: number; Glyph: Glyph; word: string } };
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
const COS = Math.cos((CAM.wide * Math.PI) / 180);
/** How much larger the field is than the pane, each way. */
const WORLD = 1.8;
/** The share of the pane left clear round the field at home, each side. */
const HOME_MARGIN = 0.035;
/** Below this zoom the type is too small to read: a drone's name and a plot's are dropped, and the callout names them. */
const READABLE = 0.72;
/** How close lock-on stands. */
const LOCK_EYE = 1.5;
/** How much one press of + or - zooms, and how far an arrow key pans, in screen pixels. */
const ZOOM_STEP = 1.25;
const KEY_PAN = 90;
/** A press that moves less than this is a click. */
const DRAG = 4;
type Cam = { x: number; y: number; eye: number; tilt: boolean };

/** Raised by a thing that is hovered or focused: what to say, what it hangs from, and where the pointer was if it is too large to have a middle. */
type OnTip = (tip: Tip | undefined, target?: Element, at?: { x: number; y: number }) => void;
/** The directories a drone touches, the biggest two, each shortened from the left. */
const pillsOf = (drone: DroneIn): string[] =>
  [...drone.touches]
    .sort((a, b) => b.w - a.w)
    .slice(0, 2)
    .map((one) => (one.path.length > 26 ? `…${one.path.slice(-25)}` : one.path));

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
  focused,
  dim,
  play,
  facts,
  loop,
  call,
  leaving,
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
  /** The drone a picked event is about: a ping ring at it. Every other drone is `dim`. */
  focused: boolean;
  dim: boolean;
  play: Play | undefined;
  facts: DroneFacts | undefined;
  /** Its action runs as a loop, rather than as the still form. */
  loop: boolean;
  /** It holds a call that has not been opened: the marker over it, amber while it is owed and red while its Check is failing. */
  call: "ask" | "bad" | undefined;
  /** The call was just opened or answered: the marker fades. */
  leaving: boolean;
  onSelect: () => void;
  onTip: OnTip;
}) {
  const wide = drone.kind === "job" ? 26 : 20;
  const said = `${drone.kind === "job" ? "Job" : "Session"}, ${drone.said}`;
  const show = (event: { currentTarget: Element }) =>
    onTip(
      {
        label: drone.kind === "job" ? "Job" : "Session",
        title: drone.title,
        hue: drone.hue,
        state: facts?.state ?? { Glyph: ShieldCheck, said: drone.said, tone: drone.hue },
        ...(drone.doing === undefined ? {} : { doing: { Glyph: DOING_GLYPH[drone.doing], word: DOING_WORD[drone.doing], ...(drone.via === undefined ? {} : { via: drone.via }) } }),
        pills: pillsOf(drone),
        ...(facts?.bay === undefined ? {} : { bay: facts.bay }),
        ...(facts?.last === undefined ? {} : { last: facts.last }),
        ...(facts?.call === undefined ? {} : { call: facts.call }),
        ...(facts === undefined ? {} : { ticks: facts.ticks }),
      },
      event.currentTarget,
    );
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
      data-doing={drone.doing}
      data-check={drone.doing === "run" ? drone.check : undefined}
      data-loop={(drone.doing !== undefined && loop) || undefined}
      data-ping={drone.ping || undefined}
      data-flare={drone.flare || undefined}
      data-selected={selected || undefined}
      data-muted={muted || undefined}
      data-away={away || undefined}
      data-focus={focused || undefined}
      data-dim={dim || undefined}
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
            <i className="bf-ping" data-call={call !== undefined || undefined} />
            <i className="bf-ping" data-late data-call={call !== undefined || undefined} />
          </>
        ) : null}
        {drone.stuck ? <i className="bf-stall" /> : null}
        {focused ? (
          <>
            <i className="bf-ping" data-focus />
            <i className="bf-ping" data-focus data-late />
          </>
        ) : null}
        <i className="bf-shadow" />
        <i className="bf-spot" />
        <i className="bf-rim" />
        {selected ? <i className="bf-ticks" /> : null}
      </div>
      <div className="bf-beam3" aria-hidden="true" />
      {call === undefined ? null : <i className="bf-pillar" data-tone={call} data-leaving={leaving || undefined} style={vars({ "--up": Math.max(0, alt - floor) + 40 * big })} aria-hidden="true" />}
      {/* The air: the drone on its altitude, a billboard so it stands upright. */}
      <div className="bf-alt">
        {call === undefined ? null : (
          <div className="bf-bb bf-call-at" data-tone={call} data-leaving={leaving || undefined} style={vars({ "--head": Math.round(40 * big) })}>
            <div className="bf-call-foot">
              <div
                className="bf-call"
                role="button"
                tabIndex={leaving ? -1 : 0}
                aria-label={`${drone.title} needs you`}
                onClick={onSelect}
                onKeyDown={(event) => keyed(event, onSelect)}
                onMouseEnter={show}
                onMouseLeave={() => onTip(undefined)}
                onFocus={show}
                onBlur={() => onTip(undefined)}
              >
                <i className="bf-call__stem" />
                <span className="bf-call__badge">
                  <Megaphone size={18} strokeWidth={2.2} aria-hidden />
                </span>
              </div>
            </div>
          </div>
        )}
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
            ) : hot ? (
              <g className="bf-reticle" data-flick>
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
  pulse,
  writing,
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
  /** A picked event touched it: its edge glows. */
  pulse: boolean;
  /** A drone is writing it: it stands taller, and rises with the laser while that is a loop. */
  writing: "loop" | "still" | undefined;
  bump: { wait: number; amp: number } | undefined;
  room: number;
  onEnter: (event: ReactMouseEvent) => void;
  onLeave: () => void;
}) {
  const { rect } = tile;
  const inset = Math.min(rect.w, rect.h * FLAT);
  return (
    <div
      className="bf-slab"
      data-lit={lit || undefined}
      data-clash={clash || undefined}
      data-pulse={pulse || undefined}
      data-writing={writing}
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
function Base({ territory, onTip }: { territory: Territory; onTip: OnTip }) {
  const { base } = territory;
  const centre = onPlane({ x: base.x, y: base.y + 4 });
  const width = base.w + 24;
  return (
    <g
      className="bf-base"
      onMouseEnter={(event) => onTip({ label: "Repository", title: territory.name, hue: "quiet" }, event.currentTarget, { x: event.clientX, y: event.clientY })}
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
  repo,
  capture,
  onPress,
  onTip,
}: {
  at: Point;
  goal: GoalIn;
  repo: string;
  capture: Play | undefined;
  onPress: () => void;
  onTip: OnTip;
}) {
  const said = `main in ${repo}: ${goal.red ? "red" : "green"}`;
  const tip: Tip = { label: "Main", title: `main in ${repo}`, hue: goal.red ? "bad" : "good", state: { Glyph: goal.red ? ShieldX : ShieldCheck, said: goal.red ? "Red" : "Green", tone: goal.red ? "bad" : "good" } };
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
        <text className="bf-flag-name" x={40} y={-62}>main</text>
        <rect className="bf-hit" x={-14} y={-84} width={92} height={90} />
      </Sprite>
    </div>
  );
}

function Pad({ pad, at, onPress, onTip }: { pad: PadIn; at: Point; onPress: () => void; onTip: OnTip }) {
  const tip: Tip = { label: "Bay", title: `slot-${pad.slot}`, hue: pad.bay === "held" ? "calm" : "quiet", state: { Glyph: pad.Glyph, said: pad.word, tone: "calm" }, pills: [pad.repo], ...(pad.holder === undefined ? {} : { lines: [pad.holder] }) };
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
function Payload({ at, from, pr, arriving, fresh, onPress, onTip }: { at: P3; from: P3; pr: PrIn; arriving: Play | undefined; fresh: Play | undefined; onPress: () => void; onTip: OnTip }) {
  const said = STAGE_SAID[pr.stage];
  const title = `#${pr.number}`;
  const tip: Tip = { label: "Pull request", title, hue: pr.stage === "red" ? "bad" : "calm", state: { Glyph: STAGE[pr.stage].Glyph, said: STAGE[pr.stage].said, tone: pr.stage === "red" ? "bad" : "calm" } };
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
  facts,
  calls,
  focus,
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
  facts: ReadonlyMap<string, DroneFacts>;
  /** The drones that hold a call he has not opened, and whether it is owed to him or its Check is failing. */
  calls: ReadonlyMap<string, "ask" | "bad">;
  /** The drone a picked event is about. */
  focus?: string | undefined;
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
  // The world is larger than the pane: drones and plots are laid out at their own size and he pans and zooms for the rest.
  const world = useMemo<Size>(() => ({ width: Math.round(size.width * WORLD), height: Math.round(size.height * WORLD) }), [size]);
  const layout = useMemo(() => layoutOf(repos, drones, world, big, prs), [repos, drones, world, big, prs]);
  const plane = { width: world.width, height: world.height * FLAT };
  const territory = new Map(layout.territories.map((one) => [one.root, one] as const));
  const padAt = new Map(layout.territories.flatMap((one) => one.pads.map((pad) => [pad.key, onPlane(pad)] as const)));
  const baseOf = (root: string): P3 => {
    const t = territory.get(root);
    const at = onPlane(t === undefined ? { x: world.width / 2, y: world.height } : { x: t.base.x - t.base.w / 2 + 36, y: t.base.y + 8 });
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
    // Reading, it dips toward the plot it is probing.
    return clear + FOOT + MARGIN + (drone.hover ? Math.round((drone.doing === "read" ? 6 : 18) * big) : 0);
  };
  const droneBy = new Map(drones.map((one) => [one.key, one] as const));
  const flying = (drone: DroneIn): P3 | undefined => {
    const at = layout.at.get(drone.key);
    return at === undefined ? undefined : { x: at.x, y: at.y * FLAT, z: altOf(drone) };
  };
  const sorted = [...drones].sort((a, b) => (layout.at.get(a.key)?.y ?? 0) - (layout.at.get(b.key)?.y ?? 0));
  const playOf = (key: string, kind: Play["kind"]) => plays.find((one) => one.key === key && one.kind === kind);

  // What the working drones are doing over their plots. Each aims at the biggest plot it touches; at most MOST_LOOPS of them run as a
  // loop, the picked and the focused first, and the rest show the same form still.
  const aimOf = (drone: DroneIn) => (drone.doing === undefined || !drone.shown ? undefined : layout.over.get(drone.key)?.[0]?.tile);
  const doers = drones.filter((one) => aimOf(one) !== undefined && layout.at.has(one.key));
  /** Where on its plot a drone's action lands: the point nearest the drone, kept far enough in that the scan plate stays on the plot. */
  const aimPoint = (drone: DroneIn, tile: Tile): Point => {
    const at = layout.at.get(drone.key)!;
    const clamp = (n: number, lo: number, hi: number) => (lo > hi ? (lo + hi) / 2 : Math.min(hi, Math.max(lo, n)));
    const hw = Math.min(tile.rect.w, 150) * 0.43;
    const hh = Math.min(tile.rect.h, 100 / FLAT) * 0.43;
    return { x: clamp(at.x, tile.rect.x + hw, tile.rect.x + tile.rect.w - hw), y: clamp(at.y, tile.rect.y + hh, tile.rect.y + tile.rect.h - hh) };
  };
  /** How far out a circling drone flies from the centre of its ring. */
  const orbit = 28 + 10 * big;

  // The camera: where it looks on the plane and how far it stands. The field's own framing is home; a pinch, the wheel, a drag or a
  // key moves it; lock-on and an arrival ease it, and a hand on it takes over. One state, one transform, drawn at most once a frame.
  const lock = selected === undefined ? undefined : layout.at.get(selected);
  const lockX = lock?.x;
  const lockY = lock?.y;
  /**
   * Where a screen offset from the middle of the pane lies on the plane, from the aim, in plane units at an eye of one: the camera's own
   * projection undone (the yaw, the tilt and the perspective), so a point held under the pointer stays under it however far it is from the middle.
   */
  const unproject = (sx: number, sy: number, tilt: boolean): Point => {
    const phi = ((tilt ? CAM.near : CAM.wide) * Math.PI) / 180;
    const yaw = (CAM.yaw * Math.PI) / 180;
    const depth = size.width * 2.8;
    const y1 = (sy * depth) / (depth * Math.cos(phi) + sy * Math.sin(phi));
    const x1 = (sx * (depth - y1 * Math.sin(phi))) / depth;
    return { x: x1 * Math.cos(yaw) + y1 * Math.sin(yaw), y: -x1 * Math.sin(yaw) + y1 * Math.cos(yaw) };
  };
  // Where a point of the plane, at some height, lands on the screen from a camera: the same turn, scale and perspective the CSS applies.
  const projectOf = (p: P3, c: Cam): Point => {
    const phi = ((c.tilt ? CAM.near : CAM.wide) * Math.PI) / 180;
    const yaw = (CAM.yaw * Math.PI) / 180;
    const [dx, dy, z] = [(p.x - c.x) * c.eye, (p.y - c.y) * c.eye, p.z * c.eye];
    const ux = dx * Math.cos(yaw) - dy * Math.sin(yaw);
    const uy = dx * Math.sin(yaw) + dy * Math.cos(yaw);
    const depth = size.width * 2.8;
    const scale = depth / (depth - (uy * Math.sin(phi) + z * Math.cos(phi)));
    return { x: ux * scale, y: (uy * Math.cos(phi) - z * Math.sin(phi)) * scale };
  };
  // The whole field at rest: every territory with its base and its flag, and every drone with its altitude and its name over it, inside the pane
  // with a small margin, at whatever size the pane is. Found from the projection itself, so the perspective's growth toward the viewer is allowed for.
  const home = useMemo<Cam>(() => {
    const reach = SLAB.high + FOOT + MARGIN + 18 * big;
    const marks: P3[] = [];
    for (const one of layout.territories) {
      const { rect } = one;
      for (const x of [rect.x, rect.x + rect.w]) for (const y of [rect.y, rect.y + rect.h + 40]) marks.push({ x, y: y * FLAT, z: 0 });
      if (one.goal !== undefined) marks.push({ x: one.goal.x, y: one.goal.y * FLAT, z: 78 }, { x: one.goal.x + 100, y: one.goal.y * FLAT, z: 40 });
    }
    for (const [key, at] of layout.at) {
      // A drone holding a call carries its badge over it, which is a size on the screen and not on the plane: room for it above.
      const over = calls.has(key) ? 150 : 0;
      marks.push({ x: at.x - 38 * big, y: at.y * FLAT, z: reach + 40 * big + over }, { x: at.x + 38 * big, y: at.y * FLAT, z: reach + 40 * big + over }, { x: at.x, y: at.y * FLAT, z: 0 });
    }
    if (marks.length === 0) return { x: plane.width / 2, y: plane.height / 2, eye: 0.8, tilt: false };
    let cam: Cam = { x: marks.reduce((sum, m) => sum + m.x, 0) / marks.length, y: marks.reduce((sum, m) => sum + m.y, 0) / marks.length, eye: 0.6, tilt: false };
    for (let pass = 0; pass < 8; pass += 1) {
      const shown = marks.map((m) => projectOf(m, cam));
      const [l, r] = [Math.min(...shown.map((m) => m.x)), Math.max(...shown.map((m) => m.x))];
      const [t, b] = [Math.min(...shown.map((m) => m.y)), Math.max(...shown.map((m) => m.y))];
      const eye = cam.eye * Math.min((size.width * (1 - 2 * HOME_MARGIN)) / (r - l), (size.height * (1 - 2 * HOME_MARGIN)) / (b - t));
      // What lay at the middle of the box now lies where the pane's middle is.
      const there = unproject((l + r) / 2, (t + b) / 2, false);
      cam = { ...cam, eye, x: cam.x + there.x / cam.eye, y: cam.y + there.y / cam.eye };
    }
    return cam;
  }, [layout, plane.width, plane.height, size, big, calls]);
  // Out as far as the whole field, which is where it rests; in as far as a drone is large.
  const eyes = { least: Math.min(home.eye, Math.max(0.3, Math.min(size.width / world.width, size.height / (plane.height * COS)) * 0.92)) * 0.98, most: 2.6 };
  const fit = (c: Cam): Cam => ({ ...c, eye: Math.min(eyes.most, Math.max(eyes.least, c.eye)), x: Math.min(plane.width, Math.max(0, c.x)), y: Math.min(plane.height, Math.max(0, c.y)) });
  const [held, setHeld] = useState<Cam | undefined>(camera.current);
  const [live, setLive] = useState(false);
  const [grab, setGrab] = useState(false);
  // A dispatch or a merge that happens while he watches is shown whole: the camera pulls back to it, the event plays, it returns.
  const arrival = plays.filter((one) => one.id.startsWith("arrived:") || one.id.startsWith("merged:"));
  const arriving = arrival.length > 0;
  const [pulled, setPulled] = useState(false);
  useEffect(() => {
    if (!arriving) return;
    setPulled(true);
    const timer = window.setTimeout(() => setPulled(false), 3600);
    return () => window.clearTimeout(timer);
  }, [arriving]);
  const resting = held === undefined ? home : fit(held);
  const pull = (() => {
    if (!pulled || arrival.length === 0) return undefined;
    const marks: Point[] = [];
    for (const play of arrival) {
      const own = play.key === undefined ? undefined : layout.at.get(play.key);
      const root = play.root ?? own?.root ?? prs.find((pr) => pr.number === play.number)?.root;
      if (own !== undefined) marks.push(own);
      const goal = root === undefined ? undefined : territory.get(root)?.goal;
      if (goal !== undefined) marks.push(goal);
    }
    if (marks.length === 0) return undefined;
    const l = Math.min(...marks.map((m) => m.x)) - 220;
    const r = Math.max(...marks.map((m) => m.x)) + 220;
    const t = Math.min(...marks.map((m) => m.y)) - 200;
    const b = Math.max(...marks.map((m) => m.y)) + 160;
    const fitted = Math.min(size.width / (r - l), size.height / ((b - t) * FLAT * COS)) * 0.92;
    return fit({ x: (l + r) / 2, y: ((t + b) / 2) * FLAT, eye: Math.min(resting.eye, fitted), tilt: false });
  })();
  const eff = pull ?? resting;
  const latest = useRef(eff);
  latest.current = eff;
  const frame = useRef<number>(0);
  const next = useRef<Cam>(eff);
  const quiet = useRef<number>(0);
  const pitch = eff.tilt ? CAM.near : CAM.wide;
  // Move the camera. A hand on it is `live`: no easing, it follows at once. Anything else eases, unless motion is off.
  const go = (to: Cam, by: "hand" | "ease") => {
    const target = fit(to);
    latest.current = target;
    next.current = target;
    camera.current = target;
    if (by === "hand") {
      setLive(true);
      window.clearTimeout(quiet.current);
      quiet.current = window.setTimeout(() => setLive(false), 180);
    } else {
      window.clearTimeout(quiet.current);
      setLive(false);
    }
    setPulled(false);
    if (frame.current === 0) {
      frame.current = window.requestAnimationFrame(() => {
        frame.current = 0;
        setHeld(next.current);
      });
    }
  };
  useEffect(
    () => () => {
      window.cancelAnimationFrame(frame.current);
      window.clearTimeout(quiet.current);
    },
    [],
  );
  const centred = () => {
    const box = host.current?.getBoundingClientRect();
    return box === undefined ? { x: 0, y: 0 } : { x: box.left + box.width / 2, y: box.top + box.height / 2 };
  };
  /** Zoom by a factor, keeping the point of the plane under (x, y) on the screen where it is. */
  const zoomAt = (factor: number, x: number, y: number, by: "hand" | "ease" = "hand") => {
    const c = latest.current;
    const eye = Math.min(eyes.most, Math.max(eyes.least, c.eye * factor));
    if (eye === c.eye) return;
    const mid = centred();
    const under = unproject(x - mid.x, y - mid.y, c.tilt);
    const spread = 1 / c.eye - 1 / eye;
    go({ ...c, eye, x: c.x + under.x * spread, y: c.y + under.y * spread }, by);
  };
  /** Move the camera so what lay under the middle of the pane now lies a (dx, dy) of the screen away: a drag, or a scroll. */
  const panBy = (dx: number, dy: number, by: "hand" | "ease" = "hand", from: Cam = latest.current) => {
    const there = unproject(dx, dy, from.tilt);
    go({ ...from, x: from.x + there.x / from.eye, y: from.y + there.y / from.eye }, by);
  };
  // Back to the fit: not a camera of his own, so the field refits as the pane changes, and what he left is forgotten.
  const homeAgain = () => {
    window.clearTimeout(quiet.current);
    window.cancelAnimationFrame(frame.current);
    frame.current = 0;
    camera.current = undefined;
    latest.current = home;
    next.current = home;
    setLive(false);
    setPulled(false);
    setHeld(undefined);
  };
  const toSelection = () => (lockX === undefined || lockY === undefined ? homeAgain() : go({ x: lockX, y: (lockY - 30) * FLAT, eye: Math.max(latest.current.eye, LOCK_EYE), tilt: true }, "ease"));
  // Lock-on: the camera eases to the drone picked, closer and lower. Not at the mount while the camera was left somewhere.
  const first = useRef(camera.current !== undefined);
  useEffect(() => {
    if (lockX === undefined || lockY === undefined) return;
    if (first.current) {
      first.current = false;
      return;
    }
    go({ x: lockX, y: (lockY - 30) * FLAT, eye: Math.max(latest.current.eye, LOCK_EYE), tilt: true }, "ease");
  }, [selected, lockX, lockY]);
  // `w` named a drone to go to: the camera eases to it once the map is drawn.
  useEffect(() => {
    const key = aimAt.key;
    const at = key === undefined ? undefined : layout.at.get(key);
    if (at === undefined) return;
    aimAt.key = undefined;
    first.current = false;
    go({ x: at.x, y: (at.y - 30) * FLAT, eye: Math.max(latest.current.eye, LOCK_EYE), tilt: true }, "ease");
  }, [layout]);
  // The hand: a drag, the wheel and a pinch.
  const press = useRef<{ id: number; x: number; y: number; at: Cam; moved: boolean } | null>(null);
  const swallow = useRef(false);
  useEffect(() => {
    const one = host.current;
    if (one === null) return;
    const wheel = (event: WheelEvent) => {
      if ((event.target as Element).closest?.(".bf-zoom") !== null) return;
      event.preventDefault();
      const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? 400 : 1;
      const dx = event.deltaX * unit;
      const dy = event.deltaY * unit;
      // A pinch arrives as a wheel with ctrl held; a mouse wheel as large whole steps; two fingers sliding as the small rest.
      const wheelStep = event.deltaMode !== 0 || (dx === 0 && Math.abs(dy) >= 50 && Number.isInteger(dy));
      if (event.ctrlKey) zoomAt(Math.exp(-dy * 0.012), event.clientX, event.clientY);
      else if (wheelStep) zoomAt(Math.exp(-Math.sign(dy) * Math.min(Math.abs(dy), 160) * 0.0032), event.clientX, event.clientY);
      else panBy(dx, dy);
      setTip(undefined);
    };
    let from = 1;
    const began = (event: Event) => {
      event.preventDefault();
      from = latest.current.eye;
    };
    const changed = (event: Event) => {
      event.preventDefault();
      const at = centred();
      zoomAt((from * (event as unknown as { scale: number }).scale) / latest.current.eye, at.x, at.y);
    };
    one.addEventListener("wheel", wheel, { passive: false });
    one.addEventListener("gesturestart", began);
    one.addEventListener("gesturechange", changed);
    return () => {
      one.removeEventListener("wheel", wheel);
      one.removeEventListener("gesturestart", began);
      one.removeEventListener("gesturechange", changed);
    };
  });
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

  const [tip, setTip] = useState<{ tip: Tip; anchor: Anchor }>();
  const [hot, setHot] = useState<string>();
  // A marker whose call was opened or answered stays a moment and fades.
  const [gone, setGone] = useState<ReadonlyMap<string, "ask" | "bad">>(new Map());
  const marked = useRef(calls);
  useEffect(() => {
    const left = [...marked.current].filter(([key]) => !calls.has(key));
    marked.current = calls;
    if (left.length === 0) return;
    setGone((was) => new Map([...was, ...left]));
    window.setTimeout(() => setGone((was) => new Map([...was].filter(([key]) => !left.some(([one]) => one === key)))), 320);
  }, [calls]);
  const callOf = (key: string) => calls.get(key) ?? gone.get(key);
  const loops = new Set(
    [...doers]
      .sort((a, b) => (a.key === focus ? 0 : a.key === selected ? 1 : a.key === hot ? 2 : 3) - (b.key === focus ? 0 : b.key === selected ? 1 : b.key === hot ? 2 : 3))
      .slice(0, MOST_LOOPS)
      .map((one) => one.key),
  );
  const writing = new Map<string, "loop" | "still">();
  for (const one of doers) if (one.doing === "write") writing.set(aimOf(one)!.path, loops.has(one.key) || writing.get(aimOf(one)!.path) === "loop" ? "loop" : "still");
  const showTip: OnTip = (next, target, at) => {
    if (next === undefined || target === undefined) return setTip(undefined);
    setTip({ tip: next, anchor: { target, ...(at === undefined ? {} : { at }) } });
  };
  // Esc puts the callout away; it comes back when the pointer or focus comes to something again.
  useEffect(() => {
    const away = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      setTip(undefined);
      setHot(undefined);
    };
    window.addEventListener("keydown", away, true);
    return () => window.removeEventListener("keydown", away, true);
  }, []);

  const locked = selected !== undefined && lock !== undefined;
  const [hotTile, setHotTile] = useState<string>();
  // The plots: a name for those the selected drone is over and those a calling drone is over, a lift for those a working
  // drone is over, an amber slab for those two or more are changing.
  const pulsing = new Set((focus === undefined ? [] : (layout.over.get(focus) ?? [])).map((one) => one.tile.path));
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
  // Zoomed out to the whole field the type is too small to read: a name is dropped rather than drawn unreadable, and the callout says it.
  const readable = eff.eye >= READABLE;
  const labels = new Set<string>();
  const taken: { l: number; r: number; t: number; b: number }[] = [];
  const ranked = drones.filter((one) => readable && one.shown && (one.key === selected || one.key === hot || one.hue !== "calm")).sort((a, b) => rank(a, selected, hot) - rank(b, selected, hot));
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
  // The pointer: a press that moves less than DRAG is still a click on whatever it landed on; past it, it is a drag, and nothing is picked.
  const zoomMid = (factor: number) => {
    const mid = centred();
    zoomAt(factor, mid.x, mid.y, "ease");
  };
  const onDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if ((event.pointerType === "mouse" && event.button !== 0) || (event.target as Element).closest(".bf-zoom, .bf-edge") !== null) return;
    press.current = { id: event.pointerId, x: event.clientX, y: event.clientY, at: latest.current, moved: false };
  };
  const onMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const one = press.current;
    if (one === null || one.id !== event.pointerId) return;
    const dx = event.clientX - one.x;
    const dy = event.clientY - one.y;
    if (!one.moved) {
      if (Math.hypot(dx, dy) < DRAG) return;
      one.moved = true;
      host.current?.setPointerCapture(event.pointerId);
      setGrab(true);
      setTip(undefined);
      setHot(undefined);
    }
    const mid = centred();
    const from = unproject(one.x - mid.x, one.y - mid.y, one.at.tilt);
    const to = unproject(event.clientX - mid.x, event.clientY - mid.y, one.at.tilt);
    go({ ...one.at, x: one.at.x + (from.x - to.x) / one.at.eye, y: one.at.y + (from.y - to.y) / one.at.eye }, "hand");
  };
  const onUp = (event: ReactPointerEvent<HTMLDivElement>) => {
    const one = press.current;
    if (one === null || one.id !== event.pointerId) return;
    press.current = null;
    if (!one.moved) return;
    swallow.current = true;
    window.setTimeout(() => (swallow.current = false), 0);
    host.current?.releasePointerCapture?.(event.pointerId);
    setGrab(false);
  };
  const onClick = (event: ReactMouseEvent<HTMLDivElement>) => {
    if (!swallow.current) return;
    event.stopPropagation();
    event.preventDefault();
  };
  const onDouble = (event: ReactMouseEvent<HTMLDivElement>) => {
    if ((event.target as Element).closest("[role=option], .bf-zoom, .bf-edge") === null) homeAgain();
  };
  // The keys: arrows pan, + and - zoom, 0 and Home go back, and nothing here reaches the deck's own.
  const onKeys = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    if (event.metaKey || event.ctrlKey || event.altKey) return;
    const step = KEY_PAN * (event.shiftKey ? 3 : 1);
    const pan: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
    const move = pan[event.key];
    if (move !== undefined) panBy(move[0], move[1], "ease");
    else if (event.key === "+" || event.key === "=") zoomMid(ZOOM_STEP);
    else if (event.key === "-" || event.key === "_") zoomMid(1 / ZOOM_STEP);
    else if (event.key === "0") homeAgain();
    else if (event.key === "Home") toSelection();
    else return;
    event.preventDefault();
    event.stopPropagation();
  };
  const cam = `rotateX(${pitch}deg) rotateZ(${CAM.yaw}deg) scale3d(${eff.eye}, ${eff.eye}, ${eff.eye}) translate3d(${px(-eff.x)}, ${px(-eff.y)}, 0px)`;

  return (
    <div
      ref={host}
      className="bf"
      tabIndex={-1}
      data-locked={locked || undefined}
      data-live={live || undefined}
      data-grab={grab || undefined}
      onPointerDown={onDown}
      onPointerMove={onMove}
      onPointerUp={onUp}
      onPointerCancel={onUp}
      onClickCapture={onClick}
      onDoubleClick={onDouble}
      onKeyDown={onKeys}
      style={vars({ "--pitch": `${pitch}deg`, "--yaw": `${CAM.yaw}deg`, "--eye": eff.eye.toFixed(3), perspective: `${Math.round(size.width * 2.8)}px` })}>
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
                const named = readable && (hotTile === tile.path || (labelled.has(tile.path) && !bodies.some(meets) && !taken.some(meets)));
                const clash = clashAt.get(tile.path);
                return (
                  <Slab
                    key={tile.path}
                    tile={tile}
                    height={heights.get(tile.path) ?? 0}
                    lit={lit.has(tile.path)}
                    clash={clash !== undefined}
                    named={named}
                    pulse={pulsing.has(tile.path)}
                    writing={writing.get(tile.path)}
                    bump={bump.get(tile.path)}
                    room={room}
                    onEnter={(event) => {
                      setHotTile(tile.path);
                      showTip(clash === undefined ? { label: "Directory", title: tile.path, hue: "quiet" } : { label: "Write clash", title: tile.path, hue: "ask", lines: ["Two drones are changing it"] }, event.currentTarget, { x: event.clientX, y: event.clientY });
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

            {/* What the working drones are doing, on the plot they aim at: a scan over it, the laser's impact and its sparks, the ring a drone circles. */}
            {doers.map((drone) => {
              const own = layout.at.get(drone.key)!;
              const tile = aimOf(drone)!;
              const loop = loops.has(drone.key);
              if (drone.doing === "run") {
                const centre = onPlane(own);
                return (
                  <div key={`${drone.key}:do`} className="bf-at bf-do" data-doing="run" data-check={drone.check} data-loop={loop || undefined} data-focus={focus === drone.key || undefined} style={{ transform: `translate3d(${px(centre.x)}, ${px(centre.y)}, ${px(floorAt(own) + 1.5)})`, ...vars({ "--ring": `${orbit.toFixed(1)}px` }) }} aria-hidden="true">
                    <i className="bf-do__orbit" />
                    <i className="bf-do__radar" />
                  </div>
                );
              }
              const at = onPlane(aimPoint(drone, tile));
              // A plot that is being written rises under the laser by up to the slab's rise: the impact rides on top of it.
              const z = (heights.get(tile.path) ?? 0) + (drone.doing === "write" ? 8.5 : 1.5);
              const wide = Math.min(tile.rect.w, 150) * 0.86;
              const deep = Math.min(tile.rect.h * FLAT, 100) * 0.86;
              return (
                <div key={`${drone.key}:do`} className="bf-at bf-do" data-doing={drone.doing} data-loop={loop || undefined} data-focus={focus === drone.key || undefined} style={{ transform: `translate3d(${px(at.x)}, ${px(at.y)}, ${px(z)})`, ...vars({ "--w": `${wide.toFixed(1)}px`, "--d": `${deep.toFixed(1)}px` }) }} aria-hidden="true">
                  {drone.doing === "read" ? (
                    <>
                      <i className="bf-do__grid">
                        <i className="bf-do__scan" />
                      </i>
                      <i className="bf-do__pulse" />
                    </>
                  ) : (
                    <>
                      <i className="bf-do__impact" />
                      {[0, 1, 2, 3].map((i) => (
                        <i key={i} className="bf-do__spark" style={vars({ "--i": i })} />
                      ))}
                    </>
                  )}
                </div>
              );
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
                    {(() => {
                      const tile = aimOf(drone);
                      if (tile === undefined || (drone.doing !== "read" && drone.doing !== "write")) return null;
                      const end = onPlane(aimPoint(drone, tile));
                      const from = lifted(to);
                      const hit = lifted({ ...end, z: (heights.get(tile.path) ?? 0) + (drone.doing === "write" ? 8 : 0) });
                      const loop = loops.has(drone.key);
                      if (drone.doing === "write") {
                        return (
                          <g className="bf-laser" data-loop={loop || undefined}>
                            <line className="bf-laser__glow" x1={from.x} y1={from.y} x2={hit.x} y2={hit.y} />
                            <line className="bf-laser__core" x1={from.x} y1={from.y} x2={hit.x} y2={hit.y} />
                          </g>
                        );
                      }
                      // The probe: a narrow cone from the drone to the plot, turning about the drone so its foot sweeps the plot's width.
                      const length = Math.hypot(hit.x - from.x, hit.y - from.y) || 1;
                      const across = (Math.min(tile.rect.w, 150) * 0.86) / 2;
                      const sweep = Math.min(26, (Math.atan(across / length) * 180) / Math.PI);
                      const nx = -(hit.y - from.y) / length;
                      const ny = (hit.x - from.x) / length;
                      return (
                        <g className="bf-probe" data-loop={loop || undefined} style={{ transformOrigin: `${from.x.toFixed(1)}px ${from.y.toFixed(1)}px`, ...vars({ "--sweep": `${sweep.toFixed(1)}deg` }) }}>
                          <polygon points={`${from.x.toFixed(1)},${from.y.toFixed(1)} ${(hit.x + nx * 6).toFixed(1)},${(hit.y + ny * 6).toFixed(1)} ${(hit.x - nx * 6).toFixed(1)},${(hit.y - ny * 6).toFixed(1)}`} />
                        </g>
                      );
                    })()}
                  </g>
                );
              })}
              {layout.clashes.map((clash) => {
                const a = droneBy.get(clash.keys[0]!);
                const b = droneBy.get(clash.keys[1]!);
                const from = a === undefined ? undefined : flying(a);
                const to = b === undefined ? undefined : flying(b);
                if (from === undefined || to === undefined) return null;
                const tipped: Tip = { label: "Write clash", title: clash.path, hue: "ask", lines: ["Two drones are changing it"] };
                return (
                  <g key={clash.path} className="bf-clash">
                    <path className="bf-clash__line" d={curve(from, to, 22)} />
                    <path className="bf-clash__hit" d={curve(from, to, 22)} onMouseEnter={(event) => showTip(tipped, event.currentTarget, { x: event.clientX, y: event.clientY })} onMouseLeave={() => showTip(undefined)} />
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
              return <Goal key={one.root} at={onPlane(one.goal)} goal={goal} repo={one.name} capture={capture} onPress={() => onGoal(one.root)} onTip={showTip} />;
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

            {sorted.map((drone, index) => {
              const at = layout.at.get(drone.key);
              if (at === undefined) return null;
              const launch = playOf(drone.key, "launch");
              const pad = drone.slot === undefined ? undefined : padAt.get(drone.slot);
              const origin = pad === undefined ? baseOf(drone.root) : { ...pad, z: 4 };
              const floor = floorAt(at);
              const on = selected === drone.key;
              const asks = drone.hue !== "calm";
              const centre = onPlane(at);
              // Running tools or Checks, it flies round a ring about the place it hovers: its start is on the ring, and the wrapper turns.
              const circling = drone.doing === "run" && aimOf(drone) !== undefined;
              const here = circling ? { x: centre.x + orbit, y: centre.y } : centre;
              const loop = loops.has(drone.key);
              const body = (
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
                  focused={focus === drone.key}
                  dim={focus !== undefined && focus !== drone.key}
                  play={launch}
                  facts={facts.get(drone.key)}
                  loop={loop}
                  call={callOf(drone.key)}
                  leaving={!calls.has(drone.key)}
                  onSelect={() => onSelect(drone.key)}
                  onTip={(next, target) => {
                    setHot(next === undefined ? undefined : drone.key);
                    showTip(next, target);
                  }}
                />
              );
              // Always wrapped, so a drone that starts or stops circling is the same element: the lap is a CSS turn of the wrapper, and a second
              // turn the other way about the drone keeps it upright.
              return (
                <div key={drone.key} className="bf-orbit" data-circling={(circling && loop) || undefined} style={{ transformOrigin: `${px(centre.x)} ${px(centre.y)}` }}>
                  <div className="bf-orbit__face" style={{ transformOrigin: `${px(here.x)} ${px(here.y)}` }}>
                    {body}
                  </div>
                </div>
              );
            })}

          </div>
        </div>
      </div>
      <div className="bf-haze" aria-hidden="true" />
      {/* A drone that holds a call and is outside the pane: its badge at the border, an arrow toward it. Pressing it eases the camera there. */}
      {[...new Map([...gone, ...calls])].map(([key, tone]) => {
        const drone = droneBy.get(key);
        const at = layout.at.get(key);
        if (drone === undefined || at === undefined) return null;
        const there = projectOf({ x: at.x, y: at.y * FLAT, z: altOf(drone) + 40 * big }, eff);
        const sy = there.y - 46;
        const room = { x: size.width / 2 - 26, y: size.height / 2 - 26 };
        if (Math.abs(there.x) <= room.x && Math.abs(sy) <= room.y) return null;
        const k = Math.min(room.x / (Math.abs(there.x) || 1e-6), room.y / (Math.abs(sy) || 1e-6));
        const angle = (Math.atan2(sy, there.x) * 180) / Math.PI;
        const leaving = !calls.has(key);
        const tipped: Tip = { label: drone.kind === "job" ? "Job" : "Session", title: drone.title, hue: tone, lines: ["Needs you"] };
        return (
          <button
            key={key}
            type="button"
            className="bf-edge"
            data-tone={tone}
            data-leaving={leaving || undefined}
            tabIndex={leaving ? -1 : 0}
            aria-label={`${drone.title} needs you`}
            style={{ left: size.width / 2 + there.x * k, top: size.height / 2 + sy * k }}
            onClick={() => go({ x: at.x, y: (at.y - 30) * FLAT, eye: latest.current.eye, tilt: latest.current.tilt }, "ease")}
            onMouseEnter={(event) => showTip(tipped, event.currentTarget)}
            onMouseLeave={() => showTip(undefined)}
            onFocus={(event) => showTip(tipped, event.currentTarget)}
            onBlur={() => showTip(undefined)}
          >
            <Megaphone size={16} strokeWidth={2.2} aria-hidden />
            <i className="bf-edge__arrow" style={{ transform: `rotate(${angle.toFixed(1)}deg) translateX(24px)` }} />
          </button>
        );
      })}
      {tip === undefined || host.current === null ? null : <Callout tip={tip.tip} anchor={tip.anchor} host={host.current} />}
      <div className="bf-zoom">
        <Tooltip label="Zoom in" shortcut="+" asChild>
          <Button variant="ghost" iconOnly aria-label="Zoom in" disabled={eff.eye >= eyes.most - 0.001} onClick={() => zoomMid(ZOOM_STEP)}>
            <Plus size={16} aria-hidden />
          </Button>
        </Tooltip>
        <Tooltip label="Zoom out" shortcut="-" asChild>
          <Button variant="ghost" iconOnly aria-label="Zoom out" disabled={eff.eye <= eyes.least + 0.001} onClick={() => zoomMid(1 / ZOOM_STEP)}>
            <Minus size={16} aria-hidden />
          </Button>
        </Tooltip>
        <Tooltip label={lock === undefined ? "Back to the whole field" : "Locate the selection"} shortcut="Home" asChild>
          <Button variant="ghost" iconOnly aria-label={lock === undefined ? "Back to the whole field" : "Locate the selection"} onClick={toSelection}>
            <Crosshair size={16} aria-hidden />
          </Button>
        </Tooltip>
      </div>
    </div>
  );
}
