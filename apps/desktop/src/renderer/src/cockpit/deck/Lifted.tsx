// The lines the field leaves in the air, on one layer above the slabs: the contrail of a launch, the tether to a bay a drone holds, the
// beams to the plots it changes, the formation from a Session to its Job, the laser and the probe of a drone at work, the line between
// two drones changing one area, the tracer of a turn and the routes of pull requests. Each is shifted by the height it stands for, so
// it shows where it is at the camera's pitch. Ported from proto/cockpit-flight-deck. #2301.

import { LAYER, ROUTE } from "./geometry";
import type { Frame } from "./frame";
import { onPlane, vars } from "./bf-parts";
import type { DroneIn, Layout, P3, Point, PrIn, Territory } from "./layout";
import { TRAIL_MS, type Play } from "./plays";
import { clashLines, type Tip } from "./tip";
import type { OnTip } from "./Drone";

export type Curve = (a: P3, b: P3, rise: number, n?: number) => string;

export type LiftedProps = {
  layout: Layout;
  frame: Frame;
  drones: readonly DroneIn[];
  droneBy: ReadonlyMap<string, DroneIn>;
  territory: ReadonlyMap<string, Territory>;
  padAt: ReadonlyMap<string, Point>;
  baseOf: (root: string) => P3;
  trails: ReadonlyMap<string, number>;
  plays: readonly Play[];
  prs: readonly PrIn[];
  selected: string | undefined;
  loops: ReadonlySet<string>;
  /** A mission's own route: where its crate starts. */
  crate: Point | undefined;
  plane: { width: number; height: number };
  pitch: number;
  easing: boolean;
  curve: Curve;
  lift: (p: P3) => Point;
  onTip: OnTip;
};

export function Lifted({ layout, frame, drones, droneBy, territory, padAt, baseOf, trails, plays, prs, selected, loops, crate, plane, pitch, easing, curve, lift, onTip }: LiftedProps) {
  const { flying, heights, aimOf, aimPoint } = frame;
  const playOf = (key: string, kind: Play["kind"]) => plays.find((one) => one.key === key && one.kind === kind);
  return (
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
            {age === undefined ? null : (
              <path
                className="bf-trail"
                pathLength={1}
                d={curve(origin, to, 46)}
                style={{ opacity: Math.max(0.05, 0.2 * (1 - age / TRAIL_MS)), ...(launching === undefined ? {} : vars({ "--wait": `${launching.delay}ms` })) }}
                data-launching={launching !== undefined || undefined}
              />
            )}
            {pad === undefined ? null : <path className="bf-tether" d={curve({ ...pad, z: 2 }, to, 30)} />}
            {above === undefined ? null : <path className="bf-formation" data-lit={on || undefined} d={curve(to, above, 26)} />}
            {(layout.over.get(drone.key) ?? []).map(({ tile }, i) => {
              const end = onPlane({ x: tile.cx, y: tile.cy });
              const top: P3 = { ...end, z: heights.get(tile.path) ?? 0 };
              const mark = lift(top);
              return (
                <g key={tile.path} className="bf-touch" data-for={drone.key} data-main={i === 0 || undefined} data-lit={selected === drone.key || undefined}>
                  <path d={curve(to, top, 0)} />
                  <ellipse cx={mark.x} cy={mark.y} rx={8} ry={8 * Math.cos((pitch * Math.PI) / 180)} />
                </g>
              );
            })}
            {(() => {
              const tile = aimOf(drone);
              if (tile === undefined || (drone.doing !== "read" && drone.doing !== "write")) return null;
              const end = onPlane(aimPoint(drone, tile));
              const from = lift(to);
              const hit = lift({ ...end, z: (heights.get(tile.path) ?? 0) + (drone.doing === "write" ? 8 : 0) });
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
        const b = clash.keys[1] === undefined ? undefined : droneBy.get(clash.keys[1]);
        const from = a === undefined ? undefined : flying(a);
        const to = b === undefined ? undefined : flying(b);
        if (from === undefined || to === undefined) return null;
        const tipped: Tip = { label: "Write clash", title: clash.path, hue: "ask", lines: clashLines(clash) };
        return (
          <g key={`${clash.root}:${clash.path}`} className="bf-clash">
            <path className="bf-clash__line" d={curve(from, to, 22)} />
            <path className="bf-clash__hit" d={curve(from, to, 22)} onMouseEnter={(event) => onTip(tipped, event.currentTarget, { x: event.clientX, y: event.clientY })} onMouseLeave={() => onTip(undefined)} />
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
        if (home?.goal === undefined || pr.stage === "landed") return null;
        const owner = pr.owner === undefined ? undefined : droneBy.get(pr.owner);
        const start = crate !== undefined ? { ...onPlane(crate), z: ROUTE } : ((owner === undefined ? undefined : flying(owner)) ?? { ...baseOf(pr.root), z: ROUTE });
        const end: P3 = { ...onPlane(home.goal), z: ROUTE };
        return <path key={pr.key} className="bf-route" data-stage={pr.stage} data-dim={(owner !== undefined && !owner.shown) || undefined} d={curve(start, end, 34)} />;
      })}
    </svg>
  );
}
