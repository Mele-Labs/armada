// The battlefield's plain parts: a plot of the repository as a slab, the base with its hangar and name, the flag that is main, a
// bay's pad and a pull request's crate. Each is drawn from one value the field hands it and says one thing when hovered. Ported
// from proto/cockpit-flight-deck. #2301.

import type { MouseEvent as ReactMouseEvent } from "react";
import { ShieldCheck, ShieldX } from "lucide-react";

import { Sprite, keyed, onPlane, px, vars } from "./bf-parts";
import { short } from "./geometry";
import { FLAT, rand, type Bar, type FileAct, type P3, type Point, type PrIn, type Territory, type Tile } from "./layout";
import { STAGE_SAID } from "./plays";
import type { Play } from "./plays";
import { STAGE } from "./shared";
import { padTip, type GoalIn, type PadIn, type Tip } from "./tip";
import type { OnTip } from "./Drone";

/** A plot of the repository: a slab on the ground, its top lit, two sides darker, its height set by what works over it. */
export function Slab({
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
export function Base({ territory, onTip }: { territory: Territory; onTip: OnTip }) {
  const { base } = territory;
  const centre = onPlane({ x: base.x, y: base.y + 4 });
  const width = base.w + 24;
  return (
    <g className="bf-base" onMouseEnter={(event) => onTip({ label: "Repository", title: territory.name, hue: "quiet" }, event.currentTarget, { x: event.clientX, y: event.clientY })} onMouseLeave={() => onTip(undefined)}>
      <rect className="bf-slab-base" x={centre.x - width / 2} y={centre.y - 27 * FLAT} width={width} height={54 * FLAT} rx={36} />
      <rect className="bf-slab-base" data-inner x={centre.x - width / 2 + 16} y={centre.y - 19 * FLAT} width={width - 32} height={38 * FLAT} rx={26} />
    </g>
  );
}

/** What stands on the compound: the hangar, and the repository's name. */
export function Hangar({ territory }: { territory: Territory }) {
  const { base } = territory;
  const centre = onPlane({ x: base.x, y: base.y + 4 });
  const hangar = onPlane({ x: base.x - base.w / 2 + 36, y: base.y - 2 });
  return (
    <>
      <div className="bf-at" style={{ transform: `translate3d(${px(hangar.x)}, ${px(hangar.y)}, ${px(24)})` }} aria-hidden="true">
        <Sprite x={-40} y={-50} w={80} h={70}>
          <polygon className="bf-hangar" data-face="top" points="-26,-8 0,-18 26,-8 0,2" />
          <polygon className="bf-hangar" data-face="left" points="-26,-8 0,2 0,16 -26,6" />
          <polygon className="bf-hangar" data-face="right" points="26,-8 0,2 0,16 26,6" />
          <path className="bf-mast-line" d="M18,-12 V-30" />
          <circle className="bf-mast-dot" cx={18} cy={-31} r={1.8} />
        </Sprite>
      </div>
      <div className="bf-at" style={{ transform: `translate3d(${px(centre.x)}, ${px(centre.y + 30 * FLAT)}, ${px(0)})` }} aria-hidden="true">
        <div className="bf-bb bf-name">{territory.name}</div>
      </div>
    </>
  );
}

/** What main says when it is hovered, and what a screen reader says of the flag. */
export const goalTip = (goal: GoalIn, repo: string): Tip => ({ label: "Main", title: `main in ${repo}`, hue: goal.red ? "bad" : "good", state: { Glyph: goal.red ? ShieldX : ShieldCheck, said: goal.red ? "Red" : "Green", tone: goal.red ? "bad" : "good" } });
export const goalSaid = (goal: GoalIn, repo: string) => `main in ${repo}: ${goal.red ? "red" : "green"}, objective`;

/** Main: the flag at the objective, green or red, its name beside it and the capture of a merge under it. */
export function Goal({ at, goal, repo, capture, onPress, onTip }: { at: Point; goal: GoalIn; repo: string; capture: Play | undefined; onPress: () => void; onTip: OnTip }) {
  const tip = goalTip(goal, repo);
  return (
    <div
      className="bf-goal"
      data-red={goal.red || undefined}
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={goalSaid(goal, repo)}
      style={{ transform: `translate3d(${px(at.x)}, ${px(at.y)}, ${px(1)})` }}
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
        <text className="bf-flag-name" x={40} y={-62}>
          main
        </text>
        <rect className="bf-hit" x={-14} y={-84} width={92} height={90} />
      </Sprite>
    </div>
  );
}

/** A bay's landing pad: dim where free, lit where leased, amber and pulsing where stranded, dark where closed. */
export function Pad({ pad, at, onPress, onTip }: { pad: PadIn; at: Point; onPress: () => void; onTip: OnTip }) {
  const tip = padTip(pad);
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

/** What a pull request says when it is hovered, and what a screen reader says of its crate. */
export const payloadTip = (pr: PrIn): Tip => ({ label: "Pull request", title: `#${pr.number}`, hue: pr.stage === "red" ? "bad" : "calm", state: { Glyph: STAGE[pr.stage].Glyph, said: STAGE[pr.stage].said, tone: pr.stage === "red" ? "bad" : "calm" } });
export const payloadSaid = (pr: PrIn) => `Pull request #${pr.number}, ${STAGE_SAID[pr.stage]}`;

/** A pull request: a small crate flying its route to the objective, further along the nearer it is to merging. */
export function Payload({ at, from, pr, arriving, fresh, onPress, onTip }: { at: P3; from: P3; pr: PrIn; arriving: Play | undefined; fresh: Play | undefined; onPress: () => void; onTip: OnTip }) {
  const tip = payloadTip(pr);
  const hop = arriving ?? fresh;
  return (
    <div
      className="bf-payload"
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={payloadSaid(pr)}
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

/** What is being done to a file, as a tooltip names it. */
export const FILE_SAID: Record<FileAct | "rest", string> = { write: "Writing", edit: "Editing", remove: "Removing", rest: "File" };

/**
 * A file as a small bar standing on the plot of its directory, at the plot's height. A file being made, changed or taken away is lit
 * in the colour of that act; one at rest is plain. A bar taken away fades, and the repository stops listing it.
 */
export function FileBar({ bar, height, onTip }: { bar: Bar; height: number; onTip: OnTip }) {
  const { rect } = bar;
  const said = FILE_SAID[bar.act ?? "rest"];
  return (
    <div
      className="bf-bar"
      data-act={bar.act}
      style={vars({ left: rect.x, top: rect.y * FLAT, width: rect.w, height: rect.h * FLAT, "--h": (height + 10).toFixed(2) })}
      onMouseEnter={(event) => onTip({ label: said, title: bar.path, hue: "quiet" }, event.currentTarget, { x: event.clientX, y: event.clientY })}
      onMouseLeave={() => onTip(undefined, undefined)}
    />
  );
}
