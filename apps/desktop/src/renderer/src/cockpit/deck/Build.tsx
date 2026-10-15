// What each Session has built, on the plot it writes most: the survey and its markers, then the structure that grows a piece at a time,
// the subagents that fly round it, and the road from it to the repository's main. The structure is the drone's work made visible: it
// draws only what `progress.ts` read off the record, and a piece is static once placed. At most `MOST_LOOPS` things loop on the whole
// field, and the workers take what the drones leave. #2301.

import { Fragment } from "react";

import { Sprite, keyed, onPlane, px, vars } from "./bf-parts";
import { MOST_LOOPS } from "./doing";
import { Gantry, Ring, Tower } from "./forms";
import type { Frame } from "./frame";
import { FLAT, type Layout, type Point, type Territory } from "./layout";
import { STOPS, STOP_GLYPH, STOP_WORD, agoOf, anchorOf, type After, type Built, type Stop, type Take } from "./progress";
import { SessionGlyph } from "./Drone";
import type { OnTip } from "./Drone";
import type { Tip } from "./tip";
import "./progress.css";

const DAY = 24 * 60 * 60_000;
/** A landed structure stands at full size for this long, then is small. */
const FRESH_LAND_MS = 15 * 60_000;

type Placed = { built: Built; tile: Territory["tiles"][number]; anchor: Point; shadow: Point | undefined };

/** Where each structure stands, in layout units. A drone's own shadow is the ring's centre; the other forms stand beside it. */
export function placeBuilds(builds: readonly Built[], take: Take, layout: Layout, territory: ReadonlyMap<string, Territory>, big: number): Placed[] {
  return builds.flatMap((built) => {
    const over = layout.over.get(built.key)?.[0]?.tile.path ?? built.tile;
    const tile = territory.get(built.root)?.tiles.find((one) => one.path === over);
    if (tile === undefined) return [];
    const shadow = layout.at.get(built.key);
    return [{ built, tile, shadow, anchor: anchorOf(take, tile.rect, shadow, built.slot, big) }];
  });
}

/** A frame whose laser lands on the structure and not on the plot's nearest corner, for the drones that are building one. */
export const aimedAt = (frame: Frame, placed: readonly Placed[]): Frame => {
  if (placed.length === 0) return frame;
  const at = new Map(placed.map((one) => [one.built.key, one.anchor] as const));
  return { ...frame, aimPoint: (drone, tile) => at.get(drone.key) ?? frame.aimPoint(drone, tile) };
};

const FORM = { tower: Tower, pad: Gantry, ring: Ring } as const;

function Survey({ big }: { big: number }) {
  const w = 96 * big;
  const d = 62 * big;
  return (
    <div className="bf-surv" style={vars({ "--w": px(w), "--d": px(d) })} aria-hidden="true">
      {[
        [-1, -1],
        [1, -1],
        [-1, 1],
        [1, 1],
      ].map(([sx, sy]) => (
        <i key={`${sx}${sy}`} className="bf-surv__stake" style={{ left: (sx! * w) / 2 - 2, top: (sy! * d) / 2 - 2 }} />
      ))}
    </div>
  );
}

/** A walk shown or a question asked: a small stake planted on the survey's edge. */
function Marker({ kind, i, big }: { kind: "walk" | "asked"; i: number; big: number }) {
  const along = -40 * big + i * 15;
  return (
    <div className="bf-at" style={{ transform: `translate3d(${px(along)}, ${px(34 * big)}, ${px(0)})` }} aria-hidden="true">
      <Sprite x={-8} y={-22} w={22} h={26}>
        <path className="bf-mark__pole" d="M0,0 V-16" />
        {kind === "walk" ? <path className="bf-mark__flag" d="M0,-16 H9 V-10 H0 Z" /> : <circle className="bf-mark__flag" cx={4} cy={-14} r={3.6} />}
      </Sprite>
    </div>
  );
}

/** A subagent: a small drone that circles the structure while it runs and leaves when it stops. Still where the loops are spent. */
function Worker({ i, detail, loop, big, onTip }: { i: number; detail: string; loop: boolean; big: number; onTip: OnTip }) {
  const radius = (34 + 9 * i) * big;
  const tip: Tip = { label: "Subagent", title: detail, hue: "quiet" };
  const show = (event: { currentTarget: Element }) => onTip(tip, event.currentTarget);
  return (
    <div className="bf-at bf-wk" data-loop={loop || undefined} data-rev={i % 2 === 1 || undefined} style={vars({ "--T": `${8 + i * 2}s`, "--phase": (i * 0.27).toFixed(2), "--arrive": `${i * 180}ms` })}>
      <div className="bf-at bf-wk__lap">
        <div className="bf-at bf-wk__out" style={{ transform: `translate3d(${px(radius)}, 0, ${px(26 + 6 * i)})` }}>
          <div className="bf-at bf-wk__face">
            <div className="bf-wk__body" role="option" tabIndex={0} aria-selected={false} aria-label={`Subagent, ${detail}`} onMouseEnter={show} onMouseLeave={() => onTip(undefined)} onFocus={show} onBlur={() => onTip(undefined)}>
              <Sprite x={-16} y={-14} w={32} h={24}>
                <g transform={`scale(${0.5 * big})`}>
                  <SessionGlyph />
                </g>
                <rect className="bf-hit" x={-14} y={-12} width={28} height={20} />
              </Sprite>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/** One stop on the road: lit once it happened, with when; faint where it is only what this repository's sessions usually do next. */
function StopNode({ stop, at, z, onTip, now }: { stop: Stop; at: Point; z: number; onTip: OnTip; now: number }) {
  const lit = stop.at !== undefined;
  const Glyph = STOP_GLYPH[stop.id];
  const tip: Tip = { label: "Road", title: STOP_WORD[stop.id], hue: lit ? "good" : "quiet", lines: [lit ? agoOf(now - stop.at!) : "Usually next on this repo"] };
  const centre = onPlane(at);
  return (
    <div
      className="bf-stop"
      data-lit={lit || undefined}
      role="option"
      tabIndex={0}
      aria-selected={false}
      aria-label={`${STOP_WORD[stop.id]}, ${lit ? agoOf(now - stop.at!) : "not yet"}`}
      style={{ transform: `translate3d(${px(centre.x)}, ${px(centre.y)}, ${px(z + 2)})` }}
      onMouseEnter={(event) => onTip(tip, event.currentTarget)}
      onMouseLeave={() => onTip(undefined)}
      onFocus={(event) => onTip(tip, event.currentTarget)}
      onBlur={() => onTip(undefined)}
      onKeyDown={(event) => keyed(event, () => undefined)}
    >
      <Sprite x={-14} y={-26} w={28} h={28}>
        <circle className="bf-stop__ring" cx={0} cy={-12} r={11} />
        <Glyph className="bf-stop__glyph" x={-7} y={-19} size={14} strokeWidth={2} aria-hidden="true" />
        <rect className="bf-hit" x={-13} y={-25} width={26} height={26} />
      </Sprite>
    </div>
  );
}

/** The road: dashes laid on whatever they cross, lit up to the last stop reached, and a stop at each fifth. */
function Road({ from, to, built, floorAt, onTip }: { from: Point; to: Point; built: Built; floorAt: (at: Point) => number; onTip: OnTip }) {
  const length = Math.hypot(to.x - from.x, to.y - from.y);
  const steps = Math.max(8, Math.round(length / 15));
  const angle = (Math.atan2((to.y - from.y) * FLAT, to.x - from.x) * 180) / Math.PI;
  const along = (t: number): Point => ({ x: from.x + (to.x - from.x) * t, y: from.y + (to.y - from.y) * t });
  // Segment i ends at stop i; the last runs into the flag, and is lit once the pull request has landed.
  const lit = (t: number) => {
    const n = Math.min(STOPS.length, Math.ceil(t * 5 - 1e-6));
    return n >= STOPS.length ? built.landedAt !== undefined : built.stops[n - 1]?.at !== undefined;
  };
  return (
    <>
      {Array.from({ length: steps }, (_, i) => {
        const t = (i + 0.5) / steps;
        const p = onPlane(along(t));
        return <i key={i} className="bf-rd" data-lit={lit(t) || undefined} style={{ transform: `translate3d(${px(p.x)}, ${px(p.y)}, ${px(floorAt(along(t)) + 1.2)}) rotateZ(${angle.toFixed(1)}deg)` }} aria-hidden="true" />;
      })}
      {built.stops.map((stop, k) => {
        const p = along((k + 1) / 5);
        return <StopNode key={stop.id} stop={stop} at={p} z={floorAt(p)} onTip={onTip} now={built.now} />;
      })}
    </>
  );
}

export type BuildsProps = {
  placed: readonly Placed[];
  take: Take;
  after: After;
  frame: Frame;
  territory: ReadonlyMap<string, Territory>;
  big: number;
  loops: ReadonlySet<string>;
  onTip: OnTip;
};

export function Builds({ placed, take, after, frame, territory, big, loops, onTip }: BuildsProps) {
  const { heights, floorAt } = frame;
  // The workers take the loops the drones leave, so the field never loops more than `MOST_LOOPS` things at once.
  let spare = Math.max(0, MOST_LOOPS - loops.size);
  const Form = FORM[take];
  return (
    <>
      {placed.map(({ built, tile, anchor, shadow }) => {
        const sinceLand = built.landedAt === undefined ? undefined : built.now - built.landedAt;
        if (sinceLand !== undefined && (after === "clear" ? sinceLand > FRESH_LAND_MS : after === "stand" && sinceLand >= DAY)) return null;
        const small = sinceLand !== undefined && sinceLand > FRESH_LAND_MS;
        const flat = onPlane(anchor);
        const z = (heights.get(tile.path) ?? 0) + 1.5;
        const goal = territory.get(built.root)?.goal;
        const to = goal === undefined ? { x: 0, y: 0 } : { x: goal.x - anchor.x, y: (goal.y - anchor.y) * 0.94 };
        const surveying = built.phase === "scope" || built.phase === "plan";
        const roaded = (built.phase === "road" || built.phase === "landed") && goal !== undefined && (sinceLand === undefined || sinceLand <= FRESH_LAND_MS);
        const writing = loops.has(built.key) && built.doing === "write" && !built.stalled;
        const orbit = take === "ring" && shadow !== undefined ? onPlane(shadow) : flat;
        return (
          <Fragment key={built.key}>
            <div className="bf-at" style={{ transform: `translate3d(${px(flat.x)}, ${px(flat.y)}, ${px(z)})` }} aria-hidden="true">
              {surveying ? <Survey big={big} /> : null}
              {built.markers.map((kind, i) => (
                <Marker key={i} kind={kind} i={i} big={big} />
              ))}
              {built.tasks.length === 0 ? null : (
                <div className="bf-built" data-take={take} data-after={after} data-landed={built.landedAt !== undefined || undefined} data-small={small || undefined} data-aged={(small && after === "stand") || undefined} style={vars({ "--age": Math.min(1, (sinceLand ?? 0) / DAY).toFixed(3) })}>
                  <Form pieces={built.tasks} big={big} dim={built.dim} landed={built.landedAt !== undefined && !small} working={writing} fly={{ x: to.x, y: to.y }} />
                </div>
              )}
            </div>
            {built.workers.map((detail, i) => {
              const loop = spare > 0;
              if (loop) spare -= 1;
              return (
                <div key={detail} className="bf-at" style={{ transform: `translate3d(${px(orbit.x)}, ${px(orbit.y)}, ${px(z)})` }}>
                  <Worker i={i} detail={detail.replace(/^[^:]*: /, "")} loop={loop} big={big} onTip={onTip} />
                </div>
              );
            })}
            {roaded && goal !== undefined ? <Road from={anchor} to={{ x: goal.x, y: goal.y + 10 }} built={built} floorAt={floorAt} onTip={onTip} /> : null}
          </Fragment>
        );
      })}
    </>
  );
}
