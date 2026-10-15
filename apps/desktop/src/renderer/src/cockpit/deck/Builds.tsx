// What each Session has built, on the ground under its drone: the survey and its markers while it is being scoped, then the ring that
// grows a piece at a time with the road to main as a thinner ring of arcs outside it, and the subagents that fly round it. It draws
// only what `build.ts` read off the record, and a piece is static once placed. At most `MOST_LOOPS` things loop on the whole field,
// and the workers take what the drones leave. #2301.

import { Fragment, type ReactNode } from "react";

import { Sprite, onPlane, px, vars } from "./bf-parts";
import { MOST_LOOPS } from "./doing";
import { Ring } from "./ring";
import type { Frame } from "./frame";
import type { Layout, Point } from "./layout";
import { arcSaid, arcWord, arcsOf, type BuildIn } from "./build";
import { SAID } from "./BuildCard";
import { SessionGlyph } from "./Drone";
import type { OnTip } from "./Drone";
import type { Tip } from "./tip";
import "./build.css";

/** A landed ring stands for this long, then is gone. */
const FRESH_LAND_MS = 15 * 60_000;

type Placed = { build: BuildIn; anchor: Point; /** The plot it stands over, which lifts it. */ tile: string | undefined };

/**
 * Where each ring lies, in layout units: round its drone's shadow. A Job's lies at the waypoint of the step that works its plan
 * (`stand`), and a Job with no such waypoint on the field draws none. A Session whose drone is not on the field draws none.
 */
export function placeBuilds(builds: readonly BuildIn[], layout: Layout, stand?: Point): Placed[] {
  return builds.flatMap<Placed>((build) => {
    if (build.key.startsWith("job:")) return stand === undefined ? [] : [{ build, anchor: stand, tile: undefined }];
    const shadow = layout.at.get(build.key);
    return shadow === undefined ? [] : [{ build, anchor: shadow, tile: layout.over.get(build.key)?.[0]?.tile.path }];
  });
}

/** A frame whose laser lands on the ring under the drone that is building it, and not on the plot's nearest corner. */
export const aimedAt = (frame: Frame, placed: readonly Placed[]): Frame => {
  if (placed.length === 0) return frame;
  const at = new Map(placed.map((one) => [one.build.key, one.anchor] as const));
  return { ...frame, aimPoint: (drone, tile) => at.get(drone.key) ?? frame.aimPoint(drone, tile) };
};

const pick = (onTip: OnTip, tip: Tip) => ({
  onMouseEnter: (event: { currentTarget: Element }) => onTip(tip, event.currentTarget),
  onMouseLeave: () => onTip(undefined),
  onFocus: (event: { currentTarget: Element }) => onTip(tip, event.currentTarget),
  onBlur: () => onTip(undefined),
});

/** What a piece says when it is hovered: the task it stands for, and where that task is. The card is the callout's, since a tooltip cannot open here. */
const speak = (build: BuildIn, onTip: OnTip) => (k: number, piece: ReactNode): ReactNode => {
  const task = build.tasks[k];
  if (task === undefined) return piece;
  const said = task.said ?? SAID[task.piece];
  const tip: Tip = { label: "Task", title: task.text, hue: task.piece === "solid" ? "good" : "quiet", lines: [said] };
  return (
    <g className="bf-pick" role="option" tabIndex={0} aria-selected={false} aria-label={`${task.text}, ${said}`} {...pick(onTip, tip)}>
      {piece}
    </g>
  );
};

/** What an arc of the road says when it is hovered: its name, and when or that it is only what comes next here. */
const speakArc = (build: BuildIn, onTip: OnTip) => (k: number, arc: ReactNode): ReactNode => {
  const one = arcsOf(build)[k];
  if (one === undefined) return arc;
  const said = arcSaid(one, build.now);
  const tip: Tip = { label: "Road", title: arcWord(one.id), hue: one.at === undefined ? "quiet" : "good", lines: [said] };
  return (
    <g className="bf-pick" role="option" tabIndex={0} aria-selected={false} aria-label={`${arcWord(one.id)}, ${said}`} {...pick(onTip, tip)}>
      {arc}
    </g>
  );
};

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

/** A subagent: a small drone that circles the ring while it runs and leaves when it stops. Still where the loops are spent. */
function Worker({ i, detail, loop, big, onTip }: { i: number; detail: string; loop: boolean; big: number; onTip: OnTip }) {
  const radius = (34 + 9 * i) * big;
  const tip: Tip = { label: "Subagent", title: detail, hue: "quiet" };
  return (
    <div className="bf-at bf-wk" data-loop={loop || undefined} data-rev={i % 2 === 1 || undefined} style={vars({ "--T": `${8 + i * 2}s`, "--phase": (i * 0.27).toFixed(2), "--arrive": `${i * 180}ms` })}>
      <div className="bf-at bf-wk__lap">
        <div className="bf-at bf-wk__out" style={{ transform: `translate3d(${px(radius)}, 0, ${px(26 + 6 * i)})` }}>
          <div className="bf-at bf-wk__face">
            <div className="bf-wk__body" role="option" tabIndex={0} aria-selected={false} aria-label={`Subagent, ${detail}`} {...pick(onTip, tip)}>
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

export type BuildsProps = { placed: readonly Placed[]; frame: Frame; big: number; loops: ReadonlySet<string>; onTip: OnTip };

export function Builds({ placed, frame, big, loops, onTip }: BuildsProps) {
  const { heights } = frame;
  // The workers take the loops the drones leave, so the field never loops more than `MOST_LOOPS` things at once.
  let spare = Math.max(0, MOST_LOOPS - loops.size);
  return (
    <>
      {placed.map(({ build, tile, anchor }) => {
        const sinceLand = build.landedAt === undefined ? undefined : build.now - build.landedAt;
        // A landed ring is cleared a quarter hour on; what landed is in the Activity feed.
        if (sinceLand !== undefined && sinceLand > FRESH_LAND_MS) return null;
        const flat = onPlane(anchor);
        const z = (tile === undefined ? 0 : (heights.get(tile) ?? 0)) + 1.5;
        const laid = build.phase === "road" || build.phase === "landed";
        return (
          <Fragment key={build.key}>
            <div className="bf-at" style={{ transform: `translate3d(${px(flat.x)}, ${px(flat.y)}, ${px(z)})` }}>
              {build.phase === "scope" ? <Survey big={big} /> : null}
              {build.markers.map((kind, i) => (
                <Marker key={i} kind={kind} i={i} big={big} />
              ))}
              {build.tasks.length === 0 ? null : (
                <div className="bf-built" data-landed={build.landedAt !== undefined || undefined}>
                  <Ring pieces={build.tasks} big={big} dim={build.dim} wrap={speak(build, onTip)} {...(laid ? { arcs: arcsOf(build), wrapArc: speakArc(build, onTip) } : {})} />
                </div>
              )}
            </div>
            {build.workers.map((detail, i) => {
              const loop = spare > 0;
              if (loop) spare -= 1;
              return (
                <div key={detail} className="bf-at" style={{ transform: `translate3d(${px(flat.x)}, ${px(flat.y)}, ${px(z)})` }}>
                  <Worker i={i} detail={detail.replace(/^[^:]*: /, "")} loop={loop} big={big} onTip={onTip} />
                </div>
              );
            })}
          </Fragment>
        );
      })}
    </>
  );
}
