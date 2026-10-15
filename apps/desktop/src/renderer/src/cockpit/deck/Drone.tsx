// One drone on the map: a Job the large X quad and a Session the small plus quad, flying at altitude on a vertical beam from its
// shadow. State is the drone and its frame, never words: rotors turn while it works, one that is idle or done is low with its rotors
// still, a beacon pings from one that holds a call, an amber ring breathes round one that is stuck, a red flare burns on one whose Check
// fails. A drone that holds a call he has not opened carries a marker over it (a pillar of light and a badge). Ported from
// proto/cockpit-flight-deck. #2301.

import { Megaphone, ShieldCheck, Swords } from "lucide-react";
import { clashSaid, useClashesOf, useOpenClash } from "@armada/components";

import { BOX, FOOT, bracketsOf, short } from "./geometry";
import { Sprite, keyed, px, vars } from "./bf-parts";
import type { CallTone } from "./field";
import type { DroneIn, P3, Point } from "./layout";
import type { Play } from "./plays";
import { clashTip, droneTip, type DroneFacts, type Tip } from "./tip";

/** Raised by a thing that is hovered or focused: what to say, what it hangs from, and where the pointer was if it is too large to have a middle. */
export type OnTip = (tip: Tip | undefined, target?: Element, at?: { x: number; y: number }) => void;

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

export type DroneProps = {
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
  /** Its rotors turn and it bobs, rather than hanging still: a working or stuck drone, up to the most the field animates at once. */
  alive: boolean;
  /** It holds a call that has not been opened: the marker over it, amber while it is owed and red while its Check is failing. */
  call: CallTone | undefined;
  /** The call was just opened or answered: the marker fades. */
  leaving: boolean;
  onSelect: () => void;
  onTip: OnTip;
};

export function Drone({ drone, at, floor, alt, from, index, big, selected, hot, labelled, muted, away, focused, dim, play, facts, loop, alive, call, leaving, onSelect, onTip }: DroneProps) {
  const wide = drone.kind === "job" ? 26 : 20;
  const said = `${drone.kind === "job" ? "Job" : "Session"}, ${drone.said}`;
  const show = (event: { currentTarget: Element }) => onTip(droneTip(drone, facts, ShieldCheck), event.currentTarget);
  const launching = play?.kind === "launch" && from !== undefined;
  // Fleet's clashes are between parties, so a drone with no path under it carries its mark as much as one over a plot.
  const clashes = useClashesOf(drone.kind, drone.key.slice(drone.kind.length + 1));
  const openClash = useOpenClash();
  return (
    <div
      className="bf-drone"
      role="option"
      tabIndex={0}
      aria-selected={selected}
      aria-label={`${drone.title}, ${said}`}
      data-key={drone.key}
      data-label={drone.title}
      data-kind={drone.kind}
      data-hue={drone.hue}
      data-hover={drone.hover || undefined}
      data-spin={drone.spin || undefined}
      data-stuck={drone.stuck || undefined}
      data-doing={drone.doing}
      data-check={drone.doing === "run" ? drone.check : undefined}
      data-loop={(drone.doing !== undefined && loop) || undefined}
      data-alive={alive || undefined}
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
        transform: `translate3d(${px(at.x)}, ${px(at.y)}, ${px(0)})`,
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
                  <Megaphone size={18} strokeWidth={2.2} aria-hidden="true" />
                </span>
              </div>
            </div>
          </div>
        )}
        {clashes.map((clash, at) => {
          const clashed = clashSaid(clash);
          const tip = (event: { currentTarget: Element }) => onTip(clashTip(clash), event.currentTarget);
          const opens = clash.with.kind !== "main";
          return (
            <div key={`${clash.with.kind}:${clash.with.id}`} className="bf-bb bf-call-at bf-clash-at" style={vars({ "--head": Math.round(40 * big), "--n": at })}>
              <div className="bf-call-foot">
                <div
                  className="bf-call bf-clash"
                  role={opens ? "button" : "img"}
                  tabIndex={opens ? 0 : -1}
                  aria-label={clashed}
                  data-opens={opens || undefined}
                  onClick={(event) => {
                    event.stopPropagation();
                    if (opens) openClash(clash.with);
                  }}
                  onKeyDown={(event) => opens && keyed(event, () => openClash(clash.with))}
                  onMouseEnter={tip}
                  onMouseLeave={() => onTip(undefined)}
                  onFocus={tip}
                  onBlur={() => onTip(undefined)}
                >
                  <span className="bf-call__badge">
                    <Swords size={14} strokeWidth={2.2} aria-hidden="true" />
                  </span>
                </div>
              </div>
            </div>
          );
        })}
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
